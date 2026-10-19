// Isolated browser test; generated secrets are never printed or written to a file.
import assert from 'node:assert/strict';
import {execFileSync, spawn} from 'node:child_process';
import {createRequire} from 'node:module';
import {createServer} from 'node:net';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const binary=process.env.ECHO_SERVER_BINARY || 'target/release/echo_server';
const generated=execFileSync(binary,['generate-key'],{encoding:'utf8'});
const key=generated.split('\n')[1], hash=generated.split('ECHO_ADMIN_KEY_HASHES=')[1].trim();
const socket=createServer(); await new Promise(resolve=>socket.listen(0,'127.0.0.1',resolve));
const port=socket.address().port; await new Promise(resolve=>socket.close(resolve));
const server=spawn(binary,[],{env:{...process.env,ECHO_BIND:`127.0.0.1:${port}`,ECHO_ADMIN_KEY_HASHES:hash,ECHO_QUOTA_JOINT:'0'},stdio:'ignore'});
const url=`http://127.0.0.1:${port}`;
let browser;
try {
  for(let i=0;i<100;i++) { try { if((await fetch(url+'/api/health')).ok) break; } catch {} await new Promise(r=>setTimeout(r,25)); }
  browser=await chromium.launch({headless:true});
  const page=await browser.newPage();
  await page.goto(url); await page.locator('#solve:not([disabled])').waitFor();
  await page.locator('#solve').click(); await page.waitForFunction(()=>document.getElementById('notice').textContent.includes('配额'));
  await page.locator('#access-open').click();
  await page.locator('#access-key').fill('invalid'); await page.locator('#access-form button[type=submit]').click();
  await page.waitForFunction(()=>document.getElementById('access-message').textContent.includes('无效'));
  await page.locator('#access-key').fill(key); await page.locator('#access-form button[type=submit]').click();
  await page.waitForFunction(()=>document.getElementById('access-message').textContent.includes('已验证'));
  assert.equal(await page.locator('#access-key').inputValue(),'');
  assert.equal(await page.evaluate(k=>Object.values(localStorage).some(v=>v.includes(k)),key),false);
  await page.locator('#access-close').click(); await page.locator('#solve').click();
  await page.waitForFunction(()=>document.getElementById('result-status').textContent==='计算完成');
  await page.reload(); await page.locator('#access-open').click();
  assert.match(await page.locator('#access-message').innerText(),/已设置/);
  await page.locator('#access-clear').click();
  assert.match(await page.locator('#access-message').innerText(),/已清除/);
  await page.locator('#access-close').click(); await page.locator('#target').fill('71'); await page.locator('#solve').click();
  await page.waitForFunction(()=>document.getElementById('notice').textContent.includes('配额'));
  await page.locator('[data-language=en]').click();
  assert.match(await page.locator('#notice').innerText(),/quota/);
  assert.equal(await page.locator('#access-open').innerText(),'Access key');
  // Clearing a key must win even if an earlier verification finishes later.
  let release;
  await page.route('**/api/auth', async route => {
    await new Promise(resolve => { release = resolve; });
    await route.continue();
  });
  await page.locator('#access-open').click();
  await page.locator('#access-key').fill(key);
  await page.locator('#access-form button[type=submit]').click();
  for (let i=0;i<100 && !release;i++) await new Promise(r=>setTimeout(r,10));
  assert.ok(release);
  await page.locator('#access-clear').click();
  release();
  await page.locator('#access-form button[type=submit]:enabled').waitFor();
  assert.equal(await page.locator('#access-message').innerText(),'Access key cleared');
  assert.equal(await page.evaluate(()=>sessionStorage.getItem('echo-studio-access-key')),null);
  assert.equal(await page.evaluate(async()=>(await import('/api.js')).hasAccessKey()),false);
  console.log('Access-key UI passed: validation, quota bypass, session-only storage, reload, clear during pending verification and bilingual errors.');
} finally {
  await browser?.close(); server.kill('SIGINT');
}
