import { t } from './i18n.js';
export const desktop = Boolean(window.__TAURI__?.core);
let accessKey = '', accessKeyRevision = 0;
try { accessKey = sessionStorage.getItem('echo-studio-access-key') || ''; } catch { /* Optional storage. */ }
export function hasAccessKey() { return Boolean(accessKey); }
export async function setAccessKey(key) {
  key = key.trim();
  const revision = ++accessKeyRevision;
  try { if (key) await request('auth', {}, key); }
  catch (error) { if (revision !== accessKeyRevision) return false; throw error; }
  // Clearing or replacing the key supersedes an older verification still in flight.
  if (revision !== accessKeyRevision) return false;
  accessKey = key;
  try { if (key) sessionStorage.setItem('echo-studio-access-key', key); else sessionStorage.removeItem('echo-studio-access-key'); } catch { /* Keep it only in memory. */ }
  return true;
}
async function request(path, payload, key = accessKey) {
  const headers = {};
  if (key) headers.Authorization = `Bearer ${key}`;
  if (payload) headers['Content-Type'] = 'application/json';
  let response;
  try { response = await fetch(`/api/${path}`, {method:payload ? 'POST' : 'GET', headers, ...(payload ? {body:JSON.stringify(payload)} : {}), cache:'no-store'}); }
  catch { throw new Error('网络连接失败，请检查求解服务。'); }
  const data = await response.json().catch(() => null);
  if (!response.ok) {
    const error = new Error(data?.message ? data.message : t('服务暂不可用（HTTP {status}）', {status:response.status}));
    error.code = data?.code;
    error.retryAfter = Number(response.headers.get('Retry-After')) || 0;
    throw error;
  }
  return {response, data};
}
export async function api(name, payload, onProgress) {
  if (desktop) {
    try { return await window.__TAURI__.core.invoke(name, payload ? { payload } : {}); }
    catch (error) { throw new Error(error?.message || String(error)); }
  }
  const {response, data} = await request(name, payload);
  if (name !== 'solve' || response.status !== 202) return data;
  const id = data.id;
  let delay = 25;
  for (;;) {
    const {data:job} = await request(`jobs/${encodeURIComponent(id)}`);
    if (job.status === 'done') return job.result;
    if (job.status === 'failed') throw new Error(job.error.message);
    onProgress?.(job.status);
    await new Promise(resolve => setTimeout(resolve, delay));
    delay = Math.min(delay * 2, 250);
  }
}
