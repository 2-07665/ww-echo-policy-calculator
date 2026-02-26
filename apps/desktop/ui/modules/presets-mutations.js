import { createPresetsDeleteActions } from './presets-delete-actions.js';
import { createPresetsSaveActions } from './presets-save-actions.js';

export function createPresetsMutations({
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
}) {
  const saveActions = createPresetsSaveActions({
    state,
    elements,
    invoke,
    scorerPresetCustom,
    scorerPresetVariantDefault,
    findPresetByName,
    findVariantByName,
    applyPresetList,
    applyPresetConfigForScorer,
    buildPresetSavePayload,
    buildPresetVariantSavePayload,
    renderScorerConfig,
    renderWeightInputs,
    renderPresetControls,
    setPresetStatus,
    onWeightsUpdated,
  });

  const deleteActions = createPresetsDeleteActions({
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
    renderScorerConfig,
    renderWeightInputs,
    renderPresetControls,
    setPresetStatus,
    onWeightsUpdated,
  });

  return {
    handleSaveCurrentPreset: saveActions.handleSaveCurrentPreset,
    handleDeleteCurrentPreset: deleteActions.handleDeleteCurrentPreset,
    handleSaveCurrentPresetVariant: saveActions.handleSaveCurrentPresetVariant,
    handleDeleteCurrentPresetVariant: deleteActions.handleDeleteCurrentPresetVariant,
  };
}
