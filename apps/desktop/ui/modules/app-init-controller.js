export function createAppInitController({
  state,
  elements,
  invoke,
  escapeHtml,
  cacheElements,
  initialiseState,
  renderScorerConfig,
  renderWeightInputs,
  renderPresetControls,
  ocrController,
  setupEventHandlers,
  loadScorerPresetsForType,
  updateTargetScoreUI,
  updateRerollTargetScoreUI,
  renderRerollSlots,
  updateComputeButtonState,
  renderResults,
  renderRerollOutput,
  computeContributions,
  setActiveTab,
}) {
  async function init() {
    cacheElements();

    try {
      const data = await invoke('bootstrap');
      initialiseState(data);

      renderScorerConfig();
      renderWeightInputs();
      renderPresetControls();
      ocrController.renderOcrPanel();
      setupEventHandlers();
      await ocrController.setupTauriEventListeners();
      await loadScorerPresetsForType(state.scorerType);
      await ocrController.refreshOcrListenerStatus();

      updateTargetScoreUI();
      updateRerollTargetScoreUI();

      renderRerollSlots();
      updateComputeButtonState();
      renderResults();
      renderRerollOutput();
      await computeContributions();
      await setActiveTab('upgrade');
    } catch (error) {
      elements.resultsSection.innerHTML = `
        <div class="error-message">初始化失败：${escapeHtml(error?.message || error)}</div>
      `;
    }
  }

  return {
    init,
  };
}
