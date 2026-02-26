export function createRerollViewMeta({
  state,
  elements,
  isFullUniqueSelection,
}) {
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

  return {
    buildAcceptSummary,
    getRerollScoreTexts,
    updateRerollSlotsMeta,
  };
}
