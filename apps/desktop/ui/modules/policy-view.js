export function createPolicyView({
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
}) {
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
      Math.round(numberOr(state.displayMaxScore, computeTopWeightsSumForType(scorerFixed))),
    );
    const normalizedMaxScore = Math.max(
      targetScoreStep,
      numberOr(state.displayMaxScore, getNormalizedMaxScore(scorerType)),
    );
    const totalScoreText = isFixedScorer(scorerType)
      ? `${formatScoreForScorer(state.totalScore, scorerType)} / ${fixedMaxScore}`
      : `${state.totalScore.toFixed(targetScoreDigits)} / ${normalizedMaxScore.toFixed(targetScoreDigits)}`;

    const targetScore = Number(state.policySummary?.targetScore ?? state.targetScore);
    const targetText = formatScoreForScorer(targetScore, scorerType);

    const successProbability = Number(state.suggestion?.successProbability);
    const successBadgeText = Number.isFinite(successProbability)
      ? `当前成功率 ${(successProbability * 100).toFixed(4)}%`
      : '当前成功率 --';
    const successBadgeClass = Number.isFinite(successProbability)
      ? 'score-success'
      : 'score-success score-success-pending';

    const summaryHtml = `
      <div class="score-summary">
        <div class="score-header">
          <div class="score-meter">
            <div class="score-label">当前总分</div>
            <div class="score-value">${totalScoreText}</div>
          </div>
          <div class="score-meta">
            <span class="score-target">目标分数 ${targetText}</span>
            <span class="${successBadgeClass}">${successBadgeText}</span>
          </div>
        </div>
        <div class="score-suggestion">
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

  return {
    renderResults,
    renderTotalScoreCard,
  };
}
