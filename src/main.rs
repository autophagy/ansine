mod metrics;
mod parser;

use std::{
    collections::HashMap,
    fs::read_to_string,
    net::SocketAddr,
    path::Path,
    str,
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::Result;
use askama::Template;
use axum::{
    body::{boxed, Full},
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Extension, Router,
};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct ServiceDescription {
    description: String,
    route: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
struct Configuration {
    port: u16,
    nixos_current_system: bool,
    services: HashMap<String, ServiceDescription>,
    refresh_interval: u16,
}

impl Default for Configuration {
    fn default() -> Configuration {
        Configuration {
            port: 3000,
            nixos_current_system: false,
            services: HashMap::new(),
            refresh_interval: 10,
        }
    }
}

struct State {
    nixos_current_system: bool,
    services: HashMap<String, ServiceDescription>,
    refresh_interval: u16,
    last_metrics: metrics::Metrics,
    metrics: metrics::Metrics,
}

type SharedState = Arc<RwLock<State>>;

fn load_configuration(path: &Path) -> Configuration {
    if path.exists() {
        let data = read_to_string(path).expect("Unable to read file");
        serde_json::from_str(&data).expect("Unable to parse JSON file")
    } else {
        Default::default()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let config_path = &args.get(1).expect("Expected argument to config path");
    let config_path = Path::new(&config_path);
    let config = load_configuration(config_path);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));

    let init_metrics: metrics::Metrics = Default::default();

    let metrics = metrics::get_metrics(&init_metrics, config.nixos_current_system)?;

    let state = State {
        nixos_current_system: config.nixos_current_system,
        services: config.services,
        refresh_interval: config.refresh_interval,
        last_metrics: init_metrics,
        metrics,
    };

    let state = Arc::new(RwLock::new(state));
    let stat_state = state.clone();

    let refresh_stat = tokio::task::spawn(refresh_metrics(stat_state, config.refresh_interval));

    let app = Router::new()
        .route("/", get(root))
        .route("/metrics", get(metrics_api))
        .route("/assets/*file", get(assets))
        .route_layer(Extension(state));
    let server = axum::Server::bind(&addr).serve(app.into_make_service());
    println!("Starting Ansíne on {addr}...");
    let (_, _) = tokio::join!(refresh_stat, server);
    Ok(())
}

async fn refresh_metrics(state: SharedState, refresh_interval: u16) -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_secs(refresh_interval.into()));

    loop {
        interval.tick().await;
        if let Ok(mut state_guard) = state.write() {
            if let Ok(metrics) =
                metrics::get_metrics(&state_guard.last_metrics, state_guard.nixos_current_system)
            {
                state_guard.last_metrics = state_guard.metrics.clone();
                state_guard.metrics = metrics;
            } else {
                eprintln!("Failed to refresh metrics")
            }
        } else {
            eprintln!("Failed to aquire write lock")
        }
    }
}

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Asset;

pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();

        match Asset::get(path.as_str()) {
            Some(content) => {
                let body = boxed(Full::from(content.data));
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Response::builder()
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    .body(body)
                    .unwrap()
            }
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(boxed(Full::from("404")))
                .unwrap(),
        }
    }
}

async fn assets(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.starts_with("assets/") {
        path = path.replace("assets/", "");
    }
    StaticFile(path)
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    services: HashMap<String, ServiceDescription>,
    refresh_interval: u16,
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {err}"),
            )
                .into_response(),
        }
    }
}

async fn root(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    match state.read() {
        Ok(state_guard) => {
            let template = IndexTemplate {
                services: state_guard.services.clone(),
                refresh_interval: state_guard.refresh_interval,
            };
            HtmlTemplate(template).into_response()
        }
        Err(_) => {
            eprintln!("Failed to aquire state lock");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        }
    }
}

async fn metrics_api(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    match state.read() {
        Ok(state_guard) => Json(state_guard.metrics.clone()).into_response(),
        Err(_) => {
            eprintln!("Failed to acquire state lock");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        }
    }
}
