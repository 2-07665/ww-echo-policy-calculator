#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod ocr;
use echo_app::{AppError, Preview, Service, SolveRequest, SolveResponse};
use std::sync::{Arc, Mutex};

#[tauri::command]
fn bootstrap() -> serde_json::Value {
    echo_app::bootstrap()
}
#[tauri::command]
fn preview(payload: SolveRequest) -> Result<Preview, AppError> {
    echo_app::preview(&payload)
}
#[tauri::command]
async fn solve(
    state: tauri::State<'_, Arc<Mutex<Service>>>,
    payload: SolveRequest,
) -> Result<SolveResponse, AppError> {
    let service = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .try_lock()
            .map_err(|_| AppError::busy())?
            .solve(payload)
    })
    .await
    .map_err(|_| AppError::input("求解任务异常结束。"))?
}
fn main() {
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(Service::default())))
        .manage(ocr::Listener::default())
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            preview,
            solve,
            ocr::toggle_ocr
        ])
        .run(tauri::generate_context!())
        .expect("error while running Echo Studio");
}
