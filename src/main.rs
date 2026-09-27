use clap::Parser;
use cynpase_bot::{create_router, AppState, HubConfig, PipelineEngine, VisionManager};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cynpase_bot=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Arc::new(HubConfig::parse());
    let pipeline = PipelineEngine::new(config.clone());
    let (telemetry_tx, _) = broadcast::channel(100);
    let (device_cmd_tx, _) = broadcast::channel(100);
    let vision = VisionManager::new();

    let state = AppState {
        config: config.clone(),
        pipeline,
        session: Arc::new(Mutex::new(None)),
        telemetry_tx,
        device_cmd_tx,
        vision,
    };

    let router = create_router(state);
    let listener = TcpListener::bind(&config.bind_addr).await?;
    info!(bind_addr = %config.bind_addr, "cynpase-bot Local AI Hub listening");

    axum::serve(listener, router).await?;

    Ok(())
}
