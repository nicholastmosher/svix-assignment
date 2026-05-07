use std::time::Duration;

use anyhow::Result;
use async_shutdown::ShutdownManager;
use clap::Parser;
use tokio::task::JoinHandle;

use crate::{
    domain::webhook_tasks::{Service, ports::WebhookTaskService},
    inbound::http::{HttpConfig, HttpServer},
    outbound::sqlite::Sqlite,
};

pub mod domain;
pub mod inbound;
pub mod outbound;

#[derive(Debug, Clone, Parser)]
pub struct AppConfig {
    #[clap(long, env = "DATABASE_URL")]
    pub database_url: String,
    #[clap(long, env = "HTTP_PORT")]
    pub http_port: u16,
    #[clap(long, default_value = "5s", value_parser = humantime::parse_duration)]
    pub shutdown_timeout: Duration,
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

pub async fn spawn_tasks(cx: AppContext) -> Result<()> {
    let sqlite = Sqlite::new(&cx.config.database_url).await?;
    let webhook_service = Service::new(sqlite);
    let _http_handle = spawn_http_server(cx, webhook_service).await?;
    Ok(())
}

pub async fn spawn_http_server(
    cx: AppContext,
    webhook_service: impl WebhookTaskService,
) -> Result<JoinHandle<Result<()>>> {
    let http_config = HttpConfig::from(&cx.config);
    let server = HttpServer::new(&http_config, webhook_service).await?;

    let future = server.run();
    // If the HTTP server ever stops, trigger the shutdown
    let future = cx.shutdown.wrap_trigger_shutdown((), future);
    let task_handle = tokio::spawn(future);

    Ok(task_handle)
}
