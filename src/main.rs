mod parser;

use std::{
    collections::HashMap,
    fs::{read_link, read_to_string},
    net::SocketAddr,
    path::Path,
    str,
    time::Duration,
};

use askama::Template;
use axum::{
    body::{boxed, Full},
    extract::State,
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
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

fn read_file(fp: &str) -> String {
    read_to_string(fp).expect("Unable to read file")
}

fn format_duration(duration: &Duration) -> String {
    let secs = duration.as_secs();
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    format!("{}d.{}h.{}m", days, hours, mins)
}

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
    let app = Router::new()
        .route("/", get(root))
        .route("/metrics", get(metrics))
        .route("/assets/*file", get(assets))
        .with_state(config);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
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
    metrics: Metrics,
    services: HashMap<String, ServiceDescription>,
    current_system: Option<String>,
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
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}

fn read_nixos_current_system() -> Option<String> {
    let link = read_link("/run/current-system").ok()?;
    let (_, current_system) = parser::parse_nix_store_path(link.to_str()?).ok()?;
    Some(current_system.to_string())
}

#[derive(Serialize)]
struct Metrics {
    uptime: String,
    cpu: usize,
    mem: usize,
    swap: usize,
}

fn get_system_metrics() -> Metrics {
    let proc_stat = read_file("/proc/stat");
    let proc_meminfo = read_file("/proc/meminfo");
    let proc_uptime = read_file("/proc/uptime");
    let proc_swaps = read_file("/proc/swaps");

    let (_, stat) = parser::parse_stat(&proc_stat).expect("Unable to parse /proc/stat");
    let (_, mem_info) =
        parser::parse_meminfo(&proc_meminfo).expect("Unable to parse /proc/meminfo");
    let (_, uptime) = parser::parse_uptime(&proc_uptime).expect("Unable to parse /proc/uptime");
    let (_, swaps) = parser::parse_swaps(&proc_swaps).expect("Unable to parse /proc/swaps");

    Metrics {
        uptime: format_duration(&uptime),
        cpu: 100 - stat.average_idle(),
        mem: mem_info.total_used(),
        swap: swaps.into_values().map(|s| s.total_used()).sum(),
    }
}

async fn root(State(config): State<Configuration>) -> impl IntoResponse {
    let metrics = get_system_metrics();

    let current_system = if config.nixos_current_system {
        read_nixos_current_system()
    } else {
        None
    };

    let template = IndexTemplate {
        metrics,
        services: config.services,
        current_system,
        refresh_interval: config.refresh_interval,
    };
    HtmlTemplate(template)
}

async fn metrics() -> impl IntoResponse {
    Json(get_system_metrics())
}
