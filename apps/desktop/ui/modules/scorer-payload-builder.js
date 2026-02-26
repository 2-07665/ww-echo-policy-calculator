export function createScorerPayloadBuilder({
  state,
  scorerFixed,
  isFixedScorer,
  getWeightMap,
}) {
  function buildUpgradePayloadWeights() {
    const weightMap = getWeightMap();
    const payloadWeights = {};

    state.buffTypes.forEach((buffName) => {
      const raw = Number(weightMap[buffName] ?? 0);
      payloadWeights[buffName] = isFixedScorer()
        ? Math.max(0, Math.round(raw))
        : Math.max(0, raw);
    });

    return payloadWeights;
  }

  function buildFixedPayloadWeights() {
    const weightMap = getWeightMap(scorerFixed);
    const payloadWeights = {};

    state.buffTypes.forEach((buffName) => {
      const raw = Number(weightMap[buffName] ?? 0);
      payloadWeights[buffName] = Math.max(0, Math.round(raw));
    });

    return payloadWeights;
  }

  return {
    buildUpgradePayloadWeights,
    buildFixedPayloadWeights,
  };
}
