// Build echo_server first. Tests hot data replacement on a running binary and the shared desktop UI.
import assert from 'node:assert/strict';
import {mkdtemp,readFile,writeFile,rename,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve} from 'node:path';
import {spawn,execFileSync} from 'node:child_process';
import {once} from 'node:events';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const dir=await mkdtemp(join(tmpdir(),'echo-data-ui-'));
const dataPath=join(dir,'echo-data.json');
const binary=resolve('target/release/echo_server');
const original=JSON.parse(await readFile('data/echo-data.json','utf8'));
await writeFile(dataPath,JSON.stringify(original));
const port=process.env.ECHO_DATA_TEST_PORT||'3012';
const url=`http://127.0.0.1:${port}`;
const server=spawn(binary,[],{env:{...process.env,ECHO_BIND:`127.0.0.1:${port}`,ECHO_DATA_FILE:dataPath,ECHO_QUOTA_JOINT:'100'},stdio:['ignore','pipe','pipe']});
const exited=new Promise(resolve=>{server.once('exit',resolve);server.once('error',resolve);});
let browser, startupTimer;
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function publish(pack) {
  await writeFile(join(dir,'next.json'),typeof pack==='string'?pack:JSON.stringify(pack));
  await rename(join(dir,'next.json'),dataPath);
  await pause(1100);
}
async function update(page) {
  await page.locator('#data-update').click();
  await page.waitForFunction(()=>!document.getElementById('data-update').disabled);
}
try {
  await Promise.race([once(server.stdout,'data'),exited.then(()=>{throw Error('Test server exited');}),new Promise((_,reject)=>{startupTimer=setTimeout(()=>reject(Error('Server did not start')),10000);})]);
  clearTimeout(startupTimer);
  browser=await chromium.launch({headless:true});
  const page=await browser.newPage({viewport:{width:1360,height:1000}});
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.goto(url);await page.locator('#solve:not([disabled])').waitFor();
  await page.locator('#preset').selectOption('builtin:linear:group:爱弥斯');
  const before=await page.locator('[data-weight]').evaluateAll(inputs=>inputs.map(i=>i.value));
  let solveRequests=0;
  page.on('request',request=>{if(request.url().endsWith('/api/solve')) solveRequests++;});
  await page.locator('#target').fill('0');
  await page.locator('#solve').click();
  await page.waitForFunction(()=>document.getElementById('result-status').textContent==='计算完成');
  const decision=await page.locator('#decision-content').innerText();
  const requestsBefore=solveRequests;
  const target=await page.locator('#target').inputValue();
  await page.locator('#data-open').click();
  assert.equal(await page.locator('#data-desktop').isVisible(),false);
  const next=structuredClone(original); next.version='test-next';
  next.characters['测试角色']={en:'Future Resonator',release_date:'2099-01-01'};
  next.variant_names['新变体']='New Variant';
  const added=structuredClone(next.presets.linear.presets[0]);added.presetName='测试角色';
  added.variants[0].id='future-test';added.variants[0].variantName='新变体';added.variants[0].weights.Crit_Rate=42;
  next.presets.linear.presets.unshift(added);
  next.presets.linear.presets[1].variants[0].weights.Crit_Rate=22;
  await publish(next);await update(page);
  assert.match(await page.locator('#data-version').innerText(),/test-next/);
  assert.equal(await page.locator('#preset').inputValue(),'');
  assert.deepEqual(await page.locator('[data-weight]').evaluateAll(inputs=>inputs.map(i=>i.value)),before);
  assert.equal(await page.locator('#target').inputValue(),target);
  assert.equal(await page.locator('#result-status').innerText(),'计算完成');
  assert.equal(await page.locator('#decision-content').innerText(),decision);
  assert.equal(solveRequests,requestsBefore);
  assert.equal((await (await fetch(`${url}/api/data-pack`)).json()).version,'test-next');
  await page.locator('#data-close').click();
  await page.locator('[data-language=en]').click();
  assert.match(await page.locator('#preset optgroup').first().locator('option').first().innerText(),/Future Resonator/);
  await page.locator('#preset-search').fill('Future Resonator');
  await page.locator('#preset-results button').click();
  assert.equal(await page.locator('#preset-variant option:checked').innerText(),'New Variant');
  assert.equal(await page.locator('#weight-0').inputValue(),'42');
  await page.locator('#preset').selectOption('builtin:linear:group:爱弥斯');
  assert.equal(await page.locator('#weight-0').inputValue(),'22');
  // Neither malformed JSON nor a future schema may displace the last good pack.
  await page.locator('#data-open').click();
  for (const broken of ['{', {...next,schema_version:999}]) {
    await publish(broken);await update(page);
    assert.match(await page.locator('#data-warning').innerText(),/invalid or incompatible/);
    assert.match(await page.locator('#data-version').innerText(),/test-next/);
    assert.equal((await (await fetch(`${url}/api/data-pack`)).json()).version,'test-next');
  }
  await publish(original);await update(page);
  assert.equal(await page.locator('#data-warning').innerText(),'');
  assert.equal(await page.locator('#weight-0').inputValue(),'22');
  await page.screenshot({path:'/tmp/echo-data-update.png'});
  await page.locator('#data-close').click();
  await page.setViewportSize({width:390,height:844});
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
  assert.deepEqual(errors,[]);

  // Exercise desktop IPC payloads and UI failure/success handling. Rust store persistence
  // and cache restart semantics are covered separately by echo_app's filesystem tests.
  const desktop=await browser.newPage();
  let cached=await (await fetch(`${url}/api/bootstrap`)).json();
  const calls=[];
  await desktop.exposeFunction('testInvoke',async (name,args)=> {
    calls.push(name);
    if(name==='bootstrap') return cached;
    if(name==='preview') return (await fetch(`${url}/api/preview`,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(args.payload)})).json();
    if(name==='data_download') {assert.ok(args.payload.url.startsWith('https:'));throw Error('数据下载失败，请检查地址和网络；已保留原数据。');}
    if(name==='data_install') {
      await writeFile(join(dir,'import.json'),args.payload.content);
      try {execFileSync(binary,['validate-data',join(dir,'import.json')],{stdio:'pipe'});} catch {throw Error('数据包无效或版本不兼容，已保留原数据。');}
      const pack=JSON.parse(args.payload.content);
      cached={...cached,presets:pack.presets,data:{...pack,revision:'import-test',status:{source:'external',warning:null}}};return cached;
    }
    throw Error(`Unexpected IPC ${name}`);
  });
  await desktop.addInitScript(()=>{window.__TAURI__={core:{invoke:(...args)=>window.testInvoke(...args)},event:{listen:async()=>{}}};});
  await desktop.goto(url);await desktop.locator('#solve:not([disabled])').waitFor();
  await desktop.locator('#data-open').click();
  assert.equal(await desktop.locator('#data-desktop').isVisible(),true);
  await update(desktop);assert.match(await desktop.locator('#data-message').innerText(),/数据下载失败/);
  await desktop.locator('#data-import').setInputFiles({name:'invalid.json',mimeType:'application/json',buffer:Buffer.from('{}')});
  await desktop.waitForFunction(()=>document.getElementById('data-message').textContent.includes('数据包无效'));
  await desktop.locator('#data-import').setInputFiles({name:'echo-data.json',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(next))});
  await desktop.waitForFunction(()=>document.getElementById('data-version').textContent.includes('test-next'));
  await desktop.reload();await desktop.locator('#solve:not([disabled])').waitFor();
  await desktop.locator('#data-open').click();assert.match(await desktop.locator('#data-version').innerText(),/test-next/);
  assert.ok(calls.includes('data_download')&&calls.includes('data_install'));
  console.log('Data updates passed: running-server reload, new bilingual character/variant, preserved inputs, invalid-pack fallback, recovery, mobile layout, desktop import/download UI and cached bootstrap.');
} finally {
  clearTimeout(startupTimer);
  await browser?.close();server.kill('SIGINT');await exited;await rm(dir,{recursive:true,force:true});
}
