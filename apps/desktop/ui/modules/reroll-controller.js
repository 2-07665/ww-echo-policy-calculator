import { createRerollPolicyController } from './reroll-policy-controller.js';
import { createRerollViewController } from './reroll-view-controller.js';

export function createRerollController({
  state,
  elements,
  invoke,
  escapeHtml,
  formatFixedOr,
  placeholderLabel,
  scorerFixed,
  computeTopWeightsSumForType,
  getWeightMap,
  formatBuffLabel,
  buildFixedPayloadWeights,
}) {
  function isFullUniqueSelection(selections) {
    const filled = selections.filter(Boolean);
    return filled.length === state.maxSelectedTypes && new Set(filled).size === state.maxSelectedTypes;
  }

  function clearRerollRecommendation() {
    state.reroll.output = null;
    state.reroll.error = null;
  }

  const rerollViewController = createRerollViewController({
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
  });

  function updateRerollSlotsMeta() {
    rerollViewController.updateRerollSlotsMeta();
  }

  function updateRerollComputeButtonState() {
    rerollViewController.updateRerollComputeButtonState();
  }

  function renderRerollOutput() {
    rerollViewController.renderRerollOutput();
  }

  function renderRerollSlots() {
    rerollViewController.renderRerollSlots();
  }

  const rerollPolicyController = createRerollPolicyController({
    state,
    elements,
    invoke,
    scorerFixed,
    computeTopWeightsSumForType,
    buildFixedPayloadWeights,
    isFullUniqueSelection,
    clearRerollRecommendation,
    updateRerollSlotsMeta,
    updateRerollComputeButtonState,
    renderRerollSlots,
    renderRerollOutput,
  });

  function onRerollSelectionChanged() {
    rerollPolicyController.onRerollSelectionChanged();
  }

  function invalidateRerollPolicy() {
    rerollPolicyController.invalidateRerollPolicy();
  }

  async function handleRerollCompute() {
    await rerollPolicyController.handleRerollCompute();
  }

  return {
    isFullUniqueSelection,
    onRerollSelectionChanged,
    invalidateRerollPolicy,
    updateRerollComputeButtonState,
    renderRerollSlots,
    renderRerollOutput,
    handleRerollCompute,
  };
}
