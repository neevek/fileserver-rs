use axum::extract::{Multipart, Path};
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::routing::{get_service, post};
use axum::Json;
use axum::{response::IntoResponse, routing::get, Router};
use axum_extra::routing::SpaRouter;
use chrono::{DateTime, Local};
use clap::Parser;
use common::{DirDesc, DirEntry, FileType, JsonRequest, JsonResponse};
use log::info;
use path_dedot::*;
use serde::Deserialize;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use tokio::io::AsyncWriteExt;
use tower_http::services::ServeDir;

use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use walkdir::WalkDir;

static mut SERVE_DIR: Option<PathBuf> = None;

// Setup the command line interface with clap.
#[derive(Parser, Debug)]
#[clap(name = "server", about = "A server for our wasm project!")]
struct Opt {
    /// set the log level
    #[clap(short = 'l', long = "log", default_value = "debug")]
    log_level: String,

    /// set the listen addr
    #[clap(short = 'a', long = "addr", default_value = "::1")]
    addr: String,

    /// set the listen port
    #[clap(short = 'p', long = "port", default_value = "8080")]
    port: u16,

    /// set the directory where static files are to be found
    #[clap(long = "static-dir", default_value = "../dist")]
    static_dir: String,

    /// the directory to serve, default to the current directory if not specified
    #[clap(long = "serve-dir", default_value = ".")]
    serve_dir: String,
}

#[tokio::main]
async fn main() {
    let opt = Opt::parse();

    // Setup logging & RUST_LOG from args
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", format!("{},hyper=info,mio=info", opt.log_level))
    }
    // enable console logging
    tracing_subscriber::fmt::init();

    match PathBuf::from(&opt.serve_dir).parse_dot() {
        Ok(serve_dir) if serve_dir.is_dir() => {
            info!("serve_dir: {:?}", serve_dir);
            unsafe {
                SERVE_DIR = Some(serve_dir.to_path_buf());
            }
            log::info!("serving directory: {:?}", serve_dir);
        }
        _ => {
            panic!("serve-dir must be a valid directory");
        }
    }

    let app = Router::new()
        .route("/api/listing", get(serve_root).post(serve_root))
        .route("/api/listing/*path", get(list_files).post(create_dir))
        .route("/api/upload/*path", post(save_request_body))
        .route("/api/delete/*path", post(delete_path))
        .nest(
            "/api/static",
            get_service(ServeDir::new(unsafe {
                SERVE_DIR.as_ref().unwrap().to_string_lossy().to_string()
            }))
            .handle_error(|_| async move { AppError("Static file not found".to_string()) }),
        )
        .merge(SpaRouter::new("/assets", "./frontend/dist").index_file("index.html"))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    let sock_addr = SocketAddr::from((
        IpAddr::from_str(opt.addr.as_str()).unwrap_or(IpAddr::V6(Ipv6Addr::LOCALHOST)),
        opt.port,
    ));

    log::info!("listening on http://{}", sock_addr);

    axum::Server::bind(&sock_addr)
        .serve(app.into_make_service())
        .await
        .expect("Unable to start server");
}

async fn create_dir(Path(path): Path<String>, Json(req): Json<JsonRequest>) -> impl IntoResponse {
    let resp = match req {
        JsonRequest::CreateDirectory { dir_name } => {
            let full_path = format!(
                "{}/{}/{}",
                unsafe { SERVE_DIR.as_ref().unwrap().to_string_lossy().to_string() },
                path.to_string(),
                dir_name
            );

            match std::fs::create_dir(&full_path) {
                Err(err) => JsonResponse::Failed {
                    msg: Some(err.to_string()),
                },
                _ => JsonResponse::Succeeded {
                    msg: Some(format!("create dir: {}", full_path)),
                },
            }
        }
    };

    (StatusCode::OK, Json(resp).into_response())
}

