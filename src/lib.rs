#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
pub mod mcp;
pub mod store;
pub mod todo;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};
use store::Store;
use todo::Todo;
use tower_http::trace::TraceLayer;

static UI: include_dir::Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/client/dist");

#[derive(Debug)]
pub enum Error {
    NotFound(String),
    Invalid(String),
    Io(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(m) | Self::Invalid(m) | Self::Io(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(format!("io: {error}"))
    }
}

impl Error {
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "not_found",
            Self::Invalid(_) => "invalid",
            Self::Io(_) => "io",
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Invalid(_) => StatusCode::BAD_REQUEST,
            Self::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(json!({"error": self.to_string(), "code": self.code()})),
        )
            .into_response()
    }
}

type Shared = State<Arc<Store>>;

pub fn app(store: Store) -> Router {
    let store = Arc::new(store);
    let api = Router::new()
        .route("/health", get(|| async { Json(json!({"status": "ok"})) }))
        .route("/todos", get(list).post(create))
        .route("/todos/{id}", get(fetch).patch(update).delete(remove))
        .route("/snapshot", get(snapshot))
        .fallback(not_found);
    Router::new()
        .route("/healthz", get(|| async { "ok\n" }))
        .route("/api", axum::routing::any(not_found))
        .route("/api/", axum::routing::any(not_found))
        .nest("/api", api)
        .merge(mcp::endpoint(store.clone()))
        .fallback_service(get(ui))
        .layer(TraceLayer::new_for_http())
        .with_state(store)
}

/// `done=true|false` filters; omitted lists open items first, then done ones.
fn done_filter(value: Option<&str>) -> Result<Option<bool>, Error> {
    match value {
        None => Ok(None),
        Some("true") => Ok(Some(true)),
        Some("false") => Ok(Some(false)),
        Some(_) => Err(Error::Invalid("done must be true or false".into())),
    }
}

async fn list(
    State(s): Shared,
    Query(q): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Todo>>, Error> {
    Ok(Json(
        s.list(done_filter(q.get("done").map(String::as_str))?)?,
    ))
}

async fn create(State(s): Shared, Json(v): Json<Value>) -> Result<(StatusCode, Json<Todo>), Error> {
    Ok((StatusCode::CREATED, Json(s.create(&v)?)))
}

async fn fetch(State(s): Shared, Path(id): Path<String>) -> Result<Json<Todo>, Error> {
    Ok(Json(s.get(&id)?))
}

async fn update(
    State(s): Shared,
    Path(id): Path<String>,
    Json(v): Json<Value>,
) -> Result<Json<Todo>, Error> {
    Ok(Json(s.update(&id, &v)?))
}

async fn remove(State(s): Shared, Path(id): Path<String>) -> Result<StatusCode, Error> {
    s.delete(&id)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn snapshot(State(s): Shared) -> Result<Json<Value>, Error> {
    Ok(Json(json!({"files": s.snapshot()?})))
}

async fn not_found() -> Error {
    Error::NotFound("API route not found".into())
}

async fn ui(uri: Uri) -> Response {
    let file = UI
        .get_file(uri.path().trim_start_matches('/'))
        .unwrap_or_else(|| {
            UI.get_file("index.html")
                .expect("build requires index.html")
        });
    let content_type = mime_guess::from_path(file.path()).first_or_octet_stream();
    (
        [(header::CONTENT_TYPE, content_type.as_ref())],
        file.contents(),
    )
        .into_response()
}
