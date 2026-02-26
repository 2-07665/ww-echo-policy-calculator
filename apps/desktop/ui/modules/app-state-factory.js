export function createInitialAppState({
  createScorerConfigMap,
  createScorerTypeMap,
  scorerLinearDefault,
  scorerPresetCustom,
  defaultMcBoostAssistantTargetScore,
  defaultQqBotTargetScore,
  defaultOcrUdpPort,
}) {
  return {
    buffTypes: [],
    buffLabels: {},
    buffTypeMaxValues: [],
    maxSelectedTypes: 5,
    buffValueOptions: new Map(),
    percentBuffs: new Set(),

    scorerType: scorerLinearDefault,
    scorerConfigs: createScorerConfigMap(),
    defaultScorerConfigs: createScorerConfigMap(),
    scorerPresets: createScorerTypeMap(() => []),
    activePresetNames: createScorerTypeMap(() => scorerPresetCustom),
    activePresetVariantNames: createScorerTypeMap(() => ''),
    scorerPresetStatus: '',
    scorerPresetStatusError: false,

    buffSelections: [],
    buffValues: [],
    contributions: [],
    mainContribution: 0,
    topWeightsSum: 0,
    totalScore: 0,
    displayMaxScore: 0,

    policySummary: null,
    policyError: null,
    policyReady: false,
    suggestion: null,

    defaultTargetScore: 60,
    defaultFixedTargetScore: 60,
    defaultWuwaEchoToolTargetScore: 60,
    defaultMcBoostAssistantTargetScore,
    defaultQqBotTargetScore,
    targetScore: 60,

    expRefundRatio: 0.66,
    blendData: false,
    costWeights: { wEcho: 0.0, wTuner: 1.0, wExp: 0.0 },

    activeTab: 'upgrade',
    scorerBeforeReroll: null,
    targetScoreBeforeReroll: null,
    ocr: {
      listening: false,
      port: defaultOcrUdpPort,
      lastError: null,
    },

    reroll: {
      targetScore: 60,
      policyReady: false,
      baselineSelections: [],
      candidateSelections: [],
      output: null,
      error: null,
    },
  };
}
