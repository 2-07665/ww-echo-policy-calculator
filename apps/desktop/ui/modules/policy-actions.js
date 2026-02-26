export function createPolicyActions({
  state,
  elements,
  invoke,
  numberOr,
  targetScoreStep,
  isFixedScorer,
  getScorerConfig,
  buildUpgradePayloadWeights,
  selectedBuffStateWithSlots,
  renderResults,
  renderTotalScoreCard,
}) {
  let suggestionRequestToken = 0;

  function resetPolicyResult() {
    state.policySummary = null;
    state.policyError = null;
    state.policyReady = false;
    state.suggestion = null;
    renderResults();
    renderTotalScoreCard();
  }

  async function updateSuggestion() {
    if (!state.policyReady) {
      state.suggestion = null;
      renderTotalScoreCard();
      return;
    }

    const selected = selectedBuffStateWithSlots();
    const token = ++suggestionRequestToken;
    try {
      const response = await invoke('policy_suggestion', {
        payload: {
          buffNames: selected.names,
          buffValues: selected.values,
        },
      });

      if (token !== suggestionRequestToken) {
        return;
      }

      state.suggestion = response;
      renderTotalScoreCard();
    } catch (error) {
      if (token !== suggestionRequestToken) {
        return;
      }
      state.suggestion = { suggestion: `获取建议失败：${error?.message || error}` };
      renderTotalScoreCard();
    }
  }

  async function handleCompute() {
    const config = getScorerConfig();

    const payload = {
      buffWeights: buildUpgradePayloadWeights(),
      targetScore: isFixedScorer() ? Math.max(0, Math.round(state.targetScore)) : state.targetScore,
      scorerType: state.scorerType,
      mainBuffScore: isFixedScorer() ? undefined : Math.max(0, numberOr(config.mainBuffScore, 0)),
      normalizedMaxScore: isFixedScorer()
        ? undefined
        : Math.max(targetScoreStep, numberOr(config.normalizedMaxScore, 0)),
      costWeights: {
        wEcho: state.costWeights.wEcho,
        wTuner: state.costWeights.wTuner,
        wExp: state.costWeights.wExp,
      },
      expRefundRatio: state.expRefundRatio,
      blendData: state.blendData,
      lambdaTolerance: 1e-6,
      lambdaMaxIter: 120,
    };

    elements.computeButton.dataset.loading = 'true';
    elements.computeButton.disabled = true;
    const originalText = elements.computeButton.textContent;
    elements.computeButton.textContent = '计算中…';

    try {
      const response = await invoke('compute_policy', { payload });
      state.policySummary = response.summary;
      state.policyError = null;
      state.policyReady = true;
      state.suggestion = null;
      renderResults();
      renderTotalScoreCard();
      await updateSuggestion();
    } catch (error) {
      state.policySummary = null;
      state.policyError = error?.message || String(error);
      state.policyReady = false;
      state.suggestion = null;
      renderResults();
      renderTotalScoreCard();
    } finally {
      elements.computeButton.dataset.loading = 'false';
      elements.computeButton.textContent = originalText;
      elements.computeButton.disabled = state.topWeightsSum <= 0;
    }
  }

  function updateComputeButtonState() {
    if (elements.computeButton.dataset.loading === 'true') {
      return;
    }
    elements.computeButton.disabled = state.topWeightsSum <= 0;
  }

  return {
    resetPolicyResult,
    updateSuggestion,
    handleCompute,
    updateComputeButtonState,
  };
}
