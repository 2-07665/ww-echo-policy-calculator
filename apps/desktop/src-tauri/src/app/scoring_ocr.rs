fn buff_index(buff_name: &str) -> Option<usize> {
    BUFF_TYPES.iter().position(|name| *name == buff_name)
}

fn parse_ocr_udp_payload(raw_message: &str) -> Result<OcrFillEntriesEvent, String> {
    let payload: OcrUdpPayload =
        serde_json::from_str(raw_message).map_err(|err| format!("Invalid JSON payload: {err}"))?;
    if payload.buff_entries.is_empty() {
        return Err("buffEntries cannot be empty".to_string());
    }
    if payload.buff_entries.len() > MAX_SELECTED_TYPES {
        return Err(format!(
            "Too many buffEntries: {}, max is {MAX_SELECTED_TYPES}",
            payload.buff_entries.len()
        ));
    }

    let mut seen = [false; NUM_BUFFS];
    let mut buff_names = Vec::with_capacity(payload.buff_entries.len());
    let mut buff_values = Vec::with_capacity(payload.buff_entries.len());

    for (entry_idx, entry) in payload.buff_entries.iter().enumerate() {
        let buff_name = entry.buff_name.trim();
        let buff_idx = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff in buffEntries[{entry_idx}]: {buff_name}"))?;
        if seen[buff_idx] {
            return Err(format!(
                "Duplicate buff in buffEntries: {}",
                BUFF_TYPES[buff_idx]
            ));
        }
        if !BUFF_VALUE_OPTIONS[buff_idx].contains(&entry.buff_value) {
            return Err(format!(
                "Invalid value {} for buff {}",
                entry.buff_value, BUFF_TYPES[buff_idx]
            ));
        }

        seen[buff_idx] = true;
        buff_names.push(BUFF_TYPES[buff_idx].to_string());
        buff_values.push(entry.buff_value);
    }

    Ok(OcrFillEntriesEvent {
        buff_names,
        buff_values,
    })
}

fn ocr_listener_status_snapshot(state: &OcrUdpListenerState) -> OcrListenerStatusResponse {
    OcrListenerStatusResponse {
        listening: state.session.is_some(),
        port: state.session.as_ref().map(|session| session.port),
        last_error: state.last_error.clone(),
    }
}

fn emit_ocr_listener_status_event(app: &tauri::AppHandle, status: &OcrListenerStatusResponse) {
    if let Err(err) = app.emit(OCR_UDP_EVENT_LISTENER_STATUS, status.clone()) {
        eprintln!("Failed to emit OCR listener status event: {err}");
    }
}

fn stop_ocr_udp_session(session: OcrUdpListenerSession) -> Result<(), String> {
    session.stop_flag.store(true, Ordering::Relaxed);
    session
        .join_handle
        .join()
        .map_err(|_| "OCR UDP listener thread panicked".to_string())
}

fn run_ocr_udp_listener_loop(app: tauri::AppHandle, socket: UdpSocket, stop_flag: Arc<AtomicBool>) {
    let mut buffer = [0u8; OCR_UDP_PACKET_BUFFER_SIZE];
    while !stop_flag.load(Ordering::Relaxed) {
        match socket.recv_from(&mut buffer) {
            Ok((size, source)) => {
                let message = match std::str::from_utf8(&buffer[..size]) {
                    Ok(text) => text,
                    Err(err) => {
                        eprintln!("Ignoring OCR UDP packet from {source}: invalid UTF-8 ({err})");
                        continue;
                    }
                };
                match parse_ocr_udp_payload(message) {
                    Ok(fill_event) => {
                        if let Err(err) = app.emit(OCR_UDP_EVENT_FILL_ENTRIES, fill_event) {
                            eprintln!("Failed to emit OCR fill event: {err}");
                        }
                    }
                    Err(err) => {
                        eprintln!("Ignoring OCR UDP packet from {source}: {err}");
                    }
                }
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut => {}
            Err(err) => {
                eprintln!("OCR UDP listener receive error: {err}");
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

