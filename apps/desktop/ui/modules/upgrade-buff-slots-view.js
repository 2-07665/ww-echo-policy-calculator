export function createUpgradeBuffSlotsView({
  state,
  elements,
  escapeHtml,
  placeholderLabel,
  getWeightMap,
  formatBuffLabel,
  formatValueLabel,
  formatScoreForScorer,
  computeContributions,
  updateSuggestion,
}) {
  function renderBuffSlots() {
    const weightMap = getWeightMap();
    const sortedBuffTypes = [...state.buffTypes].sort((left, right) => {
      const weightDiff = Number(weightMap[right] ?? 0) - Number(weightMap[left] ?? 0);
      if (Math.abs(weightDiff) > 1e-9) {
        return weightDiff;
      }
      const leftLabel = state.buffLabels[left] ?? left;
      const rightLabel = state.buffLabels[right] ?? right;
      return leftLabel.localeCompare(rightLabel, 'zh-Hans-CN');
    });

    const rows = [];

    for (let i = 0; i < state.maxSelectedTypes; i += 1) {
      const buffName = state.buffSelections[i];
      const value = state.buffValues[i];
      const score = state.contributions[i];
      const scoreText = value == null || !buffName ? '-' : formatScoreForScorer(score);
      const scoreClass = value == null || !buffName ? 'slot-score inactive' : 'slot-score';

      const selectedByOthers = new Set(
        state.buffSelections.filter((chosen, idx) => idx !== i && Boolean(chosen)),
      );
      const options = sortedBuffTypes
        .filter((name) => !selectedByOthers.has(name) || name === buffName)
        .map((name) => {
          const selected = buffName === name ? 'selected' : '';
          return `<option value="${escapeHtml(name)}" ${selected}>${escapeHtml(formatBuffLabel(name, weightMap))}</option>`;
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
            <option value="">${placeholderLabel}</option>
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
      select.addEventListener('change', async (event) => {
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

        await computeContributions();
        if (state.policyReady) {
          await updateSuggestion();
        }
      });
    });

    elements.buffSlotsContainer.querySelectorAll('.buff-value-select').forEach((select) => {
      select.addEventListener('change', async (event) => {
        const index = Number(event.target.dataset.index);
        const value = event.target.value;
        state.buffValues[index] = value ? Number(value) : null;

        await computeContributions();
        if (state.policyReady) {
          await updateSuggestion();
        }
      });
    });
  }

  return {
    renderBuffSlots,
  };
}
