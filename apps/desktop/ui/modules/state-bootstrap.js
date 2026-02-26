export function initialiseAppState({
  state,
  elements,
  data,
  createScorerConfigMap,
  copyWeightMap,
  normalizeScorerType,
  isFixedScorer,
  computeTopWeightsSumForType,
  getNormalizedMaxScore,
  syncDefaultScorerConfigs,
  resetPresetState,
  normalizeOcrPort,
  defaultOcrUdpPort,
  defaultMcBoostAssistantTargetScore,
  defaultQqBotTargetScore,
  scorerLinearDefault,
  scorerWuwaEchoTool,
  scorerMcBoostAssistant,
  scorerQqBot,
  scorerFixed,
  mcBoostAssistantLockedMainBuffScore,
  mcBoostAssistantLockedNormalizedMaxScore,
}) {
  state.buffTypes = data.buffTypes || [];
  state.buffLabels = data.buffLabels || {};
  state.buffTypeMaxValues = (data.buffTypeMaxValues || []).map(Number);
  state.maxSelectedTypes = Number(data.maxSelectedTypes || 5);

  state.defaultTargetScore = Number(data.defaultTargetScore || 60);
  state.defaultFixedTargetScore = Number(data.defaultFixedTargetScore || Math.round(state.defaultTargetScore));
  state.defaultWuwaEchoToolTargetScore = Number(
    data.defaultWuwaEchoToolTargetScore ?? state.defaultTargetScore,
  );
  state.defaultMcBoostAssistantTargetScore = Number(
    data.defaultMcBoostAssistantTargetScore ?? defaultMcBoostAssistantTargetScore,
  );
  state.defaultQqBotTargetScore = Number(
    data.defaultQqBotTargetScore ?? defaultQqBotTargetScore,
  );

  const defaultLinearWeights = data.defaultLinearBuffWeights || data.defaultBuffWeights || {};
  const defaultWuwaEchoToolWeights =
    data.defaultWuwaEchoToolBuffWeights || data.defaultBuffWeights || {};
  const defaultMcBoostAssistantWeights =
    data.defaultMcBoostAssistantBuffWeights || defaultLinearWeights;
  const defaultQQWeights = data.defaultQqBotBuffWeights || {};
  const defaultFixedWeights = data.defaultFixedBuffWeights || {};

  state.scorerConfigs = createScorerConfigMap();
  state.defaultScorerConfigs = createScorerConfigMap();

  state.scorerConfigs[scorerLinearDefault].weights = copyWeightMap(defaultLinearWeights);
  state.scorerConfigs[scorerWuwaEchoTool].weights = copyWeightMap(defaultWuwaEchoToolWeights);
  state.scorerConfigs[scorerMcBoostAssistant].weights = copyWeightMap(defaultMcBoostAssistantWeights);
  state.scorerConfigs[scorerQqBot].weights = copyWeightMap(defaultQQWeights);
  state.scorerConfigs[scorerFixed].weights = copyWeightMap(defaultFixedWeights);

  state.scorerConfigs[scorerLinearDefault].mainBuffScore = Number(
    data.defaultLinearMainBuffScore ?? 0,
  );
  state.scorerConfigs[scorerLinearDefault].normalizedMaxScore = Number(
    data.defaultLinearNormalizedMaxScore ?? 100,
  );
  state.scorerConfigs[scorerWuwaEchoTool].mainBuffScore = Number(
    data.defaultWuwaEchoToolMainBuffScore ?? 0,
  );
  state.scorerConfigs[scorerWuwaEchoTool].normalizedMaxScore = Number(
    data.defaultWuwaEchoToolNormalizedMaxScore ?? 100,
  );

  state.scorerConfigs[scorerQqBot].mainBuffScore = Number(
    data.defaultQqBotMainBuffScore ?? 0,
  );
  state.scorerConfigs[scorerQqBot].normalizedMaxScore = Number(
    data.defaultQqBotNormalizedMaxScore ?? 50,
  );
  state.scorerConfigs[scorerMcBoostAssistant].mainBuffScore =
    mcBoostAssistantLockedMainBuffScore;
  state.scorerConfigs[scorerMcBoostAssistant].normalizedMaxScore =
    mcBoostAssistantLockedNormalizedMaxScore;
  syncDefaultScorerConfigs();
  resetPresetState();

  state.scorerType = normalizeScorerType(data.defaultScorerType);
  if (isFixedScorer()) {
    state.targetScore = state.defaultFixedTargetScore;
  } else if (state.scorerType === scorerMcBoostAssistant) {
    state.targetScore = state.defaultMcBoostAssistantTargetScore;
  } else if (state.scorerType === scorerQqBot) {
    state.targetScore = state.defaultQqBotTargetScore;
  } else if (state.scorerType === scorerWuwaEchoTool) {
    state.targetScore = state.defaultWuwaEchoToolTargetScore;
  } else {
    state.targetScore = state.defaultTargetScore;
  }
  state.displayMaxScore = isFixedScorer()
    ? computeTopWeightsSumForType(scorerFixed)
    : getNormalizedMaxScore(state.scorerType);

  state.expRefundRatio = Number(data.defaultExpRefundRatio || 0.66);
  state.blendData = false;
  state.costWeights = {
    wEcho: Number(data.defaultCostWeights?.wEcho || 0),
    wTuner: Number(data.defaultCostWeights?.wTuner || 1),
    wExp: Number(data.defaultCostWeights?.wExp || 0),
  };

  const optionsObj = data.buffValueOptions || {};
  state.buffValueOptions = new Map(
    Object.entries(optionsObj).map(([name, values]) => [
      name,
      Array.isArray(values) ? values.map(Number) : [],
    ]),
  );

  state.percentBuffs = new Set(state.buffTypes.filter((name) => !name.endsWith('_Flat')));

  state.buffSelections = Array(state.maxSelectedTypes).fill(null);
  state.buffValues = Array(state.maxSelectedTypes).fill(null);
  state.contributions = Array(state.maxSelectedTypes).fill(0);
  state.mainContribution = 0;
  state.totalScore = 0;

  state.policySummary = null;
  state.policyError = null;
  state.policyReady = false;
  state.suggestion = null;

  state.reroll = {
    targetScore: state.defaultFixedTargetScore,
    policyReady: false,
    baselineSelections: Array(state.maxSelectedTypes).fill(null),
    candidateSelections: Array(state.maxSelectedTypes).fill(null),
    output: null,
    error: null,
  };

  state.ocr = {
    listening: false,
    port: normalizeOcrPort(data.defaultOcrUdpPort, defaultOcrUdpPort),
    lastError: null,
  };

  elements.scorerTypeSelect.value = state.scorerType;
  elements.costWEchoInput.value = state.costWeights.wEcho.toFixed(1);
  elements.costWTunerInput.value = state.costWeights.wTuner.toFixed(1);
  elements.costWExpInput.value = state.costWeights.wExp.toFixed(1);
  elements.blendDataSelect.value = state.blendData ? 'true' : 'false';
  elements.expRefundInput.value = state.expRefundRatio.toFixed(2);
}
