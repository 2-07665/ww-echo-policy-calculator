import { createRerollViewMeta } from './reroll-view-meta.js';
import { createRerollViewOutput } from './reroll-view-output.js';
import { createRerollViewSlots } from './reroll-view-slots.js';

export function createRerollViewController({
  state,
  elements,
  escapeHtml,
  formatFixedOr,
  placeholderLabel,
  scorerFixed,
  computeTopWeightsSumForType,
  getWeightMap,
  formatBuffLabel,
  isFullUniqueSelection,
  onRerollSelectionChanged,
}) {
  const rerollViewMeta = createRerollViewMeta({
    state,
    elements,
    isFullUniqueSelection,
  });

  function updateRerollComputeButtonState() {
    if (elements.rerollComputeButton.dataset.loading === 'true') {
      return;
    }
    elements.rerollComputeButton.disabled = computeTopWeightsSumForType(scorerFixed) <= 0;
  }

  const rerollViewSlots = createRerollViewSlots({
    state,
    elements,
    escapeHtml,
    placeholderLabel,
    scorerFixed,
    getWeightMap,
    formatBuffLabel,
    buildAcceptSummary: rerollViewMeta.buildAcceptSummary,
    getRerollScoreTexts: rerollViewMeta.getRerollScoreTexts,
    updateRerollComputeButtonState,
    onRerollSelectionChanged,
  });

  const rerollViewOutput = createRerollViewOutput({
    state,
    elements,
    escapeHtml,
    formatFixedOr,
    isFullUniqueSelection,
    onRerollSelectionChanged,
    renderRerollSlots: rerollViewSlots.renderRerollSlots,
  });

  return {
    updateRerollSlotsMeta: rerollViewMeta.updateRerollSlotsMeta,
    updateRerollComputeButtonState,
    renderRerollOutput: rerollViewOutput.renderRerollOutput,
    renderRerollSlots: rerollViewSlots.renderRerollSlots,
  };
}
