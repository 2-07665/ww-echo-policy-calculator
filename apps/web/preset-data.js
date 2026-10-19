// First availability dates, not rerun dates. Verified 2026-10-19; see CHARACTER_RELEASES.md.
const releases = [
  ['2026-10-22', ['锁暝']],
  ['2026-09-30', ['心']],
  ['2026-09-10', ['景燃']],
  ['2026-08-20', ['清宵']],
  ['2026-07-30', ['穗穗']],
  ['2026-07-10', ['秧秧·玄翎', '雷主']],
  ['2026-06-13', ['洛瑟菈']],
  ['2026-06-08', ['露西', '丽贝卡']],
  ['2026-05-21', ['达妮娅']],
  ['2026-04-30', ['绯雪']],
  ['2026-03-19', ['西格莉卡']],
  ['2026-02-26', ['陆·赫斯']],
  ['2026-02-05', ['爱弥斯']],
  ['2026-01-15', ['莫宁']],
  ['2025-12-25', ['琳奈']],
  ['2025-12-11', ['卜灵']],
  ['2025-11-20', ['千咲']],
  ['2025-10-30', ['仇远']],
  ['2025-10-09', ['嘉贝莉娜']],
  ['2025-09-17', ['尤诺']],
  ['2025-08-28', ['奥古斯塔']],
  ['2025-07-24', ['弗洛洛']],
  ['2025-07-03', ['露帕']],
  ['2025-06-12', ['卡提希娅']],
  ['2025-05-22', ['夏空']],
  ['2025-04-29', ['赞妮']],
  ['2025-03-27', ['坎特蕾拉', '风主']],
  ['2025-03-06', ['布兰特']],
  ['2025-02-13', ['菲比']],
  ['2025-01-23', ['洛可可']],
  ['2025-01-02', ['珂莱塔']],
  ['2024-12-12', ['灯灯']],
  ['2024-11-14', ['椿']],
  ['2024-10-24', ['釉瑚']],
  ['2024-09-29', ['守岸人']],
  ['2024-09-07', ['相里要']],
  ['2024-08-15', ['折枝']],
  ['2024-07-22', ['长离']],
  ['2024-06-28', ['今汐']],
  ['2024-06-06', ['吟霖']],
  ['2024-05-23', ['忌炎', '鉴心', '卡卡罗', '凌阳', '安可', '维里奈', '秧秧', '炽霞', '白芷', '丹瑾', '散华', '秋水', '莫特斐', '渊武', '桃祈', '光主', '暗主']],
];
const aliases = {'雪豹':'凌阳', '赤霞':'炽霞', '桃祁':'桃祈', '坎特雷拉':'坎特蕾拉'};
export const characterReleaseDates = Object.fromEntries(releases.flatMap(([date,names]) => names.map(name => [name,date])));
export function releaseDate(name) { return characterReleaseDates[aliases[name] || name] || ''; }
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
    // IDs use original source positions. Sort only the derived view, so existing selections survive.
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
