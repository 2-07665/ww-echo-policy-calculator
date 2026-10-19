// Run only against a reviewed checkout. Evaluate scoring definitions, never page/UI code.
// Usage: node extract_wuwa_echo_tool.cjs /path/to/WuwaEchoTool [output.json]
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const crypto = require('node:crypto');
const {execFileSync} = require('node:child_process');
const root = path.resolve(process.argv[2]);
const file = path.join(root, 'js/base.js');
const source = fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '');
function between(start, end) {
  const a = source.indexOf(start), b = source.indexOf(end, a);
  if (a < 0 || b < 0) throw Error(`Missing source boundary: ${start}`);
  return source.slice(a, b);
}
const definitions = between('const toolVersion', '// END GENERATED NEW CHARACTER MODELS')
  + between('function getRoleScoreConfig(role)', '// 按整套累计共鸣效率')
  + between('function getScoreDetails(ct, role)', 'function countScores(ct, role)');
const extraction = `JSON.stringify(roleList.map(base => {
  const modes = newCharacterModels.settings[base.id]?.modes.map(([id]) => id)
    || (base.id === 49 ? ['tune','fusion'] : base.id === 53 ? ['fusion','harmony'] : ['default']);
  return {id:base.id, name:base.name, modes:modes.map(mode => ({mode,
    profiles:Array.from({length:7}, (_,chain) => {
      const role = {roleListId:base.id, ming:chain, damageMode:mode, extraEnergy:0, referenceHealth:40000};
      const properties = ['暴击','暴伤','大攻击','大防御','大生命','小攻击','小防御','小生命','共鸣效率','普攻伤害','重击伤害','技能伤害','解放伤害'];
      return {chain, coefficients:properties.map(property => getScoreDetails({property,value:'1'},role).coefficient)};
    })
  }))};
}))`;
const roles = JSON.parse(vm.runInNewContext(definitions + extraction, {}, {
  timeout: 5000, contextCodeGeneration: {strings:false, wasm:false},
}));
const dictionaries = fs.readFileSync(path.join(root,'js/i18n-dictionaries.js'),'utf8');
const modes = {};
for (const mode of new Set(roles.flatMap(r => r.modes.map(m => m.mode)))) {
  const match = dictionaries.match(new RegExp('"register\\.mode\\.'+mode+'":(\\[[^\\n]+?\\])'));
  if (!match) throw Error(`Missing upstream mode label: ${mode}`);
  const labels = JSON.parse(match[1]);
  modes[mode] = {zh:labels[1],en:labels[2]};
}
const snapshot = {source:{repository:'https://github.com/2-07665/WuwaEchoTool',
  commit:execFileSync('git',['-C',root,'rev-parse','HEAD'],{encoding:'utf8'}).trim(),
  file:'js/base.js',sha256:crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'),
  defaults:{extraEnergy:0,referenceHealth:40000}},modes,roles};
const output = process.argv[3] || path.join(__dirname,'sources/wuwa_echo_tool.json');
fs.writeFileSync(output,JSON.stringify(snapshot,null,2)+'\n');
console.log(`Extracted ${roles.length} source roles at ${snapshot.source.commit}`);
