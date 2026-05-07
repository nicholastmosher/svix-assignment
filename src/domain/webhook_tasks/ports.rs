use anyhow::Result;

use crate::domain::webhook_tasks::model::{CreateWebhookTaskRequest, WebhookTask};

/// Domain behavior for services working with WebhookTasks.
pub trait WebhookTaskService: 'static + Clone + Send + Sync {
    fn create_webhook_task(
        &self,
        req: &CreateWebhookTaskRequest,
    ) -> impl Future<Output = Result<WebhookTask>> + Send;
}

/// Storage behavior for WebhookTasks.
///
/// For this demo this will be 1:1 with the service, but in a real system
/// more functionality could be added under the service.
pub trait WebhookTaskRepository {
    fn create_webhook_task(
        &self,
        req: &CreateWebhookTaskRequest,
    ) -> impl Future<Output = Result<WebhookTask>> + Send;
}
