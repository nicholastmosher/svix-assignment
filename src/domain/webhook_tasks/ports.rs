use std::pin::Pin;

use anyhow::Result;

use crate::domain::webhook_tasks::model::{CreateWebhookTaskRequest, WebhookTask, WebhookTaskId};

/// Domain behavior for services working with WebhookTasks.
pub trait WebhookTaskService: 'static + Clone + Send + Sync {
    fn create_webhook_task(
        &self,
        req: &CreateWebhookTaskRequest,
    ) -> impl Future<Output = Result<WebhookTask>> + Send;

    fn get_ready_webhook_tasks(
        &self,
        count: u32,
    ) -> impl Future<Output = Result<Vec<WebhookTask>>> + Send;

    fn finish_webhook_task(&self, id: &WebhookTaskId) -> impl Future<Output = Result<()>> + Send;
}

pub trait DynWebhookTaskService: 'static + Send + Sync {
    fn create_webhook_task(
        &self,
        req: &CreateWebhookTaskRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WebhookTask>> + Send>>;

    fn get_ready_webhook_tasks(
        &self,
        count: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<WebhookTask>>> + Send>>;

    fn finish_webhook_task(
        &self,
        id: &WebhookTaskId,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
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

    fn get_ready_webhook_tasks(
        &self,
        count: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<WebhookTask>>> + Send>> {
        let this = self.clone();
        Box::pin(async move {
            let webhook_tasks = this.get_ready_webhook_tasks(count).await?;
            Ok(webhook_tasks)
        })
    }

    fn finish_webhook_task(
        &self,
        id: &WebhookTaskId,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
        let this = self.clone();
        let id = id.clone();
        Box::pin(async move {
            this.finish_webhook_task(&id).await?;
            Ok(())
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

    fn get_ready_webhook_tasks(
        &self,
        count: u32,
    ) -> impl Future<Output = Result<Vec<WebhookTask>>> + Send;

    fn finish_webhook_task(&self, id: &WebhookTaskId) -> impl Future<Output = Result<()>> + Send;
}
