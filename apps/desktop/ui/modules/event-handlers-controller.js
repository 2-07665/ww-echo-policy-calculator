import { createEventHandlersOcrTabs } from './event-handlers-ocr-tabs.js';
import { createEventHandlersReroll } from './event-handlers-reroll.js';
import { createEventHandlersUpgrade } from './event-handlers-upgrade.js';

export function createEventHandlersController({
  state,
  elements,
  ocrController,
  numberOr,
  normalizeOcrPort,
  targetScoreDigits,
  targetScoreStep,
  scorerPresetCustom,
  isFixedScorer,
  isMcBoostAssistantScorer,
  isQqBotScorer,
  isFullUniqueSelection,
  getMainBuffScore,
  getScorerConfig,
  roundToStep,
  setActiveTab,
  applyScorerType,
  applySelectedScorerPreset,
  applySelectedScorerPresetVariant,
  handleSaveCurrentPreset,
  handleDeleteCurrentPreset,
  handleSaveCurrentPresetVariant,
  handleDeleteCurrentPresetVariant,
  updateTargetScoreUI,
  updateRerollTargetScoreUI,
  resetPolicyResult,
  onScorerParamsUpdated,
  computeContributions,
  updateSuggestion,
  handleCompute,
  handleRerollCompute,
  invalidateRerollPolicy,
  renderRerollSlots,
  onRerollSelectionChanged,
}) {
  const eventHandlersOcrTabs = createEventHandlersOcrTabs({
    state,
    elements,
    ocrController,
    normalizeOcrPort,
    setActiveTab,
  });

  const eventHandlersUpgrade = createEventHandlersUpgrade({
    state,
    elements,
    numberOr,
    targetScoreDigits,
    targetScoreStep,
    scorerPresetCustom,
    isFixedScorer,
    isMcBoostAssistantScorer,
    isQqBotScorer,
    getMainBuffScore,
    getScorerConfig,
    roundToStep,
    applyScorerType,
    applySelectedScorerPreset,
    applySelectedScorerPresetVariant,
    handleSaveCurrentPreset,
    handleDeleteCurrentPreset,
    handleSaveCurrentPresetVariant,
    handleDeleteCurrentPresetVariant,
    updateTargetScoreUI,
    resetPolicyResult,
    onScorerParamsUpdated,
    computeContributions,
    updateSuggestion,
    handleCompute,
  });

  const eventHandlersReroll = createEventHandlersReroll({
    state,
    elements,
    numberOr,
    isFullUniqueSelection,
    updateRerollTargetScoreUI,
    invalidateRerollPolicy,
    handleRerollCompute,
    renderRerollSlots,
    onRerollSelectionChanged,
  });

  function setupEventHandlers() {
    eventHandlersOcrTabs.setupOcrAndTabHandlers();
    eventHandlersUpgrade.setupUpgradeHandlers();
    eventHandlersReroll.setupRerollHandlers();
  }

  return {
    setupEventHandlers,
  };
}
