export function createPresetsViewController({
  state,
  elements,
  scorerPresetCustom,
  scorerPresetVariantDefault,
  setHelpTooltip,
  findPresetByName,
  findVariantByName,
}) {
  function renderPresetStatus() {
    if (!elements.scorerPresetStatus) {
      return;
    }
    elements.scorerPresetStatus.textContent = state.scorerPresetStatus || '';
    elements.scorerPresetStatus.classList.toggle('error', state.scorerPresetStatusError);
  }

  function setPresetStatus(message, { error = false } = {}) {
    state.scorerPresetStatus = message || '';
    state.scorerPresetStatusError = Boolean(error);
    renderPresetStatus();
  }

  function renderPresetHint(selectedPresetName, selectedVariantName) {
    const help = elements.scorerPresetHelp;
    if (!help) {
      return;
    }

    const activeName = String(selectedPresetName || scorerPresetCustom);
    if (activeName === scorerPresetCustom) {
      setHelpTooltip(help, '', { hideWhenEmpty: true });
      return;
    }

    const preset = findPresetByName(state.scorerType, activeName);
    const variant = findVariantByName(preset, selectedVariantName);
    const intro = String(variant?.presetIntro || '').trim();
    if (!intro) {
      setHelpTooltip(help, '', { hideWhenEmpty: true });
      return;
    }

    setHelpTooltip(help, intro, { hideWhenEmpty: true });
    help.setAttribute('aria-label', `当前选中预设说明：${activeName} / ${String(variant?.variantName || '')}`);
  }

  function renderPresetControls() {
    const currentType = state.scorerType;
    const presets = state.scorerPresets[currentType] || [];
    const select = elements.scorerPresetSelect;
    const nameInput = elements.scorerPresetNameInput;
    const variantSelect = elements.scorerPresetVariantSelect;
    const variantNameInput = elements.scorerPresetVariantNameInput;

    select.innerHTML = '';
    const defaultOption = document.createElement('option');
    defaultOption.value = scorerPresetCustom;
    defaultOption.textContent = scorerPresetCustom;
    select.appendChild(defaultOption);

    const userDefinedPresets = presets.filter((preset) => Boolean(preset?.userDefined));
    const builtInPresets = presets.filter(
      (preset) => Boolean(preset?.builtIn) && !preset?.userDefined,
    );
    const otherPresets = presets.filter(
      (preset) => !preset?.userDefined && !preset?.builtIn,
    );

    const appendPresetOptions = (container, list) => {
      list.forEach((preset) => {
        const option = document.createElement('option');
        option.value = preset.presetName;
        option.textContent = preset.presetName;
        container.appendChild(option);
      });
    };

    if (userDefinedPresets.length) {
      const userGroup = document.createElement('optgroup');
      userGroup.label = '自定义预设';
      appendPresetOptions(userGroup, userDefinedPresets);
      select.appendChild(userGroup);
    }

    if (builtInPresets.length) {
      const builtInGroup = document.createElement('optgroup');
      builtInGroup.label = '内置预设';
      appendPresetOptions(builtInGroup, builtInPresets);
      select.appendChild(builtInGroup);
    }

    if (otherPresets.length) {
      appendPresetOptions(select, otherPresets);
    }

    const activeName = state.activePresetNames[currentType];
    const activeExists =
      activeName === scorerPresetCustom ||
      presets.some((preset) => preset.presetName === activeName);
    select.value = activeExists ? activeName : scorerPresetCustom;
    if (!activeExists) {
      state.activePresetNames[currentType] = scorerPresetCustom;
    }

    if (document.activeElement !== nameInput) {
      nameInput.value = select.value === scorerPresetCustom ? '' : select.value;
    }
    const activePreset = findPresetByName(currentType, select.value);
    const hasPreset = select.value !== scorerPresetCustom && Boolean(activePreset);
    const presetUserDefined = hasPreset && Boolean(activePreset?.userDefined);

    variantSelect.innerHTML = '';
    if (!hasPreset) {
      const emptyOption = document.createElement('option');
      emptyOption.value = '';
      emptyOption.textContent = '-';
      variantSelect.appendChild(emptyOption);
      variantSelect.value = '';
      variantSelect.disabled = true;
      state.activePresetVariantNames[currentType] = '';
      const customVariantName = String(variantNameInput.value || '').trim();
      if (document.activeElement !== variantNameInput) {
        variantNameInput.value = customVariantName || scorerPresetVariantDefault;
      }
      variantNameInput.disabled = false;
      elements.scorerPresetVariantSaveButton.disabled = true;
      elements.scorerPresetVariantDeleteButton.disabled = true;
    } else {
      const variants = activePreset.variants || [];
      variants.forEach((variant) => {
        const option = document.createElement('option');
        option.value = variant.variantName;
        option.textContent = variant.variantName;
        variantSelect.appendChild(option);
      });
      const activeVariantName = state.activePresetVariantNames[currentType];
      const activeVariantExists = variants.some((variant) => variant.variantName === activeVariantName);
      const selectedVariantName = activeVariantExists
        ? activeVariantName
        : String(variants[0]?.variantName || '');
      state.activePresetVariantNames[currentType] = selectedVariantName;
      variantSelect.value = selectedVariantName;
      variantSelect.disabled = false;
      if (document.activeElement !== variantNameInput) {
        variantNameInput.value = selectedVariantName;
      }
      variantNameInput.disabled = false;
      elements.scorerPresetVariantSaveButton.disabled = !presetUserDefined;
      const defaultVariantName = String(variants[0]?.variantName || scorerPresetVariantDefault);
      elements.scorerPresetVariantDeleteButton.disabled =
        !presetUserDefined ||
        !selectedVariantName ||
        selectedVariantName === defaultVariantName;
    }

    if (elements.scorerPresetDeleteButton) {
      elements.scorerPresetDeleteButton.disabled = !presetUserDefined;
    }

    renderPresetHint(select.value, state.activePresetVariantNames[currentType]);
    renderPresetStatus();
  }

  return {
    setPresetStatus,
    renderPresetControls,
  };
}
