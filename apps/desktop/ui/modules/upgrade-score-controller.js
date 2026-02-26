export function createUpgradeScoreController({
  state,
  invoke,
  numberOr,
  targetScoreStep,
  scorerFixed,
  isFixedScorer,
  getScorerConfig,
  getNormalizedMaxScore,
  computeTopWeightsSumForType,
  buildUpgradePayloadWeights,
  renderBuffSlots,
  renderTotalScoreCard,
  updateComputeButtonState,
}) {
  let scorePreviewRequestToken = 0;

  function selectedBuffStateWithSlots() {
    const names = [];
    const values = [];
    const slotIndices = [];

    for (let i = 0; i < state.maxSelectedTypes; i += 1) {
      const buffName = state.buffSelections[i];
      const buffValue = state.buffValues[i];
      if (!buffName || buffValue == null) {
        continue;
      }
      names.push(buffName);
      values.push(Math.max(0, Math.round(Number(buffValue))));
      slotIndices.push(i);
    }

    return { names, values, slotIndices };
  }

  async function computeContributions() {
    const scorerType = state.scorerType;
    state.topWeightsSum = computeTopWeightsSumForType(scorerType);

    const { names, values, slotIndices } = selectedBuffStateWithSlots();
    const token = ++scorePreviewRequestToken;
    const config = getScorerConfig(scorerType);

    try {
      const response = await invoke('preview_upgrade_score', {
        payload: {
          buffWeights: buildUpgradePayloadWeights(),
          scorerType,
          mainBuffScore: isFixedScorer(scorerType)
            ? undefined
            : Math.max(0, numberOr(config.mainBuffScore, 0)),
          normalizedMaxScore: isFixedScorer(scorerType)
            ? undefined
            : Math.max(targetScoreStep, numberOr(config.normalizedMaxScore, 0)),
          buffNames: names,
          buffValues: values,
        },
      });

      if (token !== scorePreviewRequestToken) {
        return;
      }

      const nextContributions = Array(state.maxSelectedTypes).fill(0);
      const responseContributions = Array.isArray(response?.contributions)
        ? response.contributions
        : [];
      slotIndices.forEach((slotIndex, idx) => {
        nextContributions[slotIndex] = Math.max(0, numberOr(responseContributions[idx], 0));
      });

      state.contributions = nextContributions;
      state.mainContribution = Math.max(0, numberOr(response?.mainContribution, 0));
      state.totalScore = Math.max(0, numberOr(response?.totalScore, 0));
      state.displayMaxScore = Math.max(0, numberOr(response?.maxScore, 0));
    } catch (_error) {
      if (token !== scorePreviewRequestToken) {
        return;
      }
      state.contributions = Array(state.maxSelectedTypes).fill(0);
      state.mainContribution = 0;
      state.totalScore = 0;
      state.displayMaxScore = isFixedScorer(scorerType)
        ? Math.max(0, Math.round(computeTopWeightsSumForType(scorerFixed)))
        : getNormalizedMaxScore(scorerType);
    }

    renderBuffSlots();
    renderTotalScoreCard();
    updateComputeButtonState();
  }

  return {
    selectedBuffStateWithSlots,
    computeContributions,
  };
}
