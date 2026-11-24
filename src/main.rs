mod models;
mod search;

use anyhow::{Context, Result};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use models::{HealthResponse, SearchQuery};
use reqwest::Client;
use search::{run_search, validate_query};
use std::{env, net::{IpAddr, SocketAddr}, time::Instant};
use tokio::net::TcpListener;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::{info, warn};

#[derive(Clone)]
pub struct AppState {
    pub client: Client,
    pub searxng_url: Option<String>,
    pub brave_api_key: Option<String>,
}

async fn search_handler(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let query = params.q.trim();
    if let Err(message) = validate_query(query) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": message })),
        )
            .into_response();
    }

    let started = Instant::now();
    let lang = params
        .lang
        .as_deref()
        .filter(|value| matches!(*value, "fa" | "en"))
        .map(str::to_owned)
        .unwrap_or_else(|| search::infer_language(query).to_owned());
    let tab = params
        .tab
        .as_deref()
        .filter(|value| matches!(*value, "all" | "web" | "wiki"))
        .unwrap_or("all");

    let mut response = run_search(&state, query, &lang, tab).await;
    response.elapsed_ms = started.elapsed().as_millis();
    (StatusCode::OK, Json(response)).into_response()
}

async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        app: "Griffin Search".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        providers: vec![
            models::ProviderStatus {
                name: "DuckDuckGo".to_owned(),
                configured: true,
                available: None,
                detail: "آزمون اتصال هنگام جست‌وجو انجام می‌شود".to_owned(),
            },
            models::ProviderStatus {
                name: "SearXNG".to_owned(),
                configured: state.searxng_url.is_some(),
                available: None,
                detail: if state.searxng_url.is_some() {
                    "آدرس جایگزین تنظیم شده است".to_owned()
                } else {
                    "اختیاری؛ از طریق فایل .env فعال می‌شود".to_owned()
                },
            },
            models::ProviderStatus {
                name: "Brave Search".to_owned(),
                configured: state.brave_api_key.is_some(),
                available: None,
                detail: if state.brave_api_key.is_some() {
                    "کلید API در سرور تنظیم شده است".to_owned()
                } else {
                    "اختیاری؛ به کلید API نیاز دارد".to_owned()
                },
            },
            models::ProviderStatus {
                name: "Wikipedia".to_owned(),
                configured: true,
                available: None,
                detail: "جست‌وجوی دانشنامه‌ای فارسی و انگلیسی".to_owned(),
            },
        ],
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("griffin_search=info,tower_http=info")),
        )
        .init();

    let host: IpAddr = env::var("GRIFFIN_HOST")
        .unwrap_or_else(|_| "127.0.0.1".to_owned())
        .parse()
        .context("GRIFFIN_HOST must be a valid IP address")?;
    let port: u16 = env::var("GRIFFIN_PORT")
        .unwrap_or_else(|_| "8080".to_owned())
        .parse()
        .context("GRIFFIN_PORT must be a valid port number")?;

    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent(concat!("GriffinSearch/", env!("CARGO_PKG_VERSION"), " (local search app)"))
        .build()
        .context("could not initialize HTTP client")?;

    let state = AppState {
        client,
        searxng_url: env::var("SEARXNG_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_owned())
            .filter(|value| !value.is_empty()),
        brave_api_key: env::var("BRAVE_SEARCH_API_KEY")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
    };

    let app = Router::new()
        .route("/api/search", get(search_handler))
        .route("/api/health", get(health_handler))
        .fallback_service(ServeDir::new("static").append_index_html_on_directories(true))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let address = SocketAddr::new(host, port);
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("could not bind Griffin Search to {address}"))?;

    info!("Griffin Search listening at http://{address}");
    if host.is_unspecified() {
        warn!("Griffin is bound to all network interfaces. Keep it on localhost unless you understand the security implications.");
    }
    println!("\n  🦅 Griffin Search v{}\n  Open http://{}:{}\n", env!("CARGO_PKG_VERSION"), host, port);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            info!("shutdown signal received");
        })
        .await
        .context("HTTP server stopped unexpectedly")?;

    Ok(())
}

