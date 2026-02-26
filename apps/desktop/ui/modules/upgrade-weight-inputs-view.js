export function createUpgradeWeightInputsView({
  state,
  elements,
  numberOr,
  isFixedScorer,
  getWeightMap,
  onWeightsUpdated,
}) {
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

  return {
    renderWeightInputs,
  };
}
