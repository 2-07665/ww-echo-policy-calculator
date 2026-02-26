export function invoke(command, args = {}) {
  const api = window.__TAURI__?.core;
  if (!api || typeof api.invoke !== 'function') {
    throw new Error('Tauri API is unavailable');
  }
  return api.invoke(command, args);
}
