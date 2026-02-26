export function createUpgradeLabelFormatters({ state, getWeightMap }) {
  function formatBuffLabel(buffName, weightMap = getWeightMap()) {
    const label = state.buffLabels[buffName] ?? buffName;
    const weight = Number(weightMap[buffName] ?? 0);
    if (Math.abs(weight) < 1e-9) {
      return `（无效词条）${label}`;
    }
    return label;
  }

  function formatValueLabel(buffName, value) {
    if (state.percentBuffs.has(buffName)) {
      return `${(Number(value) / 10).toFixed(1)}%`;
    }
    return String(value);
  }

  return {
    formatBuffLabel,
    formatValueLabel,
  };
}
