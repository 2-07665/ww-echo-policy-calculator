// Character metadata is supplied by the validated data pack at bootstrap/update.
export let characterReleaseDates = {};
export function setCharacterData(characters) {
  characterReleaseDates = Object.fromEntries(Object.entries(characters).map(([name,info]) => [name,info.release_date]));
}
export function releaseDate(name) { return characterReleaseDates[name] || ''; }
export function upcoming(name, today = new Date().toISOString().slice(0,10)) { return releaseDate(name) > today; }

// The old desktop stores later variants as overrides of the first variant.
function resolvedVariants(group) {
  const base = group.variants?.[0] || group;
  return (group.variants || [group]).map(variant => ({ ...group, ...base, ...variant,
    weights: {...group.weights, ...base.weights, ...variant.weights},
    mainBuffScore: variant.mainBuffScore ?? base.mainBuffScore ?? group.mainBuffScore,
    normalizedMaxScore: variant.normalizedMaxScore ?? base.normalizedMaxScore ?? group.normalizedMaxScore,
    presetIntro: variant.presetIntro ?? base.presetIntro ?? group.presetIntro,
  }));
}

export function importPresetList(data, scoring, defaultWeights, statIds) {
  if (data.version === 3) return data.presets;
  if (data.version !== undefined) throw new Error('无效的预设文件。');
  return data.presets.flatMap(group => {
    if (!group || typeof group.presetName !== 'string' || !group.presetName.trim()) throw new Error('无效的预设文件。');
    return resolvedVariants(group).map(v => ({
      mainBuffScore: v.mainBuffScore, normalizedMaxScore: v.normalizedMaxScore, presetIntro: v.presetIntro,
      name: `${group.presetName}${v.variantName ? ` · ${v.variantName}` : ''}`,
      scoring,
      weights: Object.fromEntries(statIds.map((id,i) => [id, v.weights[id] ?? defaultWeights[i]])),
    }));
  });
}

export function buildPresetList(sources, custom = []) {
  const result = [];
  for (const [scoring, data] of Object.entries(sources)) {
    // Packs provide stable IDs; positional IDs remain a fallback for legacy sources.
    // Sort only the derived view, so existing selections survive.
    const groups = (data.presets || []).map((group, index) => ({group,index}));
    groups.sort((a,b) => releaseDate(b.group.presetName).localeCompare(releaseDate(a.group.presetName)));
    for (const {group,index} of groups) {
      resolvedVariants(group).forEach((variant, vi) => result.push({ ...variant,
        id: variant.id ? `builtin:${scoring}:${variant.id}` : `builtin:${scoring}:${index}:${vi}`,
        groupId: `builtin:${scoring}:group:${group.presetName}`,
        scoring, character:group.presetName, variant:variant.variantName || '默认', builtin:true,
      }));
    }
  }
  // Global search uses the same newest-first order; equal-date entries retain source/variant order.
  result.sort((a,b) => releaseDate(b.character).localeCompare(releaseDate(a.character)));
  result.push(...custom.map(p => ({...p, id:`custom:${p.scoring}:${p.name}`, groupId:`custom:${p.scoring}:${p.name}`, variant:'默认', builtin:false})));
  return result;
}