async fn delete_path(Path(path): Path<String>) -> impl IntoResponse {
    let str_path = format!(
        "{}/{}",
        unsafe { SERVE_DIR.as_ref().unwrap().to_string_lossy().to_string() },
        path.to_string(),
    );

    let mut error_msg: Option<String> = None;
    let full_path = PathBuf::from(str_path);
    if full_path.is_file() {
        if let Err(e) = std::fs::remove_file(full_path) {
            error_msg = Some(format!("failed to remove file: {}, error: {}", path, e));
        }
    } else if full_path.is_dir() {
        if let Err(e) = std::fs::remove_dir_all(full_path) {
            error_msg = Some(format!("failed to remove dir: {}, error: {}", path, e));
        }
    }

    let json_resp = if error_msg.is_some() {
        JsonResponse::Failed { msg: error_msg }
    } else {
        JsonResponse::Succeeded { msg: None }
    };

    (StatusCode::OK, Json(json_resp).into_response())
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct UploadParams {
    filename: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct UploadForm {
    filename: String,
}

async fn save_request_body(
    Path(path): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<JsonResponse>, AppError> {
    let parent_dir = unsafe {
        SERVE_DIR
            .as_ref()
            .unwrap()
            .join(path.trim_start_matches('/'))
    };
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError("failed to iterate over uploaded files".to_string()))?
    {
        let name = field.file_name().unwrap().to_string();
        let filename = parent_dir.join(name.as_str());
        let mut file = tokio::fs::File::create(filename).await.map_err(|_| {
            AppError(format!(
                "failed to create file at: {}/{}",
                parent_dir.to_str().unwrap(),
                name
            ))
        })?;

        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|_| AppError(format!("failed to read from file: {}", name)))
            .unwrap()
        {
            file.write_all(&chunk[..])
                .await
                .map_err(|_| AppError("failed to write file".to_string()))?;
        }
    }

    Ok(Json(JsonResponse::Succeeded {
        msg: Some(format!("file uploaded: {}", "file")),
    }))
}

async fn serve_root() -> impl IntoResponse {
    list_files(Path("/".to_string())).await
}

async fn list_files(Path(path): Path<String>) -> impl IntoResponse {
    let mut abs_path;
    unsafe {
        abs_path = SERVE_DIR.as_ref().unwrap().to_path_buf();
    }
    let path = path.trim_start_matches('/');
    abs_path.push(path);

    log::debug!("list files for path: {:?}", abs_path);

    if abs_path.is_dir() {
        let mut descendants = vec![];
        for entry in WalkDir::new(&abs_path)
            .follow_links(true)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.depth() > 0 {
                let entry = convert_dir_entry(&entry);
                descendants.push(entry);
            }
        }

        let dir_desc = DirDesc {
            dir_name: format!("/{}", path),
            descendants,
        };

        return (StatusCode::OK, Json(dir_desc)).into_response();
    } else if abs_path.is_symlink() || abs_path.is_file() {
        return Redirect::permanent(format!("/static/{}", path).as_str()).into_response();
    }

    let dir_desc = DirDesc {
        dir_name: "".to_string(),
        descendants: vec![],
    };
    (StatusCode::OK, Json(dir_desc)).into_response()
}

fn convert_dir_entry(entry: &walkdir::DirEntry) -> DirEntry {
    let file_name = entry.file_name().to_string_lossy().to_string();
    let file_type = if entry.file_type().is_file() {
        FileType::File
    } else if entry.file_type().is_dir() {
        FileType::Directory
    } else {
        FileType::SymbolicLink
    };

    let (file_size, last_accessed) = if let Ok(metadata) = entry.metadata() {
        let last_accessed = if let Ok(accessed) = metadata.accessed() {
            let local_time: DateTime<Local> = accessed.into();
            local_time.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            "".to_string()
        };
        (metadata.len(), last_accessed)
    } else {
        (0, "".to_string())
    };

    DirEntry {
        file_name,
        file_type,
        file_size,
        last_accessed,
    }
}

#[derive(Debug)]
struct AppError(String);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let json_resp = Json(JsonResponse::Failed { msg: Some(self.0) });
        (StatusCode::OK, json_resp).into_response()
    }
}
