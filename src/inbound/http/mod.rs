use std::sync::Arc;

use anyhow::{Context as _, Result};
use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    AppConfig,
    domain::{hash_tasks::ports::HashTaskService, webhook_tasks::ports::WebhookTaskService},
    inbound::http::{hash_tasks::create_hash_task, webhook_tasks::create_webhook_task},
};

pub mod hash_tasks;
pub mod webhook_tasks;

pub struct HttpConfig {
    pub port: u16,
}

impl From<&AppConfig> for HttpConfig {
    fn from(config: &AppConfig) -> Self {
        HttpConfig {
            port: config.http_port,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState<H, W>
where
    H: HashTaskService,
    W: WebhookTaskService,
{
    hash_service: Arc<H>,
    webhook_service: Arc<W>,
}

pub struct HttpServer {
    router: Router,
    listener: tokio::net::TcpListener,
}

impl HttpServer {
    pub async fn new(
        //
        config: &HttpConfig,
        hash_service: impl HashTaskService,
        webhook_service: impl WebhookTaskService,
    ) -> Result<Self> {
        let state = AppState {
            //
            hash_service: Arc::new(hash_service),
            webhook_service: Arc::new(webhook_service),
        };

        let router = Router::new()
            //
            .route("/health", get(async || "Alive"))
            .nest("/api", api_routes())
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(
            //
            format!("0.0.0.0:{}", config.port),
        )
        .await
        .with_context(|| format!("failed to listen on port {}", config.port))?;

        Ok(Self { router, listener })
    }

    pub async fn run(self) -> Result<()> {
        axum::serve(self.listener, self.router)
            .await
            .context("received error from running http server")?;
        Ok(())
    }
}

fn api_routes<HS, WS>() -> Router<AppState<HS, WS>>
where
    HS: HashTaskService,
    WS: WebhookTaskService,
{
    Router::new()
        .route("/webhook_tasks", post(create_webhook_task::<HS, WS>))
        .route("/hash_tasks", post(create_hash_task::<HS, WS>))
}
