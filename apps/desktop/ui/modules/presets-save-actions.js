import { withButtonLoading } from './presets-button-loading.js';

export function createPresetsSaveActions({
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
}) {
  async function handleSaveCurrentPreset() {
    const currentType = state.scorerType;
    const activePresetName = String(state.activePresetNames[currentType] || scorerPresetCustom);
    const activeVariantName = String(state.activePresetVariantNames[currentType] || '');
    const activePreset = findPresetByName(currentType, activePresetName);
    const presetName = String(elements.scorerPresetNameInput.value || '').trim();
    const typedVariantName = String(elements.scorerPresetVariantNameInput.value || '').trim();
    if (!presetName) {
      setPresetStatus('请输入预设名称后再保存。', { error: true });
      return;
    }
    if (presetName === scorerPresetCustom) {
      setPresetStatus(`“${scorerPresetCustom}”为默认项名称，不能保存为预设。`, { error: true });
      return;
    }
    if (
      activePreset &&
      activePreset.builtIn &&
      !activePreset.userDefined &&
      presetName === activePreset.presetName
    ) {
      setPresetStatus('内置预设为只读。请使用新的预设名称进行保存。', { error: true });
      return;
    }
    const activeDefaultVariantName = String(
      activePreset?.variants?.[0]?.variantName || scorerPresetVariantDefault,
    );
    const shouldSaveActiveVariant =
      Boolean(activePreset?.userDefined) &&
      presetName === activePresetName &&
      Boolean(activeVariantName) &&
      activeVariantName !== activeDefaultVariantName;
    const presetSaveVariantName = typedVariantName || activeVariantName || scorerPresetVariantDefault;

    await withButtonLoading(elements.scorerPresetSaveButton, '保存中…', async () => {
      try {
        const response = shouldSaveActiveVariant
          ? await invoke('save_scorer_preset_variant', {
              payload: buildPresetVariantSavePayload(presetName, activeVariantName),
            })
          : await invoke('save_scorer_preset', {
              payload: buildPresetSavePayload(presetName, presetSaveVariantName),
            });

        applyPresetList(currentType, response?.presets || []);
        const savedPresetName = String(response?.savedPresetName || presetName);
        const returnedVariantName = String(response?.savedVariantName || scorerPresetVariantDefault);
        state.activePresetNames[currentType] = savedPresetName;
        state.activePresetVariantNames[currentType] = returnedVariantName;
        const savedPreset = findPresetByName(currentType, state.activePresetNames[currentType]);
        const savedVariant = findVariantByName(savedPreset, state.activePresetVariantNames[currentType]);
        if (savedVariant) {
          applyPresetConfigForScorer(currentType, savedVariant);
        }
        renderScorerConfig();
        renderWeightInputs();
        renderPresetControls();
        elements.scorerPresetNameInput.value = state.activePresetNames[currentType];
        if (shouldSaveActiveVariant) {
          setPresetStatus(
            `已保存变体：${state.activePresetNames[currentType]} / ${state.activePresetVariantNames[currentType]}`,
          );
        } else {
          setPresetStatus(`已保存预设：${state.activePresetNames[currentType]}`);
        }
        await onWeightsUpdated();
      } catch (error) {
        setPresetStatus(`保存失败：${error?.message || error}`, { error: true });
      }
      renderPresetControls();
    });
  }

  async function handleSaveCurrentPresetVariant() {
    const currentType = state.scorerType;
    const presetName = String(state.activePresetNames[currentType] || scorerPresetCustom);
    if (presetName === scorerPresetCustom) {
      setPresetStatus('请先选择一个预设，再保存变体。', { error: true });
      return;
    }
    const preset = findPresetByName(currentType, presetName);
    if (!preset) {
      setPresetStatus(`未找到预设：${presetName}`, { error: true });
      renderPresetControls();
      return;
    }
    if (!preset.userDefined) {
      setPresetStatus('内置预设为只读。请先另存为自定义预设。', { error: true });
      return;
    }
    const variantName = String(elements.scorerPresetVariantNameInput.value || '').trim();
    if (!variantName) {
      setPresetStatus('请输入变体名称后再保存。', { error: true });
      return;
    }
    const defaultVariantName = String(preset.variants?.[0]?.variantName || scorerPresetVariantDefault);
    if (variantName === defaultVariantName) {
      setPresetStatus(`“${defaultVariantName}”是默认变体，请使用预设保存按钮。`, { error: true });
      return;
    }
    await withButtonLoading(elements.scorerPresetVariantSaveButton, '保存中…', async () => {
      try {
        const response = await invoke('save_scorer_preset_variant', {
          payload: buildPresetVariantSavePayload(presetName, variantName),
        });

        applyPresetList(currentType, response?.presets || []);
        state.activePresetNames[currentType] = String(response?.savedPresetName || presetName);
        state.activePresetVariantNames[currentType] = String(
          response?.savedVariantName || variantName,
        );
        const savedPreset = findPresetByName(currentType, state.activePresetNames[currentType]);
        const savedVariant = findVariantByName(savedPreset, state.activePresetVariantNames[currentType]);
        if (savedVariant) {
          applyPresetConfigForScorer(currentType, savedVariant);
        }
        renderScorerConfig();
        renderWeightInputs();
        renderPresetControls();
        setPresetStatus(
          `已保存变体：${state.activePresetNames[currentType]} / ${state.activePresetVariantNames[currentType]}`,
        );
        await onWeightsUpdated();
      } catch (error) {
        setPresetStatus(`保存变体失败：${error?.message || error}`, { error: true });
        renderPresetControls();
      }
      renderPresetControls();
    });
  }

  return {
    handleSaveCurrentPreset,
    handleSaveCurrentPresetVariant,
  };
}
