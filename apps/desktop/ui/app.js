(() => {
  const PLACEHOLDER_LABEL = '选择词条';

  const state = {
    buffTypes: [],
    buffLabels: {},
    buffTypeMaxValues: [],
    maxSelectedTypes: 5,
    defaultBuffWeights: {},
    buffValueOptions: new Map(),
    buffSelections: [],
    buffValues: [],
    contributions: [],
    percentBuffs: new Set(),
    topWeightsSum: 0,
    totalScore: 0,
    policySummary: null,
    policyError: null,
    policyReady: false,
    suggestion: null,
    weightMap: {},
    defaultTargetScore: 60,
    targetScore: 60,
    expRefundRatio: 0.66,
    scorerType: 'linear',
    costWeights: { wEcho: 0.0, wTuner: 1.0, wExp: 0.0 },
    activeTab: 'upgrade',
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
    elements.targetScoreInput = document.getElementById('target-score-input');
    elements.costWEchoInput = document.getElementById('cost-w-echo');
    elements.costWTunerInput = document.getElementById('cost-w-tuner');
    elements.costWExpInput = document.getElementById('cost-w-exp');
    elements.expRefundSlider = document.getElementById('exp-refund-slider');
    elements.expRefundValue = document.getElementById('exp-refund-value');
    elements.computeButton = document.getElementById('compute-button');
    elements.resultsSection = document.getElementById('results-section');
    elements.rerollTargetScoreInput = document.getElementById('reroll-target-score-input');
    elements.rerollComputeButton = document.getElementById('reroll-compute-button');
    elements.rerollClearBaselineButton = document.getElementById('reroll-clear-baseline-button');
    elements.rerollReplaceButton = document.getElementById('reroll-replace-button');
    elements.rerollSlots = document.getElementById('reroll-slots');
    elements.rerollOutput = document.getElementById('reroll-output');
  }

  function initialiseState(data) {
    state.buffTypes = data.buffTypes || [];
    state.buffLabels = data.buffLabels || {};
    state.buffTypeMaxValues = (data.buffTypeMaxValues || []).map(Number);
    state.maxSelectedTypes = Number(data.maxSelectedTypes || 5);
    state.defaultBuffWeights = data.defaultBuffWeights || {};
    state.defaultTargetScore = Number(data.defaultTargetScore || 60);
    state.targetScore = state.defaultTargetScore;
    state.expRefundRatio = Number(data.defaultExpRefundRatio || 0.66);
    state.scorerType = String(data.defaultScorerType || 'linear').toLowerCase();

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

    state.weightMap = {};
    state.buffTypes.forEach((name) => {
      state.weightMap[name] = Number(state.defaultBuffWeights[name] ?? 0);
    });

    state.buffSelections = Array(state.maxSelectedTypes).fill(null);
    state.buffValues = Array(state.maxSelectedTypes).fill(null);
    state.contributions = Array(state.maxSelectedTypes).fill(0);
    state.totalScore = 0;
    state.policySummary = null;
    state.policyError = null;
    state.policyReady = false;
    state.suggestion = null;
    state.reroll = {
      targetScore: state.defaultTargetScore,
      policyReady: false,
      baselineSelections: Array(state.maxSelectedTypes).fill(null),
      candidateSelections: Array(state.maxSelectedTypes).fill(null),
      output: null,
      error: null,
    };

    elements.targetScoreInput.value = state.targetScore.toFixed(1);
    elements.scorerTypeSelect.value = state.scorerType === 'fixed' ? 'fixed' : 'linear';
    elements.costWEchoInput.value = state.costWeights.wEcho.toFixed(1);
    elements.costWTunerInput.value = state.costWeights.wTuner.toFixed(1);
    elements.costWExpInput.value = state.costWeights.wExp.toFixed(1);
    elements.expRefundSlider.value = state.expRefundRatio.toFixed(2);
    elements.expRefundValue.textContent = state.expRefundRatio.toFixed(2);
    elements.rerollTargetScoreInput.value = state.reroll.targetScore.toFixed(1);
  }

  function roundToStep(value, step = 0.1) {
    if (!Number.isFinite(value)) {
      return 0;
    }
    return Math.round(value / step) * step;
  }

  function recommendedTargetForCurrentScorer() {
    if (state.scorerType === 'fixed') {
      const maxScore = computeTopWeightsSum();
      if (maxScore <= 0) {
        return 0;
      }
      return roundToStep(maxScore * 0.7, 0.1);
    }
    return state.defaultTargetScore;
  }

  function updateTargetScoreUI({ setRecommended = false } = {}) {
    if (state.scorerType === 'fixed') {
      const maxScore = computeTopWeightsSum();
      const recommended = recommendedTargetForCurrentScorer();
      elements.targetScoreInput.removeAttribute('max');

      if (setRecommended) {
        state.targetScore = recommended;
      } else if (maxScore > 0 && state.targetScore > maxScore) {
        state.targetScore = maxScore;
      }
    } else {
      elements.targetScoreInput.max = '100';
      if (setRecommended) {
        state.targetScore = state.defaultTargetScore;
      }
    }

    state.targetScore = Math.max(0, roundToStep(state.targetScore, 0.1));
    elements.targetScoreInput.value = state.targetScore.toFixed(1);
  }

  function setActiveTab(tab) {
    state.activeTab = tab === 'reroll' ? 'reroll' : 'upgrade';
    const upgradeActive = state.activeTab === 'upgrade';

    elements.upgradeTab.hidden = !upgradeActive;
    elements.rerollTab.hidden = upgradeActive;
    elements.tabUpgrade.classList.toggle('active', upgradeActive);
    elements.tabReroll.classList.toggle('active', !upgradeActive);
  }

  function computeTopWeightsSum() {
    const weights = state.buffTypes
      .map((name) => Number(state.weightMap[name] ?? 0))
      .sort((a, b) => b - a)
      .slice(0, state.maxSelectedTypes);
    return weights.reduce((sum, weight) => sum + weight, 0);
  }

  function computeContributions() {
    const sum = computeTopWeightsSum();
    state.topWeightsSum = sum;

    for (let i = 0; i < state.maxSelectedTypes; i += 1) {
      const buffName = state.buffSelections[i];
      const rawValue = state.buffValues[i];

      if (!buffName || rawValue == null) {
        state.contributions[i] = 0;
        continue;
      }

      const buffIndex = state.buffTypes.indexOf(buffName);
      if (buffIndex < 0) {
        state.contributions[i] = 0;
        continue;
      }

      const weight = Number(state.weightMap[buffName] ?? 0);
      if (state.scorerType === 'fixed') {
        state.contributions[i] = weight;
      } else {
        if (sum <= 0) {
          state.contributions[i] = 0;
          continue;
        }
        const maxValue = Number(state.buffTypeMaxValues[buffIndex] ?? 1);
        const ratio = Math.max(0, Math.min(1, Number(rawValue) / maxValue));
        state.contributions[i] = (100.0 * weight * ratio) / sum;
      }
    }

    state.totalScore = state.contributions.reduce((acc, value) => acc + value, 0);
  }

  function formatBuffLabel(buffName) {
    const label = state.buffLabels[buffName] ?? buffName;
    const weight = Number(state.weightMap[buffName] ?? 0);
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
      input.type = 'number';
      input.step = '0.1';
      input.min = '0';
      input.value = Number(state.weightMap[buffName] ?? 0);

      input.addEventListener('change', () => {
        state.weightMap[buffName] = Math.max(0, numberOr(input.valueAsNumber, 0));
        input.value = state.weightMap[buffName];
        onWeightsUpdated();
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
    const rows = [];
    const sortedBuffTypes = [...state.buffTypes].sort((left, right) => {
      const weightDiff = Number(state.weightMap[right] ?? 0) - Number(state.weightMap[left] ?? 0);
      if (Math.abs(weightDiff) > 1e-9) {
        return weightDiff;
      }
      const leftLabel = state.buffLabels[left] ?? left;
      const rightLabel = state.buffLabels[right] ?? right;
      return leftLabel.localeCompare(rightLabel, 'zh-Hans-CN');
    });

    for (let i = 0; i < state.maxSelectedTypes; i += 1) {
      const buffName = state.buffSelections[i];
      const value = state.buffValues[i];
      const score = state.contributions[i];
      const scoreText = value == null || !buffName ? '-' : score.toFixed(2);
      const scoreClass = value == null || !buffName ? 'slot-score inactive' : 'slot-score';

      const selectedByOthers = new Set(
        state.buffSelections.filter((chosen, idx) => idx !== i && Boolean(chosen)),
      );
      const options = sortedBuffTypes
        .filter((name) => !selectedByOthers.has(name) || name === buffName)
        .map((name) => {
          const selected = buffName === name ? 'selected' : '';
          return `<option value="${escapeHtml(name)}" ${selected}>${escapeHtml(formatBuffLabel(name))}</option>`;
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
      select.addEventListener('change', (event) => {
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

        computeContributions();
        renderBuffSlots();
        renderTotalScoreCard();
        if (state.policyReady) {
          updateSuggestion();
        }
      });
    });

    elements.buffSlotsContainer.querySelectorAll('.buff-value-select').forEach((select) => {
      select.addEventListener('change', (event) => {
        const index = Number(event.target.dataset.index);
        const value = event.target.value;
        state.buffValues[index] = value ? Number(value) : null;

        computeContributions();
        renderBuffSlots();
        renderTotalScoreCard();
        if (state.policyReady) {
          updateSuggestion();
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
        ? Number(output.baselineScore).toFixed(2)
        : '--';
    const candidateScoreText =
      output &&
      output.valid !== false &&
      output.candidateScore != null &&
      Number.isFinite(Number(output.candidateScore))
        ? Number(output.candidateScore).toFixed(2)
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
    elements.rerollComputeButton.disabled = state.topWeightsSum <= 0;
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
    const sortedBuffTypes = [...state.buffTypes].sort((left, right) => {
      const weightDiff = Number(state.weightMap[right] ?? 0) - Number(state.weightMap[left] ?? 0);
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
          return `<option value="${escapeHtml(name)}" ${selected}>${escapeHtml(formatBuffLabel(name))}</option>`;
        })
        .join('');

      const candidateOptions = sortedBuffTypes
        .filter((name) => !candidateSelectedByOthers.has(name) || name === candidateSelected)
        .map((name) => {
          const selected = candidateSelected === name ? 'selected' : '';
          return `<option value="${escapeHtml(name)}" ${selected}>${escapeHtml(formatBuffLabel(name))}</option>`;
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

  function selectedBuffState() {
    const names = [];
    const values = [];

    for (let i = 0; i < state.maxSelectedTypes; i += 1) {
      const buffName = state.buffSelections[i];
      const buffValue = state.buffValues[i];
      if (!buffName || buffValue == null) {
        continue;
      }
      names.push(buffName);
      values.push(Number(buffValue));
    }

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
    const totalScoreText =
      state.scorerType === 'fixed'
        ? `${state.totalScore.toFixed(2)}`
        : `${state.totalScore.toFixed(2)} / 100.00`;
    const selectedCount = state.buffSelections.filter(Boolean).length;
    const targetScore = Number(state.policySummary?.targetScore ?? state.targetScore);
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
            <span class="score-target">目标分数 ${targetScore.toFixed(2)}</span>
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

  function onWeightsUpdated() {
    computeContributions();
    renderBuffSlots();
    updateTargetScoreUI();
    resetPolicyResult();
    updateComputeButtonState();
    invalidateRerollPolicy();
  }

  function setupEventHandlers() {
    elements.tabUpgrade.addEventListener('click', () => {
      setActiveTab('upgrade');
    });

    elements.tabReroll.addEventListener('click', () => {
      setActiveTab('reroll');
    });

    elements.clearBuffsButton.addEventListener('click', () => {
      state.buffSelections.fill(null);
      state.buffValues.fill(null);
      state.contributions.fill(0);
      state.totalScore = 0;
      renderBuffSlots();
      renderTotalScoreCard();
      if (state.policyReady) {
        updateSuggestion();
      }
    });

    elements.targetScoreInput.addEventListener('change', () => {
      state.targetScore = Math.max(0, numberOr(elements.targetScoreInput.valueAsNumber, state.targetScore));
      updateTargetScoreUI();
      resetPolicyResult();
    });

    elements.scorerTypeSelect.addEventListener('change', () => {
      state.scorerType = elements.scorerTypeSelect.value === 'fixed' ? 'fixed' : 'linear';
      computeContributions();
      updateTargetScoreUI({ setRecommended: true });
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

    elements.expRefundSlider.addEventListener('input', () => {
      state.expRefundRatio = Math.max(0, Math.min(0.75, numberOr(elements.expRefundSlider.valueAsNumber, state.expRefundRatio)));
      elements.expRefundValue.textContent = state.expRefundRatio.toFixed(2);
    });

    elements.expRefundSlider.addEventListener('change', () => {
      resetPolicyResult();
    });

    elements.computeButton.addEventListener('click', () => {
      handleCompute();
    });

    elements.rerollTargetScoreInput.addEventListener('change', () => {
      state.reroll.targetScore = Math.max(
        0,
        numberOr(elements.rerollTargetScoreInput.valueAsNumber, state.reroll.targetScore),
      );
      elements.rerollTargetScoreInput.value = state.reroll.targetScore.toFixed(1);
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

  async function handleCompute() {
    const payload = {
      buffWeights: state.weightMap,
      targetScore: state.targetScore,
      scorerType: state.scorerType,
      costWeights: {
        wEcho: state.costWeights.wEcho,
        wTuner: state.costWeights.wTuner,
        wExp: state.costWeights.wExp,
      },
      expRefundRatio: state.expRefundRatio,
      blendData: false,
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
    if (state.topWeightsSum <= 0) {
      clearRerollRecommendation();
      state.reroll.error = '请先设置至少一个大于 0 的权重。';
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
          buffWeights: state.weightMap,
          targetScore: state.reroll.targetScore,
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
      renderWeightInputs();
      setupEventHandlers();
      computeContributions();
      updateTargetScoreUI();
      renderBuffSlots();
      renderRerollSlots();
      renderTotalScoreCard();
      updateComputeButtonState();
      renderResults();
      renderRerollOutput();
      setActiveTab('upgrade');
    } catch (error) {
      elements.resultsSection.innerHTML = `
        <div class="error-message">初始化失败：${escapeHtml(error?.message || error)}</div>
      `;
    }
  }

  document.addEventListener('DOMContentLoaded', init);
})();
