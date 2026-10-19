//! Desktop-only loopback adapter for the existing ok-ww OCR packet format.
use serde::Deserialize;
use std::{
    net::UdpSocket,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tauri::Emitter;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Packet {
    buff_entries: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    buff_name: String,
    buff_value: u16,
}
struct Session {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
#[derive(Default)]
pub struct Listener(Mutex<Option<Session>>);
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
#[tauri::command]
pub fn toggle_ocr(
    app: tauri::AppHandle,
    state: tauri::State<'_, Listener>,
    enabled: bool,
) -> Result<bool, String> {
    let mut listener = state.0.lock().map_err(|_| "OCR listener unavailable")?;
    if !enabled {
        *listener = None;
        return Ok(false);
    }
    if listener.is_some() {
        return Ok(true);
    }
    let socket =
        UdpSocket::bind("127.0.0.1:9999").map_err(|e| format!("无法监听 UDP 9999：{e}"))?;
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .map_err(|e| e.to_string())?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let handle = thread::spawn(move || {
        let mut buffer = [0; 16 * 1024];
        while !flag.load(Ordering::Relaxed) {
            match socket.recv(&mut buffer) {
                Ok(n) => {
                    if let Ok(echo) = parse(&buffer[..n]) {
                        let _ = app.emit("ocr-echo", echo);
                    }
                }
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(_) => thread::sleep(Duration::from_millis(100)),
            }
        }
    });
    *listener = Some(Session {
        stop,
        thread: Some(handle),
    });
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_existing_packets_and_rejects_invalid_values() {
        assert_eq!(
            parse(br#"{"buffEntries":[{"buffName":"Crit_Rate","buffValue":63}]}"#).unwrap(),
            vec![(0, 63)]
        );
        assert!(parse(br#"{"buffEntries":[{"buffName":"Crit_Rate","buffValue":64}]}"#).is_err());
        assert!(parse(br#"{"buffEntries":[]}"#).is_err());
    }
}
