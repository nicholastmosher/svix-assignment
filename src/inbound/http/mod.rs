use std::sync::Arc;

use anyhow::{Context as _, Result};
use axum::{Router, routing::post};

use crate::{
    AppConfig, domain::webhook_tasks::ports::WebhookTaskService,
    inbound::http::webhook_tasks::create_webhook_task,
};

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
pub struct AppState<S: WebhookTaskService> {
    webhook_service: Arc<S>,
}

pub struct HttpServer {
    router: Router,
    listener: tokio::net::TcpListener,
}

impl HttpServer {
    pub async fn new(
        //
        config: &HttpConfig,
        webhook_service: impl WebhookTaskService,
    ) -> Result<Self> {
        let state = AppState {
            //
            webhook_service: Arc::new(webhook_service),
        };

        let router = Router::new()
            //
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

fn api_routes<WS: WebhookTaskService>() -> Router<AppState<WS>> {
    Router::new().route("/webhook_tasks", post(create_webhook_task::<WS>))
}
