import { createPolicyActions } from './policy-actions.js';
import { createPolicyView } from './policy-view.js';

export function createPolicyController({
  state,
  elements,
  invoke,
  escapeHtml,
  numberOr,
  targetScoreDigits,
  targetScoreStep,
  scorerFixed,
  isFixedScorer,
  getScorerConfig,
  buildUpgradePayloadWeights,
  formatScoreForScorer,
  computeTopWeightsSumForType,
  getNormalizedMaxScore,
  selectedBuffStateWithSlots,
}) {
  const policyView = createPolicyView({
    state,
    elements,
    escapeHtml,
    numberOr,
    targetScoreDigits,
    targetScoreStep,
    scorerFixed,
    isFixedScorer,
    formatScoreForScorer,
    computeTopWeightsSumForType,
    getNormalizedMaxScore,
  });

  const policyActions = createPolicyActions({
    state,
    elements,
    invoke,
    numberOr,
    targetScoreStep,
    isFixedScorer,
    getScorerConfig,
    buildUpgradePayloadWeights,
    selectedBuffStateWithSlots,
    renderResults: policyView.renderResults,
    renderTotalScoreCard: policyView.renderTotalScoreCard,
  });

  return {
    renderResults: policyView.renderResults,
    renderTotalScoreCard: policyView.renderTotalScoreCard,
    resetPolicyResult: policyActions.resetPolicyResult,
    handleCompute: policyActions.handleCompute,
    updateSuggestion: policyActions.updateSuggestion,
    updateComputeButtonState: policyActions.updateComputeButtonState,
  };
}
