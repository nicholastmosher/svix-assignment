//! The ScheduleWorker runs on a stream of [`ScheduleWorkerInput`] events,
//! dispatching tasks as necessary

use std::{collections::HashMap, ops::ControlFlow, sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use futures::{FutureExt, Stream, StreamExt as _};
use tokio_stream::wrappers::IntervalStream;
use tracing::{debug, error, info};

use crate::{
    AppContext,
    domain::{
        hash_tasks::{
            model::HashTask,
            ports::{DynHashTaskService, HashTaskService},
        },
        webhook_tasks::{
            model::{WebhookTask, WebhookTaskId},
            ports::{DynWebhookTaskService, WebhookTaskService},
        },
    },
};

/// External handle-API to spawn and interact with the ScheduleWorker
pub struct ScheduleWorker {
    worker_handle: tokio::task::JoinHandle<()>,
}

impl ScheduleWorker {
    pub fn spawn(
        context: Arc<AppContext>,
        hash_task_service: impl HashTaskService,
        webhook_task_service: impl WebhookTaskService,
    ) -> Result<Self> {
        let shutdown = context.shutdown.clone();
        let schedule_worker =
            ScheduleWorkerState::new(context, hash_task_service, webhook_task_service)?;

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
    /// Emitted when it's time to check for new ready hash tasks to process
    DispatchHash,
    /// Emitted when it's time to check for new ready webhooks to dispatch
    DispatchWebhook,
}

/// Inner state-machine's state, may be edited by handlers processing input events
pub struct ScheduleWorkerState {
    context: Arc<AppContext>,
    client: reqwest::Client,
    hash_task_service: Arc<dyn DynHashTaskService>,
    webhook_task_service: Arc<dyn DynWebhookTaskService>,
    inflight_tasks: HashMap<WebhookTaskId, WebhookTask>,
}

impl ScheduleWorkerState {
    pub fn new(
        context: Arc<AppContext>,
        hash_task_service: impl HashTaskService,
        webhook_task_service: impl WebhookTaskService,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()?;

        Ok(Self {
            context,
            client,
            hash_task_service: Arc::new(hash_task_service),
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
        let hash_dispatch_interval =
            tokio::time::interval(self.context.config.hash_dispatch_period);
        let hash_dispatch_stream =
            IntervalStream::new(hash_dispatch_interval).map(|_| ScheduleWorkerInput::DispatchHash);
        let webhook_dispatch_interval =
            tokio::time::interval(self.context.config.webhook_dispatch_period);
        let webhook_dispatch_stream = IntervalStream::new(webhook_dispatch_interval)
            .map(|_| ScheduleWorkerInput::DispatchWebhook);

        use futures_concurrency::prelude::*;
        (
            shutdown_stream,
            hash_dispatch_stream,
            webhook_dispatch_stream,
        )
            .merge()
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
            ScheduleWorkerInput::DispatchHash => {
                self.try_handle_dispatch_hash().await?;
            }
            ScheduleWorkerInput::DispatchWebhook => {
                info!("DispatchWebhook tick");
                self.try_handle_dispatch_webhook().await?;
            }
        }

        Ok(ControlFlow::Continue(()))
    }

    async fn try_handle_dispatch_hash(&mut self) -> Result<()> {
        let upcoming_tasks = self
            .hash_task_service
            .get_ready_hash_tasks(10)
            .await
            .context("failed to fetch upcoming tasks from service")?;

        for task in upcoming_tasks {
            let future = dispatch_hash_task(task);
            // Prevent shutdown until all tasks complete
            let future = self
                .context
                .shutdown
                .wrap_delay_shutdown(future)
                .context("refusing to dispatch new hash tasks, shutdown in progress")?;
            let _handle = tokio::spawn(future);
        }

        Ok(())
    }

    async fn try_handle_dispatch_webhook(&mut self) -> Result<()> {
        let ready_tasks = self
            .webhook_task_service
            .get_ready_webhook_tasks(10)
            .await
            .context("failed to fetch upcoming tasks from service")?;

        debug!(?ready_tasks, "Handling webhook tasks");
        for task in ready_tasks {
            let future =
                dispatch_webhook_task(self.client.clone(), self.webhook_task_service.clone(), task);
            // Prevent shutdown until all tasks complete
            let future = self
                .context
                .shutdown
                .wrap_delay_shutdown(future)
                .context("refusing to dispatch new webhook tasks, shutdown in progress")?;
            let _handle = tokio::spawn(future);
        }

        Ok(())
    }
}

pub async fn dispatch_hash_task(task: HashTask) -> Result<()> {
    //
    Ok(())
}

pub async fn try_dispatch_hash_task() -> Result<()> {
    Ok(())
}

/// Top-level of a new task, dispatches one webhook task
pub async fn dispatch_webhook_task(
    client: reqwest::Client,
    service: Arc<dyn DynWebhookTaskService>,
    task: WebhookTask,
) {
    let id = task.id.clone();
    let result = try_dispatch_webhook_task(client, service, task).await;

    // On error, log and quit the task.
    // On the next dispatch cycle, this task will be retried.
    if let Err(error) = result {
        tracing::error!("failed to dispatch webhook task: {}", error);
    } else {
        info!(?id, "Dispatched webhook task");
    }
}

pub async fn try_dispatch_webhook_task(
    client: reqwest::Client,
    service: Arc<dyn DynWebhookTaskService>,
    task: WebhookTask,
) -> Result<()> {
    let response = client
        .post(task.url().url().clone())
        .body(task.body.into_body())
        .send()
        .await?;

    if response.status().is_success() {
        service.finish_webhook_task(&task.id).await?;
    }

    Ok(())
}
