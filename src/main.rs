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
struct Configuration {
    port: u16,
    nixos_current_system: bool,
    services: HashMap<String, ServiceDescription>,
    refresh_interval: u16,
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
        Configuration {
            port: 3000,
            nixos_current_system: false,
            services: HashMap::new(),
            refresh_interval: 10,
        }
    }
}

#[tokio::main]
async fn main() {
    let config_path = std::env::var("ANSINE_CONFIG_PATH").expect("Expected ANSINE_CONFIG_PATH");
    let config_path = Path::new(&config_path);
    let config = load_configuration(config_path);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));

    let init_metrics: metrics::Metrics = Default::default();

    let metrics = metrics::get_metrics(&init_metrics, config.nixos_current_system).unwrap();

    let state = State {
        nixos_current_system: config.nixos_current_system,
        services: config.services,
        refresh_interval: config.refresh_interval,
        last_metrics: init_metrics,
        metrics,
    };

    let state = Arc::new(RwLock::new(state));
    let stat_state = state.clone();

    let refresh_stat = tokio::task::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_secs(config.refresh_interval.into()));

        loop {
            interval.tick().await;
            let mut state = stat_state.write().unwrap();
            let metrics =
                metrics::get_metrics(&state.last_metrics, state.nixos_current_system).unwrap();
            state.last_metrics = state.metrics.clone();
            state.metrics = metrics;
        }
    });
    let app = Router::new()
        .route("/", get(root))
        .route("/metrics", get(metrics_api))
        .route("/assets/*file", get(assets))
        .route_layer(Extension(state));
    let server = axum::Server::bind(&addr).serve(app.into_make_service());
    let (_, _) = tokio::join!(refresh_stat, server);
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
    let state = &state.read().unwrap();

    let template = IndexTemplate {
        services: state.services.clone(),
        refresh_interval: state.refresh_interval,
    };
    HtmlTemplate(template)
}

async fn metrics_api(Extension(state): Extension<SharedState>) -> impl IntoResponse {
    let state = &state.read().unwrap();
    Json(state.metrics.clone())
}
