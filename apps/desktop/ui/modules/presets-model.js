export function createPresetsModel({
  state,
  numberOr,
  targetScoreStep,
  copyWeightMap,
  scorerPresetCustom,
}) {
  function normalizePresetVariantEntry(raw) {
    const variantName = String(raw?.variantName || '').trim();
    if (!variantName) {
      return null;
    }
    const next = {
      variantName,
      weights: copyWeightMap(raw?.weights),
    };
    if (raw?.mainBuffScore != null) {
      next.mainBuffScore = Math.max(0, numberOr(raw.mainBuffScore, 0));
    }
    if (raw?.normalizedMaxScore != null) {
      next.normalizedMaxScore = Math.max(
        targetScoreStep,
        numberOr(raw.normalizedMaxScore, targetScoreStep),
      );
    }
    const presetIntro = String(raw?.presetIntro || '').trim();
    if (presetIntro) {
      next.presetIntro = presetIntro;
    }
    return next;
  }

  function normalizePresetEntry(raw) {
    const presetName = String(raw?.presetName || '').trim();
    if (!presetName || presetName === scorerPresetCustom) {
      return null;
    }
    const variants = Array.isArray(raw?.variants)
      ? raw.variants.map(normalizePresetVariantEntry).filter(Boolean)
      : [];
    if (!variants.length) {
      return null;
    }
    return {
      presetName,
      variants,
      builtIn: Boolean(raw?.builtIn),
      userDefined: Boolean(raw?.userDefined),
    };
  }

  function findPresetByName(type, presetName) {
    return (state.scorerPresets[type] || []).find((item) => item.presetName === presetName) || null;
  }

  function findVariantByName(preset, variantName) {
    if (!preset || !Array.isArray(preset.variants)) {
      return null;
    }
    return preset.variants.find((item) => item.variantName === variantName) || null;
  }

  function applyPresetList(type, rawPresets) {
    const normalized = Array.isArray(rawPresets)
      ? rawPresets.map(normalizePresetEntry).filter(Boolean)
      : [];
    state.scorerPresets[type] = normalized;

    const activeName = state.activePresetNames[type];
    const activePreset = activeName === scorerPresetCustom ? null : findPresetByName(type, activeName);
    if (!activePreset) {
      state.activePresetNames[type] = scorerPresetCustom;
      state.activePresetVariantNames[type] = '';
      return;
    }

    const activeVariantName = state.activePresetVariantNames[type];
    const activeVariant = findVariantByName(activePreset, activeVariantName);
    state.activePresetVariantNames[type] = activeVariant
      ? activeVariant.variantName
      : String(activePreset.variants[0]?.variantName || '');
  }

  return {
    findPresetByName,
    findVariantByName,
    applyPresetList,
  };
}
