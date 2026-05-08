use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{Executor, Transaction};
use tracing::{debug, info};
use url::Url;

use crate::{
    domain::webhook_tasks::{
        model::{
            CreateWebhookTaskRequest, WebhookTask, WebhookTaskBody, WebhookTaskDeadline,
            WebhookTaskId, WebhookTaskUrl,
        },
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

    async fn get_ready_webhook_tasks(
        &self,
        pool: &sqlx::SqlitePool,
        limit: u32,
    ) -> Result<Vec<WebhookTaskDto>> {
        debug!(limit, "Querying Ready Webhook Tasks");
        let query = sqlx::query_as!(
            WebhookTaskDto,
            "SELECT id, deadline, url, body \
            FROM webhooks \
            WHERE executed_at IS NULL AND deadline <= datetime('now') \
            ORDER BY deadline ASC \
            LIMIT $1",
            limit,
        );
        let rows = query.fetch_all(pool).await?;
        debug!(
            limit,
            count = rows.len(),
            "Finished Querying Ready Webhook Tasks"
        );
        Ok(rows)
    }

    async fn finish_webhook_task(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        id: &WebhookTaskId,
    ) -> Result<()> {
        let id = &id.to_string();
        let query = sqlx::query!(
            "UPDATE webhooks SET executed_at = datetime('now') WHERE id = $1",
            id
        );
        tx.execute(query).await?;
        Ok(())
    }
}

impl WebhookTaskRepository for Sqlite {
    async fn create_webhook_task(&self, req: &CreateWebhookTaskRequest) -> Result<WebhookTask> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start create_webhook_task transaction")?;

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

    async fn get_ready_webhook_tasks(&self, count: u32) -> Result<Vec<WebhookTask>> {
        let dtos = self.get_ready_webhook_tasks(&self.pool, count).await?;
        let webhook_tasks = dtos
            .into_iter()
            .map(|dto| WebhookTask::try_from(dto))
            .collect::<Result<Vec<WebhookTask>>>()?;
        debug!("Found {} ready webhook tasks", webhook_tasks.len());
        Ok(webhook_tasks)
    }

    async fn finish_webhook_task(&self, id: &WebhookTaskId) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start finish_webhook_task transaction")?;

        self.finish_webhook_task(&mut tx, id).await?;

        tx.commit()
            .await
            .context("failed to commit finish_webhook_task transaction")?;
        Ok(())
    }
}

struct WebhookTaskDto {
    id: String,
    deadline: String,
    url: String,
    body: String,
}

impl TryFrom<WebhookTaskDto> for WebhookTask {
    type Error = anyhow::Error;

    fn try_from(dto: WebhookTaskDto) -> Result<Self> {
        let deadline = dto.deadline.parse::<chrono::DateTime<Utc>>()?;
        let url = dto.url.parse::<Url>()?;
        Ok(WebhookTask {
            id: WebhookTaskId::from_str(&dto.id).context("invalid WebhookTask id from Db")?,
            deadline: WebhookTaskDeadline::from(deadline),
            url: WebhookTaskUrl::from(url),
            body: WebhookTaskBody::from(dto.body),
            executed_at: None,
        })
    }
}
