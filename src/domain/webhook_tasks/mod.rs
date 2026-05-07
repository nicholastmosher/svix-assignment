use anyhow::Result;

use crate::domain::webhook_tasks::{
    model::WebhookTask,
    ports::{WebhookTaskRepository, WebhookTaskService},
};

pub mod model;
pub mod ports;
pub mod service;

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
    async fn create_webhook_task(
        &self,
        req: &model::CreateWebhookTaskRequest,
    ) -> Result<WebhookTask> {
        let webhook_task = self.repo.create_webhook_task(req).await?;
        Ok(webhook_task)
    }

    async fn get_upcoming_webhook_tasks(&self, count: u32) -> Result<Vec<WebhookTask>> {
        let upcoming_tasks = self.repo.get_upcoming_webhook_tasks(count).await?;
        Ok(upcoming_tasks)
    }
}
