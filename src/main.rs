use clap::Parser;
use cynpase_bot::api::{create_api_router, AppState};
use cynpase_bot::bus::MqttBus;
use cynpase_bot::config::AppConfig;
use cynpase_bot::vision::{VisionDetector, VisionTracker};
use cynpase_bot::voice::VoiceOrchestrator;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about = "StackChan Local Mind: Offline perception and voice orchestrator")]
struct Args {
    #[arg(short, long, default_value = "hub/config.yaml")]
    config: String,

    #[arg(short, long, default_value_t = 8088)]
    port: u16,

    #[arg(long, default_value = "0.0.0.0")]
    host: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cynpase_bot=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();
    let config = AppConfig::load_from_file(&args.config);

    tracing::info!("Starting StackChan Local Mind Orchestrator...");

    let detector = VisionDetector::new("yolo11s", 0.5);
    let tracker = Mutex::new(VisionTracker::new());

    let persona_prompt = std::fs::read_to_string(&config.llm.system_prompt_path)
        .unwrap_or_else(|_| "You are StackChan, an observant robot desktop companion. Keep spoken answers under 2 sentences.".to_string());

    let voice = VoiceOrchestrator::new(
        &config.stt.url,
        &config.llm.host,
        &config.llm.model,
        &persona_prompt,
        &config.tts.url,
        &config.tts.voice,
    );

    // Optional MQTT client
    let bus = {
        let (bus_client, mut eventloop) = MqttBus::new("127.0.0.1", 1883, "localmind-orchestrator");
        tokio::spawn(async move {
            while let Ok(_event) = eventloop.poll().await {
                // Background event handling
            }
        });
        Some(bus_client)
    };

    let app_state = Arc::new(AppState {
        start_time: Instant::now(),
        detector,
        tracker,
        voice,
        bus,
        frame_counter: AtomicU64::new(0),
    });

    let app = create_api_router(app_state);
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;

    tracing::info!("Local Mind API listening on http://{}", addr);
    tracing::info!("Endpoints available: /health, /stats, /say, /api/vision/frame");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
