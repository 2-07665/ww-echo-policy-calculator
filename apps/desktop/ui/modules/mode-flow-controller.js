export function createModeFlowController({
  state,
  elements,
  scorerFixed,
  normalizeScorerType,
  renderScorerConfig,
  renderWeightInputs,
  loadScorerPresetsForType,
  updateTargetScoreUI,
  resetPolicyResult,
  updateRerollTargetScoreUI,
  invalidateRerollPolicy,
  renderRerollSlots,
  updateRerollComputeButtonState,
  computeContributions,
  renderTotalScoreCard,
}) {
  async function applyScorerType(
    nextScorerType,
    { setRecommendedTarget = true, preservePolicyState = false } = {},
  ) {
    state.scorerType = normalizeScorerType(nextScorerType);
    renderScorerConfig();
    renderWeightInputs();
    await loadScorerPresetsForType(state.scorerType);
    updateTargetScoreUI({ setRecommended: setRecommendedTarget });
    if (!preservePolicyState) {
      resetPolicyResult();
    }

    if (state.scorerType === scorerFixed) {
      updateRerollTargetScoreUI();
      invalidateRerollPolicy();
    } else {
      renderRerollSlots();
      updateRerollComputeButtonState();
    }

    await computeContributions();
  }

  async function setActiveTab(tab) {
    state.activeTab = tab === 'reroll' ? 'reroll' : 'upgrade';
    const upgradeActive = state.activeTab === 'upgrade';

    if (!upgradeActive) {
      if (state.scorerType !== scorerFixed) {
        state.scorerBeforeReroll = state.scorerType;
        state.targetScoreBeforeReroll = state.targetScore;
        await applyScorerType(scorerFixed, {
          setRecommendedTarget: true,
          preservePolicyState: true,
        });
      } else {
        state.targetScoreBeforeReroll = state.targetScore;
      }
      elements.scorerTypeSelect.disabled = true;
    } else {
      elements.scorerTypeSelect.disabled = false;
      if (state.scorerBeforeReroll && state.scorerType === scorerFixed) {
        const restoreScorer = state.scorerBeforeReroll;
        const restoreTargetScore = state.targetScoreBeforeReroll;
        state.scorerBeforeReroll = null;
        state.targetScoreBeforeReroll = null;
        await applyScorerType(restoreScorer, {
          setRecommendedTarget: false,
          preservePolicyState: true,
        });
        if (Number.isFinite(Number(restoreTargetScore))) {
          state.targetScore = Number(restoreTargetScore);
          updateTargetScoreUI({ setRecommended: false });
          renderTotalScoreCard();
        }
      } else {
        state.scorerBeforeReroll = null;
        state.targetScoreBeforeReroll = null;
        renderScorerConfig();
      }
    }

    elements.upgradeTab.hidden = !upgradeActive;
    elements.rerollTab.hidden = upgradeActive;
    elements.tabUpgrade.classList.toggle('active', upgradeActive);
    elements.tabReroll.classList.toggle('active', !upgradeActive);
  }

  return {
    setActiveTab,
    applyScorerType,
  };
}
