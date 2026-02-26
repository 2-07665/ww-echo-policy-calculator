export function createOcrController({
  state,
  elements,
  invoke,
  normalizeOcrPort,
  numberOr,
  defaultOcrUdpPort,
  eventFillEntries,
  eventListenerStatus,
  computeContributions,
  updateSuggestion,
}) {
  let ocrFillRequestToken = 0;

  function renderOcrPanel() {
    if (!elements.ocrStatusText || !elements.ocrPortInput || !elements.ocrToggleButton) {
      return;
    }

    const listening = Boolean(state.ocr.listening);
    elements.ocrStatusText.textContent = listening ? '监听中' : '未监听';
    elements.ocrStatusText.classList.toggle('listening', listening);

    elements.ocrPortInput.value = String(normalizeOcrPort(state.ocr.port, defaultOcrUdpPort));
    const loading = elements.ocrToggleButton.dataset.loading === 'true';
    elements.ocrPortInput.disabled = listening || loading;

    elements.ocrToggleButton.textContent = listening ? '停止监听' : '开始监听';
    elements.ocrToggleButton.disabled = loading;

    if (elements.ocrError) {
      const message = String(state.ocr.lastError || '').trim();
      elements.ocrError.hidden = !message;
      elements.ocrError.textContent = message;
    }
  }

  function applyOcrStatusPayload(payload) {
    const parsedPort =
      payload?.port == null
        ? normalizeOcrPort(state.ocr.port, defaultOcrUdpPort)
        : normalizeOcrPort(payload.port, state.ocr.port);
    state.ocr.listening = Boolean(payload?.listening);
    state.ocr.port = parsedPort;
    state.ocr.lastError = payload?.lastError ? String(payload.lastError) : null;
    renderOcrPanel();
  }

  async function refreshOcrListenerStatus() {
    try {
      const response = await invoke('get_ocr_udp_listener_status');
      applyOcrStatusPayload(response);
    } catch (error) {
      state.ocr.listening = false;
      state.ocr.lastError = `读取 OCR 状态失败：${error?.message || error}`;
      renderOcrPanel();
    }
  }

  async function handleOcrToggle() {
    if (!elements.ocrToggleButton || elements.ocrToggleButton.dataset.loading === 'true') {
      return;
    }

    elements.ocrToggleButton.dataset.loading = 'true';
    renderOcrPanel();

    try {
      const listening = Boolean(state.ocr.listening);
      if (listening) {
        const response = await invoke('stop_ocr_udp_listener');
        applyOcrStatusPayload(response);
      } else {
        state.ocr.port = normalizeOcrPort(elements.ocrPortInput?.valueAsNumber, state.ocr.port);
        const response = await invoke('start_ocr_udp_listener', {
          payload: {
            port: state.ocr.port,
          },
        });
        applyOcrStatusPayload(response);
      }
    } catch (error) {
      state.ocr.lastError = `OCR 监听操作失败：${error?.message || error}`;
      renderOcrPanel();
    } finally {
      elements.ocrToggleButton.dataset.loading = 'false';
      renderOcrPanel();
    }
  }

  async function applyIncomingOcrEntries(payload) {
    const buffNames = Array.isArray(payload?.buffNames) ? payload.buffNames : [];
    const buffValues = Array.isArray(payload?.buffValues) ? payload.buffValues : [];
    if (!buffNames.length || buffNames.length !== buffValues.length) {
      return;
    }

    const token = ++ocrFillRequestToken;
    const nextSelections = Array(state.maxSelectedTypes).fill(null);
    const nextValues = Array(state.maxSelectedTypes).fill(null);

    for (let i = 0; i < Math.min(state.maxSelectedTypes, buffNames.length); i += 1) {
      const buffName = String(buffNames[i] || '');
      if (!state.buffTypes.includes(buffName)) {
        continue;
      }
      const rawValue = Math.max(0, Math.round(numberOr(buffValues[i], 0)));
      const allowedValues = state.buffValueOptions.get(buffName) || [];
      if (!allowedValues.includes(rawValue)) {
        continue;
      }
      nextSelections[i] = buffName;
      nextValues[i] = rawValue;
    }

    state.buffSelections = nextSelections;
    state.buffValues = nextValues;
    await computeContributions();
    if (token !== ocrFillRequestToken) {
      return;
    }
    if (state.policyReady) {
      await updateSuggestion();
    }
  }

  async function setupTauriEventListeners() {
    const eventApi = window.__TAURI__?.event;
    if (!eventApi || typeof eventApi.listen !== 'function') {
      return;
    }

    await eventApi.listen(eventFillEntries, (event) => {
      void applyIncomingOcrEntries(event?.payload);
    });
    await eventApi.listen(eventListenerStatus, (event) => {
      applyOcrStatusPayload(event?.payload);
    });
  }

  return {
    renderOcrPanel,
    applyOcrStatusPayload,
    refreshOcrListenerStatus,
    handleOcrToggle,
    setupTauriEventListeners,
  };
}
