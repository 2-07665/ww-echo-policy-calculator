export function createEventHandlersReroll({
  state,
  elements,
  numberOr,
  isFullUniqueSelection,
  updateRerollTargetScoreUI,
  invalidateRerollPolicy,
  handleRerollCompute,
  renderRerollSlots,
  onRerollSelectionChanged,
}) {
  function setupRerollHandlers() {
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
      onRerollSelectionChanged();
    });

    elements.rerollReplaceButton.addEventListener('click', () => {
      if (!isFullUniqueSelection(state.reroll.candidateSelections)) {
        return;
      }
      state.reroll.baselineSelections = [...state.reroll.candidateSelections];
      state.reroll.candidateSelections = Array(state.maxSelectedTypes).fill(null);
      renderRerollSlots();
      onRerollSelectionChanged();
    });
  }

  return {
    setupRerollHandlers,
  };
}
