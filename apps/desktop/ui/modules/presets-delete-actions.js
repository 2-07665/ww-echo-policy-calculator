import { withButtonLoading } from './presets-button-loading.js';

export function createPresetsDeleteActions({
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
}) {
  async function handleDeleteCurrentPreset() {
    const presetName = String(elements.scorerPresetSelect.value || '').trim();
    if (!presetName || presetName === scorerPresetCustom) {
      setPresetStatus('请选择要删除的预设。', { error: true });
      return;
    }
    const preset = findPresetByName(state.scorerType, presetName);
    if (!preset || !preset.userDefined) {
      setPresetStatus('内置预设不能删除。', { error: true });
      return;
    }
    await withButtonLoading(elements.scorerPresetDeleteButton, '删除中…', async () => {
      try {
        const response = await invoke('delete_scorer_preset', {
          payload: {
            scorerType: state.scorerType,
            presetName,
          },
        });

        const currentType = state.scorerType;
        applyPresetList(currentType, response?.presets || []);
        const activePresetName = String(state.activePresetNames[currentType] || scorerPresetCustom);
        const activeVariantName = String(state.activePresetVariantNames[currentType] || '');
        if (
          activePresetName === presetName ||
          !findPresetByName(currentType, activePresetName)
        ) {
          state.activePresetNames[currentType] = scorerPresetCustom;
          state.activePresetVariantNames[currentType] = '';
          applyBuiltinCustomPresetForScorer(currentType);
        } else {
          const fallbackPreset = findPresetByName(currentType, activePresetName);
          const fallbackVariant =
            findVariantByName(fallbackPreset, activeVariantName) || fallbackPreset?.variants?.[0];
          state.activePresetNames[currentType] = String(
            fallbackPreset?.presetName || scorerPresetCustom,
          );
          state.activePresetVariantNames[currentType] = String(fallbackVariant?.variantName || '');
          if (fallbackPreset && fallbackVariant) {
            applyPresetConfigForScorer(currentType, fallbackVariant);
          } else {
            state.activePresetNames[currentType] = scorerPresetCustom;
            state.activePresetVariantNames[currentType] = '';
            applyBuiltinCustomPresetForScorer(currentType);
          }
        }
        renderScorerConfig();
        renderWeightInputs();
        renderPresetControls();
        setPresetStatus(`已删除预设：${String(response?.deletedPresetName || presetName)}`);
        await onWeightsUpdated();
      } catch (error) {
        setPresetStatus(`删除失败：${error?.message || error}`, { error: true });
        renderPresetControls();
      }
      renderPresetControls();
    });
  }

  async function handleDeleteCurrentPresetVariant() {
    const currentType = state.scorerType;
    const presetName = String(state.activePresetNames[currentType] || scorerPresetCustom);
    if (presetName === scorerPresetCustom) {
      setPresetStatus('请先选择一个预设变体。', { error: true });
      return;
    }
    const preset = findPresetByName(currentType, presetName);
    if (!preset) {
      setPresetStatus(`未找到预设：${presetName}`, { error: true });
      renderPresetControls();
      return;
    }
    if (!preset.userDefined) {
      setPresetStatus('内置预设的变体不能删除。', { error: true });
      return;
    }
    const variantName = String(state.activePresetVariantNames[currentType] || '').trim();
    if (!variantName) {
      setPresetStatus('请选择要删除的变体。', { error: true });
      return;
    }
    const defaultVariantName = String(
      preset.variants?.[0]?.variantName || scorerPresetVariantDefault,
    );
    if (variantName === defaultVariantName) {
      setPresetStatus(`默认变体“${defaultVariantName}”不能删除。`, { error: true });
      return;
    }
    await withButtonLoading(elements.scorerPresetVariantDeleteButton, '删除中…', async () => {
      try {
        const response = await invoke('delete_scorer_preset_variant', {
          payload: {
            scorerType: currentType,
            presetName,
            variantName,
          },
        });

        applyPresetList(currentType, response?.presets || []);
        const fallbackPreset = findPresetByName(currentType, presetName);
        const fallbackVariant = fallbackPreset?.variants?.[0];
        if (fallbackPreset) {
          state.activePresetNames[currentType] = presetName;
          state.activePresetVariantNames[currentType] = String(fallbackVariant?.variantName || '');
          applyPresetConfigForScorer(currentType, fallbackVariant);
        } else {
          state.activePresetNames[currentType] = scorerPresetCustom;
          state.activePresetVariantNames[currentType] = '';
          applyBuiltinCustomPresetForScorer(currentType);
        }
        renderScorerConfig();
        renderWeightInputs();
        renderPresetControls();
        setPresetStatus(`已删除变体：${String(response?.deletedVariantName || variantName)}`);
        await onWeightsUpdated();
      } catch (error) {
        setPresetStatus(`删除变体失败：${error?.message || error}`, { error: true });
        renderPresetControls();
      }
      renderPresetControls();
    });
  }

  return {
    handleDeleteCurrentPreset,
    handleDeleteCurrentPresetVariant,
  };
}
