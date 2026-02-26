export function createEventHandlersOcrTabs({
  state,
  elements,
  ocrController,
  normalizeOcrPort,
  setActiveTab,
}) {
  function setupOcrAndTabHandlers() {
    elements.ocrToggleButton.addEventListener('click', () => {
      void ocrController.handleOcrToggle();
    });

    elements.ocrPortInput.addEventListener('change', () => {
      state.ocr.port = normalizeOcrPort(elements.ocrPortInput.valueAsNumber, state.ocr.port);
      state.ocr.lastError = null;
      ocrController.renderOcrPanel();
    });

    elements.tabUpgrade.addEventListener('click', async () => {
      await setActiveTab('upgrade');
    });

    elements.tabReroll.addEventListener('click', async () => {
      await setActiveTab('reroll');
    });
  }

  return {
    setupOcrAndTabHandlers,
  };
}
