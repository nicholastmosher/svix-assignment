use std::time::Duration;

use async_shutdown::ShutdownManager;
use clap::Parser;

mod domain;
mod inbound;
mod outbound;

pub type Shutdown = ShutdownManager<()>;

#[derive(Debug, Clone, Parser)]
pub struct Config {
    #[clap(long, env = "DATABASE_URL")]
    pub database_url: String,
    #[clap(long, default_value = "5s", value_parser = humantime::parse_duration)]
    pub shutdown_timeout: Duration,
}

pub struct AppContext {
    shutdown: Shutdown,
    config: Config,
}

impl AppContext {
    pub fn new(config: Config, shutdown: Shutdown) -> Self {
        Self { shutdown, config }
    }
}

pub async fn spawn_tasks(cx: AppContext) {
    //
}
