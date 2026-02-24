(() => {
  const PLACEHOLDER_LABEL = '选择词条';
  const TARGET_SCORE_STEP = 0.01;
  const TARGET_SCORE_DIGITS = 2;

  const SCORER_LINEAR_DEFAULT = 'linear_default';
  const SCORER_MC_BOOST_ASSISTANT = 'mc_boost_assistant';
  const SCORER_QQ_BOT = 'qq_bot';
  const SCORER_FIXED = 'fixed';
  const DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE = 95.0;
  const DEFAULT_QQ_BOT_TARGET_SCORE = 35.0;
  const MC_BOOST_ASSISTANT_LOCKED_MAIN_BUFF_SCORE = 0.0;
  const MC_BOOST_ASSISTANT_LOCKED_NORMALIZED_MAX_SCORE = 120.0;
  const QQ_BOT_LOCKED_NORMALIZED_MAX_SCORE = 50.0;

  const state = {
    buffTypes: [],
    buffLabels: {},
    buffTypeMaxValues: [],
    maxSelectedTypes: 5,
    buffValueOptions: new Map(),
    percentBuffs: new Set(),

    scorerType: SCORER_LINEAR_DEFAULT,
    scorerConfigs: {
      [SCORER_LINEAR_DEFAULT]: {
        mainBuffScore: 0,
        normalizedMaxScore: 100,
        weights: {},
      },
      [SCORER_MC_BOOST_ASSISTANT]: {
        mainBuffScore: MC_BOOST_ASSISTANT_LOCKED_MAIN_BUFF_SCORE,
        normalizedMaxScore: MC_BOOST_ASSISTANT_LOCKED_NORMALIZED_MAX_SCORE,
        weights: {},
      },
      [SCORER_QQ_BOT]: {
        mainBuffScore: 0,
        normalizedMaxScore: QQ_BOT_LOCKED_NORMALIZED_MAX_SCORE,
        weights: {},
      },
      [SCORER_FIXED]: {
        weights: {},
      },
    },

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
    defaultMcBoostAssistantTargetScore: DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE,
    defaultQqBotTargetScore: DEFAULT_QQ_BOT_TARGET_SCORE,
    targetScore: 60,

    expRefundRatio: 0.66,
    blendData: false,
    costWeights: { wEcho: 0.0, wTuner: 1.0, wExp: 0.0 },

    activeTab: 'upgrade',
    scorerBeforeReroll: null,

    reroll: {
      targetScore: 60,
      policyReady: false,
      baselineSelections: [],
      candidateSelections: [],
      output: null,
      error: null,
    },
  };

  const elements = {};
  let suggestionRequestToken = 0;
  let rerollRecommendationToken = 0;
  let scorePreviewRequestToken = 0;

  function escapeHtml(value) {
    return String(value)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function invoke(command, args = {}) {
    const api = window.__TAURI__?.core;
    if (!api || typeof api.invoke !== 'function') {
      throw new Error('Tauri API is unavailable');
    }
    return api.invoke(command, args);
  }

  function numberOr(value, fallback = 0) {
    const num = Number(value);
    return Number.isFinite(num) ? num : fallback;
  }

  function formatFixedOr(value, digits, fallback = '--') {
    const num = Number(value);
    return Number.isFinite(num) ? num.toFixed(digits) : fallback;
  }

  function normalizeScorerType(value) {
    const lowered = String(value || '').toLowerCase();
    if (lowered === 'linear') {
      return SCORER_LINEAR_DEFAULT;
    }
    if (lowered === SCORER_MC_BOOST_ASSISTANT) {
      return SCORER_MC_BOOST_ASSISTANT;
    }
    if (lowered === SCORER_QQ_BOT) {
      return SCORER_QQ_BOT;
    }
    if (lowered === SCORER_FIXED) {
      return SCORER_FIXED;
    }
    return SCORER_LINEAR_DEFAULT;
  }

  function isFixedScorer(type = state.scorerType) {
    return type === SCORER_FIXED;
  }

  function isQqBotScorer(type = state.scorerType) {
    return type === SCORER_QQ_BOT;
  }

  function isMcBoostAssistantScorer(type = state.scorerType) {
    return type === SCORER_MC_BOOST_ASSISTANT;
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
      return MC_BOOST_ASSISTANT_LOCKED_MAIN_BUFF_SCORE;
    }
    return Math.max(0, numberOr(getScorerConfig(type).mainBuffScore, 0));
  }

  function getNormalizedMaxScore(type = state.scorerType) {
    if (isFixedScorer(type)) {
      return 0;
    }
    if (isMcBoostAssistantScorer(type)) {
      return MC_BOOST_ASSISTANT_LOCKED_NORMALIZED_MAX_SCORE;
    }
    if (isQqBotScorer(type)) {
      return QQ_BOT_LOCKED_NORMALIZED_MAX_SCORE;
    }
    return Math.max(TARGET_SCORE_STEP, numberOr(getScorerConfig(type).normalizedMaxScore, 0));
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
    return numeric.toFixed(TARGET_SCORE_DIGITS);
  }

  function cacheElements() {
    elements.weightInputsContainer = document.getElementById('weight-inputs');
    elements.tabUpgrade = document.getElementById('tab-upgrade');
    elements.tabReroll = document.getElementById('tab-reroll');
    elements.upgradeTab = document.getElementById('upgrade-tab');
    elements.rerollTab = document.getElementById('reroll-tab');
    elements.buffSlotsContainer = document.getElementById('buff-slots');
    elements.clearBuffsButton = document.getElementById('clear-buffs');
    elements.scoreCard = document.getElementById('score-card');
    elements.scorerTypeSelect = document.getElementById('scorer-type-select');
    elements.linearParams = document.getElementById('linear-params');
    elements.linearParamsFields = document.getElementById('linear-params-fields');
    elements.mainBuffScoreInput = document.getElementById('main-buff-score-input');
    elements.normalizedMaxScoreInput = document.getElementById('normalized-max-score-input');
    elements.scorerConfigHint = document.getElementById('scorer-config-hint');
    elements.targetScoreInput = document.getElementById('target-score-input');
    elements.blendDataSelect = document.getElementById('blend-data-select');
    elements.costWEchoInput = document.getElementById('cost-w-echo');
    elements.costWTunerInput = document.getElementById('cost-w-tuner');
    elements.costWExpInput = document.getElementById('cost-w-exp');
    elements.expRefundInput = document.getElementById('exp-refund-input');
    elements.computeButton = document.getElementById('compute-button');
    elements.resultsSection = document.getElementById('results-section');
    elements.rerollTargetScoreInput = document.getElementById('reroll-target-score-input');
    elements.rerollComputeButton = document.getElementById('reroll-compute-button');
    elements.rerollClearBaselineButton = document.getElementById('reroll-clear-baseline-button');
    elements.rerollReplaceButton = document.getElementById('reroll-replace-button');
    elements.rerollSlots = document.getElementById('reroll-slots');
    elements.rerollOutput = document.getElementById('reroll-output');
  }

  function copyWeightMap(src) {
    const out = {};
    state.buffTypes.forEach((name) => {
      out[name] = numberOr(src?.[name], 0);
    });
    return out;
  }

  function initialiseState(data) {
    state.buffTypes = data.buffTypes || [];
    state.buffLabels = data.buffLabels || {};
    state.buffTypeMaxValues = (data.buffTypeMaxValues || []).map(Number);
    state.maxSelectedTypes = Number(data.maxSelectedTypes || 5);

    state.defaultTargetScore = Number(data.defaultTargetScore || 60);
    state.defaultFixedTargetScore = Number(data.defaultFixedTargetScore || Math.round(state.defaultTargetScore));
    state.defaultMcBoostAssistantTargetScore = DEFAULT_MC_BOOST_ASSISTANT_TARGET_SCORE;
    state.defaultQqBotTargetScore = DEFAULT_QQ_BOT_TARGET_SCORE;

    const defaultLinearWeights = data.defaultLinearBuffWeights || data.defaultBuffWeights || {};
    const defaultMcBoostAssistantWeights =
      data.defaultMcBoostAssistantBuffWeights || defaultLinearWeights;
    const defaultQQWeights = data.defaultQqBotBuffWeights || {};
    const defaultFixedWeights = data.defaultFixedBuffWeights || {};

    state.scorerConfigs[SCORER_LINEAR_DEFAULT].weights = copyWeightMap(defaultLinearWeights);
    state.scorerConfigs[SCORER_MC_BOOST_ASSISTANT].weights = copyWeightMap(defaultMcBoostAssistantWeights);
    state.scorerConfigs[SCORER_QQ_BOT].weights = copyWeightMap(defaultQQWeights);
    state.scorerConfigs[SCORER_FIXED].weights = copyWeightMap(defaultFixedWeights);

    state.scorerConfigs[SCORER_LINEAR_DEFAULT].mainBuffScore = Number(
      data.defaultLinearMainBuffScore ?? 0,
    );
    state.scorerConfigs[SCORER_LINEAR_DEFAULT].normalizedMaxScore = Number(
      data.defaultLinearNormalizedMaxScore ?? 100,
    );

    state.scorerConfigs[SCORER_QQ_BOT].mainBuffScore = Number(
      data.defaultQqBotMainBuffScore ?? 0,
    );
    state.scorerConfigs[SCORER_QQ_BOT].normalizedMaxScore = Number(
      data.defaultQqBotNormalizedMaxScore ?? 50,
    );
    state.scorerConfigs[SCORER_MC_BOOST_ASSISTANT].mainBuffScore =
      MC_BOOST_ASSISTANT_LOCKED_MAIN_BUFF_SCORE;
    state.scorerConfigs[SCORER_MC_BOOST_ASSISTANT].normalizedMaxScore =
      MC_BOOST_ASSISTANT_LOCKED_NORMALIZED_MAX_SCORE;

    state.scorerType = normalizeScorerType(data.defaultScorerType);
    if (isFixedScorer()) {
      state.targetScore = state.defaultFixedTargetScore;
    } else if (state.scorerType === SCORER_MC_BOOST_ASSISTANT) {
      state.targetScore = state.defaultMcBoostAssistantTargetScore;
    } else if (state.scorerType === SCORER_QQ_BOT) {
      state.targetScore = state.defaultQqBotTargetScore;
    } else {
      state.targetScore = state.defaultTargetScore;
    }
    state.displayMaxScore = isFixedScorer()
      ? computeTopWeightsSumForType(SCORER_FIXED)
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

    elements.scorerTypeSelect.value = state.scorerType;
    elements.costWEchoInput.value = state.costWeights.wEcho.toFixed(1);
    elements.costWTunerInput.value = state.costWeights.wTuner.toFixed(1);
    elements.costWExpInput.value = state.costWeights.wExp.toFixed(1);
    elements.blendDataSelect.value = state.blendData ? 'true' : 'false';
    elements.expRefundInput.value = state.expRefundRatio.toFixed(2);
  }

  function roundToStep(value, step = TARGET_SCORE_STEP) {
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
    if (type === SCORER_MC_BOOST_ASSISTANT) {
      return state.defaultMcBoostAssistantTargetScore;
    }
    if (type === SCORER_QQ_BOT) {
      return state.defaultQqBotTargetScore;
    }
    return state.defaultTargetScore;
  }

  function updateTargetScoreUI({ setRecommended = false } = {}) {
    if (isFixedScorer()) {
      const maxScore = computeTopWeightsSumForType(SCORER_FIXED);
      if (setRecommended) {
        state.targetScore = recommendedTargetForScorer(SCORER_FIXED);
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

    state.targetScore = Math.max(0, roundToStep(numberOr(state.targetScore, 0), TARGET_SCORE_STEP));
    elements.targetScoreInput.step = String(TARGET_SCORE_STEP);
    elements.targetScoreInput.max = normalizedMax.toFixed(TARGET_SCORE_DIGITS);
    elements.targetScoreInput.value = state.targetScore.toFixed(TARGET_SCORE_DIGITS);
  }

  function updateRerollTargetScoreUI({ setRecommended = false } = {}) {
    const maxScore = computeTopWeightsSumForType(SCORER_FIXED);
    if (setRecommended) {
      state.reroll.targetScore = recommendedTargetForScorer(SCORER_FIXED);
    } else if (maxScore > 0 && state.reroll.targetScore > maxScore) {
      state.reroll.targetScore = maxScore;
    }

    state.reroll.targetScore = Math.max(0, Math.round(numberOr(state.reroll.targetScore, 0)));
    elements.rerollTargetScoreInput.value = String(state.reroll.targetScore);
  }

  async function setActiveTab(tab) {
    state.activeTab = tab === 'reroll' ? 'reroll' : 'upgrade';
    const upgradeActive = state.activeTab === 'upgrade';

    if (!upgradeActive) {
      if (state.scorerType !== SCORER_FIXED) {
        state.scorerBeforeReroll = state.scorerType;
        await applyScorerType(SCORER_FIXED, { setRecommendedTarget: true });
      }
      elements.scorerTypeSelect.disabled = true;
    } else {
      elements.scorerTypeSelect.disabled = false;
      if (state.scorerBeforeReroll && state.scorerType === SCORER_FIXED) {
        const restoreScorer = state.scorerBeforeReroll;
        state.scorerBeforeReroll = null;
        await applyScorerType(restoreScorer, { setRecommendedTarget: false });
      } else {
        state.scorerBeforeReroll = null;
        renderScorerConfig();
      }
    }

    elements.upgradeTab.hidden = !upgradeActive;
    elements.rerollTab.hidden = upgradeActive;
    elements.tabUpgrade.classList.toggle('active', upgradeActive);
    elements.tabReroll.classList.toggle('active', !upgradeActive);
  }

  async function applyScorerType(nextScorerType, { setRecommendedTarget = true } = {}) {
    state.scorerType = normalizeScorerType(nextScorerType);
    renderScorerConfig();
    renderWeightInputs();
    updateTargetScoreUI({ setRecommended: setRecommendedTarget });
    resetPolicyResult();

    if (state.scorerType === SCORER_FIXED) {
      updateRerollTargetScoreUI();
      invalidateRerollPolicy();
    } else {
      renderRerollSlots();
      updateRerollComputeButtonState();
    }

    await computeContributions();
  }

  function renderScorerConfig() {
    elements.scorerTypeSelect.value = state.scorerType;

    if (isFixedScorer()) {
      elements.linearParams.hidden = true;
      elements.linearParams.style.display = 'none';
      elements.mainBuffScoreInput.disabled = true;
      elements.normalizedMaxScoreInput.disabled = true;
      elements.scorerConfigHint.textContent =
        '固定评分模式下，每个词条直接按设置的权重计分，不受词条数值高低影响。\n权重和目标分数只能为整数。\n此评分模式主要用于只关注词条数目的场景。';
      return;
    }

    const config = getScorerConfig();
    elements.linearParams.hidden = false;
    elements.linearParams.style.display = '';

    config.mainBuffScore = Math.max(0, numberOr(config.mainBuffScore, 0));
    config.normalizedMaxScore = Math.max(TARGET_SCORE_STEP, numberOr(config.normalizedMaxScore, 0));

    if (isMcBoostAssistantScorer()) {
      config.mainBuffScore = MC_BOOST_ASSISTANT_LOCKED_MAIN_BUFF_SCORE;
      config.normalizedMaxScore = MC_BOOST_ASSISTANT_LOCKED_NORMALIZED_MAX_SCORE;
      elements.mainBuffScoreInput.disabled = true;
      elements.normalizedMaxScoreInput.disabled = true;
    } else if (isQqBotScorer()) {
      config.normalizedMaxScore = QQ_BOT_LOCKED_NORMALIZED_MAX_SCORE;
      elements.mainBuffScoreInput.disabled = false;
      elements.normalizedMaxScoreInput.disabled = true;
    } else {
      elements.mainBuffScoreInput.disabled = false;
      elements.normalizedMaxScoreInput.disabled = false;
    }

    elements.mainBuffScoreInput.value = config.mainBuffScore.toFixed(TARGET_SCORE_DIGITS);
    elements.normalizedMaxScoreInput.value = config.normalizedMaxScore.toFixed(TARGET_SCORE_DIGITS);

    if (state.scorerType === SCORER_MC_BOOST_ASSISTANT) {
      elements.scorerConfigHint.textContent =
        '漂泊者强化助手按线性规则评分。归一化总分 120.00，其中主词条固定贡献 50.00。请在小程序中查看词条权重。\n这里的单词条分数为小程序内显示的0.7倍，且未计入对无效生存词条的“安慰分数”。\n注意，该来源的评分权重由词条对期望伤害的提升计算得到，但考虑的是词条对0+1角色与对6+5角色的提升平均值。';
    } else if (state.scorerType === SCORER_QQ_BOT) {
      elements.scorerConfigHint.textContent =
        'WutheringWavesUID （QQ机器人）按线性规则评分，归一化总分 50.00。请使用“ww(角色名)权重”查看角色权重。\n主词条原始分数填写对应Cost声骸两个主词条的权重与数值乘积之和。\n注意，该来源的评分权重与实际词条价值存在相当偏差。';
    } else {
      elements.scorerConfigHint.textContent =
        '自定义线性评分模式下，按词条数值占该词条最大值比例线性计分。词条权重对应词条最大值的原始分数。\n可设置声骸主词条的原始分数与归一化总分。权重与分数支持小数。\n建议配合拉表计算得到的权重结果使用。';
    }
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
            : Math.max(TARGET_SCORE_STEP, numberOr(config.normalizedMaxScore, 0)),
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
        ? Math.max(0, Math.round(computeTopWeightsSumForType(SCORER_FIXED)))
        : getNormalizedMaxScore(scorerType);
    }

    renderBuffSlots();
    renderTotalScoreCard();
    updateComputeButtonState();
  }

  function formatBuffLabel(buffName, weightMap = getWeightMap()) {
    const label = state.buffLabels[buffName] ?? buffName;
    const weight = Number(weightMap[buffName] ?? 0);
    if (Math.abs(weight) < 1e-9) {
      return `（无效词条）${label}`;
    }
    return label;
  }

  function formatValueLabel(buffName, value) {
    if (state.percentBuffs.has(buffName)) {
      return `${(Number(value) / 10).toFixed(1)}%`;
    }
    return String(value);
  }

  function renderWeightInputs() {
    const layout = [
      ['Crit_Rate', 'Crit_Damage'],
      ['Attack', 'Attack_Flat'],
      ['HP', 'HP_Flat'],
      ['Defence', 'Defence_Flat'],
      ['Basic_Attack_Damage', 'Heavy_Attack_Damage'],
      ['Skill_Damage', 'Ult_Damage'],
      ['ER'],
    ];

    const container = elements.weightInputsContainer;
    container.innerHTML = '';

    const weightMap = getWeightMap();
    const fixedMode = isFixedScorer();

    const available = new Set(state.buffTypes);
    const seen = new Set();

    const appendField = (buffName) => {
      if (!available.has(buffName)) {
        return;
      }
      seen.add(buffName);

      const wrapper = document.createElement('div');
      wrapper.className = 'weight-field';

      const label = document.createElement('span');
      label.textContent = state.buffLabels[buffName] ?? buffName;

      const input = document.createElement('input');
      input.type = fixedMode ? 'number' : 'text';
      if (fixedMode) {
        input.step = '1';
        input.min = '0';
      } else {
        input.inputMode = 'decimal';
      }
      input.value = fixedMode
        ? String(Math.max(0, Math.round(numberOr(weightMap[buffName], 0))))
        : String(numberOr(weightMap[buffName], 0));

      if (!fixedMode) {
        input.addEventListener('input', () => {
          let sanitized = input.value
            .replace(/[，。,]/g, '.')
            .replace(/[^\d.]/g, '');
          const firstDot = sanitized.indexOf('.');
          if (firstDot !== -1) {
            sanitized =
              sanitized.slice(0, firstDot + 1) + sanitized.slice(firstDot + 1).replace(/\./g, '');
          }
          if (sanitized.startsWith('.')) {
            sanitized = `0${sanitized}`;
          }
          input.value = sanitized;
        });
      }

      input.addEventListener('change', async () => {
        if (fixedMode) {
          const nextValue = Math.max(
            0,
            Math.min(65535, Math.round(numberOr(input.valueAsNumber, 0))),
          );
          weightMap[buffName] = nextValue;
          input.value = String(nextValue);
        } else {
          const nextValue = Math.max(0, numberOr(Number(input.value), 0));
          weightMap[buffName] = nextValue;
        }
        await onWeightsUpdated();
      });

      wrapper.appendChild(label);
      wrapper.appendChild(input);
      container.appendChild(wrapper);
    };

    layout.forEach((row) => {
      row.forEach(appendField);
      if (row.length === 1) {
        const placeholder = document.createElement('div');
        placeholder.className = 'weight-field weight-placeholder';
        container.appendChild(placeholder);
      }
    });

    state.buffTypes.forEach((buffName) => {
      if (!seen.has(buffName)) {
        appendField(buffName);
      }
    });
  }

  function renderBuffSlots() {
    const weightMap = getWeightMap();
    const sortedBuffTypes = [...state.buffTypes].sort((left, right) => {
      const weightDiff = Number(weightMap[right] ?? 0) - Number(weightMap[left] ?? 0);
      if (Math.abs(weightDiff) > 1e-9) {
        return weightDiff;
      }
      const leftLabel = state.buffLabels[left] ?? left;
      const rightLabel = state.buffLabels[right] ?? right;
      return leftLabel.localeCompare(rightLabel, 'zh-Hans-CN');
    });

    const rows = [];

    for (let i = 0; i < state.maxSelectedTypes; i += 1) {
      const buffName = state.buffSelections[i];
      const value = state.buffValues[i];
      const score = state.contributions[i];
      const scoreText = value == null || !buffName ? '-' : formatScoreForScorer(score);
      const scoreClass = value == null || !buffName ? 'slot-score inactive' : 'slot-score';

      const selectedByOthers = new Set(
        state.buffSelections.filter((chosen, idx) => idx !== i && Boolean(chosen)),
      );
      const options = sortedBuffTypes
        .filter((name) => !selectedByOthers.has(name) || name === buffName)
        .map((name) => {
          const selected = buffName === name ? 'selected' : '';
          return `<option value="${escapeHtml(name)}" ${selected}>${escapeHtml(formatBuffLabel(name, weightMap))}</option>`;
        })
        .join('');

      const valueOptions = buffName
        ? (state.buffValueOptions.get(buffName) || [])
            .map((raw) => {
              const selected = Number(value) === Number(raw) ? 'selected' : '';
              return `<option value="${raw}" ${selected}>${escapeHtml(formatValueLabel(buffName, raw))}</option>`;
            })
            .join('')
        : '';

      rows.push(`
        <div class="buff-slot">
          <div class="slot-index">${i + 1}</div>
          <select class="styled-select buff-type-select" data-index="${i}">
            <option value="">${PLACEHOLDER_LABEL}</option>
            ${options}
          </select>
          <select class="styled-select buff-value-select ${buffName ? '' : 'inactive-select'}" data-index="${i}" ${buffName ? '' : 'disabled'}>
            ${buffName ? valueOptions : `<option value=""></option>`}
          </select>
          <div class="${scoreClass}">${scoreText}</div>
        </div>
      `);
    }

    elements.buffSlotsContainer.innerHTML = rows.join('');

    elements.buffSlotsContainer.querySelectorAll('.buff-type-select').forEach((select) => {
      select.addEventListener('change', async (event) => {
        const index = Number(event.target.dataset.index);
        const selected = event.target.value;

        if (!selected) {
          state.buffSelections[index] = null;
          state.buffValues[index] = null;
        } else {
          state.buffSelections[index] = selected;
          const values = state.buffValueOptions.get(selected) || [];
          state.buffValues[index] = values.length > 0 ? values[0] : null;
        }

        await computeContributions();
        if (state.policyReady) {
          await updateSuggestion();
        }
      });
    });

    elements.buffSlotsContainer.querySelectorAll('.buff-value-select').forEach((select) => {
      select.addEventListener('change', async (event) => {
        const index = Number(event.target.dataset.index);
        const value = event.target.value;
        state.buffValues[index] = value ? Number(value) : null;

        await computeContributions();
        if (state.policyReady) {
          await updateSuggestion();
        }
      });
    });
  }

  function isFullUniqueSelection(selections) {
    const filled = selections.filter(Boolean);
    return filled.length === state.maxSelectedTypes && new Set(filled).size === state.maxSelectedTypes;
  }

  function fullSelectionOrNull(selections) {
    if (!isFullUniqueSelection(selections)) {
      return null;
    }
    return selections.map((name) => String(name));
  }

  function clearRerollRecommendation() {
    state.reroll.output = null;
    state.reroll.error = null;
  }

  function buildAcceptSummary() {
    if (!state.reroll.policyReady) {
      return { text: '未计算重抽策略', className: 'info' };
    }
    if (!isFullUniqueSelection(state.reroll.baselineSelections)) {
      return { text: '请补全当前词条', className: 'info' };
    }
    if (state.reroll.error) {
      return { text: '重抽建议查询失败', className: 'no' };
    }
    if (!state.reroll.output) {
      return { text: '计算中…', className: 'info' };
    }
    if (state.reroll.output.valid === false) {
      return { text: '当前输入无有效重抽建议', className: 'no' };
    }
    if (state.reroll.output.acceptCandidate === true) {
      return { text: '可接纳候选结果', className: 'yes' };
    }
    if (state.reroll.output.acceptCandidate === false) {
      return { text: '建议保持当前词条', className: 'no' };
    }
    return { text: '候选结果未补全', className: 'info' };
  }

  function getRerollScoreTexts() {
    const output = state.reroll.output;
    const baselineScoreText =
      output && output.valid !== false && Number.isFinite(Number(output.baselineScore))
        ? String(Math.round(Number(output.baselineScore)))
        : '--';
    const candidateScoreText =
      output &&
      output.valid !== false &&
      output.candidateScore != null &&
      Number.isFinite(Number(output.candidateScore))
        ? String(Math.round(Number(output.candidateScore)))
        : '--';
    return { baselineScoreText, candidateScoreText };
  }

  function updateRerollSlotsMeta() {
    const { baselineScoreText, candidateScoreText } = getRerollScoreTexts();
    const acceptSummary = buildAcceptSummary();

    const baselineScoreNode = elements.rerollSlots.querySelector('[data-role="baseline-score"]');
    const candidateScoreNode = elements.rerollSlots.querySelector('[data-role="candidate-score"]');
    const acceptSummaryNode = elements.rerollSlots.querySelector('[data-role="accept-summary"]');

    if (baselineScoreNode) {
      baselineScoreNode.textContent = baselineScoreText;
    }
    if (candidateScoreNode) {
      candidateScoreNode.textContent = candidateScoreText;
    }
    if (acceptSummaryNode) {
      acceptSummaryNode.textContent = acceptSummary.text;
      acceptSummaryNode.classList.remove('yes', 'no', 'info');
      acceptSummaryNode.classList.add(acceptSummary.className);
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

  function updateRerollComputeButtonState() {
    if (elements.rerollComputeButton.dataset.loading === 'true') {
      return;
    }
    elements.rerollComputeButton.disabled = computeTopWeightsSumForType(SCORER_FIXED) <= 0;
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

  function renderRerollSlots() {
    const fixedWeights = getWeightMap(SCORER_FIXED);
    const sortedBuffTypes = [...state.buffTypes].sort((left, right) => {
      const weightDiff = Number(fixedWeights[right] ?? 0) - Number(fixedWeights[left] ?? 0);
      if (Math.abs(weightDiff) > 1e-9) {
        return weightDiff;
      }
      const leftLabel = state.buffLabels[left] ?? left;
      const rightLabel = state.buffLabels[right] ?? right;
      return leftLabel.localeCompare(rightLabel, 'zh-Hans-CN');
    });

    const { baselineScoreText, candidateScoreText } = getRerollScoreTexts();
    const acceptSummary = buildAcceptSummary();

    const rows = [
      `
      <div class="reroll-slot reroll-slot-header">
        <div>#</div>
        <div>当前词条</div>
        <div>候选结果</div>
      </div>
      <div class="reroll-slot reroll-slot-score">
        <div class="slot-index">分数</div>
        <div class="reroll-score-value" data-role="baseline-score">${baselineScoreText}</div>
        <div class="reroll-score-value" data-role="candidate-score">${candidateScoreText}</div>
      </div>
    `,
    ];

    for (let i = 0; i < state.maxSelectedTypes; i += 1) {
      const baselineSelected = state.reroll.baselineSelections[i];
      const candidateSelected = state.reroll.candidateSelections[i];

      const baselineSelectedByOthers = new Set(
        state.reroll.baselineSelections.filter((name, idx) => idx !== i && Boolean(name)),
      );
      const candidateSelectedByOthers = new Set(
        state.reroll.candidateSelections.filter((name, idx) => idx !== i && Boolean(name)),
      );

      const baselineOptions = sortedBuffTypes
        .filter((name) => !baselineSelectedByOthers.has(name) || name === baselineSelected)
        .map((name) => {
          const selected = baselineSelected === name ? 'selected' : '';
          return `<option value="${escapeHtml(name)}" ${selected}>${escapeHtml(formatBuffLabel(name, fixedWeights))}</option>`;
        })
        .join('');

      const candidateOptions = sortedBuffTypes
        .filter((name) => !candidateSelectedByOthers.has(name) || name === candidateSelected)
        .map((name) => {
          const selected = candidateSelected === name ? 'selected' : '';
          return `<option value="${escapeHtml(name)}" ${selected}>${escapeHtml(formatBuffLabel(name, fixedWeights))}</option>`;
        })
        .join('');

      rows.push(`
        <div class="reroll-slot">
          <div class="slot-index">${i + 1}</div>
          <select class="styled-select reroll-baseline-select" data-index="${i}">
            <option value="">${PLACEHOLDER_LABEL}</option>
            ${baselineOptions}
          </select>
          <select class="styled-select reroll-candidate-select" data-index="${i}">
            <option value="">${PLACEHOLDER_LABEL}</option>
            ${candidateOptions}
          </select>
        </div>
      `);
    }

    rows.push(`
      <div class="reroll-slot-status ${acceptSummary.className}" data-role="accept-summary">
        ${escapeHtml(acceptSummary.text)}
      </div>
    `);

    elements.rerollSlots.innerHTML = rows.join('');

    elements.rerollSlots.querySelectorAll('.reroll-baseline-select').forEach((select) => {
      select.addEventListener('change', (event) => {
        const index = Number(event.target.dataset.index);
        state.reroll.baselineSelections[index] = event.target.value || null;
        renderRerollSlots();
        if (state.reroll.policyReady) {
          updateRerollRecommendation();
        } else {
          clearRerollRecommendation();
          renderRerollOutput();
        }
      });
    });

    elements.rerollSlots.querySelectorAll('.reroll-candidate-select').forEach((select) => {
      select.addEventListener('change', (event) => {
        const index = Number(event.target.dataset.index);
        state.reroll.candidateSelections[index] = event.target.value || null;
        renderRerollSlots();
        if (state.reroll.policyReady) {
          updateRerollRecommendation();
        } else {
          clearRerollRecommendation();
          renderRerollOutput();
        }
      });
    });

    updateRerollComputeButtonState();
  }

  function renderRerollOutput() {
    if (state.reroll.error) {
      elements.rerollOutput.innerHTML = `<div class="error-message">${escapeHtml(state.reroll.error)}</div>`;
      return;
    }

    if (!state.reroll.policyReady) {
      elements.rerollOutput.innerHTML = '<div class="empty-state">暂无重抽建议</div>';
      return;
    }

    const baselineValid = isFullUniqueSelection(state.reroll.baselineSelections);
    if (!baselineValid) {
      elements.rerollOutput.innerHTML = '<div class="empty-state">请补全当前词条</div>';
      return;
    }

    if (!state.reroll.output) {
      elements.rerollOutput.innerHTML = '<div class="empty-state">计算中…</div>';
      return;
    }

    const output = state.reroll.output;
    if (output.valid === false) {
      elements.rerollOutput.innerHTML = `<div class="warning">${escapeHtml(output.reason || '当前输入无有效重抽建议。')}</div>`;
      return;
    }
    const choices = output.recommendedLockChoices || [];
    const tableRows =
      choices.length === 0
        ? `
          <tr class="reroll-table-empty">
            <td colspan="6">当前状态已达到目标，无需锁定。</td>
          </tr>
        `
        : choices
            .map((choice, index) => {
              const lockSlotText =
                Array.isArray(choice.lockSlotIndices) && choice.lockSlotIndices.length > 0
                  ? choice.lockSlotIndices.map((slotNo) => Number(slotNo)).filter(Number.isFinite).join(', ')
                  : '无';
              const expectedCostText = formatFixedOr(choice.expectedCost, 2);
              const successProbability = Number(choice.successProbability);
              const successText = Number.isFinite(successProbability)
                ? `${(successProbability * 100).toFixed(2)}%`
                : '--';
              const regretValue = Number(choice.regret);
              const regretText =
                index === 0 || !Number.isFinite(regretValue)
                  ? '--'
                  : `${regretValue >= 0 ? '+' : ''}${regretValue.toFixed(2)}`;

              return `
          <tr>
            <td>${index + 1}</td>
            <td><strong class="reroll-lock-slots">${lockSlotText}</strong></td>
            <td>${expectedCostText}</td>
            <td>${successText}</td>
            <td>${regretText}</td>
            <td><button type="button" class="secondary-button reroll-pick-button" data-choice-index="${index}">选择</button></td>
          </tr>
        `;
            })
            .join('');

    elements.rerollOutput.innerHTML = `
      <div class="reroll-table-wrap">
        <table class="reroll-table">
          <thead>
            <tr>
              <th>序号</th>
              <th>锁定槽位</th>
              <th>期望消耗</th>
              <th>重抽成功率</th>
              <th>额外消耗</th>
              <th>操作</th>
            </tr>
          </thead>
          <tbody>${tableRows}</tbody>
        </table>
      </div>
    `;

    elements.rerollOutput.querySelectorAll('.reroll-pick-button').forEach((button) => {
      button.addEventListener('click', () => {
        const choiceIndex = Number(button.dataset.choiceIndex);
        const choice = state.reroll.output?.recommendedLockChoices?.[choiceIndex];
        if (!choice || !Array.isArray(choice.lockSlotIndices)) {
          return;
        }

        const nextCandidate = Array(state.maxSelectedTypes).fill(null);
        choice.lockSlotIndices.forEach((slotNo) => {
          const slotIndex = Number(slotNo) - 1;
          if (slotIndex < 0 || slotIndex >= state.maxSelectedTypes) {
            return;
          }
          nextCandidate[slotIndex] = state.reroll.baselineSelections[slotIndex] || null;
        });

        state.reroll.candidateSelections = nextCandidate;
        renderRerollSlots();
        if (state.reroll.policyReady) {
          updateRerollRecommendation();
        } else {
          clearRerollRecommendation();
          renderRerollOutput();
        }
      });
    });
  }

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

  function selectedBuffState() {
    const { names, values } = selectedBuffStateWithSlots();
    return { names, values };
  }

  function renderSuggestionBlock() {
    if (!state.policyReady) {
      return '<div class="empty-state">计算策略后将显示强化建议。</div>';
    }

    const selectedCount = state.buffSelections.filter(Boolean).length;
    const targetScore = Number(state.policySummary?.targetScore ?? state.targetScore);
    if (selectedCount >= state.maxSelectedTypes) {
      const reached = state.totalScore + 1e-9 >= targetScore;
      const className = reached
        ? 'suggestion-box suggestion-continue'
        : 'suggestion-box suggestion-abandon';
      const text = reached ? '已达成目标分数' : '未达成目标分数';
      return `<div class="${className}"><div>${escapeHtml(text)}</div></div>`;
    }

    const rawSuggestion = state.suggestion?.suggestion ?? null;
    if (!rawSuggestion) {
      return '<div class="suggestion-box suggestion-info">暂无强化建议。</div>';
    }

    let className = 'suggestion-box';
    let text = rawSuggestion;

    if (rawSuggestion === 'Continue') {
      className += ' suggestion-continue';
      text = '建议继续';
    } else if (rawSuggestion === 'Abandon') {
      className += ' suggestion-abandon';
      text = '建议放弃';
    } else {
      className += ' suggestion-info';
    }

    return `<div class="${className}"><div>${escapeHtml(text)}</div></div>`;
  }

  function renderTotalScoreCard() {
    const scorerType = state.scorerType;
    const fixedMaxScore = Math.max(
      0,
      Math.round(numberOr(state.displayMaxScore, computeTopWeightsSumForType(SCORER_FIXED))),
    );
    const normalizedMaxScore = Math.max(
      TARGET_SCORE_STEP,
      numberOr(state.displayMaxScore, getNormalizedMaxScore(scorerType)),
    );
    const totalScoreText = isFixedScorer(scorerType)
      ? `${formatScoreForScorer(state.totalScore, scorerType)} / ${fixedMaxScore}`
      : `${state.totalScore.toFixed(TARGET_SCORE_DIGITS)} / ${normalizedMaxScore.toFixed(TARGET_SCORE_DIGITS)}`;

    const selectedCount = state.buffSelections.filter(Boolean).length;
    const targetScore = Number(state.policySummary?.targetScore ?? state.targetScore);
    const targetText = formatScoreForScorer(targetScore, scorerType);

    const stage = state.suggestion?.stage ?? selectedCount;
    const successProbability = Number(state.suggestion?.successProbability);
    const successBadge = Number.isFinite(successProbability)
      ? `<span class="score-success">当前成功率 ${(successProbability * 100).toFixed(4)}%</span>`
      : '';

    const summaryHtml = `
      <div class="score-summary">
        <div class="score-header">
          <div class="score-meter">
            <div class="score-label">当前总分</div>
            <div class="score-value">${totalScoreText}</div>
          </div>
          <div class="score-meta">
            <span class="score-target">目标分数 ${targetText}</span>
            <span class="score-stage">已揭示 ${stage}/${state.maxSelectedTypes} 词条</span>
            ${successBadge}
          </div>
        </div>
        <div class="score-suggestion">
          <div class="score-suggestion-title">强化建议</div>
          ${renderSuggestionBlock()}
        </div>
      </div>
    `;

    const warningHtml =
      state.topWeightsSum <= 0
        ? '<div class="warning">请输入至少一个大于 0 的权重以计算评分。</div>'
        : '';

    elements.scoreCard.innerHTML = summaryHtml + warningHtml;
  }

  function renderMetric(label, value) {
    return `
      <div class="metric">
        <span class="label">${escapeHtml(label)}</span>
        <span class="value">${escapeHtml(value)}</span>
      </div>
    `;
  }

  function renderResults() {
    const container = elements.resultsSection;
    container.innerHTML = '';

    if (state.policyError) {
      container.innerHTML = `<div class="error-message">策略计算失败：${escapeHtml(state.policyError)}</div>`;
      return;
    }

    if (!state.policySummary) {
      return;
    }

    const s = state.policySummary;
    const summaryCard = document.createElement('section');
    summaryCard.className = 'card';
    summaryCard.innerHTML = `
      <div class="card-title">策略计算结果</div>
      <div class="simulation-grid">
        <div class="simulation-row">
          ${renderMetric('λ*', Number(s.lambdaStar).toFixed(6))}
          ${renderMetric('期望成本', Number.isFinite(Number(s.expectedCostPerSuccess)) ? Number(s.expectedCostPerSuccess).toFixed(2) : '∞')}
          ${renderMetric('成功率', `${(Number(s.successProbability) * 100).toFixed(4)}%`)}
        </div>
        <div class="simulation-row">
          ${renderMetric('胚子消耗', Number(s.echoPerSuccess).toFixed(2))}
          ${renderMetric('调谐器消耗', Number(s.tunerPerSuccess).toFixed(2))}
          ${renderMetric('金密音筒消耗', Number(s.expPerSuccess).toFixed(2))}
        </div>
      </div>
      <div class="result-meta">DP 计算耗时 ${Number(s.computeSeconds).toFixed(3)} 秒</div>
    `;

    container.appendChild(summaryCard);
  }

  function resetPolicyResult() {
    state.policySummary = null;
    state.policyError = null;
    state.policyReady = false;
    state.suggestion = null;
    renderResults();
    renderTotalScoreCard();
  }

  async function onWeightsUpdated() {
    updateTargetScoreUI();
    resetPolicyResult();

    if (state.scorerType === SCORER_FIXED) {
      updateRerollTargetScoreUI();
      invalidateRerollPolicy();
    } else {
      renderRerollSlots();
      updateRerollComputeButtonState();
    }

    await computeContributions();
  }

  async function onScorerParamsUpdated() {
    updateTargetScoreUI();
    resetPolicyResult();
    await computeContributions();
  }

  function setupEventHandlers() {
    elements.tabUpgrade.addEventListener('click', async () => {
      await setActiveTab('upgrade');
    });

    elements.tabReroll.addEventListener('click', async () => {
      await setActiveTab('reroll');
    });

    elements.clearBuffsButton.addEventListener('click', async () => {
      state.buffSelections.fill(null);
      state.buffValues.fill(null);
      state.contributions.fill(0);
      state.mainContribution = 0;
      state.totalScore = isFixedScorer() ? 0 : getMainBuffScore();
      await computeContributions();
      if (state.policyReady) {
        await updateSuggestion();
      }
    });

    elements.scorerTypeSelect.addEventListener('change', async () => {
      await applyScorerType(elements.scorerTypeSelect.value, { setRecommendedTarget: true });
    });

    elements.mainBuffScoreInput.addEventListener('change', async () => {
      if (isFixedScorer() || isMcBoostAssistantScorer()) {
        return;
      }
      const config = getScorerConfig();
      config.mainBuffScore = Math.max(0, numberOr(elements.mainBuffScoreInput.valueAsNumber, config.mainBuffScore));
      elements.mainBuffScoreInput.value = config.mainBuffScore.toFixed(TARGET_SCORE_DIGITS);
      await onScorerParamsUpdated();
    });

    elements.normalizedMaxScoreInput.addEventListener('change', async () => {
      if (isFixedScorer() || isQqBotScorer() || isMcBoostAssistantScorer()) {
        return;
      }
      const config = getScorerConfig();
      config.normalizedMaxScore = Math.max(
        TARGET_SCORE_STEP,
        numberOr(elements.normalizedMaxScoreInput.valueAsNumber, config.normalizedMaxScore),
      );
      elements.normalizedMaxScoreInput.value = config.normalizedMaxScore.toFixed(TARGET_SCORE_DIGITS);
      await onScorerParamsUpdated();
    });

    elements.targetScoreInput.addEventListener('change', () => {
      if (isFixedScorer()) {
        state.targetScore = Math.max(0, Math.round(numberOr(elements.targetScoreInput.valueAsNumber, state.targetScore)));
      } else {
        state.targetScore = Math.max(
          0,
          roundToStep(numberOr(elements.targetScoreInput.valueAsNumber, state.targetScore), TARGET_SCORE_STEP),
        );
      }
      updateTargetScoreUI();
      resetPolicyResult();
    });

    elements.blendDataSelect.addEventListener('change', () => {
      state.blendData = elements.blendDataSelect.value === 'true';
      resetPolicyResult();
    });

    elements.costWEchoInput.addEventListener('change', () => {
      state.costWeights.wEcho = Math.max(0, numberOr(elements.costWEchoInput.valueAsNumber, state.costWeights.wEcho));
      elements.costWEchoInput.value = state.costWeights.wEcho.toFixed(1);
      resetPolicyResult();
    });

    elements.costWTunerInput.addEventListener('change', () => {
      state.costWeights.wTuner = Math.max(0, numberOr(elements.costWTunerInput.valueAsNumber, state.costWeights.wTuner));
      elements.costWTunerInput.value = state.costWeights.wTuner.toFixed(1);
      resetPolicyResult();
    });

    elements.costWExpInput.addEventListener('change', () => {
      state.costWeights.wExp = Math.max(0, numberOr(elements.costWExpInput.valueAsNumber, state.costWeights.wExp));
      elements.costWExpInput.value = state.costWeights.wExp.toFixed(1);
      resetPolicyResult();
    });

    elements.expRefundInput.addEventListener('change', () => {
      state.expRefundRatio = Math.max(0, Math.min(0.75, numberOr(elements.expRefundInput.valueAsNumber, state.expRefundRatio)));
      state.expRefundRatio = roundToStep(state.expRefundRatio, TARGET_SCORE_STEP);
      elements.expRefundInput.value = state.expRefundRatio.toFixed(TARGET_SCORE_DIGITS);
      resetPolicyResult();
    });

    elements.computeButton.addEventListener('click', () => {
      handleCompute();
    });

    elements.rerollTargetScoreInput.addEventListener('change', () => {
      state.reroll.targetScore = Math.max(
        0,
        Math.round(numberOr(elements.rerollTargetScoreInput.valueAsNumber, state.reroll.targetScore)),
      );
      updateRerollTargetScoreUI();
      invalidateRerollPolicy();
    });

    elements.rerollComputeButton.addEventListener('click', () => {
      handleRerollCompute();
    });

    elements.rerollClearBaselineButton.addEventListener('click', () => {
      state.reroll.baselineSelections = Array(state.maxSelectedTypes).fill(null);
      renderRerollSlots();
      if (state.reroll.policyReady) {
        updateRerollRecommendation();
      } else {
        clearRerollRecommendation();
        renderRerollOutput();
      }
    });

    elements.rerollReplaceButton.addEventListener('click', () => {
      if (!isFullUniqueSelection(state.reroll.candidateSelections)) {
        return;
      }
      state.reroll.baselineSelections = [...state.reroll.candidateSelections];
      state.reroll.candidateSelections = Array(state.maxSelectedTypes).fill(null);
      renderRerollSlots();
      if (state.reroll.policyReady) {
        updateRerollRecommendation();
      } else {
        clearRerollRecommendation();
        renderRerollOutput();
      }
    });
  }

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
    const weightMap = getWeightMap(SCORER_FIXED);
    const payloadWeights = {};

    state.buffTypes.forEach((buffName) => {
      const raw = Number(weightMap[buffName] ?? 0);
      payloadWeights[buffName] = Math.max(0, Math.round(raw));
    });

    return payloadWeights;
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
        : Math.max(TARGET_SCORE_STEP, numberOr(config.normalizedMaxScore, 0)),
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
      updateSuggestion();
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

  async function handleRerollCompute() {
    if (computeTopWeightsSumForType(SCORER_FIXED) <= 0) {
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

  async function updateSuggestion() {
    if (!state.policyReady) {
      state.suggestion = null;
      renderTotalScoreCard();
      return;
    }

    const selected = selectedBuffState();
    const token = ++suggestionRequestToken;
    try {
      const response = await invoke('policy_suggestion', {
        payload: {
          buffNames: selected.names,
          buffValues: selected.values,
          totalScore: state.totalScore,
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

  function updateComputeButtonState() {
    if (elements.computeButton.dataset.loading === 'true') {
      return;
    }
    elements.computeButton.disabled = state.topWeightsSum <= 0;
  }

  async function init() {
    cacheElements();

    try {
      const data = await invoke('bootstrap');
      initialiseState(data);

      renderScorerConfig();
      renderWeightInputs();
      setupEventHandlers();

      updateTargetScoreUI();
      updateRerollTargetScoreUI();

      renderRerollSlots();
      updateComputeButtonState();
      renderResults();
      renderRerollOutput();
      await computeContributions();
      await setActiveTab('upgrade');
    } catch (error) {
      elements.resultsSection.innerHTML = `
        <div class="error-message">初始化失败：${escapeHtml(error?.message || error)}</div>
      `;
    }
  }

  document.addEventListener('DOMContentLoaded', init);
})();
