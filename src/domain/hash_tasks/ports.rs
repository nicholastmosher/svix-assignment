use std::{future::Future, pin::Pin};

use anyhow::Result;

use crate::domain::hash_tasks::model::{CreateHashTaskRequest, HashTask};

/// Domain behavior for services working with HashTasks.
pub trait HashTaskService: 'static + Clone + Send + Sync {
    fn create_hash_task(
        &self,
        req: &CreateHashTaskRequest,
    ) -> impl Future<Output = Result<HashTask>> + Send;

    fn get_ready_hash_tasks(
        &self,
        limit: u32,
    ) -> impl Future<Output = Result<Vec<HashTask>>> + Send;
}

pub trait DynHashTaskService: 'static + Send + Sync {
    fn create_hash_task(
        &self,
        req: &CreateHashTaskRequest,
    ) -> Pin<Box<dyn Future<Output = Result<HashTask>> + Send>>;

    fn get_ready_hash_tasks(
        &self,
        limit: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<HashTask>>> + Send>>;
}

impl<T: HashTaskService> DynHashTaskService for T {
    fn create_hash_task(
        &self,
        req: &CreateHashTaskRequest,
    ) -> Pin<Box<dyn Future<Output = Result<HashTask>> + Send>> {
        let this = self.clone();
        let req = req.clone();
        Box::pin(async move {
            let hash_task = this.create_hash_task(&req).await?;
            Ok(hash_task)
        })
    }

    fn get_ready_hash_tasks(
        &self,
        limit: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<HashTask>>> + Send>> {
        let this = self.clone();
        Box::pin(async move {
            let hash_tasks = this.get_ready_hash_tasks(limit).await?;
            Ok(hash_tasks)
        })
    }
}

/// Storage behavior for HashTasks.
///
/// For this demo this will be 1:1 with the service, but in a real system
/// more functionality could be added under the service.
pub trait HashTaskRepository: 'static + Clone + Send + Sync {
    fn create_hash_task(
        &self,
        req: &CreateHashTaskRequest,
    ) -> impl Future<Output = Result<HashTask>> + Send;

    fn get_ready_hash_tasks(
        &self,
        count: u32,
    ) -> impl Future<Output = Result<Vec<HashTask>>> + Send;
}
