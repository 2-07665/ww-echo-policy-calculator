// Desktop OCR lifecycle tests without a live game or Tauri window.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { messages, setLanguage } from '../apps/web/i18n.js';

class Element {
  constructor(value = '') { this.value = value; this.hidden = false; this.disabled = false; this.listeners = {}; this.options = []; this.nativeOption = {}; }
  parentElement = { hidden: false };
  querySelector() { return this.nativeOption; }
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
let duringToggle = null;
let capabilities = { native_supported:true, native_available:true };
globalThis.window = { __TAURI__: { core: { invoke: async (command, args) => {
  calls.push({ command, args });
  if (fail) throw new Error('test-startup-error');
  if (command === 'ocr_capabilities') return capabilities;
  if (command === 'ocr_windows') return [{ id:'123', title:'鸣潮' }, { id:'456', title:'鸣潮' }];
  if (duringToggle) duringToggle();
  return args.enabled;
}}, event: { listen: async (name, callback) => { handlers[name] = callback; } } } };
const { setupOcr, renderOcr } = await import('../apps/web/ocr-ui.js');
setLanguage('zh-CN');
const setupOptions = { desktop:true, receive:value => received.push(value), notice:value => notices.push(value) };
await setupOcr(setupOptions);
assert.equal(elements['ocr-mode'].parentElement.hidden,false);
await elements['ocr-refresh'].trigger('click');
assert.equal(elements['ocr-window'].options.length,3);
elements['ocr-window'].value = '456';
await elements['ocr-toggle'].trigger('click');
assert.deepEqual(calls.at(-1), { command:'toggle_ocr', args:{ enabled:true, mode:'native', windowId:'456' } });
assert.equal(elements['ocr-mode'].disabled,true);
handlers['ocr-echo']({ payload:[[0,63]] });
assert.deepEqual(received,[[[0,63]]]);
for (const group of [
  [[4,86],[0,63],[9,86],[11,101],[5,40]],
  [[10,79],[5,40],[11,86],[3,100],[9,94]],
]) {
  for (let count=1;count<=5;count++) {
    const payload=group.slice(0,count);
    handlers['ocr-echo']({payload});
    assert.deepEqual(received.at(-1),payload,'Each confirmed upgrade replaces the complete slot list');
  }
}
const receivedBeforeExit=received.length;
handlers['ocr-status']({ payload:{status:'paused',enabled:true} });
assert.match(elements['ocr-status'].textContent,/最小化/);
setLanguage('en'); renderOcr();
assert.match(elements['ocr-status'].textContent,/minimized/);
handlers['ocr-status']({ payload:{status:'exited',enabled:false} });
assert.equal(elements['ocr-mode'].disabled,false);
handlers['ocr-echo']({ payload:[[0,105]] });
assert.equal(received.length,receivedBeforeExit,'Late result after process exit must be ignored');
elements['ocr-mode'].value = 'external';
assert.equal(elements['ocr-window'].value,'456','Keep the stale selection to reproduce the external-mode regression');
await elements['ocr-mode'].trigger('change');
assert.equal(elements['ocr-window-row'].hidden,true);
await elements['ocr-toggle'].trigger('click');
assert.equal(calls.at(-1).args.mode,'external');
assert.equal(calls.at(-1).args.windowId,null,'External OCR must not validate a stale native window');
await elements['ocr-toggle'].trigger('click');
assert.equal(calls.at(-1).args.enabled,false);
fail = true;
await elements['ocr-toggle'].trigger('click');
assert.equal(elements['ocr-mode'].disabled,false);
assert.equal(notices.length,1);
fail = false;
capabilities = { native_supported:true, native_available:false };
await setupOcr(setupOptions);
assert.equal(elements['ocr-mode'].value,'external');
assert.equal(elements['ocr-mode'].nativeOption.disabled,true);
assert.equal(elements['ocr-mode'].nativeOption.hidden,false);
assert.equal(elements['ocr-mode'].parentElement.hidden,true);
const beforeRefresh = calls.length;
await elements['ocr-refresh'].trigger('click');
assert.equal(calls.length,beforeRefresh,'No sidecar lookup when the native component is absent');
await elements['ocr-toggle'].trigger('click');
assert.deepEqual(calls.at(-1).args,{enabled:true,mode:'external',windowId:null});
await elements['ocr-toggle'].trigger('click');
capabilities = { native_supported:false, native_available:false };
await setupOcr(setupOptions);
assert.equal(elements['ocr-mode'].value,'external');
assert.equal(elements['ocr-mode'].nativeOption.hidden,true,'Unsupported platforms hide native OCR');
assert.equal(elements['ocr-mode'].parentElement.hidden,true,'External-only platforms hide the source selector');
assert.match(elements['ocr-status'].textContent,/UDP · 127\.0\.0\.1:9999/,'Hidden selector must still expose the external UDP address');
await elements['ocr-toggle'].trigger('click');
assert.equal(calls.at(-1).args.mode,'external');
await elements['ocr-toggle'].trigger('click');
const source = await readFile(new URL('../apps/web/ocr-ui.js', import.meta.url),'utf8');
capabilities = { native_supported:true, native_available:true };
await setupOcr(setupOptions);
setLanguage('zh-CN');
for (const terminal of ['exited', 'error']) {
  duringToggle = () => {
    handlers['ocr-status']({ payload:{status:'ready',enabled:true} });
    handlers['ocr-status']({ payload:{status:terminal,enabled:false} });
  };
  await elements['ocr-toggle'].trigger('click');
  assert.equal(calls.at(-1).args.enabled,true,'Retry must start a new session');
  assert.equal(elements['ocr-mode'].disabled,false,'Startup failure must unlock controls');
  assert.match(elements['ocr-status'].textContent,/重新开启/);
  assert.equal(elements['ocr-toggle'].textContent,'开启 OCR 自动填入');
}
duringToggle = null;
await elements['ocr-toggle'].trigger('click');
assert.equal(elements['ocr-mode'].disabled,true,'Can restart after a startup exit');
await elements['ocr-toggle'].trigger('click');
for (const [,key] of source.matchAll(/'([^'\n]*[\p{Script=Han}][^'\n]*)'/gu)) {
  assert.ok(Object.hasOwn(messages,key), `Untranslated OCR label: ${key}`);
}
console.log('OCR UI passed: stale native selection, external-only installs, platform capabilities, lifecycle and localization.');
