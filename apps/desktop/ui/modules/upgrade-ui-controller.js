import { createUpgradeBuffSlotsView } from './upgrade-buff-slots-view.js';
import { createUpgradeLabelFormatters } from './upgrade-label-formatters.js';
import { createUpgradeScorerConfigView } from './upgrade-scorer-config-view.js';
import { createUpgradeWeightInputsView } from './upgrade-weight-inputs-view.js';

export function createUpgradeUiController({
  state,
  elements,
  numberOr,
  escapeHtml,
  placeholderLabel,
  targetScoreDigits,
  targetScoreStep,
  scorerWuwaEchoTool,
  scorerMcBoostAssistant,
  scorerQqBot,
  mcBoostAssistantLockedMainBuffScore,
  mcBoostAssistantLockedNormalizedMaxScore,
  qqBotLockedNormalizedMaxScore,
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
}) {
  const labelFormatters = createUpgradeLabelFormatters({
    state,
    getWeightMap,
  });

  const scorerConfigView = createUpgradeScorerConfigView({
    state,
    elements,
    numberOr,
    targetScoreDigits,
    targetScoreStep,
    scorerWuwaEchoTool,
    scorerMcBoostAssistant,
    scorerQqBot,
    mcBoostAssistantLockedMainBuffScore,
    mcBoostAssistantLockedNormalizedMaxScore,
    qqBotLockedNormalizedMaxScore,
    isFixedScorer,
    isMcBoostAssistantScorer,
    isQqBotScorer,
    getScorerConfig,
    setHelpTooltip,
  });

  const weightInputsView = createUpgradeWeightInputsView({
    state,
    elements,
    numberOr,
    isFixedScorer,
    getWeightMap,
    onWeightsUpdated,
  });

  const buffSlotsView = createUpgradeBuffSlotsView({
    state,
    elements,
    escapeHtml,
    placeholderLabel,
    getWeightMap,
    formatBuffLabel: labelFormatters.formatBuffLabel,
    formatValueLabel: labelFormatters.formatValueLabel,
    formatScoreForScorer,
    computeContributions,
    updateSuggestion,
  });

  return {
    formatBuffLabel: labelFormatters.formatBuffLabel,
    renderScorerConfig: scorerConfigView.renderScorerConfig,
    renderWeightInputs: weightInputsView.renderWeightInputs,
    renderBuffSlots: buffSlotsView.renderBuffSlots,
  };
}
