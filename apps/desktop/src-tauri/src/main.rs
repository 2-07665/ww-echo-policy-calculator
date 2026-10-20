#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod data;
mod ocr;
use echo_app::{AppError, Preview, Service, SolveRequest, SolveResponse};
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[tauri::command]
fn bootstrap(state: tauri::State<'_, Arc<echo_app::data::DataStore>>) -> serde_json::Value {
    let (pack, status) = state.snapshot();
    echo_app::bootstrap_with_data(&pack, status)
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
        .manage(Arc::new(ocr::Listener::default()))
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                window.state::<Arc<ocr::Listener>>().shutdown();
            }
        })
        .setup(|app| {
            let path = app.path().app_data_dir()?.join("echo-data.json");
            app.manage(Arc::new(echo_app::data::DataStore::cached(path)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            data::data_download,
            data::data_install,
            preview,
            solve,
            ocr::toggle_ocr,
            ocr::ocr_windows,
            ocr::ocr_capabilities
        ])
        .run(tauri::generate_context!())
        .expect("error while running Echo Studio");
}
