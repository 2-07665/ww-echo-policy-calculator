import {
  DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE,
  DEFAULT_OCR_UDP_PORT,
  DEFAULT_QQ_BOT_TARGET_SCORE,
  MC_BOOST_ASSISTANT_LOCKED_MAIN_BUFF_SCORE,
  MC_BOOST_ASSISTANT_LOCKED_NORMALIZED_MAX_SCORE,
  OCR_UDP_EVENT_FILL_ENTRIES,
  OCR_UDP_EVENT_LISTENER_STATUS,
  PLACEHOLDER_LABEL,
  QQ_BOT_LOCKED_NORMALIZED_MAX_SCORE,
  SCORER_FIXED,
  SCORER_LINEAR_DEFAULT,
  SCORER_MC_BOOST_ASSISTANT,
  SCORER_PRESET_CUSTOM,
  SCORER_PRESET_VARIANT_DEFAULT,
  SCORER_QQ_BOT,
  SCORER_TYPES,
  SCORER_WUWA_ECHO_TOOL,
  TARGET_SCORE_DIGITS,
  TARGET_SCORE_STEP,
  createScorerConfigMap,
  createScorerTypeMap,
} from './constants.js';
import { createOcrController } from './ocr-controller.js';
import { createAppInitController } from './app-init-controller.js';
import { createEventHandlersController } from './event-handlers-controller.js';
import { setHelpTooltip } from './help-tooltip.js';
import { createModeFlowController } from './mode-flow-controller.js';
import { createPolicyController } from './policy-controller.js';
import { createPresetsController } from './presets-controller.js';
import { createRerollController } from './reroll-controller.js';
import { createInitialAppState } from './app-state-factory.js';
import { cacheDomElements } from './dom-cache.js';
import { createScorerStateController } from './scorer-state-controller.js';
import { createScorerConfigCopyHelpers } from './scorer-config-copy.js';
import { createScorerPayloadBuilder } from './scorer-payload-builder.js';
import { initialiseAppState } from './state-bootstrap.js';
import { createTargetScoreController } from './target-score-controller.js';
import { createUpgradeScoreController } from './upgrade-score-controller.js';
import { createUpgradeUiController } from './upgrade-ui-controller.js';
import { invoke } from './tauri-api.js';
import { escapeHtml, formatFixedOr, normalizeOcrPort, numberOr } from './utils.js';

