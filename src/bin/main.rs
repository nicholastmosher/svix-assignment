use std::time::Duration;

use async_shutdown::ShutdownManager;
use clap::Parser;
use svix_takehome::{AppContext, Config, Shutdown, spawn_tasks};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::parse();
    let shutdown: ShutdownManager<()> = Shutdown::new();
    let context = AppContext::new(config.clone(), shutdown.clone());
    spawn_tasks(context).await;

    // Wait for shutdown to be triggered
    tokio::select! {
        // Probably internal error, move to graceful shutdown
        _ = shutdown.wait_shutdown_triggered() => {
            info!("Shutdown triggered by internal error");
        }
        // User cancellation, trigger shutdown so services clean up
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown triggered by user cancellation (^C)");
            shutdown.trigger_shutdown(()).ok();
        }
    }

    // Wait for shutdown to be completed, or timeout/force-close
    tokio::select! {
        _ = shutdown.wait_shutdown_complete() => {
            info!("Graceful shutdown complete");
        }
        _ = tokio::time::sleep(config.shutdown_timeout) => {
            error!("Graceful shutdown timed out, force-quitting");
            std::process::exit(1);
        }
    }

    Ok(())
}
