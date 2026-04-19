use axum::{
    extract::{DefaultBodyLimit, State},
    routing::post,
    Json, Router,
    middleware,
    http::{Request, StatusCode},
    response::{Response, IntoResponse},
    body::Body,
};
use std::{env, net::{IpAddr, Ipv4Addr, SocketAddr}, sync::Arc};
use tokio::net::TcpListener;

#[cfg(feature = "ui")]
use axum::{response::Html, routing::get};

#[cfg(feature = "ui")]
use tower_http::services::ServeDir;

use dotenvy::dotenv;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod downloader;
mod executor;
mod models;
mod params;
mod routes;
mod routing;
mod state;
mod error;
mod ui_logs;

use routes::handle;
use state::AppState;
use error::{AppError, Result};
use crate::models::RootJson;

const DEFAULT_PORT: u16 = 8787;
const MAX_API_BODY_BYTES: usize = 1024 * 1024 * 1024;

#[cfg(feature = "ui")]
const MAX_UI_UPLOAD_BYTES: usize = 1024 * 1024 * 1024;

fn parse_port() -> Result<u16> {
    env::var("PORT")
        .unwrap_or_else(|_| DEFAULT_PORT.to_string())
        .parse::<u16>()
        .map_err(|e| AppError::InternalServerError(format!(
            "Invalid PORT environment variable: {}",
            e
        )))
}

//
// ✅ AUTH MIDDLEWARE (fixed for axum 0.7)
//
async fn auth_middleware(req: Request<Body>, next: middleware::Next) -> Response {
    let qauth = env::var("QAUTH").ok();
    let lauth = env::var("LAUTH").ok();

    // If no auth configured → allow everything
    if qauth.is_none() && lauth.is_none() {
        return next.run(req).await;
    }

    let path = req.uri().path();

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok());

    // Protect /query
    if path.starts_with("/query") {
        if let Some(expected) = qauth {
            if auth_header != Some(expected.as_str()) {
                warn!("Unauthorized access to /query");
                return StatusCode::UNAUTHORIZED.into_response();
            }
        }
    }

    // Protect /load_agent
    if path.starts_with("/load_agent") {
        if let Some(expected) = lauth {
            if auth_header != Some(expected.as_str()) {
                warn!("Unauthorized access to /load_agent");
                return StatusCode::UNAUTHORIZED.into_response();
            }
        }
    }

    next.run(req).await
}

// Existing handler (unchanged)
async fn load_agent_handler(
    State(state): State<Arc<AppState>>,
    Json(new_config): Json<RootJson>,
) -> Result<Json<serde_json::Value>> {
    state.update_agent_config(new_config).await?;
    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "Agent loaded successfully"
    })))
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok(); // Load .env file if it exists

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "quicktasks=debug".into()),
        )
        .with(ui_logs::layer())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = Arc::new(AppState::new().await?);

    // ✅ ONLY CHANGE: middleware added here
    let app = Router::new()
        .route("/query", post(handle).layer(DefaultBodyLimit::max(MAX_API_BODY_BYTES)))
        .route("/load_agent", post(load_agent_handler).layer(DefaultBodyLimit::max(MAX_API_BODY_BYTES)))
        .layer(middleware::from_fn(auth_middleware));

    #[cfg(feature = "ui")]
    let app = {
        app.route("/", get(|| async {
            let html_content = tokio::fs::read_to_string("static/index.html")
                .await
                .unwrap_or_else(|_| "<h1>UI not found. Build with --features ui</h1>".to_string());
            Html(html_content)
        }))
        .route(
            "/upload/ui",
            post(routes::upload_ui_file).layer(DefaultBodyLimit::max(MAX_UI_UPLOAD_BYTES)),
        )
        .route(
            "/models/upload/ui",
            post(routes::upload_model_file).layer(DefaultBodyLimit::max(MAX_UI_UPLOAD_BYTES)),
        )
        .route("/models/reload/ui", post(routes::reload_models_handler))
        .route("/logs", get(routes::logs_handler))
        .nest_service("/static", ServeDir::new("static"))
    };

    let app = app.with_state(state);

    let port = parse_port()?;

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::InternalServerError(format!(
            "Failed to bind to address {}: {}",
            addr, e
        )))?;

    info!(
        "🚀 Listening on {}",
        listener.local_addr().map_err(|e| AppError::InternalServerError(format!(
            "Failed to get local address: {}",
            e
        )))?
    );

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| AppError::InternalServerError(format!("Server failed: {}", e)))?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down gracefully...");
        },
        _ = terminate => {
            info!("Received SIGTERM, shutting down gracefully...");
        },
    }
}
