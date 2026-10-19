// Joint workflow: confirm configuration, upgrade with live advice, hand off to reroll.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const browser = await chromium.launch({headless:true});
let hold=false,release,rejectHeld=false;
try {
  const page=await browser.newPage();
  const url=process.env.ECHO_URL || 'http://127.0.0.1:3000';
  const boot=await (await page.request.get(url+'/api/bootstrap')).json();
  await page.addInitScript(config=>localStorage.setItem('echo-studio-session-v3',JSON.stringify({config,echo:[]})),{...boot.defaults,mode:'joint',costs:[0,1,0,1]});
  const requests=[],responses=[],errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  await page.route('**/api/solve',async route=>{
    requests.push(route.request().postDataJSON());
    if(hold){hold=false;const fail=rejectHeld;rejectHeld=false;await new Promise(resolve=>{release=resolve;});if(fail)return route.fulfill({status:422,contentType:'application/json',body:JSON.stringify({message:'obsolete error'})});}
    await route.continue();
  });
  page.on('response', async r => { if (r.url().endsWith('/api/solve') && !r.ok()) responses.push(await r.json()); if (r.url().includes('/api/jobs/') && r.ok()) { const job = await r.json(); if (job.status === 'done') responses.push(job.result); } });
  const pause=()=>new Promise(r=>setTimeout(r,550));
  const until=async check=>{for(let i=0;i<300;i++){if(check())return;await new Promise(r=>setTimeout(r,25));}throw Error('Timed out');};
  const ready=async()=>{
    await page.waitForFunction(()=>['计算完成','Calculated'].includes(document.getElementById('result-status').textContent)&&document.querySelector('.results-column').getAttribute('aria-busy')!=='true'&&!document.getElementById('solve').disabled);
    await until(()=>requests.length===responses.length);
  };
  const noRerollPanels=async()=>{
    for(const id of ['candidate-panel','comparison-panel','locks-panel'])assert.equal(await page.locator('#'+id).isVisible(),false,id);
  };
  await page.goto(url);await page.locator('#solve:not([disabled])').waitFor();await pause();
  assert.equal(requests.length,0,'Joint configuration must be confirmed explicitly');
  assert.equal(await page.locator('#solve').getAttribute('class'),'primary');
  await noRerollPanels();
  await page.locator('#sample').click();await pause();assert.equal(requests.length,0);
  await page.locator('#solve').click();await ready();
  assert.equal(requests.length,1);
  assert.equal(responses.at(-1).decision,'upgrade');
  const value=page.locator('[data-kind=echo][data-slot="0"][data-field=value]');
  const immediate=page.waitForRequest(r=>r.url().endsWith('/api/solve'),{timeout:300});
  await value.selectOption('105');await immediate;await ready();
  assert.equal(responses.at(-1).cached,true);
  assert.equal(await page.locator('#compute-hint').isVisible(),false);
  // A held query preserves the existing recommendation, resources, score and button.
  const visual=()=>page.evaluate(()=>({decision:document.getElementById('decision-content').innerHTML,resources:document.getElementById('resources').innerHTML,score:document.getElementById('current-score').textContent,button:document.getElementById('solve').textContent,color:getComputedStyle(document.getElementById('solve')).backgroundColor,opacity:getComputedStyle(document.getElementById('resources')).opacity}));
  await page.mouse.move(0,0);await pause();
  const beforeVisual=await visual();hold=true;release=null;
  await value.selectOption('75');await until(()=>Boolean(release));await pause();
  assert.deepEqual(await visual(),beforeVisual);
  await value.selectOption('81');await value.selectOption('93');let before=requests.length;
  release();await ready();assert.equal(requests.length,before+1);
  assert.equal(requests.at(-1).echo[0][1],93);
  // A failed obsolete query must still let the latest Echo edit update the advice.
  hold=true;rejectHeld=true;release=null;
  await value.selectOption('81');await until(()=>Boolean(release));
  await value.selectOption('99');before=requests.length;
  release();await ready();
  assert.equal(requests.length,before+1);
  assert.equal(requests.at(-1).echo[0][1],99);
  assert.equal(await page.locator('#notice').isVisible(),false);
  // Finish the Echo and move to reroll without entering the same inputs again.
  for(const [slot,type] of [[2,'2'],[3,'8'],[4,'9']]){
    await page.locator(`[data-kind=echo][data-slot="${slot}"][data-field=type]`).selectOption(type);await ready();
  }
  await noRerollPanels();
  assert.equal(responses.at(-1).decision,'reroll');
  assert.equal(await page.locator('#decision-content p').count(),0,'No lock detail in joint advice');
  assert.equal(await page.locator('#go-reroll').isVisible(),true);
  const joint=structuredClone(requests.at(-1));
  await page.locator('[data-language=en]').click();
  assert.equal(await page.locator('#go-reroll').innerText(),'Go to Reroll policy');
  await page.locator('#go-reroll').click();await ready();
  assert.equal(requests.at(-1).config.mode,'reroll');
  assert.deepEqual(requests.at(-1).config,{...joint.config,mode:'reroll'});
  assert.deepEqual(requests.at(-1).echo,joint.echo);
  assert.equal(requests.at(-1).candidate,null);
  assert.equal(await page.locator('#candidate-details').evaluate(el=>el.open),true);
  assert.equal(await page.locator('#comparison-panel').isVisible(),true);
  assert.equal(await page.locator('#locks-panel').isVisible(),true);
  assert.equal(await page.locator('#comparison-panel').evaluate(el=>Boolean(el.compareDocumentPosition(document.getElementById('locks-panel'))&Node.DOCUMENT_POSITION_FOLLOWING)),true);
  assert.match(await page.locator('#locks-panel').innerText(),/up to 5/);
  // A prior reroll candidate is never sent or displayed in joint mode.
  await page.locator('#copy-candidate').click();await ready();
  await page.locator('[data-mode=joint]').click();await pause();
  await noRerollPanels();
  await page.locator('#solve').click();await ready();
  assert.equal(requests.at(-1).candidate,null);
  // Configuration edits cancel queued Echo updates and invalidate the handoff.
  hold=true;release=null;await value.selectOption('87');await until(()=>Boolean(release));
  assert.equal(await page.locator('#go-reroll').isDisabled(),true);
  await value.selectOption('99');await page.locator('#target').fill('75');before=requests.length;
  assert.equal(await page.locator('#go-reroll').isVisible(),false);
  release();await pause();assert.equal(requests.length,before);
  assert.equal(await page.locator('#result-status').innerText(),'Policy required');
  await value.selectOption('105');await pause();assert.equal(requests.length,before);
  // Costs, scoring and target all wait for explicit confirmation.
  await page.locator('#cost-details summary').click();await page.locator('#cost-3').fill('1000');
  await page.locator('#scoring').selectOption('linear');await page.locator('#target').fill('0');await pause();
  assert.equal(requests.length,before);
  await page.locator('#solve').click();await ready();
  assert.equal(responses.at(-1).decision,'keep');assert.equal(await page.locator('#go-reroll').isVisible(),false);
  await noRerollPanels();
  await page.setViewportSize({width:390,height:844});
  assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true);
  assert.deepEqual(errors,[]);
  console.log('Joint workflow passed: manual confirmation, immediate cached queries, stable refresh, queued edits, reroll handoff with preserved inputs, no reroll panels, configuration invalidation, target completion, bilingual labels and mobile layout.');
} finally {if(release)release();await browser.close();}
