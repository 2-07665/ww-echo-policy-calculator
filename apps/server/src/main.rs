use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use echo_app::{AppError, Service, SolveRequest};
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;

#[derive(Clone)]
struct Server {
    service: Arc<Mutex<Service>>,
    gate: Arc<Semaphore>,
}
fn error(status: StatusCode, e: AppError) -> Response {
    (status, Json(e)).into_response()
}
async fn solve(
    State(state): State<Server>,
    body: Result<Json<SolveRequest>, JsonRejection>,
) -> Response {
    let Json(request) = match body {
        Ok(r) => r,
        Err(e) => return error(StatusCode::BAD_REQUEST, AppError::input(e.body_text())),
    };
    let permit = match state.gate.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => return error(StatusCode::TOO_MANY_REQUESTS, AppError::busy()),
    };
    // Keep admission held inside the blocking task even if the HTTP client disconnects.
    match tokio::task::spawn_blocking(move || {
        let _permit = permit;
        state
            .service
            .lock()
            .map_err(|_| AppError::input("求解服务需要重启。"))?
            .solve(request)
    })
    .await
    {
        Ok(Ok(result)) => Json(result).into_response(),
        Ok(Err(e)) => error(StatusCode::UNPROCESSABLE_ENTITY, e),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            AppError::input("求解任务异常结束。"),
        ),
    }
}
async fn preview(body: Result<Json<SolveRequest>, JsonRejection>) -> Response {
    match body {
        Ok(Json(r)) => match echo_app::preview(&r) {
            Ok(v) => Json(v).into_response(),
            Err(e) => error(StatusCode::UNPROCESSABLE_ENTITY, e),
        },
        Err(e) => error(StatusCode::BAD_REQUEST, AppError::input(e.body_text())),
    }
}
fn app(service: Service) -> Router {
    let state = Server {
        service: Arc::new(Mutex::new(service)),
        gate: Arc::new(Semaphore::new(1)),
    };
    Router::new()
        .route(
            "/api/health",
            get(|| async {
                Json(serde_json::json!({"status":"ok","version":env!("CARGO_PKG_VERSION")}))
            }),
        )
        .route(
            "/api/bootstrap",
            get(|| async { Json(echo_app::bootstrap()) }),
        )
        .route("/api/preview", post(preview))
        .route("/api/solve", post(solve))
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
    let address = std::env::var("ECHO_BIND").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let mut limits = echo_app::Service::default();
    if std::env::var_os("ECHO_MAX_STATES").is_some()
        || std::env::var_os("ECHO_MAX_TRANSITIONS").is_some()
    {
        let number = |name, default| {
            std::env::var(name)
                .map(|s| {
                    s.parse::<usize>()
                        .expect("solver limits must be positive integers")
                })
                .unwrap_or(default)
        };
        let states = number("ECHO_MAX_STATES", 750_000);
        let transitions = number("ECHO_MAX_TRANSITIONS", 24_000_000);
        assert!(
            states > 0 && transitions > 0,
            "solver limits must be positive"
        );
        limits = echo_app::service_with_limits(states, transitions);
    }
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .expect("cannot bind ECHO_BIND");
    println!("Echo Studio listening on http://{address}");
    axum::serve(listener, app(limits))
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
    async fn http_contract_and_embedded_frontend() {
        let app = app(Service::default());
        for path in [
            "/",
            "/app.js",
            "/styles.css",
            "/api.js",
            "/i18n.js",
            "/preset-data.js",
            "/api/bootstrap",
            "/api/health",
        ] {
            let r = app
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(r.status(), StatusCode::OK, "{path}");
        }
        let r = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/solve")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        let bytes = to_bytes(r.into_body(), 4096).await.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["code"],
            "invalid_input"
        );
        let config = echo_app::Config {
            target: 0.,
            mode: echo_app::Mode::Upgrade,
            ..Default::default()
        };
        let r = app
            .oneshot(
                Request::builder()
                    .uri("/api/solve")
                    .method("POST")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"config":config,"echo":[]}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
    }
    #[tokio::test]
    async fn refuses_work_when_worker_busy() {
        let state = Server {
            service: Arc::new(Mutex::new(Service::default())),
            gate: Arc::new(Semaphore::new(1)),
        };
        let _permit = state.gate.acquire().await.unwrap();
        let result = solve(
            State(state.clone()),
            Ok(Json(SolveRequest {
                config: echo_app::Config::default(),
                echo: vec![],
                candidate: None,
            })),
        )
        .await;
        assert_eq!(result.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
