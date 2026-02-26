export function createTargetScoreController({
  state,
  elements,
  numberOr,
  targetScoreDigits,
  targetScoreStep,
  scorerFixed,
  scorerWuwaEchoTool,
  scorerMcBoostAssistant,
  scorerQqBot,
  isFixedScorer,
  getNormalizedMaxScore,
  effectiveWeightForBuff,
}) {
  function roundToStep(value, step = targetScoreStep) {
    if (!Number.isFinite(value)) {
      return 0;
    }
    return Math.round(value / step) * step;
  }

  function computeTopWeightsSumForType(type = state.scorerType) {
    const weights = state.buffTypes
      .map((name) => effectiveWeightForBuff(name, type))
      .sort((a, b) => b - a)
      .slice(0, state.maxSelectedTypes);
    return weights.reduce((sum, weight) => sum + weight, 0);
  }

  function recommendedTargetForScorer(type = state.scorerType) {
    if (isFixedScorer(type)) {
      const maxScore = computeTopWeightsSumForType(type);
      const defaultFixedTarget = Math.max(0, Math.round(numberOr(state.defaultFixedTargetScore, 0)));
      if (maxScore <= 0) {
        return defaultFixedTarget;
      }
      return Math.min(defaultFixedTarget, Math.round(maxScore));
    }
    if (type === scorerMcBoostAssistant) {
      return state.defaultMcBoostAssistantTargetScore;
    }
    if (type === scorerQqBot) {
      return state.defaultQqBotTargetScore;
    }
    if (type === scorerWuwaEchoTool) {
      return state.defaultWuwaEchoToolTargetScore;
    }
    return state.defaultTargetScore;
  }

  function updateTargetScoreUI({ setRecommended = false } = {}) {
    if (isFixedScorer()) {
      const maxScore = computeTopWeightsSumForType(scorerFixed);
      if (setRecommended) {
        state.targetScore = recommendedTargetForScorer(scorerFixed);
      } else if (maxScore > 0 && state.targetScore > maxScore) {
        state.targetScore = maxScore;
      }

      state.targetScore = Math.max(0, Math.round(numberOr(state.targetScore, 0)));
      elements.targetScoreInput.step = '1';
      elements.targetScoreInput.removeAttribute('max');
      elements.targetScoreInput.value = String(state.targetScore);
      return;
    }

    const normalizedMax = getNormalizedMaxScore();
    if (setRecommended) {
      state.targetScore = recommendedTargetForScorer(state.scorerType);
    }

    state.targetScore = Math.max(0, roundToStep(numberOr(state.targetScore, 0), targetScoreStep));
    elements.targetScoreInput.step = String(targetScoreStep);
    elements.targetScoreInput.max = normalizedMax.toFixed(targetScoreDigits);
    elements.targetScoreInput.value = state.targetScore.toFixed(targetScoreDigits);
  }

  function updateRerollTargetScoreUI({ setRecommended = false } = {}) {
    const maxScore = computeTopWeightsSumForType(scorerFixed);
    if (setRecommended) {
      state.reroll.targetScore = recommendedTargetForScorer(scorerFixed);
    } else if (maxScore > 0 && state.reroll.targetScore > maxScore) {
      state.reroll.targetScore = maxScore;
    }

    state.reroll.targetScore = Math.max(0, Math.round(numberOr(state.reroll.targetScore, 0)));
    elements.rerollTargetScoreInput.value = String(state.reroll.targetScore);
  }

  return {
    roundToStep,
    computeTopWeightsSumForType,
    updateTargetScoreUI,
    updateRerollTargetScoreUI,
  };
}
