import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { messages, setLanguage, t, statNames, characterName, variantName, errorText } from '../apps/web/i18n.js';
const html = await readFile(new URL('../apps/web/index.html',import.meta.url),'utf8');
for (const [,key] of html.matchAll(/data-i18n(?:-placeholder|-aria)?="([^"]+)"/g)) assert.ok(Object.hasOwn(messages,key),`Untranslated HTML: ${key}`);
const app = await readFile(new URL('../apps/web/app.js',import.meta.url),'utf8');
for (const [,key] of app.matchAll(/\b(?:t|setComputeHint)\('([^']+)'/g)) assert.ok(Object.hasOwn(messages,key) || !/\p{Script=Han}/u.test(key),`Untranslated JS: ${key}`);
assert.equal(statNames.length,13);
setLanguage('en');
assert.equal(t('频整器'),'Transducers');
assert.equal(t('特级密音筒'),'Premium Sealed Tubes');
assert.equal(t('调谐器'),'Premium Tuners');
let total = 0;
for (const file of ['linear_default','fixed','wuwa_echo_tool','mc_boost_assistant','qq_bot']) {
  const data = JSON.parse(await readFile(new URL(`../crates/echo_app/presets/${file}.json`,import.meta.url),'utf8'));
  for (const group of data.presets) {
    assert.ok(!/\p{Script=Han}/u.test(characterName(group.presetName)),`Missing character: ${group.presetName}`);
    for (const variant of group.variants || [group]) {
      total++;
      assert.ok(!/\p{Script=Han}/u.test(variantName(variant.variantName || '默认')),`Missing variant: ${variant.variantName}`);
      if (variant.presetIntro) assert.ok(Object.hasOwn(messages,variant.presetIntro));
    }
  }
}
assert.equal(total,467);
assert.match(errorText({message:'当前评分规则无法达到这个目标，请降低目标分数。（TargetScoreImpossible）'}),/cannot be reached/);
console.log('Localization coverage passed: all static labels, literal message keys, 13 stats, 467 preset variants and descriptions.');
