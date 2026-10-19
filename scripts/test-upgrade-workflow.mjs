import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const {chromium} = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const browser = await chromium.launch({headless:true});
try {
  const page = await browser.newPage();
  const url = process.env.ECHO_URL || 'http://127.0.0.1:3000';
  const boot = await (await page.request.get(url+'/api/bootstrap')).json();
  await page.addInitScript(config => localStorage.setItem('echo-studio-session-v3',JSON.stringify({config,echo:[]})),{...boot.defaults,mode:'upgrade',target:40});
  const requests = [], results = [], errors = [];
  page.on('pageerror', e=>errors.push(e.message));
  let holdNext = false, release;
  await page.route('**/api/solve',async route=>{
    requests.push(route.request().postDataJSON());
    if(holdNext){holdNext=false;await new Promise(resolve=>{release=resolve;});}
    await route.continue();
  });
  page.on('response',async response=>{if(response.url().endsWith('/api/solve') && response.ok()) results.push(await response.json());});
  const pause = () => new Promise(r=>setTimeout(r,550));
  const ready = () => page.waitForFunction(()=>document.getElementById('result-status').textContent==='计算完成');
  const settled = async () => {await page.waitForFunction(()=>document.querySelector('.results-column').getAttribute('aria-busy') !== 'true'); await ready(); await page.waitForFunction(()=>!document.getElementById('solve').disabled);};
  const until = async check=>{for(let i=0;i<200;i++){if(check())return;await new Promise(r=>setTimeout(r,25));}throw Error('Timed out');};
  await page.goto(url); await page.locator('#solve:not([disabled])').waitFor(); await pause();
  assert.equal(requests.length,0,'Opening upgrade mode must not solve');
  await page.locator('#target').fill('42');
  await page.locator('#sample').click(); await pause();
  assert.equal(requests.length,0,'Before confirming the policy, Echo edits must not solve');
  await page.locator('#solve').click(); await settled();
  assert.equal(requests.length,1);
  assert.equal(requests[0].config.target,42);
  const value=page.locator('[data-kind=echo][data-slot="0"][data-field=value]');
  // A request must start immediately, without the old 350 ms debounce.
  const immediate=page.waitForRequest(r=>r.url().endsWith('/api/solve'),{timeout:300});
  await value.selectOption('105'); await immediate; await settled();
  assert.equal(requests.length,2);
  assert.equal(results.at(-1).cached,true,'Echo-only edits reuse the solved policy');
  // Clear and fill the next Echo under the same target.
  await page.locator('#clear-echo').click(); await settled();
  assert.deepEqual(requests.at(-1).echo,[]);
  assert.equal(results.at(-1).cached,true);
  await page.locator('#sample').click(); await settled();
  for(const [slot,type] of [[2,'2'],[3,'8'],[4,'9']]) {
    await page.locator(`[data-kind=echo][data-slot="${slot}"][data-field=type]`).selectOption(type); await settled();
  }
  assert.equal(results.at(-1).decision,'keep');
  assert.equal(results.at(-1).cached,true);
  let before=requests.length;
  await page.locator('#target').fill('45'); await pause();
  await value.selectOption('93'); await pause();
  assert.equal(requests.length,before,'New targets require explicit solve even after subsequent Echo edits');
  assert.equal(await page.locator('#result-status').innerText(),'待求解');
  await page.locator('[data-language=en]').click();
  assert.equal(await page.locator('#result-status').innerText(),'Policy required');
  assert.equal(await page.locator('#compute-hint').innerText(),'Confirm scoring and target, then solve the policy');
  await page.locator('[data-language="zh-CN"]').click();
  await page.locator('#solve').click(); await settled();
  assert.equal(requests.at(-1).config.target,45);
  // Hold an Echo query to check every visible loading frame, not just its result.
  await page.evaluate(() => {
    window.stableNodes = ['#decision-content h2','.decision-detail strong','#resources > div','#score-note','#solve > span','[data-kind=echo][data-slot="0"][data-field=value]'].map(s=>[s,document.querySelector(s)]);
  });
  const visualState = () => page.evaluate(() => {
    const card=document.getElementById('decision-card'), button=document.getElementById('solve');
    return {
      decision:document.getElementById('decision-content').innerHTML,
      resources:document.getElementById('resources').innerHTML,
      score:document.getElementById('current-score').textContent,
      gap:document.getElementById('score-note').textContent,
      button:button.textContent, buttonColor:getComputedStyle(button).backgroundColor,
      cardColor:getComputedStyle(card).backgroundColor,
      opacity:getComputedStyle(document.getElementById('resources')).opacity,
      height:card.getBoundingClientRect().height,
    };
  });
  await pause();
  const visualBefore=await visualState();
  holdNext=true;release=null;
  await value.selectOption('87');await until(()=>Boolean(release));await pause();
  assert.deepEqual(await visualState(),visualBefore,'Loading must not change card layout, resource opacity, button or score');
  assert.equal(await page.locator('#result-status').innerText(),'更新中');
  assert.equal(await page.locator('#decision-content p').count(),0);
  release();await settled();
  assert.equal(await page.evaluate(()=>window.stableNodes.every(([s,n])=>document.querySelector(s)===n)),true,'Refresh should preserve the existing nodes');
  assert.notEqual(await page.locator('#current-score').innerText(),visualBefore.score);
  assert.equal(await page.locator('#compute-hint').isVisible(),false);
  // Configuration changes cancel any queued Echo update while a query is in flight.
  holdNext=true;release=null;
  await value.selectOption('81');await until(()=>Boolean(release));
  await value.selectOption('99');
  await page.locator('#target').fill('46');before=requests.length;
  release();await pause();
  assert.equal(requests.length,before,'Configuration edit cancels queued query');
  assert.equal(await page.locator('#result-status').innerText(),'待求解');
  // Echo changes during explicit solving query the latest Echo after it completes.
  holdNext=true;release=null;
  await page.locator('#solve').click();await until(()=>Boolean(release));
  await value.selectOption('105');before=requests.length;
  release();await settled();
  assert.equal(requests.length,before+1);
  assert.equal(requests.at(-1).echo[0][1],105);
  assert.equal(results.at(-1).cached,true);
  // Scorer, preset, weights, normalization, and costs never solve on edit.
  before=requests.length;
  await page.locator('#scoring').selectOption('wuwa_echo_tool');
  await page.locator('#preset').selectOption({index:1});
  await page.locator('#weight-details summary').click();
  await page.locator('#weight-0').fill('100');
  await page.locator('#main-score').fill('10');
  await page.locator('#max-score').fill('100');
  await page.locator('#cost-details summary').click();
  await page.locator('#cost-1').fill('2');
  await pause();await value.selectOption('99');await pause();
  assert.equal(requests.length,before);
  assert.deepEqual(errors,[]);
  console.log('Upgrade workflow passed: manual policy confirmation, immediate cached Echo queries, new Echoes, target reached, config invalidation, queued edits, bilingual states.');
} finally {await browser.close();}
