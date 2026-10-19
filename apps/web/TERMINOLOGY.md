# Localization references

Checked on 2026-10-19. `i18n.js` is shared by the web UI and the desktop UI. Simplified Chinese is the default; the selected language is stored locally. Game terminology is taken from the references below. Application terms such as “joint policy”, “weighted expected cost”, and “linear scoring” describe this calculator, rather than in-game UI labels.

| Chinese | English in this UI | Reference |
| --- | --- | --- |
| 声骸 / 主词条 / 副词条 | Echo / Mainstat / Substat | [Echo Stats](https://wutheringwaves.fandom.com/wiki/Echo/Stats) |
| 暴击 / 暴击伤害 | Crit. Rate / Crit. DMG | Same stat table |
| 攻击、防御、生命（固定 / 百分比） | ATK, DEF, HP / ATK%, DEF%, HP% | Same stat table |
| 共鸣效率 | Energy Regen | Same stat table |
| 普攻 / 重击伤害加成 | Basic Attack DMG Bonus / Heavy Attack DMG Bonus | Same stat table |
| 共鸣技能 / 共鸣解放伤害加成 | Resonance Skill DMG Bonus / Resonance Liberation DMG Bonus | Same stat table |
| 频整器 | Transducer | [灰机 Wiki：频整器](https://wuwa.huijiwiki.com/wiki/%E9%A2%91%E6%95%B4%E5%99%A8), [Version 3.1: Substat Data Modification](https://wutheringwaves.fandom.com/wiki/Version/3.1) |
| 特级调谐器（界面简写“调谐器”） | Premium Tuner | [Premium Tuner, Other Languages](https://wutheringwaves.fandom.com/wiki/Premium_Tuner) |
| 特级密音筒 | Premium Sealed Tube | [Premium Sealed Tube, Other Languages](https://wutheringwaves.fandom.com/wiki/Premium_Sealed_Tube) |
| 调谐 / 强化 | Tuning / Enhancement (upgrade in solver labels) | [Echo Stats](https://wutheringwaves.fandom.com/wiki/Echo/Stats), [Version 2.7](https://wutheringwaves.fandom.com/wiki/Version/2.7) |
| 共鸣链节点 | Sequence Node | [Resonance Chain](https://wutheringwaves.fandom.com/wiki/Resonance_Chain) |

The material for rerolling **Substats** is **Transducer**, not “Frequency Tuner” or “Modifier”. Modifier is associated with Mainstat modification. These names must not be interchanged.

Character names were checked against the [Wuwa Wiki Resonator index](https://wuwa.wiki/en/codex/resonators), its [Illusive Realm roster](https://wuwa.wiki/en/events/list/100600006), and the [Wuthering Waves Wiki Resonator list](https://wutheringwaves.fandom.com/wiki/Resonator/List). Examples: 爱弥斯 → Aemeath, 莫宁 → Mornye, 琳奈 → Lynae, 共鸣者昵称“雪豹” → [Lingyang / 凌阳](https://wuwa.huijiwiki.com/wiki/%E5%87%8C%E9%98%B3). Both source spellings 坎特雷拉 / 坎特蕾拉 map to Cantarella. Rover variants are displayed as Rover: Havoc / Spectro / Aero.

Built-in preset variants use the same stat and element names, `1-Cost` / `3-Cost` / `4-Cost`, and `Sequence 0–6`. The author's `0+1` notation is preserved. User-created preset names are never machine-translated. “MC Boost Assistant” identifies the existing third-party preset source (the legacy scorer identifier is `mc_boost_assistant`); it is not an in-game name.

Some older Wiki prose still describes pre-3.1 mechanics. Only terminology was adopted; no probabilities, resource costs, or solver rules were changed based on those pages.

## Preset coverage

XutheringWavesUID weights were refreshed from the source snapshot below; the other bundled preset files are unchanged. The preset picker shows character groups for the current scoring system, with a separate variant picker. Characters are sorted by first release date, newest first; see [release sources](CHARACTER_RELEASES.md). Global search above it lists matching presets from every source; clicking a result also selects its scoring system:

- Linear: 1 variant.
- Wuwa Echo Tool: 173 variants across 46 character groups.
- MC Boost Assistant: 15 variants.
- XutheringWavesUID: 278 variants across 61 character groups (64 distinct templates; 4 duplicate Rover templates merged).
- Fixed: no bundled named variants (default weights remain available).

Total: **467 built-in variants**. Search accepts either Chinese or English character names. Custom presets are also searchable and stay separate from the built-ins.


### XutheringWavesUID snapshot (2026-10-19)

- Distribution: [IceHe/okww-xwuid-echo-score, commit 843cd078](https://github.com/IceHe/okww-xwuid-echo-score/blob/843cd0783af7ba2230b01f910d97de3e93d4e027/echo-score/xwuid_echo_data.py).
- Original resources: `https://cnb.cool/loping151/XutheringWavesUID-Resources`, commit `c2bd6cd0036a1065ed56ac23243a7779c8f310e0`.
- Decoded source data and attribution: `scripts/convert_weights/sources/xwuid.json`. Regenerate with `python3 scripts/convert_weights/generate_qq_bot_presets.py`.
- All 68 source templates are covered, including Lucilla's four modes; four identical male/female Rover templates are deduplicated. Each generated variant explicitly stores all 13 weights and its raw main-stat score. Damage-type weights multiply `sub_props[技能伤害加成]` by the corresponding `skill_weight`.
- The existing `qq_bot` scorer and saved custom-preset format remain compatible. This update imports weights, not the upstream scoring algorithm: Echo Studio still normalizes the selected main stat plus the best five substats to 50, with the solver's integer quantization. Upstream instead uses its per-cost `score_max` and truncates each contribution to two decimals; displayed scores can differ. Upstream denominators are retained as `sourceScoreMax` for reference.
- New character translations use the [Wuwa Wiki roster](https://wuwa.wiki/en/codex/resonators). Mode terminology: [Glacio Chafe](https://wuwa.wiki/en/codex/tutorials/31004), [Song of Feathered Trace](https://wuwa.wiki/en/codex/items/50033001). Main/Sub DPS and XWUID are descriptions of author-defined templates, not in-game names.
