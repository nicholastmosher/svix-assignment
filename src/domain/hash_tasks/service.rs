use anyhow::Result;

use crate::domain::hash_tasks::{
    model::{CreateHashTaskRequest, HashTask},
    ports::{HashTaskRepository, HashTaskService},
};

#[derive(Debug, Clone)]
pub struct Service<R>
where
    R: HashTaskRepository,
{
    repo: R,
}

impl<R> Service<R>
where
    R: HashTaskRepository,
{
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

impl<R> HashTaskService for Service<R>
where
    R: HashTaskRepository,
{
    async fn create_hash_task(&self, req: &CreateHashTaskRequest) -> Result<HashTask> {
        let hash_task = self.repo.create_hash_task(req).await?;
        Ok(hash_task)
    }

    async fn get_upcoming_hash_tasks(&self, count: u32) -> Result<Vec<HashTask>> {
        let upcoming_tasks = self.repo.get_upcoming_hash_tasks(count).await?;
        Ok(upcoming_tasks)
    }
}
