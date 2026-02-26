export function createEventHandlersUpgrade({
  state,
  elements,
  numberOr,
  targetScoreDigits,
  targetScoreStep,
  scorerPresetCustom,
  isFixedScorer,
  isMcBoostAssistantScorer,
  isQqBotScorer,
  getMainBuffScore,
  getScorerConfig,
  roundToStep,
  applyScorerType,
  applySelectedScorerPreset,
  applySelectedScorerPresetVariant,
  handleSaveCurrentPreset,
  handleDeleteCurrentPreset,
  handleSaveCurrentPresetVariant,
  handleDeleteCurrentPresetVariant,
  updateTargetScoreUI,
  resetPolicyResult,
  onScorerParamsUpdated,
  computeContributions,
  updateSuggestion,
  handleCompute,
}) {
  function setupUpgradeHandlers() {
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

    elements.scorerPresetSelect.addEventListener('change', async () => {
      await applySelectedScorerPreset(elements.scorerPresetSelect.value);
    });

    elements.scorerPresetVariantSelect.addEventListener('change', async () => {
      await applySelectedScorerPresetVariant(elements.scorerPresetVariantSelect.value);
    });

    elements.scorerPresetSaveButton.addEventListener('click', async () => {
      await handleSaveCurrentPreset();
    });

    elements.scorerPresetDeleteButton.addEventListener('click', async () => {
      await handleDeleteCurrentPreset();
    });

    elements.scorerPresetVariantSaveButton.addEventListener('click', async () => {
      await handleSaveCurrentPresetVariant();
    });

    elements.scorerPresetVariantDeleteButton.addEventListener('click', async () => {
      await handleDeleteCurrentPresetVariant();
    });

    elements.scorerPresetNameInput.addEventListener('keydown', async (event) => {
      if (event.key !== 'Enter') {
        return;
      }
      event.preventDefault();
      await handleSaveCurrentPreset();
    });

    elements.scorerPresetVariantNameInput.addEventListener('keydown', async (event) => {
      if (event.key !== 'Enter') {
        return;
      }
      event.preventDefault();
      if (state.activePresetNames[state.scorerType] === scorerPresetCustom) {
        await handleSaveCurrentPreset();
      } else {
        await handleSaveCurrentPresetVariant();
      }
    });

    elements.mainBuffScoreInput.addEventListener('change', async () => {
      if (isFixedScorer() || isMcBoostAssistantScorer()) {
        return;
      }
      const config = getScorerConfig();
      config.mainBuffScore = Math.max(
        0,
        numberOr(elements.mainBuffScoreInput.valueAsNumber, config.mainBuffScore),
      );
      elements.mainBuffScoreInput.value = config.mainBuffScore.toFixed(targetScoreDigits);
      await onScorerParamsUpdated();
    });

    elements.normalizedMaxScoreInput.addEventListener('change', async () => {
      if (isFixedScorer() || isQqBotScorer() || isMcBoostAssistantScorer()) {
        return;
      }
      const config = getScorerConfig();
      config.normalizedMaxScore = Math.max(
        targetScoreStep,
        numberOr(elements.normalizedMaxScoreInput.valueAsNumber, config.normalizedMaxScore),
      );
      elements.normalizedMaxScoreInput.value = config.normalizedMaxScore.toFixed(targetScoreDigits);
      await onScorerParamsUpdated();
    });

    elements.targetScoreInput.addEventListener('change', () => {
      if (isFixedScorer()) {
        state.targetScore = Math.max(
          0,
          Math.round(numberOr(elements.targetScoreInput.valueAsNumber, state.targetScore)),
        );
      } else {
        state.targetScore = Math.max(
          0,
          roundToStep(numberOr(elements.targetScoreInput.valueAsNumber, state.targetScore), targetScoreStep),
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
      state.costWeights.wEcho = Math.max(
        0,
        numberOr(elements.costWEchoInput.valueAsNumber, state.costWeights.wEcho),
      );
      elements.costWEchoInput.value = state.costWeights.wEcho.toFixed(1);
      resetPolicyResult();
    });

    elements.costWTunerInput.addEventListener('change', () => {
      state.costWeights.wTuner = Math.max(
        0,
        numberOr(elements.costWTunerInput.valueAsNumber, state.costWeights.wTuner),
      );
      elements.costWTunerInput.value = state.costWeights.wTuner.toFixed(1);
      resetPolicyResult();
    });

    elements.costWExpInput.addEventListener('change', () => {
      state.costWeights.wExp = Math.max(
        0,
        numberOr(elements.costWExpInput.valueAsNumber, state.costWeights.wExp),
      );
      elements.costWExpInput.value = state.costWeights.wExp.toFixed(1);
      resetPolicyResult();
    });

    elements.expRefundInput.addEventListener('change', () => {
      state.expRefundRatio = Math.max(
        0,
        Math.min(0.75, numberOr(elements.expRefundInput.valueAsNumber, state.expRefundRatio)),
      );
      state.expRefundRatio = roundToStep(state.expRefundRatio, targetScoreStep);
      elements.expRefundInput.value = state.expRefundRatio.toFixed(targetScoreDigits);
      resetPolicyResult();
    });

    elements.computeButton.addEventListener('click', () => {
      handleCompute();
    });
  }

  return {
    setupUpgradeHandlers,
  };
}
