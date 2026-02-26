export function createPresetsSelectionActions({
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
}) {
  async function loadScorerPresetsForType(type = state.scorerType) {
    try {
      const response = await invoke('load_scorer_presets', {
        payload: { scorerType: type },
      });
      applyPresetList(type, response?.presets || []);
      if (type === state.scorerType) {
        renderPresetControls();
        setPresetStatus('');
      }
    } catch (error) {
      applyPresetList(type, []);
      state.activePresetNames[type] = scorerPresetCustom;
      state.activePresetVariantNames[type] = '';
      if (type === state.scorerType) {
        renderPresetControls();
        setPresetStatus(`读取预设失败：${error?.message || error}`, { error: true });
      }
    }
  }

  async function applySelectedScorerPreset(presetName) {
    const type = state.scorerType;
    const selectedName = String(presetName || scorerPresetCustom);

    if (selectedName === scorerPresetCustom) {
      applyBuiltinCustomPresetForScorer(type);
      state.activePresetNames[type] = scorerPresetCustom;
      state.activePresetVariantNames[type] = '';
    } else {
      const preset = findPresetByName(type, selectedName);
      if (!preset) {
        state.activePresetNames[type] = scorerPresetCustom;
        state.activePresetVariantNames[type] = '';
        renderPresetControls();
        setPresetStatus(`未找到预设：${selectedName}`, { error: true });
        return;
      }
      const defaultVariant = preset.variants[0];
      applyPresetConfigForScorer(type, defaultVariant);
      state.activePresetNames[type] = preset.presetName;
      state.activePresetVariantNames[type] = String(defaultVariant?.variantName || '');
    }

    renderScorerConfig();
    renderWeightInputs();
    renderPresetControls();
    setPresetStatus('');
    await onWeightsUpdated();
  }

  async function applySelectedScorerPresetVariant(variantName) {
    const type = state.scorerType;
    const presetName = String(state.activePresetNames[type] || scorerPresetCustom);
    if (presetName === scorerPresetCustom) {
      return;
    }
    const preset = findPresetByName(type, presetName);
    if (!preset) {
      state.activePresetNames[type] = scorerPresetCustom;
      state.activePresetVariantNames[type] = '';
      renderPresetControls();
      setPresetStatus(`未找到预设：${presetName}`, { error: true });
      return;
    }
    const selectedVariantName = String(variantName || '').trim();
    const variant = findVariantByName(preset, selectedVariantName);
    if (!variant) {
      setPresetStatus(`未找到预设变体：${selectedVariantName}`, { error: true });
      renderPresetControls();
      return;
    }

    applyPresetConfigForScorer(type, variant);
    state.activePresetVariantNames[type] = variant.variantName;
    renderScorerConfig();
    renderWeightInputs();
    renderPresetControls();
    setPresetStatus('');
    await onWeightsUpdated();
  }

  return {
    loadScorerPresetsForType,
    applySelectedScorerPreset,
    applySelectedScorerPresetVariant,
  };
}
