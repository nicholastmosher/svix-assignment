use anyhow::{Context, Result};
use sqlx::{Executor, Transaction};

use crate::{
    domain::webhook_tasks::{
        model::{WebhookTask, WebhookTaskBody, WebhookTaskDeadline, WebhookTaskId, WebhookTaskUrl},
        ports::WebhookTaskRepository,
    },
    outbound::sqlite::Sqlite,
};

impl Sqlite {
    async fn save_webhook_task(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        deadline: &WebhookTaskDeadline,
        url: &WebhookTaskUrl,
        body: &WebhookTaskBody,
    ) -> Result<WebhookTaskId, sqlx::Error> {
        let id = WebhookTaskId::generate();
        let id_string = &id.to_string();
        let deadline = &deadline.to_string();
        let url = &url.to_string();
        let body = &body.to_string();

        let query = sqlx::query!(
            "INSERT INTO webhooks (id, deadline, url, body) VALUES ($1, $2, $3, $4)",
            id_string,
            deadline,
            url,
            body,
        );
        tx.execute(query).await?;
        Ok(id)
    }
}

impl WebhookTaskRepository for Sqlite {
    async fn create_webhook_task(
        &self,
        req: &crate::domain::webhook_tasks::model::CreateWebhookTaskRequest,
    ) -> Result<WebhookTask> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start WebhookTask transaction")?;

        let id = self
            .save_webhook_task(&mut tx, &req.deadline, &req.url, &req.body)
            .await?;

        tx.commit()
            .await
            .context("failed to commit Sqlite transaction for WebhookTask")?;

        let webhook_task =
            WebhookTask::new(id, req.deadline.clone(), req.url.clone(), req.body.clone());
        Ok(webhook_task)
    }
}
