use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{Executor, Transaction};
use url::Url;

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

    async fn get_upcoming_webhook_tasks(
        &self,
        pool: &sqlx::SqlitePool,
        count: u32,
    ) -> Result<Vec<WebhookTaskDto>> {
        let query = sqlx::query_as!(
            WebhookTaskDto,
            "SELECT id, deadline, url, body FROM webhooks WHERE executed_at IS NULL ORDER BY deadline ASC LIMIT $1",
            count,
        );
        let rows = query.fetch_all(pool).await?;
        Ok(rows)
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

    async fn get_upcoming_webhook_tasks(&self, count: u32) -> Result<Vec<WebhookTask>> {
        let dtos = self.get_upcoming_webhook_tasks(&self.pool, count).await?;
        let webhook_tasks = dtos
            .into_iter()
            .map(|dto| WebhookTask::try_from(dto))
            .collect::<Result<Vec<WebhookTask>>>()?;
        Ok(webhook_tasks)
    }
}

struct WebhookTaskDto {
    //
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