(() => {
  const state = createInitialAppState({
    createScorerConfigMap,
    createScorerTypeMap,
    scorerLinearDefault: SCORER_LINEAR_DEFAULT,
    scorerPresetCustom: SCORER_PRESET_CUSTOM,
    defaultMcBoostAssistantTargetScore: DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE,
    defaultQqBotTargetScore: DEFAULT_QQ_BOT_TARGET_SCORE,
    defaultOcrUdpPort: DEFAULT_OCR_UDP_PORT,
  });

  const elements = {};

  const {
    normalizeScorerType,
    isFixedScorer,
    isQqBotScorer,
    isMcBoostAssistantScorer,
    getScorerConfig,
    getWeightMap,
    getMainBuffScore,
    getNormalizedMaxScore,
    effectiveWeightForBuff,
    formatScoreForScorer,
  } = createScorerStateController({
    state,
    numberOr,
    targetScoreDigits: TARGET_SCORE_DIGITS,
    targetScoreStep: TARGET_SCORE_STEP,
    scorerLinearDefault: SCORER_LINEAR_DEFAULT,
    scorerWuwaEchoTool: SCORER_WUWA_ECHO_TOOL,
    scorerMcBoostAssistant: SCORER_MC_BOOST_ASSISTANT,
    scorerQqBot: SCORER_QQ_BOT,
    scorerFixed: SCORER_FIXED,
    mcBoostAssistantLockedMainBuffScore: MC_BOOST_ASSISTANT_LOCKED_MAIN_BUFF_SCORE,
    mcBoostAssistantLockedNormalizedMaxScore: MC_BOOST_ASSISTANT_LOCKED_NORMALIZED_MAX_SCORE,
    qqBotLockedNormalizedMaxScore: QQ_BOT_LOCKED_NORMALIZED_MAX_SCORE,
  });

  const cacheElements = () => cacheDomElements(elements);

  const { copyWeightMap, copyScorerConfig } = createScorerConfigCopyHelpers({
    state,
    numberOr,
    targetScoreStep: TARGET_SCORE_STEP,
  });

  const { buildUpgradePayloadWeights, buildFixedPayloadWeights } = createScorerPayloadBuilder({
    state,
    scorerFixed: SCORER_FIXED,
    isFixedScorer,
    getWeightMap,
  });

  let upgradeScoreController = null;
  let policyController = null;

  async function computeContributions() {
    if (!upgradeScoreController) {
      return;
    }
    await upgradeScoreController.computeContributions();
  }

  async function updateSuggestion() {
    if (!policyController) {
      return;
    }
    await policyController.updateSuggestion();
  }

  const ocrController = createOcrController({
    state,
    elements,
    invoke,
    normalizeOcrPort,
    numberOr,
    defaultOcrUdpPort: DEFAULT_OCR_UDP_PORT,
    eventFillEntries: OCR_UDP_EVENT_FILL_ENTRIES,
    eventListenerStatus: OCR_UDP_EVENT_LISTENER_STATUS,
    computeContributions: async () => computeContributions(),
    updateSuggestion: async () => updateSuggestion(),
  });

  const upgradeUiController = createUpgradeUiController({
    state,
    elements,
    numberOr,
    escapeHtml,
    placeholderLabel: PLACEHOLDER_LABEL,
    targetScoreDigits: TARGET_SCORE_DIGITS,
    targetScoreStep: TARGET_SCORE_STEP,
    scorerWuwaEchoTool: SCORER_WUWA_ECHO_TOOL,
    scorerMcBoostAssistant: SCORER_MC_BOOST_ASSISTANT,
    scorerQqBot: SCORER_QQ_BOT,
    mcBoostAssistantLockedMainBuffScore: MC_BOOST_ASSISTANT_LOCKED_MAIN_BUFF_SCORE,
    mcBoostAssistantLockedNormalizedMaxScore: MC_BOOST_ASSISTANT_LOCKED_NORMALIZED_MAX_SCORE,
    qqBotLockedNormalizedMaxScore: QQ_BOT_LOCKED_NORMALIZED_MAX_SCORE,
    isFixedScorer,
    isMcBoostAssistantScorer,
    isQqBotScorer,
    getScorerConfig,
    getWeightMap,
    formatScoreForScorer,
    setHelpTooltip,
    onWeightsUpdated,
    computeContributions,
    updateSuggestion,
  });

  const {
    renderScorerConfig,
    formatBuffLabel,
    renderWeightInputs,
    renderBuffSlots,
  } = upgradeUiController;

  const presetsController = createPresetsController({
    state,
    elements,
    invoke,
    numberOr,
    scorerTypes: SCORER_TYPES,
    scorerPresetCustom: SCORER_PRESET_CUSTOM,
    scorerPresetVariantDefault: SCORER_PRESET_VARIANT_DEFAULT,
    scorerLinearDefault: SCORER_LINEAR_DEFAULT,
    scorerWuwaEchoTool: SCORER_WUWA_ECHO_TOOL,
    scorerQqBot: SCORER_QQ_BOT,
    targetScoreStep: TARGET_SCORE_STEP,
    copyWeightMap,
    copyScorerConfig,
    getScorerConfig,
    buildUpgradePayloadWeights,
    isFixedScorer,
    isMcBoostAssistantScorer,
    isQqBotScorer,
    renderScorerConfig,
    renderWeightInputs,
    onWeightsUpdated,
    setHelpTooltip,
  });

  const {
    syncDefaultScorerConfigs,
    resetPresetState,
    renderPresetControls,
    loadScorerPresetsForType,
    applySelectedScorerPreset,
    applySelectedScorerPresetVariant,
    handleSaveCurrentPreset,
    handleDeleteCurrentPreset,
    handleSaveCurrentPresetVariant,
    handleDeleteCurrentPresetVariant,
  } = presetsController;

  const targetScoreController = createTargetScoreController({
    state,
    elements,
    numberOr,
    targetScoreDigits: TARGET_SCORE_DIGITS,
    targetScoreStep: TARGET_SCORE_STEP,
    scorerFixed: SCORER_FIXED,
    scorerWuwaEchoTool: SCORER_WUWA_ECHO_TOOL,
    scorerMcBoostAssistant: SCORER_MC_BOOST_ASSISTANT,
    scorerQqBot: SCORER_QQ_BOT,
    isFixedScorer,
    getNormalizedMaxScore,
    effectiveWeightForBuff,
  });

  const {
    roundToStep,
    computeTopWeightsSumForType,
    updateTargetScoreUI,
    updateRerollTargetScoreUI,
  } = targetScoreController;

  function initialiseState(data) {
    initialiseAppState({
      state,
      elements,
      data,
      createScorerConfigMap,
      copyWeightMap,
      normalizeScorerType,
      isFixedScorer,
      computeTopWeightsSumForType,
      getNormalizedMaxScore,
      syncDefaultScorerConfigs,
      resetPresetState,
      normalizeOcrPort,
      defaultOcrUdpPort: DEFAULT_OCR_UDP_PORT,
      defaultMcBoostAssistantTargetScore: DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE,
      defaultQqBotTargetScore: DEFAULT_QQ_BOT_TARGET_SCORE,
      scorerLinearDefault: SCORER_LINEAR_DEFAULT,
      scorerWuwaEchoTool: SCORER_WUWA_ECHO_TOOL,
      scorerMcBoostAssistant: SCORER_MC_BOOST_ASSISTANT,
      scorerQqBot: SCORER_QQ_BOT,
      scorerFixed: SCORER_FIXED,
      mcBoostAssistantLockedMainBuffScore: MC_BOOST_ASSISTANT_LOCKED_MAIN_BUFF_SCORE,
      mcBoostAssistantLockedNormalizedMaxScore: MC_BOOST_ASSISTANT_LOCKED_NORMALIZED_MAX_SCORE,
    });
  }

  const rerollController = createRerollController({
    state,
    elements,
    invoke,
    escapeHtml,
    formatFixedOr,
    placeholderLabel: PLACEHOLDER_LABEL,
    scorerFixed: SCORER_FIXED,
    computeTopWeightsSumForType,
    getWeightMap,
    formatBuffLabel,
    buildFixedPayloadWeights,
  });

  const {
    isFullUniqueSelection,
    onRerollSelectionChanged,
    invalidateRerollPolicy,
    updateRerollComputeButtonState,
    renderRerollSlots,
    renderRerollOutput,
    handleRerollCompute,
  } = rerollController;

  policyController = createPolicyController({
    state,
    elements,
    invoke,
    escapeHtml,
    numberOr,
    targetScoreDigits: TARGET_SCORE_DIGITS,
    targetScoreStep: TARGET_SCORE_STEP,
    scorerFixed: SCORER_FIXED,
    isFixedScorer,
    getScorerConfig,
    buildUpgradePayloadWeights,
    formatScoreForScorer,
    computeTopWeightsSumForType,
    getNormalizedMaxScore,
    selectedBuffStateWithSlots: () =>
      upgradeScoreController
        ? upgradeScoreController.selectedBuffStateWithSlots()
        : { names: [], values: [], slotIndices: [] },
  });

  const {
    renderResults,
    renderTotalScoreCard,
    resetPolicyResult,
    handleCompute,
    updateComputeButtonState,
  } = policyController;

  upgradeScoreController = createUpgradeScoreController({
    state,
    invoke,
    numberOr,
    targetScoreStep: TARGET_SCORE_STEP,
    scorerFixed: SCORER_FIXED,
    isFixedScorer,
    getScorerConfig,
    getNormalizedMaxScore,
    computeTopWeightsSumForType,
    buildUpgradePayloadWeights,
    renderBuffSlots,
    renderTotalScoreCard,
    updateComputeButtonState,
  });

  async function onWeightsUpdated() {
    updateTargetScoreUI();
    resetPolicyResult();

    if (state.scorerType === SCORER_FIXED) {
      updateRerollTargetScoreUI();
      invalidateRerollPolicy();
    } else {
      renderRerollSlots();
      updateRerollComputeButtonState();
    }

    await computeContributions();
  }

  async function onScorerParamsUpdated() {
    updateTargetScoreUI();
    resetPolicyResult();
    await computeContributions();
  }

  const modeFlowController = createModeFlowController({
    state,
    elements,
    scorerFixed: SCORER_FIXED,
    normalizeScorerType,
    renderScorerConfig,
    renderWeightInputs,
    loadScorerPresetsForType,
    updateTargetScoreUI,
    resetPolicyResult,
    updateRerollTargetScoreUI,
    invalidateRerollPolicy,
    renderRerollSlots,
    updateRerollComputeButtonState,
    computeContributions,
    renderTotalScoreCard,
  });

  const { setActiveTab, applyScorerType } = modeFlowController;

  const eventHandlersController = createEventHandlersController({
    state,
    elements,
    ocrController,
    numberOr,
    normalizeOcrPort,
    targetScoreDigits: TARGET_SCORE_DIGITS,
    targetScoreStep: TARGET_SCORE_STEP,
    scorerPresetCustom: SCORER_PRESET_CUSTOM,
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
  });

  const { setupEventHandlers } = eventHandlersController;

  const appInitController = createAppInitController({
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
  });

  document.addEventListener('DOMContentLoaded', () => {
    void appInitController.init();
  });
})();
