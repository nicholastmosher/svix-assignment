use anyhow::Result;

use crate::domain::webhook_tasks::{
    model::{CreateWebhookTaskRequest, WebhookTask, WebhookTaskId},
    ports::{WebhookTaskRepository, WebhookTaskService},
};

#[derive(Debug, Clone)]
pub struct Service<R>
where
    R: WebhookTaskRepository,
{
    repo: R,
}

impl<R> Service<R>
where
    R: WebhookTaskRepository,
{
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R> WebhookTaskService for Service<R>
where
    R: WebhookTaskRepository,
{
    async fn create_webhook_task(&self, req: &CreateWebhookTaskRequest) -> Result<WebhookTask> {
        let webhook_task = self.repo.create_webhook_task(req).await?;
        Ok(webhook_task)
    }

    async fn get_ready_webhook_tasks(&self, count: u32) -> Result<Vec<WebhookTask>> {
        let upcoming_tasks = self.repo.get_ready_webhook_tasks(count).await?;
        Ok(upcoming_tasks)
    }

    async fn finish_webhook_task(&self, id: &WebhookTaskId) -> Result<()> {
        self.repo.finish_webhook_task(id).await?;
        Ok(())
    }
}
