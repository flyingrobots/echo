// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! JITOS Daemon (jitosd)
//!
//! The main daemon process for the JITOS Causal Operating System.
//! It initializes and runs the kernel, exposes its API via HTTP, and manages the overall system.
use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use echo_kernel::Kernel;
use echo_tasks::slaps::Slaps;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Milliseconds between scheduler ticks
    #[clap(short, long, default_value_t = 1000)]
    tick_interval: u64,

    /// Port for the HTTP API
    #[clap(short, long, default_value_t = 3000)]
    api_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing (logging)
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting JITOS Daemon (jitosd)...");
    info!("Scheduler tick interval: {}ms", args.tick_interval);
    info!("HTTP API will listen on port {}", args.api_port);

    // Initialize the kernel (wrapped in Arc<Mutex> for shared state across async tasks)
    let kernel = Arc::new(Mutex::new(Kernel::new()));

    // Clone the kernel Arc for the HTTP server task
    let kernel_for_api = Arc::clone(&kernel);

    // Build our application with routes
    let app = Router::new()
        .route("/rmg_state", get(get_rmg_state_handler))
        .route("/sws/create", post(create_sws_handler))
        .route("/sws/:id/collapse", post(collapse_sws_handler))
        .route("/intent", post(submit_intent_handler))
        .with_state(kernel_for_api);

    // Start the HTTP server in a separate task
    let addr = format!("0.0.0.0:{}", args.api_port).parse::<std::net::SocketAddr>()?;
    let listener = TcpListener::bind(addr).await?;
    info!("HTTP API server listening on {}", listener.local_addr()?);
    tokio::spawn(async {
        axum::serve(listener, app)
            .await
            .expect("HTTP server failed");
    });

    // Run the kernel's main loop (this will block until an error or shutdown)
    if let Err(e) = kernel.lock().await.run().await {
        error!("JITOS Kernel encountered a fatal error: {:?}", e);
        return Err(e);
    }

    Ok(())
}

async fn get_rmg_state_handler(State(kernel): State<Arc<Mutex<Kernel>>>) -> impl IntoResponse {
    let kernel_locked = kernel.lock().await;
    let state = kernel_locked.get_rmg_state();
    (StatusCode::OK, state)
}

async fn create_sws_handler(State(kernel): State<Arc<Mutex<Kernel>>>) -> impl IntoResponse {
    let mut kernel_locked = kernel.lock().await;
    let sws_id = kernel_locked.create_sws();
    (StatusCode::CREATED, format!("{{\"sws_id\": {}}}", sws_id))
}

async fn collapse_sws_handler(
    State(kernel): State<Arc<Mutex<Kernel>>>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let mut kernel_locked = kernel.lock().await;
    match kernel_locked.collapse_sws(id) {
        Ok(_) => (StatusCode::OK, "Collapsed SWS".to_string()),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()),
    }
}

async fn submit_intent_handler(
    State(kernel): State<Arc<Mutex<Kernel>>>,
    Json(slaps): Json<Slaps>,
) -> impl IntoResponse {
    let mut kernel_locked = kernel.lock().await;
    match kernel_locked.submit_intent(slaps) {
        Ok(id) => (StatusCode::CREATED, format!("{{\"sws_id\": {}}}", id)),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}
