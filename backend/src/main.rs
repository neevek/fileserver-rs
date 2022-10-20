use axum::body::Bytes;
use axum::extract::{BodyStream, Path, Query};
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::routing::post;
use axum::{response::IntoResponse, routing::get, Router};
use axum::{BoxError, Json};
use axum_extra::routing::SpaRouter;
use chrono::{DateTime, Local};
use clap::Parser;
use common::{DirDesc, DirEntry, FileType, JsonRequest, JsonResponse};
use futures::{Stream, TryStreamExt};
use log::error;
use path_dedot::*;
use serde::Deserialize;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;
use tokio::fs::File;
use tokio::io::{self, BufWriter};
use tokio_util::io::StreamReader;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use walkdir::WalkDir;

static mut ROOT_DIR: Option<PathBuf> = None;

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

    /// the root directory of the file server, default to the current directory if not specified
    #[clap(long = "root-dir", default_value = ".")]
    root_dir: String,
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

    match PathBuf::from(&opt.root_dir).parse_dot() {
        Ok(root_dir) if root_dir.is_dir() => {
            unsafe {
                ROOT_DIR = Some(root_dir.to_path_buf());
            }
            log::info!("serving directory: {:?}", root_dir);
        }
        _ => {
            panic!("root-dir must be a valid directory");
        }
    }

    let app = Router::new()
        .merge(SpaRouter::new("/static", unsafe {
            ROOT_DIR.as_ref().unwrap().to_string_lossy().to_string()
        }))
        .route("/api/listing/*path", get(list_files).post(create_dir))
        .route("/api/upload/*path", post(save_request_body))
        // .merge(SpaRouter::new("/assets", opt.static_dir))
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
                unsafe { ROOT_DIR.as_ref().unwrap().to_string_lossy().to_string() },
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

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct UploadParams {
    filename: String,
}

async fn save_request_body(
    Path(path): Path<String>,
    Query(params): Query<UploadParams>,
    body: BodyStream,
) -> impl IntoResponse {
    log::info!(
        ">>>>>>>>>>>>>> upload path: {}, filename: {}",
        path,
        params.filename
    );
    save_file(path.as_str(), params.filename.as_str(), body).await
}

async fn save_file(
    subdir: &str,
    filename: &str,
    stream: BodyStream,
) -> Result<Json<JsonResponse>, AppError> {
    let dir;
    unsafe {
        dir = ROOT_DIR.as_ref().unwrap().to_path_buf().join(subdir);
    }

    if !dir.is_dir() {
        unsafe {
            error!("error 1: {:?}, {:?}", ROOT_DIR, dir);
        }
        return Err(AppError::SaveFileFailed {
            file_path: dir.to_str().unwrap().to_string(),
        });
    }

    async {
        let body_with_io_error = stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
        let body_reader = StreamReader::new(body_with_io_error);
        futures::pin_mut!(body_reader);

        log::info!("receive new uploaded file: {:?}", dir);

        let path = dir.join(filename);
        let mut file = BufWriter::new(File::create(path.clone()).await.map_err(|_| {
            error!("error 2");
            AppError::SaveFileFailed {
                file_path: path.to_str().unwrap().to_string(),
            }
        })?);
        error!("error 3");
        tokio::io::copy(&mut body_reader, &mut file)
            .await
            .map_err(|_| AppError::SaveFileFailed {
                file_path: path.to_str().unwrap().to_string(),
            })?;
        return Ok::<_, AppError>(());
    }
    .await?;

    Ok(Json(JsonResponse::Succeeded {
        msg: Some(format!("file uploaded: {}", filename)),
    }))
}

async fn list_files(Path(path): Path<String>) -> impl IntoResponse {
    let mut abs_path;
    unsafe {
        abs_path = ROOT_DIR.as_ref().unwrap().to_path_buf();
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

enum AppError {
    SaveFileFailed { file_path: String },
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status_code, message) = match self {
            AppError::SaveFileFailed { file_path } => (
                StatusCode::OK,
                format!("failed to save file to path: {}", file_path),
            ),
        };

        let json_resp = Json(JsonResponse::Failed { msg: Some(message) });
        (status_code, json_resp).into_response()
    }
}
