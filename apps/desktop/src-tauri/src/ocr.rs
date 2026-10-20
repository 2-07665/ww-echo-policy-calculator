//! CPU sidecar lifecycle and the compatible loopback OCR packet adapter.
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Write},
    net::UdpSocket,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Packet {
    buff_entries: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    buff_name: String,
    buff_value: u16,
}

struct Session {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    output: Option<JoinHandle<()>>,
    child: Option<Arc<Mutex<Child>>>,
    _job: Option<ChildJob>,
}
// The OS closes this non-inherited handle on parent crashes as well as normal exit.
// Store the handle as an integer so the owned guard can cross worker threads.
struct ChildJob(isize);
impl ChildJob {
    #[cfg(windows)]
    fn assign(child: &Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err("无法创建 OCR 进程作业。".into());
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of_val(&info) as u32,
            ) != 0;
            if !configured || AssignProcessToJobObject(handle, child.as_raw_handle()) == 0 {
                CloseHandle(handle);
                return Err("无法绑定 OCR 进程作业。".into());
            }
            Ok(Self(handle as isize))
        }
    }
    #[cfg(not(windows))]
    fn assign(_: &Child) -> Result<Self, String> {
        Err("内置 OCR 仅支持 Windows。".into())
    }
}
impl Drop for ChildJob {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0 as _);
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(child) = &self.child {
            if let Ok(mut child) = child.lock() {
                if let Some(mut input) = child.stdin.take() {
                    let _ = input.write_all(b"q\n");
                }
                let deadline = Instant::now() + Duration::from_secs(4);
                loop {
                    match child.try_wait() {
                        Ok(Some(_)) => break,
                        _ if Instant::now() >= deadline => {
                            let _ = child.kill();
                            let _ = child.wait();
                            break;
                        }
                        _ => thread::sleep(Duration::from_millis(50)),
                    }
                }
            }
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        if let Some(t) = self.output.take() {
            let _ = t.join();
        }
    }
}
#[derive(Default)]
pub struct Listener {
    session: Mutex<Option<Session>>,
    generation: Arc<AtomicU64>,
}
impl Listener {
    pub fn shutdown(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        if let Ok(mut session) = self.session.lock() {
            *session = None;
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct OcrWindow {
    id: String,
    title: String,
}
#[derive(Serialize)]
pub struct OcrCapabilities {
    native_supported: bool,
    native_available: bool,
}
fn native_supported() -> bool {
    cfg!(all(target_os = "windows", target_arch = "x86_64"))
}
#[tauri::command]
pub fn ocr_capabilities(app: tauri::AppHandle) -> OcrCapabilities {
    OcrCapabilities {
        native_supported: native_supported(),
        native_available: native_supported() && sidecar(&app).is_ok(),
    }
}

fn parse(bytes: &[u8]) -> Result<echo_app::Echo, String> {
    let packet: Packet = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if packet.buff_entries.is_empty() {
        return Err("empty packet".into());
    }
    let echo = packet
        .buff_entries
        .iter()
        .map(|e| {
            echo_app::metadata::BUFF_TYPES
                .iter()
                .position(|name| *name == e.buff_name.trim())
                .map(|i| (i, e.buff_value))
                .ok_or_else(|| "unknown stat".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    echo_app::validate_echo(&echo, false).map_err(|e| e.message)?;
    Ok(echo)
}
fn sidecar(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let mut candidates = vec![];
    if let Ok(path) = std::env::current_exe() {
        if let Some(parent) = path.parent() {
            candidates.push(parent.join("ocr/echo-ocr.exe"));
        }
    }
    if let Ok(path) = app.path().resource_dir() {
        candidates.push(path.join("ocr/echo-ocr.exe"));
    }
    #[cfg(debug_assertions)]
    candidates
        .push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ocr/runtime/echo-ocr.exe"));
    let executable = candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| "OCR 运行文件缺失，请先准备并构建 OCR 组件。".to_string())?;
    let directory = executable.parent().ok_or("OCR 资源路径无效。")?;
    for name in [
        "inference.onnx",
        "dictionary.txt",
        "onnxruntime.dll",
        "msvcp140.dll",
        "msvcp140_1.dll",
        "vcruntime140.dll",
        "vcruntime140_1.dll",
    ] {
        if !directory.join(name).is_file() {
            return Err(format!("OCR 运行文件缺失：{name}，请重新准备 OCR 组件。"));
        }
    }
    Ok(executable)
}
fn hidden_command(path: PathBuf) -> Command {
    let mut command = Command::new(path);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
#[tauri::command]
pub async fn ocr_windows(app: tauri::AppHandle) -> Result<Vec<OcrWindow>, String> {
    tauri::async_runtime::spawn_blocking(move || list_windows(&app))
        .await
        .map_err(|_| "OCR 窗口查询异常。".to_string())?
}
fn list_windows(app: &tauri::AppHandle) -> Result<Vec<OcrWindow>, String> {
    if !native_supported() {
        return Err("内置 OCR 仅支持 Windows x64。".into());
    }
    let output = hidden_command(sidecar(&app)?)
        .arg("--windows")
        .stdin(Stdio::null())
        .output()
        .map_err(|_| "无法查询鸣潮窗口。".to_string())?;
    if !output.status.success() {
        return Err("无法查询鸣潮窗口。".into());
    }
    serde_json::from_slice(&output.stdout).map_err(|_| "OCR 窗口列表格式错误。".into())
}
#[tauri::command]
pub async fn toggle_ocr(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Listener>>,
    enabled: bool,
    mode: Option<String>,
    window_id: Option<String>,
) -> Result<bool, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || toggle(app, &state, enabled, mode, window_id))
        .await
        .map_err(|_| "OCR 任务异常。".to_string())?
}
fn toggle(
    app: tauri::AppHandle,
    state: &Listener,
    enabled: bool,
    mode: Option<String>,
    window_id: Option<String>,
) -> Result<bool, String> {
    let mut listener = state
        .session
        .lock()
        .map_err(|_| "OCR listener unavailable")?;
    if !enabled {
        state.generation.fetch_add(1, Ordering::AcqRel);
        *listener = None;
        return Ok(false);
    }
    if listener
        .as_ref()
        .is_some_and(|s| !s.stop.load(Ordering::Acquire))
    {
        return Ok(true);
    }
    *listener = None;
    // Calls made against the original UDP-only API must remain dependency-free.
    let native = match mode.as_deref().unwrap_or("external") {
        "native" => true,
        "external" => false,
        _ => return Err("未知 OCR 模式。".into()),
    };
    if native && !native_supported() {
        return Err("内置 OCR 仅支持 Windows x64。".into());
    }
    let id = if native {
        window_id.filter(|s| !s.is_empty())
    } else {
        None
    };
    if let Some(id) = &id {
        if !list_windows(&app)?.iter().any(|w| &w.id == id) {
            return Err("选择的鸣潮窗口已失效，请刷新窗口列表。".into());
        }
    }
    let socket =
        UdpSocket::bind("127.0.0.1:9999").map_err(|e| format!("无法监听 UDP 9999：{e}"))?;
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|e| e.to_string())?;
    let generation = state.generation.fetch_add(1, Ordering::AcqRel) + 1;
    let current = state.generation.clone();
    let stop = Arc::new(AtomicBool::new(false));
    let mut session = Session {
        stop: stop.clone(),
        thread: None,
        output: None,
        child: None,
        _job: None,
    };
    if native {
        let mut command = hidden_command(sidecar(&app)?);
        if let Some(id) = id {
            command.args(["--hwnd", &id]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("无法启动 OCR：{e}"))?;
        match ChildJob::assign(&child) {
            Ok(job) => session._job = Some(job),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        }
        let output = child.stdout.take().ok_or("OCR 输出通道不可用")?;
        session.child = Some(Arc::new(Mutex::new(child)));
        let (tx, rx) = mpsc::channel::<Result<(), String>>();
        let app = app.clone();
        let stop = stop.clone();
        let current = current.clone();
        session.output = Some(thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };
                let code = value["status"].as_str().unwrap_or("").to_owned();
                if code == "ready" {
                    let _ = tx.send(Ok(()));
                }
                if code == "error" {
                    let _ = tx.send(Err(value["detail"]
                        .as_str()
                        .unwrap_or("OCR 启动失败")
                        .to_owned()));
                }
                if stop.load(Ordering::Acquire) || current.load(Ordering::Acquire) != generation {
                    continue;
                }
                value["enabled"] = serde_json::json!(code != "error");
                let _ = app.emit("ocr-status", value);
            }
            if !stop.swap(true, Ordering::AcqRel) && current.load(Ordering::Acquire) == generation {
                let _ = app.emit(
                    "ocr-status",
                    serde_json::json!({"status":"exited","enabled":false}),
                );
            }
        }));
        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(())) => {}
            Ok(Err(error)) => return Err(format!("OCR 启动失败：{error}")),
            Err(_) => return Err("OCR 启动超时或运行库缺失，请重新准备 OCR 组件。".into()),
        }
    }
    let flag = stop.clone();
    session.thread = Some(thread::spawn(move || {
        let mut buffer = [0; 16 * 1024];
        while !flag.load(Ordering::Acquire) && current.load(Ordering::Acquire) == generation {
            if let Ok(n) = socket.recv(&mut buffer) {
                if let Ok(echo) = parse(&buffer[..n]) {
                    if flag.load(Ordering::Acquire) || current.load(Ordering::Acquire) != generation
                    {
                        break;
                    }
                    let _ = app.emit("ocr-echo", &echo);
                }
            }
        }
    }));
    *listener = Some(session);
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn closing_owned_job_terminates_child() {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{
            Foundation::WAIT_OBJECT_0, System::Threading::WaitForSingleObject,
        };
        let mut child = hidden_command(PathBuf::from("cmd.exe"))
            .args(["/d", "/c", "ping -n 30 127.0.0.1 >nul"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let job = ChildJob::assign(&child).unwrap();
        drop(job);
        assert_eq!(
            unsafe { WaitForSingleObject(child.as_raw_handle(), 2000) },
            WAIT_OBJECT_0
        );
        let _ = child.wait();
    }
    #[test]
    fn reads_existing_packets_and_rejects_invalid_values() {
        assert_eq!(
            parse(br#"{"buffEntries":[{"buffName":"Crit_Rate","buffValue":63}]}"#).unwrap(),
            vec![(0, 63)]
        );
        for value in [
            r#"{"buffEntries":[{"buffName":"Crit_Rate","buffValue":64}]}"#,
            r#"{"buffEntries":[]}"#,
            r#"{"buffEntries":[{"buffName":"Crit_Rate","buffValue":63},{"buffName":"Crit_Rate","buffValue":69}]}"#,
            r#"{"buffEntries":[{"buffName":"Attack_Flat","buffValue":150}]}"#,
            r#"{"buffEntries":[{"buffName":"Crit_Rate","buffValue":63}],"extra":1}"#,
        ] {
            assert!(parse(value.as_bytes()).is_err(), "{value}");
        }
    }
}
