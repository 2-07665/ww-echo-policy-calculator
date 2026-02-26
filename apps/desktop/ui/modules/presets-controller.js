import { createPresetsMutations } from './presets-mutations.js';
import { createPresetsModel } from './presets-model.js';
import { createPresetsSelectionActions } from './presets-selection-actions.js';
import { createPresetsViewController } from './presets-view-controller.js';

export function createPresetsController({
  state,
  elements,
  invoke,
  numberOr,
  scorerTypes,
  scorerPresetCustom,
  scorerPresetVariantDefault,
  scorerLinearDefault,
  scorerWuwaEchoTool,
  scorerQqBot,
  targetScoreStep,
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
}) {
  const {
    findPresetByName,
    findVariantByName,
    applyPresetList,
  } = createPresetsModel({
    state,
    numberOr,
    targetScoreStep,
    copyWeightMap,
    scorerPresetCustom,
  });

  function applyBuiltinCustomPresetForScorer(type) {
    state.scorerConfigs[type] = copyScorerConfig(state.defaultScorerConfigs[type]);
  }

  function applyPresetConfigForScorer(type, presetVariant) {
    const next = copyScorerConfig(state.defaultScorerConfigs[type]);
    next.weights = copyWeightMap(presetVariant?.weights);

    if (
      type === scorerLinearDefault ||
      type === scorerWuwaEchoTool ||
      type === scorerQqBot
    ) {
      next.mainBuffScore = Math.max(
        0,
        numberOr(presetVariant?.mainBuffScore, numberOr(next.mainBuffScore, 0)),
      );
    }
    if (type === scorerLinearDefault || type === scorerWuwaEchoTool) {
      next.normalizedMaxScore = Math.max(
        targetScoreStep,
        numberOr(
          presetVariant?.normalizedMaxScore,
          numberOr(next.normalizedMaxScore, targetScoreStep),
        ),
      );
    }

    state.scorerConfigs[type] = next;
  }

  const presetsViewController = createPresetsViewController({
    state,
    elements,
    scorerPresetCustom,
    scorerPresetVariantDefault,
    setHelpTooltip,
    findPresetByName,
    findVariantByName,
  });

  const {
    setPresetStatus,
    renderPresetControls,
  } = presetsViewController;

  const {
    loadScorerPresetsForType,
    applySelectedScorerPreset,
    applySelectedScorerPresetVariant,
  } = createPresetsSelectionActions({
    state,
    invoke,
    scorerPresetCustom,
    findPresetByName,
    findVariantByName,
    applyPresetList,
    applyBuiltinCustomPresetForScorer,
    applyPresetConfigForScorer,
    renderScorerConfig,
    renderWeightInputs,
    renderPresetControls,
    setPresetStatus,
    onWeightsUpdated,
  });

  function buildPresetSavePayload(presetName, variantName = '') {
    const config = getScorerConfig();
    const payload = {
      scorerType: state.scorerType,
      presetName,
      weights: buildUpgradePayloadWeights(),
    };

    if (!isFixedScorer() && !isMcBoostAssistantScorer()) {
      payload.mainBuffScore = Math.max(0, numberOr(config.mainBuffScore, 0));
    }
    if (!isFixedScorer() && !isMcBoostAssistantScorer() && !isQqBotScorer()) {
      payload.normalizedMaxScore = Math.max(
        targetScoreStep,
        numberOr(config.normalizedMaxScore, targetScoreStep),
      );
    }
    const normalizedVariantName = String(variantName || '').trim();
    if (normalizedVariantName) {
      payload.variantName = normalizedVariantName;
    }

    return payload;
  }

  function buildPresetVariantSavePayload(presetName, variantName) {
    const payload = buildPresetSavePayload(presetName);
    payload.variantName = variantName;
    return payload;
  }

  const presetMutations = createPresetsMutations({
    state,
    elements,
    invoke,
    scorerPresetCustom,
    scorerPresetVariantDefault,
    findPresetByName,
    findVariantByName,
    applyPresetList,
    applyBuiltinCustomPresetForScorer,
    applyPresetConfigForScorer,
    buildPresetSavePayload,
    buildPresetVariantSavePayload,
    renderScorerConfig,
    renderWeightInputs,
    renderPresetControls,
    setPresetStatus,
    onWeightsUpdated,
  });

  const {
    handleSaveCurrentPreset,
    handleDeleteCurrentPreset,
    handleSaveCurrentPresetVariant,
    handleDeleteCurrentPresetVariant,
  } = presetMutations;

  function syncDefaultScorerConfigs() {
    scorerTypes.forEach((type) => {
      state.defaultScorerConfigs[type] = copyScorerConfig(state.scorerConfigs[type]);
    });
  }

  function resetPresetState() {
    scorerTypes.forEach((type) => {
      state.scorerPresets[type] = [];
      state.activePresetNames[type] = scorerPresetCustom;
      state.activePresetVariantNames[type] = '';
    });
    state.scorerPresetStatus = '';
    state.scorerPresetStatusError = false;
  }

  return {
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
  };
}
