use std::{sync::Arc, time::Duration};

use anyhow::Result;
use async_shutdown::ShutdownManager;
use clap::Parser;
use tokio::task::JoinHandle;

use crate::{
    domain::{
        hash_tasks::{self, ports::HashTaskService},
        webhook_tasks::{self, ports::WebhookTaskService},
    },
    inbound::http::{HttpConfig, HttpServer},
    outbound::{schedule_worker::ScheduleWorker, sqlite::Sqlite},
};

pub mod domain;
pub mod inbound;
pub mod outbound;

#[derive(Debug, Clone, Parser)]
pub struct AppConfig {
    #[clap(long, env = "DATABASE_URL")]
    pub database_url: String,
    #[clap(long, env = "HTTP_PORT", default_value = "8080")]
    pub http_port: u16,
    #[clap(long, default_value = "5s", value_parser = humantime::parse_duration)]
    pub shutdown_timeout: Duration,
    /// The time between checking for new hash tasks to processs
    #[clap(long, default_value = "5s", value_parser = humantime::parse_duration)]
    pub hash_dispatch_period: Duration,
    /// The time between checking for new webhook tasks to dispatch
    #[clap(long, default_value = "5s", value_parser = humantime::parse_duration)]
    pub webhook_dispatch_period: Duration,

    #[clap(subcommand)]
    pub subcommand: Option<AppCmd>,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum AppCmd {
    PrintDeadline {
        #[clap(value_parser = humantime::parse_duration)]
        duration: Duration,
    },
}

#[derive(derive_more::Debug, Clone)]
pub struct AppContext {
    #[debug("Shutdown")]
    shutdown: ShutdownManager<()>,
    config: AppConfig,
}

impl AppContext {
    pub fn new(config: AppConfig, shutdown: ShutdownManager<()>) -> Self {
        Self { shutdown, config }
    }
}

pub async fn spawn_tasks(cx: Arc<AppContext>) -> Result<()> {
    let sqlite = Sqlite::new(&cx.config.database_url).await?;
    let hash_service = hash_tasks::service::Service::new(sqlite.clone());
    let webhook_service = webhook_tasks::service::Service::new(sqlite);
    let _http_handle =
        spawn_http_server(cx.clone(), hash_service.clone(), webhook_service.clone()).await?;
    let _worker_handle = spawn_schedule_worker(cx, hash_service, webhook_service).await?;
    Ok(())
}

pub async fn spawn_http_server(
    cx: Arc<AppContext>,
    hash_task_service: impl HashTaskService,
    webhook_task_service: impl WebhookTaskService,
) -> Result<JoinHandle<Result<()>>> {
    let http_config = HttpConfig::from(&cx.config);
    let server = HttpServer::new(&http_config, hash_task_service, webhook_task_service).await?;

    let future = server.run();
    // If the HTTP server ever stops, trigger the shutdown
    let future = cx.shutdown.wrap_trigger_shutdown((), future);
    let task_handle = tokio::spawn(future);

    Ok(task_handle)
}

pub async fn spawn_schedule_worker(
    cx: Arc<AppContext>,
    hash_task_service: impl HashTaskService,
    webhook_task_service: impl WebhookTaskService,
) -> Result<ScheduleWorker> {
    let handle = ScheduleWorker::spawn(cx, hash_task_service, webhook_task_service)?;
    Ok(handle)
}
