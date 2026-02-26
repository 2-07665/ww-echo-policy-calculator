export function createRerollViewSlots({
  state,
  elements,
  escapeHtml,
  placeholderLabel,
  scorerFixed,
  getWeightMap,
  formatBuffLabel,
  buildAcceptSummary,
  getRerollScoreTexts,
  updateRerollComputeButtonState,
  onRerollSelectionChanged,
}) {
  function renderRerollSlots() {
    const fixedWeights = getWeightMap(scorerFixed);
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
            <option value="">${placeholderLabel}</option>
            ${baselineOptions}
          </select>
          <select class="styled-select reroll-candidate-select" data-index="${i}">
            <option value="">${placeholderLabel}</option>
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
        onRerollSelectionChanged();
      });
    });

    elements.rerollSlots.querySelectorAll('.reroll-candidate-select').forEach((select) => {
      select.addEventListener('change', (event) => {
        const index = Number(event.target.dataset.index);
        state.reroll.candidateSelections[index] = event.target.value || null;
        renderRerollSlots();
        onRerollSelectionChanged();
      });
    });

    updateRerollComputeButtonState();
  }

  return {
    renderRerollSlots,
  };
}
