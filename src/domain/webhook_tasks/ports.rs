use std::pin::Pin;

use anyhow::Result;

use crate::domain::webhook_tasks::model::{CreateWebhookTaskRequest, WebhookTask};

/// Domain behavior for services working with WebhookTasks.
pub trait WebhookTaskService: 'static + Clone + Send + Sync {
    fn create_webhook_task(
        &self,
        req: &CreateWebhookTaskRequest,
    ) -> impl Future<Output = Result<WebhookTask>> + Send;

    fn get_upcoming_webhook_tasks(
        &self,
        count: u32,
    ) -> impl Future<Output = Result<Vec<WebhookTask>>> + Send;
}

pub trait DynWebhookTaskService: 'static + Send + Sync {
    fn create_webhook_task(
        &self,
        req: &CreateWebhookTaskRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WebhookTask>> + Send>>;

    fn get_upcoming_webhook_tasks(
        &self,
        count: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<WebhookTask>>> + Send>>;
}

impl<T: WebhookTaskService> DynWebhookTaskService for T {
    fn create_webhook_task(
        &self,
        req: &CreateWebhookTaskRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WebhookTask>> + Send>> {
        let this = self.clone();
        let req = req.clone();
        Box::pin(async move {
            let webhook_task = this.create_webhook_task(&req).await?;
            Ok(webhook_task)
        })
    }

    fn get_upcoming_webhook_tasks(
        &self,
        count: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<WebhookTask>>> + Send>> {
        let this = self.clone();
        Box::pin(async move {
            let webhook_tasks = this.get_upcoming_webhook_tasks(count).await?;
            Ok(webhook_tasks)
        })
    }
}

/// Storage behavior for WebhookTasks.
///
/// For this demo this will be 1:1 with the service, but in a real system
/// more functionality could be added under the service.
pub trait WebhookTaskRepository: 'static + Clone + Send + Sync {
    fn create_webhook_task(
        &self,
        req: &CreateWebhookTaskRequest,
    ) -> impl Future<Output = Result<WebhookTask>> + Send;

    fn get_upcoming_webhook_tasks(
        &self,
        count: u32,
    ) -> impl Future<Output = Result<Vec<WebhookTask>>> + Send;
}
