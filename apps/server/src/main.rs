mod access;
mod scheduler;
use access::Access;
use axum::{
    Json, Router,
    extract::{ConnectInfo, DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use echo_app::{AppError, SolveRequest};
use scheduler::{Rejection, Scheduler, Settings};
use std::{net::SocketAddr, sync::Arc};
fn error(status: StatusCode, e: AppError) -> Response {
    (status, [(header::CACHE_CONTROL, "no-store")], Json(e)).into_response()
}
fn rejected(e: Rejection) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [
            (header::RETRY_AFTER, e.retry.to_string()),
            (header::CACHE_CONTROL, "no-store".into()),
        ],
        Json(serde_json::json!({"code":e.code,"message":e.message,"retry_after":e.retry})),
    )
        .into_response()
}
fn identity(
    state: &Scheduler,
    peer: SocketAddr,
    headers: &HeaderMap,
) -> Result<(std::net::IpAddr, bool), AppError> {
    let admin = state.access.admin(headers);
    if headers.contains_key(header::AUTHORIZATION) && !admin {
        return Err(AppError {
            code: "invalid_key",
            message: "访问密钥无效。".into(),
        });
    }
    Ok((state.access.ip(peer.ip(), headers), admin))
}
async fn auth(State(state): State<Arc<Scheduler>>, headers: HeaderMap) -> Response {
    if state.access.admin(&headers) {
        Json(serde_json::json!({"admin":true})).into_response()
    } else {
        error(
            StatusCode::UNAUTHORIZED,
            AppError {
                code: "invalid_key",
                message: "访问密钥无效。".into(),
            },
        )
    }
}
async fn solve(
    State(state): State<Arc<Scheduler>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<SolveRequest>, JsonRejection>,
) -> Response {
    let (ip, admin) = match identity(&state, peer, &headers) {
        Ok(v) => v,
        Err(e) => return error(StatusCode::UNAUTHORIZED, e),
    };
    let Json(request) = match body {
        Ok(r) => r,
        Err(e) => return error(StatusCode::BAD_REQUEST, AppError::input(e.body_text())),
    };
    if let Err(e) = echo_app::validate_request(&request) {
        return error(StatusCode::UNPROCESSABLE_ENTITY, e);
    }
    match state.submit(request, ip, admin) {
        Ok(v) => (StatusCode::ACCEPTED, Json(v)).into_response(),
        Err(e) => rejected(e),
    }
}
async fn poll(
    State(state): State<Arc<Scheduler>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Response {
    let (ip, _) = match identity(&state, peer, &headers) {
        Ok(v) => v,
        Err(e) => return error(StatusCode::UNAUTHORIZED, e),
    };
    match state.poll(&id, ip) {
        Some(v) => ([(header::CACHE_CONTROL, "no-store")], Json(v)).into_response(),
        None => error(
            StatusCode::NOT_FOUND,
            AppError {
                code: "job_expired",
                message: "计算任务已过期，请重新求解。".into(),
            },
        ),
    }
}
async fn preview(
    State(state): State<Arc<Scheduler>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Result<Json<SolveRequest>, JsonRejection>,
) -> Response {
    let (ip, admin) = match identity(&state, peer, &headers) {
        Ok(v) => v,
        Err(e) => return error(StatusCode::UNAUTHORIZED, e),
    };
    if let Err(e) = state.charge_request(ip, admin) {
        return rejected(e);
    }
    match body {
        Ok(Json(r)) => match echo_app::preview(&r) {
            Ok(v) => Json(v).into_response(),
            Err(e) => error(StatusCode::UNPROCESSABLE_ENTITY, e),
        },
        Err(e) => error(StatusCode::BAD_REQUEST, AppError::input(e.body_text())),
    }
}
fn app(state: Arc<Scheduler>) -> Router {
    let data = Arc::new(echo_app::data::DataStore::new(
        std::env::var_os("ECHO_DATA_FILE").map(Into::into),
    ));
    app_with_data(state, data)
}
fn app_with_data(state: Arc<Scheduler>, data: Arc<echo_app::data::DataStore>) -> Router {
    let export_data = data.clone();
    Router::new()
        .route(
            "/api/health",
            get(|| async {
                Json(serde_json::json!({"status":"ok","version":env!("CARGO_PKG_VERSION")}))
            }),
        )
        .route(
            "/api/bootstrap",
            get(move || {
                let data = data.clone();
                async move {
                    let (pack, status) = tokio::task::spawn_blocking(move || data.snapshot())
                        .await
                        .expect("data loader");
                    (
                        [(header::CACHE_CONTROL, "no-store")],
                        Json(echo_app::bootstrap_with_data(&pack, status)),
                    )
                }
            }),
        )
        .route(
            "/api/data-pack",
            get(move || {
                let data = export_data.clone();
                async move {
                    let (pack, _) = tokio::task::spawn_blocking(move || data.snapshot())
                        .await
                        .expect("data loader");
                    (
                        [(header::CACHE_CONTROL, "no-store")],
                        Json(pack.value.clone()),
                    )
                }
            }),
        )
        .route("/api/preview", post(preview))
        .route("/api/solve", post(solve))
        .route("/api/jobs/{id}", get(poll))
        .route("/api/auth", post(auth))
        .route(
            "/",
            get(|| async {
                asset(
                    "text/html; charset=utf-8",
                    include_str!("../../web/index.html"),
                )
            }),
        )
        .route(
            "/app.js",
            get(|| async {
                asset(
                    "text/javascript; charset=utf-8",
                    include_str!("../../web/app.js"),
                )
            }),
        )
        .route(
            "/i18n.js",
            get(|| async {
                asset(
                    "text/javascript; charset=utf-8",
                    include_str!("../../web/i18n.js"),
                )
            }),
        )
        .route(
            "/preset-data.js",
            get(|| async {
                asset(
                    "text/javascript; charset=utf-8",
                    include_str!("../../web/preset-data.js"),
                )
            }),
        )
        .route(
            "/api.js",
            get(|| async {
                asset(
                    "text/javascript; charset=utf-8",
                    include_str!("../../web/api.js"),
                )
            }),
        )
        .route(
            "/styles.css",
            get(|| async {
                asset(
                    "text/css; charset=utf-8",
                    include_str!("../../web/styles.css"),
                )
            }),
        )
        .layer(DefaultBodyLimit::max(32 * 1024))
        .with_state(state)
}
fn asset(content_type: &'static str, body: &'static str) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (
                header::CONTENT_SECURITY_POLICY,
                "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'",
            ),
        ],
        body,
    )
}
#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("generate-key") {
        access::generate_key();
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("validate-data") {
        let Some(path) = std::env::args_os().nth(2) else {
            eprintln!("Usage: echo_server validate-data <echo-data.json>");
            std::process::exit(2);
        };
        match echo_app::data::DataPack::from_file(std::path::Path::new(&path)) {
            Ok(pack) => println!(
                "Valid data pack {} ({})",
                pack.value["version"], pack.revision
            ),
            Err(error) => {
                eprintln!("{}", error.message);
                std::process::exit(1);
            }
        }
        return;
    }
    let address = std::env::var("ECHO_BIND").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let state = Scheduler::new(Settings::from_env(), Access::from_env());
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect("cannot bind ECHO_BIND");
    println!("Echo Studio listening on http://{address}");
    axum::serve(
        listener,
        app(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
    })
    .await
    .expect("HTTP server failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;
    #[tokio::test]
    async fn http_admission_auth_and_private_jobs() {
        let mut access = Access::from_env();
        access.hourly = [0; 3];
        access.hashes = vec![access::digest("test-key")];
        let router = app(Scheduler::new(Settings::from_env(), access));
        let make = |path: &str, key: Option<&str>, peer: &str, body: Option<String>| {
            let mut r = Request::builder()
                .uri(path)
                .extension(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
            if let Some(k) = key {
                r = r.header("authorization", format!("Bearer {k}"));
            }
            if body.is_some() {
                r = r.method("POST").header("content-type", "application/json");
            }
            r.body(Body::from(body.unwrap_or_default())).unwrap()
        };
        let body=serde_json::json!({"config":echo_app::Config{mode:echo_app::Mode::Upgrade,target:0.,..Default::default()},"echo":[]}).to_string();
        let r = router
            .clone()
            .oneshot(make("/api/solve", None, "1.1.1.1:123", Some(body.clone())))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(r.headers().contains_key(header::RETRY_AFTER));
        let r = router
            .clone()
            .oneshot(make(
                "/api/solve",
                Some("wrong"),
                "1.1.1.1:123",
                Some(body.clone()),
            ))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        let r = router
            .clone()
            .oneshot(make(
                "/api/solve",
                Some("test-key"),
                "1.1.1.1:123",
                Some(body),
            ))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::ACCEPTED);
        let data: serde_json::Value =
            serde_json::from_slice(&to_bytes(r.into_body(), 4096).await.unwrap()).unwrap();
        let path = format!("/api/jobs/{}", data["id"].as_str().unwrap());
        let r = router
            .clone()
            .oneshot(make(&path, None, "2.2.2.2:123", None))
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "no-store");
        for path in ["/", "/api.js", "/app.js", "/api/bootstrap", "/api/health"] {
            assert_eq!(
                router
                    .clone()
                    .oneshot(make(path, None, "1.1.1.1:123", None))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::OK
            );
        }
    }
}
