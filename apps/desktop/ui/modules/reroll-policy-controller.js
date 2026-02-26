export function createRerollPolicyController({
  state,
  elements,
  invoke,
  scorerFixed,
  computeTopWeightsSumForType,
  buildFixedPayloadWeights,
  isFullUniqueSelection,
  clearRerollRecommendation,
  updateRerollSlotsMeta,
  updateRerollComputeButtonState,
  renderRerollSlots,
  renderRerollOutput,
}) {
  let rerollRecommendationToken = 0;

  function fullSelectionOrNull(selections) {
    if (!isFullUniqueSelection(selections)) {
      return null;
    }
    return selections.map((name) => String(name));
  }

  async function updateRerollRecommendation() {
    if (!state.reroll.policyReady) {
      clearRerollRecommendation();
      updateRerollSlotsMeta();
      renderRerollOutput();
      return;
    }

    const baselineFull = fullSelectionOrNull(state.reroll.baselineSelections);
    if (!baselineFull) {
      clearRerollRecommendation();
      updateRerollSlotsMeta();
      renderRerollOutput();
      return;
    }
    const candidateFull = fullSelectionOrNull(state.reroll.candidateSelections);

    const token = ++rerollRecommendationToken;
    clearRerollRecommendation();
    updateRerollSlotsMeta();
    renderRerollOutput();

    try {
      const response = await invoke('query_reroll_recommendation', {
        payload: {
          baselineBuffNames: baselineFull,
          candidateBuffNames: candidateFull || [],
          topK: 3,
        },
      });
      if (token !== rerollRecommendationToken) {
        return;
      }
      state.reroll.output = response;
      state.reroll.error = null;
      updateRerollSlotsMeta();
      renderRerollOutput();
    } catch (error) {
      if (token !== rerollRecommendationToken) {
        return;
      }
      state.reroll.output = null;
      state.reroll.error = `查询重抽建议失败：${error?.message || error}`;
      updateRerollSlotsMeta();
      renderRerollOutput();
    }
  }

  function onRerollSelectionChanged() {
    if (state.reroll.policyReady) {
      void updateRerollRecommendation();
    } else {
      clearRerollRecommendation();
      renderRerollOutput();
    }
  }

  function invalidateRerollPolicy() {
    state.reroll.policyReady = false;
    rerollRecommendationToken += 1;
    clearRerollRecommendation();
    renderRerollSlots();
    renderRerollOutput();
    updateRerollComputeButtonState();
  }

  async function handleRerollCompute() {
    if (computeTopWeightsSumForType(scorerFixed) <= 0) {
      clearRerollRecommendation();
      state.reroll.error = '请先设置至少一个大于 0 的 FixedScorer 权重。';
      renderRerollSlots();
      renderRerollOutput();
      return;
    }

    elements.rerollComputeButton.dataset.loading = 'true';
    elements.rerollComputeButton.disabled = true;
    const originalText = elements.rerollComputeButton.textContent;
    elements.rerollComputeButton.textContent = '计算中…';

    try {
      await invoke('compute_reroll_policy', {
        payload: {
          buffWeights: buildFixedPayloadWeights(),
          targetScore: Math.max(0, Math.round(state.reroll.targetScore)),
        },
      });

      state.reroll.policyReady = true;
      rerollRecommendationToken += 1;
      clearRerollRecommendation();
      state.reroll.error = null;
      renderRerollSlots();
      renderRerollOutput();
      await updateRerollRecommendation();
    } catch (error) {
      state.reroll.policyReady = false;
      rerollRecommendationToken += 1;
      state.reroll.output = null;
      state.reroll.error = `重抽策略计算失败：${error?.message || error}`;
      renderRerollSlots();
      renderRerollOutput();
    } finally {
      elements.rerollComputeButton.dataset.loading = 'false';
      elements.rerollComputeButton.textContent = originalText;
      updateRerollComputeButtonState();
    }
  }

  return {
    onRerollSelectionChanged,
    invalidateRerollPolicy,
    handleRerollCompute,
  };
}
