use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{Executor, Transaction};

use crate::{
    domain::hash_tasks::{
        model::{HashTask, HashTaskDeadline, HashTaskId, HashTaskSecret},
        ports::HashTaskRepository,
    },
    outbound::sqlite::Sqlite,
};

impl Sqlite {
    async fn save_hash_task(
        &self,
        tx: &mut Transaction<'_, sqlx::Sqlite>,
        deadline: &HashTaskDeadline,
        secret: &HashTaskSecret,
    ) -> Result<HashTaskId, sqlx::Error> {
        let id = HashTaskId::generate();
        let id_string = &id.to_string();
        let deadline = &deadline.to_string();
        let secret = &secret.to_string();

        let query = sqlx::query!(
            "INSERT INTO hashtasks (id, deadline, secret) VALUES ($1, $2, $3)",
            id_string,
            deadline,
            secret,
        );
        tx.execute(query).await?;
        Ok(id)
    }

    async fn get_ready_hash_tasks(
        &self,
        pool: &sqlx::SqlitePool,
        limit: u32,
    ) -> Result<Vec<HashTaskDto>> {
        let query = sqlx::query_as!(
            HashTaskDto,
            "SELECT id, deadline, secret \
            FROM hashtasks \
            WHERE executed_at IS NULL AND deadline <= datetime('now') \
            ORDER BY deadline ASC \
            LIMIT $1",
            limit,
        );
        let rows = query.fetch_all(pool).await?;
        Ok(rows)
    }
}

impl HashTaskRepository for Sqlite {
    async fn create_hash_task(
        &self,
        req: &crate::domain::hash_tasks::model::CreateHashTaskRequest,
    ) -> Result<HashTask> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to start HashTask transaction")?;

        let id = self
            .save_hash_task(&mut tx, &req.deadline, &req.secret)
            .await?;

        tx.commit()
            .await
            .context("failed to commit Sqlite transaction for HashTask")?;

        let hash_task = HashTask::new(id, req.deadline.clone(), req.secret.clone());
        Ok(hash_task)
    }

    async fn get_ready_hash_tasks(&self, count: u32) -> Result<Vec<HashTask>> {
        let dtos = self.get_ready_hash_tasks(&self.pool, count).await?;
        let hash_tasks = dtos
            .into_iter()
            .map(|dto| HashTask::try_from(dto))
            .collect::<Result<Vec<HashTask>>>()?;
        Ok(hash_tasks)
    }
}

struct HashTaskDto {
    //
    id: String,
    deadline: String,
    secret: String,
}

impl TryFrom<HashTaskDto> for HashTask {
    type Error = anyhow::Error;

    fn try_from(dto: HashTaskDto) -> Result<Self> {
        let deadline = dto.deadline.parse::<chrono::DateTime<Utc>>()?;
        Ok(HashTask {
            id: HashTaskId::from_str(&dto.id).context("invalid HashTask id from Db")?,
            deadline: HashTaskDeadline::from(deadline),
            secret: HashTaskSecret::from(dto.secret),
            executed_at: None,
        })
    }
}
