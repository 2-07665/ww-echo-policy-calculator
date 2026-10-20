// Desktop OCR lifecycle tests without a live game or Tauri window.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { messages, setLanguage } from '../apps/web/i18n.js';

class Element {
  constructor(value = '') { this.value = value; this.hidden = false; this.disabled = false; this.listeners = {}; this.options = []; }
  addEventListener(name, callback) { this.listeners[name] = callback; }
  replaceChildren(...options) { this.options = options; this.value = options[0]?.value || ''; }
  add(option) { this.options.push(option); }
  async trigger(name) { return this.listeners[name](); }
}
const ids = ['ocr-controls','ocr-toggle','ocr-mode','ocr-window','ocr-refresh','ocr-window-row','ocr-status'];
const elements = Object.fromEntries(ids.map(id => [id, new Element()]));
elements['ocr-mode'].value = 'native';
globalThis.document = { getElementById: id => elements[id] };
globalThis.Option = class { constructor(text, value) { this.text = text; this.value = value; } };
const handlers = {}, calls = [], received = [], notices = [];
let fail = false;
globalThis.window = { __TAURI__: { core: { invoke: async (command, args) => {
  calls.push({ command, args });
  if (fail) throw new Error('test-startup-error');
  if (command === 'ocr_windows') return [{ id:'123', title:'鸣潮' }, { id:'456', title:'鸣潮' }];
  return args.enabled;
}}, event: { listen: async (name, callback) => { handlers[name] = callback; } } } };
const { setupOcr, renderOcr } = await import('../apps/web/ocr-ui.js');
setLanguage('zh-CN');
setupOcr({ desktop:true, receive:value => received.push(value), notice:value => notices.push(value) });
await elements['ocr-refresh'].trigger('click');
assert.equal(elements['ocr-window'].options.length,3);
elements['ocr-window'].value = '456';
await elements['ocr-toggle'].trigger('click');
assert.deepEqual(calls.at(-1), { command:'toggle_ocr', args:{ enabled:true, mode:'native', windowId:'456' } });
assert.equal(elements['ocr-mode'].disabled,true);
handlers['ocr-echo']({ payload:[[0,63]] });
assert.deepEqual(received,[[[0,63]]]);
handlers['ocr-status']({ payload:{status:'paused',enabled:true} });
assert.match(elements['ocr-status'].textContent,/最小化/);
setLanguage('en'); renderOcr();
assert.match(elements['ocr-status'].textContent,/minimized/);
handlers['ocr-status']({ payload:{status:'exited',enabled:false} });
assert.equal(elements['ocr-mode'].disabled,false);
handlers['ocr-echo']({ payload:[[0,105]] });
assert.equal(received.length,1,'Late result after process exit must be ignored');
elements['ocr-mode'].value = 'external';
elements['ocr-window'].value = '';
await elements['ocr-mode'].trigger('change');
assert.equal(elements['ocr-window-row'].hidden,true);
await elements['ocr-toggle'].trigger('click');
assert.equal(calls.at(-1).args.mode,'external');
await elements['ocr-toggle'].trigger('click');
assert.equal(calls.at(-1).args.enabled,false);
fail = true;
await elements['ocr-toggle'].trigger('click');
assert.equal(elements['ocr-mode'].disabled,false);
assert.equal(notices.length,1);
const source = await readFile(new URL('../apps/web/ocr-ui.js', import.meta.url),'utf8');
for (const [,key] of source.matchAll(/'([^'\n]*[\p{Script=Han}][^'\n]*)'/gu)) {
  assert.ok(Object.hasOwn(messages,key), `Untranslated OCR label: ${key}`);
}
console.log('OCR UI passed: modes, window selection, state changes, stale results, errors and localization.');
