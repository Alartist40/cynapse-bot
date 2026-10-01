use clap::Parser;
use cynapse_bot::api::{create_api_router, AppState};
use cynapse_bot::bus::{HubStatus, MqttBus};
use cynapse_bot::config::AppConfig;
use cynapse_bot::vision::{VisionDetector, VisionTracker};
use cynapse_bot::voice::VoiceOrchestrator;
use std::net::SocketAddr;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(author, version, about = "StackChan Local Mind: Offline perception and voice orchestrator")]
struct Args {
    #[arg(short, long, default_value = "hub/config.yaml")]
    config: String,

    #[arg(short, long)]
    port: Option<u16>,

    #[arg(long)]
    host: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cynapse_bot=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();
    let config = AppConfig::load_from_file(&args.config);

    tracing::info!("Starting StackChan Local Mind Orchestrator...");

    let detector = VisionDetector::new(
        "yolo11s",
        config.vision_orchestrator.confidence_threshold,
        config.vision_orchestrator.simulated_vision,
    );
    let tracker = Mutex::new(VisionTracker::new(config.vision_orchestrator.mirror_pan));

    let persona_prompt = std::fs::read_to_string(&config.llm.system_prompt_path)
        .unwrap_or_else(|_| "You are StackChan, an observant robot desktop companion. Keep spoken answers under 2 sentences.".to_string());

    let voice = VoiceOrchestrator::new(
        &config.stt.url,
        &config.llm.host,
        &config.llm.model,
        &persona_prompt,
        config.llm.temperature,
        config.llm.max_tokens,
        &config.tts.url,
        &config.tts.voice,
        false, // dev_fallbacks = false (honest errors)
    );

    // Dynamic MQTT Bus client from config
    let (bus_client, eventloop) = MqttBus::new(
        &config.mqtt.host,
        config.mqtt.port,
        &config.mqtt.client_id,
    );

    // Spawn robust MQTT background eventloop with auto-reconnect subscription management
    let bus_client_for_loop = bus_client.clone_client();
    tokio::spawn(MqttBus::run_event_loop(bus_client_for_loop, eventloop));

    // Periodic retained hub/status publisher (FR-O2)
    let bus_status_clone = bus_client.clone();
    let start_time = Instant::now();
    let is_simulated = config.vision_orchestrator.simulated_vision;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let status = HubStatus {
                uptime_secs: start_time.elapsed().as_secs(),
                services: serde_json::json!({
                    "orchestrator": "online",
                    "vision": if is_simulated { "simulated" } else { "idle_waiting_npu" },
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }),
            };
            if let Err(e) = bus_status_clone.publish_status(&status).await {
                tracing::debug!("Failed to publish retained hub status: {}", e);
            }
        }
    });


    let app_state = Arc::new(AppState {
        start_time,
        detector,
        tracker,
        voice,
        bus: Some(bus_client),
        frame_counter: AtomicU64::new(0),
    });

    let host = args.host.unwrap_or(config.vision_orchestrator.host);
    let port = args.port.unwrap_or(config.vision_orchestrator.port);
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    let app = create_api_router(app_state);

    tracing::info!("Local Mind API listening on http://{}", addr);
    tracing::info!("Endpoints available: /health, /stats, /say, /api/vision/frame");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
