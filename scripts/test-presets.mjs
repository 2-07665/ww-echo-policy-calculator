import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {buildPresetList,importPresetList,releaseDate,upcoming,setCharacterData} from '../apps/web/preset-data.js';
const pack = JSON.parse(await readFile(new URL('../data/echo-data.json',import.meta.url),'utf8'));
const sources = pack.presets;
setCharacterData(pack.characters);
const presets = buildPresetList(sources);
assert.equal(presets.length,578);
assert.equal(new Set(presets.map(p => p.id)).size,578);
for (const scoring of Object.keys(sources)) {
  const items = presets.filter(p => p.scoring === scoring);
  assert.equal(new Set(items.map(p => p.groupId)).size,sources[scoring].presets.length);
  const dates = items.map(p => releaseDate(p.character));
  assert.deepEqual(dates,[...dates].sort().reverse(),`${scoring} is not newest first`);
  for (const p of items) {
    if (!['角色','默认','自定义'].includes(p.character)) assert.ok(releaseDate(p.character),`Missing release date: ${p.character}`);
    assert.equal(Object.keys(p.weights).length,13,`Incomplete weights: ${p.id}`);
  }
}
const groups = [...new Map(presets.filter(p => p.scoring === 'qq_bot').map(p => [p.groupId,p])).values()];
assert.equal(groups.length,61);
assert.deepEqual(groups.slice(0,5).map(p => p.character),['锁暝','心','景燃','清宵','穗穗']);
assert.equal(groups.at(-1).character,'角色');
assert.equal(upcoming('锁暝','2026-10-19'),true);
assert.equal(upcoming('锁暝','2026-10-22'),false);
assert.equal(releaseDate('雪豹'),releaseDate('凌阳'));
assert.equal(releaseDate('雷主'),'2026-07-10');
assert.equal(releaseDate('风主'),'2025-03-27');
const second = presets.find(p => p.id === 'builtin:wuwa_echo_tool:0:1');
assert.equal(second.character,'今汐');
assert.equal(second.weights.Crit_Rate,189);
assert.equal(second.weights.Skill_Damage,73.31);
const lucilla = presets.filter(p => p.groupId === 'builtin:qq_bot:group:洛瑟菈');
assert.equal(lucilla.length,18);
assert.equal(new Set(lucilla.map(p => p.sourceTemplate.modal)).size,4);
const custom = {name:'Personal',scoring:'qq_bot',weights:second.weights};
const last = buildPresetList(sources,[custom]).at(-1);
assert.equal(last.id,'custom:qq_bot:Personal');
assert.equal(last.groupId,last.id);
assert.equal(last.builtin,false);
const statIds = Object.keys(second.weights);
const defaults = statIds.map(() => 1);
const imported = importPresetList({presets:[{presetName:'Legacy',variants:[
  {variantName:'Base',weights:{Crit_Rate:100,Skill_Damage:50},mainBuffScore:20,normalizedMaxScore:100},
  {variantName:'Override',weights:{Skill_Damage:70},mainBuffScore:null,normalizedMaxScore:null},
]}]},'linear',defaults,statIds);
assert.equal(imported[1].name,'Legacy · Override');
assert.equal(imported[1].weights.Crit_Rate,100);
assert.equal(imported[1].weights.Skill_Damage,70);
assert.equal(imported[1].weights.HP,1);
assert.equal(imported[1].mainBuffScore,20);
assert.equal(imported[1].normalizedMaxScore,100);
assert.equal(imported[1].variants,undefined,'Do not copy an entire legacy group into each custom preset');
assert.equal(importPresetList({presets:[{presetName:'Sparse',weights:{Crit_Rate:2}}]},'linear',defaults,statIds)[0].weights.HP,1);
assert.deepEqual(importPresetList({version:3,presets:[custom]},'linear',defaults,statIds),[custom]);
assert.throws(()=>importPresetList({version:4,presets:[]},'linear',defaults,statIds));
console.log('Preset model passed: grouped variants, newest-first chronology, stable IDs, inherited weights, custom presets.');
