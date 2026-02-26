export function createRerollViewOutput({
  state,
  elements,
  escapeHtml,
  formatFixedOr,
  isFullUniqueSelection,
  onRerollSelectionChanged,
  renderRerollSlots,
}) {
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
        onRerollSelectionChanged();
      });
    });
  }

  return {
    renderRerollOutput,
  };
}
