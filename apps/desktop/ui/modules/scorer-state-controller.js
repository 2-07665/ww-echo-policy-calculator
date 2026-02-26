export function createScorerStateController({
  state,
  numberOr,
  targetScoreDigits,
  targetScoreStep,
  scorerLinearDefault,
  scorerWuwaEchoTool,
  scorerMcBoostAssistant,
  scorerQqBot,
  scorerFixed,
  mcBoostAssistantLockedMainBuffScore,
  mcBoostAssistantLockedNormalizedMaxScore,
  qqBotLockedNormalizedMaxScore,
}) {
  function normalizeScorerType(value) {
    const lowered = String(value || '').toLowerCase();
    if (lowered === 'linear') {
      return scorerLinearDefault;
    }
    if (lowered === scorerWuwaEchoTool) {
      return scorerWuwaEchoTool;
    }
    if (lowered === scorerMcBoostAssistant) {
      return scorerMcBoostAssistant;
    }
    if (lowered === scorerQqBot) {
      return scorerQqBot;
    }
    if (lowered === scorerFixed) {
      return scorerFixed;
    }
    return scorerLinearDefault;
  }

  function isFixedScorer(type = state.scorerType) {
    return type === scorerFixed;
  }

  function isQqBotScorer(type = state.scorerType) {
    return type === scorerQqBot;
  }

  function isMcBoostAssistantScorer(type = state.scorerType) {
    return type === scorerMcBoostAssistant;
  }

  function getScorerConfig(type = state.scorerType) {
    return state.scorerConfigs[type];
  }

  function getWeightMap(type = state.scorerType) {
    return getScorerConfig(type).weights;
  }

  function getMainBuffScore(type = state.scorerType) {
    if (isFixedScorer(type)) {
      return 0;
    }
    if (isMcBoostAssistantScorer(type)) {
      return mcBoostAssistantLockedMainBuffScore;
    }
    return Math.max(0, numberOr(getScorerConfig(type).mainBuffScore, 0));
  }

  function getNormalizedMaxScore(type = state.scorerType) {
    if (isFixedScorer(type)) {
      return 0;
    }
    if (isMcBoostAssistantScorer(type)) {
      return mcBoostAssistantLockedNormalizedMaxScore;
    }
    if (isQqBotScorer(type)) {
      return qqBotLockedNormalizedMaxScore;
    }
    return Math.max(targetScoreStep, numberOr(getScorerConfig(type).normalizedMaxScore, 0));
  }

  function effectiveWeightForBuff(buffName, type = state.scorerType) {
    const rawWeight = Math.max(0, Number(getWeightMap(type)[buffName] ?? 0));
    if (!isQqBotScorer(type)) {
      return rawWeight;
    }

    const buffIndex = state.buffTypes.indexOf(buffName);
    if (buffIndex < 0) {
      return 0;
    }
    const buffMaxValue = Number(state.buffTypeMaxValues[buffIndex] ?? 0);
    if (buffMaxValue <= 0) {
      return 0;
    }
    const isFlatBuff = buffName.endsWith('_Flat');
    const qqFactor = isFlatBuff ? 1.0 : 0.1;
    return rawWeight * qqFactor * buffMaxValue;
  }

  function formatScoreForScorer(value, type = state.scorerType) {
    const numeric = numberOr(value, 0);
    if (isFixedScorer(type)) {
      return String(Math.round(numeric));
    }
    return numeric.toFixed(targetScoreDigits);
  }

  return {
    normalizeScorerType,
    isFixedScorer,
    isQqBotScorer,
    isMcBoostAssistantScorer,
    getScorerConfig,
    getWeightMap,
    getMainBuffScore,
    getNormalizedMaxScore,
    effectiveWeightForBuff,
    formatScoreForScorer,
  };
}
