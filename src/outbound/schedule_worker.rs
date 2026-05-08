//! The ScheduleWorker runs on a stream of [`ScheduleWorkerInput`] events,
//! dispatching tasks as necessary

use std::{collections::HashMap, ops::ControlFlow, sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use futures::{FutureExt, Stream, StreamExt as _};
use tokio_stream::wrappers::IntervalStream;
use tracing::{error, info};

use crate::{
    AppContext,
    domain::webhook_tasks::{
        model::{WebhookTask, WebhookTaskId},
        ports::{DynWebhookTaskService, WebhookTaskService},
    },
};

/// External handle-API to spawn and interact with the ScheduleWorker
pub struct ScheduleWorker {
    worker_handle: tokio::task::JoinHandle<()>,
}

impl ScheduleWorker {
    pub fn spawn(
        context: Arc<AppContext>,
        webhook_task_service: impl WebhookTaskService,
    ) -> Result<Self> {
        let shutdown = context.shutdown.clone();
        let schedule_worker = ScheduleWorkerState::new(context, webhook_task_service)?;

        let future = schedule_worker.run();
        // If the schedule worker quits for any reason, trigger the whole system to shut down
        let future = shutdown.wrap_trigger_shutdown((), future);
        // Delay completing shutdown until the schedule worker has finished running
        let future = shutdown
            .wrap_delay_shutdown(future)
            .context("Cannot launch ScheduleWorker, system is already shutting down")?;
        let worker_handle = tokio::spawn(future);

        Ok(Self { worker_handle })
    }
}

/// Input events for the ScheduleWorker, processed by the state machine
pub enum ScheduleWorkerInput {
    /// Emitted when the schedule worker is being shut down
    Shutdown,
    /// Emitted when it's time to check for new ready webhooks to dispatch
    DispatchWebhook,
}

/// Inner state-machine's state, may be edited by handlers processing input events
pub struct ScheduleWorkerState {
    context: Arc<AppContext>,
    client: reqwest::Client,
    webhook_task_service: Arc<dyn DynWebhookTaskService>,
    inflight_tasks: HashMap<WebhookTaskId, WebhookTask>,
}

impl ScheduleWorkerState {
    pub fn new(
        context: Arc<AppContext>,
        webhook_task_service: impl WebhookTaskService,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()?;

        Ok(Self {
            context,
            client,
            webhook_task_service: Arc::new(webhook_task_service),
            inflight_tasks: Default::default(),
        })
    }

    /// Constructs a stream of input events from the given context
    fn create_input_stream(&self) -> impl Stream<Item = ScheduleWorkerInput> + use<> {
        let shutdown_stream = self
            .context
            .shutdown
            .wait_shutdown_triggered()
            .into_stream()
            .map(|_| ScheduleWorkerInput::Shutdown);
        let interval = tokio::time::interval(self.context.config.dispatch_period);
        let dispatch_stream =
            IntervalStream::new(interval).map(|_| ScheduleWorkerInput::DispatchWebhook);

        use futures_concurrency::prelude::*;
        (shutdown_stream, dispatch_stream).merge()
    }

    /// Top-level run loop, responsible for error handling
    pub async fn run(mut self) {
        // If the task quits and this token gets dropped, the system shuts down
        let _shutdown_guard = self.context.shutdown.trigger_shutdown_token(());

        let mut input_stream = self.create_input_stream();
        loop {
            let result = self.try_run(&mut input_stream).await;
            match result {
                Ok(ControlFlow::Continue(())) => {}
                Ok(ControlFlow::Break(())) => {
                    info!("ScheduleWorker gracefully shutting down");
                    return;
                }
                Err(error) => {
                    // Naive error handling: Just log errors and continue
                    tracing::error!(?error, "ScheduleWorker error");
                }
            }
        }
    }

    /// Inner run loop, happy-path stays here and consumes events from input stream
    pub async fn try_run(
        &mut self,
        input: &mut (impl Unpin + Stream<Item = ScheduleWorkerInput>),
    ) -> Result<ControlFlow<()>> {
        while let Some(event) = input.next().await {
            // Errors bubble up eagerly to be handled by upper loop
            let flow = self.try_handle_event(event).await?;
            if let ControlFlow::Break(_) = flow {
                // Graceful shutdown path
                return Ok(ControlFlow::Break(()));
            }
        }

        error!("Unexpected end of input stream, moving to shutdown");
        Ok(ControlFlow::Break(()))
    }

    /// Handle one event at a time, bubbles up errors and uses ControlFlow::Break to gracefully shutdown
    async fn try_handle_event(&mut self, event: ScheduleWorkerInput) -> Result<ControlFlow<()>> {
        match event {
            ScheduleWorkerInput::Shutdown => {
                // Graceful shutdown path
                return Ok(ControlFlow::Break(()));
            }
            ScheduleWorkerInput::DispatchWebhook => {
                self.try_handle_dispatch().await?;
            }
        }

        Ok(ControlFlow::Continue(()))
    }

    async fn try_handle_dispatch(&mut self) -> Result<()> {
        let upcoming_tasks = self
            .webhook_task_service
            .get_upcoming_webhook_tasks(10)
            .await
            .context("failed to fetch upcoming tasks from service")?;

        for task in upcoming_tasks {
            let future = dispatch_task(self.client.clone(), task);
            // Prevent shutdown until all tasks complete
            let future = self
                .context
                .shutdown
                .wrap_delay_shutdown(future)
                .context("refusing to dispatch new tasks, shutdown in progress")?;
            let _handle = tokio::spawn(future);
        }

        Ok(())
    }
}

/// Top-level of a new task, dispatches one batch of tasks
pub async fn dispatch_task(client: reqwest::Client, task: WebhookTask) {
    let result = try_dispatch_task(client, task).await;
}

pub async fn try_dispatch_task(client: reqwest::Client, task: WebhookTask) -> Result<()> {
    let response = client
        .post(task.url().url().clone())
        .body(task.body.into_body())
        .send()
        .await?;

    Ok(())
}
