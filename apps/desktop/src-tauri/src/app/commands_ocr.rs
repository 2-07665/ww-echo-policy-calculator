#[tauri::command]
fn get_ocr_udp_listener_status(
    state: State<'_, AppState>,
) -> Result<OcrListenerStatusResponse, String> {
    let listener = state
        .ocr_udp_listener
        .lock()
        .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
    Ok(ocr_listener_status_snapshot(&listener))
}

#[tauri::command]
fn start_ocr_udp_listener(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    payload: StartOcrUdpListenerRequest,
) -> Result<OcrListenerStatusResponse, String> {
    if payload.port == 0 {
        return Err("port must be between 1 and 65535".to_string());
    }

    {
        let listener = state
            .ocr_udp_listener
            .lock()
            .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
        if let Some(session) = listener.session.as_ref()
            && session.port == payload.port
        {
            let status = ocr_listener_status_snapshot(&listener);
            emit_ocr_listener_status_event(&app, &status);
            return Ok(status);
        }
    }

    let socket = UdpSocket::bind(("127.0.0.1", payload.port))
        .map_err(|err| format!("Failed to bind UDP port {}: {err}", payload.port))?;
    socket
        .set_read_timeout(Some(Duration::from_millis(OCR_UDP_READ_TIMEOUT_MS)))
        .map_err(|err| format!("Failed to configure UDP socket timeout: {err}"))?;

    let previous_session = {
        let mut listener = state
            .ocr_udp_listener
            .lock()
            .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
        listener.session.take()
    };
    if let Some(session) = previous_session {
        stop_ocr_udp_session(session)?;
    }

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_for_thread = Arc::clone(&stop_flag);
    let app_for_thread = app.clone();
    let listener_thread = thread::spawn(move || {
        run_ocr_udp_listener_loop(app_for_thread, socket, stop_flag_for_thread)
    });

    let status = {
        let mut listener = state
            .ocr_udp_listener
            .lock()
            .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
        listener.last_error = None;
        listener.session = Some(OcrUdpListenerSession {
            port: payload.port,
            stop_flag,
            join_handle: listener_thread,
        });
        ocr_listener_status_snapshot(&listener)
    };
    emit_ocr_listener_status_event(&app, &status);
    Ok(status)
}

#[tauri::command]
fn stop_ocr_udp_listener(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<OcrListenerStatusResponse, String> {
    let previous_session = {
        let mut listener = state
            .ocr_udp_listener
            .lock()
            .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
        listener.session.take()
    };

    let stop_error = if let Some(session) = previous_session {
        stop_ocr_udp_session(session).err()
    } else {
        None
    };

    let status = {
        let mut listener = state
            .ocr_udp_listener
            .lock()
            .map_err(|_| "Failed to lock OCR UDP listener state".to_string())?;
        listener.last_error = stop_error;
        ocr_listener_status_snapshot(&listener)
    };
    emit_ocr_listener_status_event(&app, &status);
    Ok(status)
}

