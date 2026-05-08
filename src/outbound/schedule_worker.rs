//! The ScheduleWorker runs on a stream of input events, dispatching tasks as necessary

use std::ops::ControlFlow;

use anyhow::Result;
use futures::{Stream, StreamExt as _, stream::empty};

use crate::domain::webhook_tasks::ports::{DynWebhookTaskService, WebhookTaskService};

/// External handle-API to spawn and interact with the ScheduleWorker
pub struct ScheduleWorker {
    //
}

impl ScheduleWorker {
    pub fn spawn(webhook_task_service: impl WebhookTaskService) -> Self {
        let schedule_worker = ScheduleWorkerState::new(webhook_task_service);

        Self {}
    }
}

/// Input events for the ScheduleWorker, processed by the state machine
pub enum ScheduleWorkerInput {
    Shutdown,
    Tick,
}

/// Inner state-machine's state, may be edited by handlers processing input events
pub struct ScheduleWorkerState {
    //
    webhook_task_service: Box<dyn DynWebhookTaskService>,
}

impl ScheduleWorkerState {
    pub fn new(webhook_task_service: impl WebhookTaskService) -> Self {
        Self {
            webhook_task_service: Box::new(webhook_task_service),
        }
    }

    fn create_input_stream(&self) -> impl Stream<Item = ScheduleWorkerInput> + use<> {
        empty()
    }

    /// Top-level run loop, responsible for error handling
    pub async fn run(mut self) {
        let mut input_stream = self.create_input_stream();
        loop {
            let result = self.try_run(&mut input_stream).await;

            // Naive error handling: Just log errors and continue
            if let Err(error) = result {
                tracing::error!(?error, "ScheduleWorker error");
            }
        }
    }

    /// Inner run loop, happy-path stays here and consumes events from input stream
    pub async fn try_run(
        &mut self,
        input: &mut (impl Unpin + Stream<Item = ScheduleWorkerInput>),
    ) -> Result<()> {
        while let Some(event) = input.next().await {
            // Errors bubble up eagerly to be handled by upper loop
            let flow = self.try_handle_event(event).await?;
            if let ControlFlow::Break(_) = flow {
                // Graceful shutdown path
                return Ok(());
            }
        }
        Ok(())
    }

    pub async fn try_handle_event(
        &mut self,
        event: ScheduleWorkerInput,
    ) -> Result<ControlFlow<()>> {
        match event {
            ScheduleWorkerInput::Shutdown => {
                // Graceful shutdown path
                return Ok(ControlFlow::Break(()));
            }
            ScheduleWorkerInput::Tick => {
                //
            }
        }

        Ok(ControlFlow::Continue(()))
    }
}
