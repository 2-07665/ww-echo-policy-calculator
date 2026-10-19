import { t } from './i18n.js';
export const desktop = Boolean(window.__TAURI__?.core);
export async function api(name, payload) {
  if (desktop) {
    try { return await window.__TAURI__.core.invoke(name, payload ? { payload } : {}); }
    catch (error) { throw new Error(error?.message || String(error)); }
  }
  let response;
  try {
    response = await fetch(`/api/${name}`, payload ? {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload),
    } : {});
  } catch { throw new Error('网络连接失败，请检查求解服务。'); }
  const data = await response.json().catch(() => null);
  if (!response.ok) throw new Error(data?.message ? data.message : t('服务暂不可用（HTTP {status}）', {status:response.status}));
  return data;
}
