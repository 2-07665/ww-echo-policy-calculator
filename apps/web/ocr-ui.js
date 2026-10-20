import { t, errorText } from './i18n.js';

let enabled = false, pending = false, status = 'off';
let nativeSupported = false, nativeAvailable = false;
const $ = id => document.getElementById(id);
const labels = {
  off: 'OCR 未开启', ready: '等待鸣潮窗口', waiting_game: '等待鸣潮窗口', select_window: '发现多个鸣潮窗口，请停止后选择窗口',
  paused: '游戏窗口不可用或已最小化，识别暂停', recognized: '副词条已识别', confirming: '正在确认副词条',
  not_echo_page: '等待装备或强化页面', unsupported_size: '暂不支持此分辨率比例，请使用 16:9',
  capture_error: '窗口捕获失败，正在重试', error: 'OCR 运行失败，请重新开启', exited: 'OCR 进程已退出，请重新开启',
  external: '等待外部 OCR 结果', no_substats: '当前没有已解锁的副词条',
};
export function renderOcr() {
  const panel = $('ocr-controls');
  if (!panel || panel.hidden) return;
  $('ocr-toggle').textContent = pending ? t('OCR 正在切换…') : enabled ? t('OCR 已开启 · 点击停止') : t('开启 OCR 自动填入');
  $('ocr-toggle').disabled = pending;
  for (const id of ['ocr-mode','ocr-window','ocr-refresh']) $(id).disabled = enabled || pending;
  const native = nativeAvailable && $('ocr-mode').value === 'native';
  $('ocr-window-row').hidden = !native;
  $('ocr-status').textContent = status === 'off' && nativeSupported && !nativeAvailable
    ? t('内置 OCR 组件未安装，可使用外部 OCR 或安装含 OCR 的 Windows 版本。')
    : t(labels[status] || '词条尚未确认，继续识别');
}
export async function setupOcr({ desktop, receive, notice }) {
  if (!desktop) return;
  const invoke = (...args) => window.__TAURI__.core.invoke(...args);
  try {
    const capabilities = await invoke('ocr_capabilities');
    nativeSupported = capabilities.native_supported === true;
    nativeAvailable = nativeSupported && capabilities.native_available === true;
  } catch (error) {
    nativeSupported = nativeAvailable = false;
    notice(() => errorText(error));
  }
  const nativeOption = $('ocr-mode').querySelector('option[value="native"]');
  if (nativeOption) { nativeOption.hidden = !nativeSupported; nativeOption.disabled = !nativeAvailable; }
  $('ocr-mode').value = nativeAvailable ? 'native' : 'external';
  $('ocr-controls').hidden = false;
  const refresh = async () => {
    if (!nativeAvailable || enabled || pending) return;
    const selected = $('ocr-window').value;
    try {
      const windows = await invoke('ocr_windows');
      $('ocr-window').replaceChildren(new Option(t('自动选择唯一鸣潮窗口'), ''));
      for (const item of windows) $('ocr-window').add(new Option(`${item.title} · ${item.id}`, item.id));
      if (windows.some(item => item.id === selected)) $('ocr-window').value = selected;
    } catch (error) { notice(() => errorText(error)); }
  };
  $('ocr-refresh').addEventListener('click', refresh);
  $('ocr-mode').addEventListener('change', () => { status = 'off'; renderOcr(); });
  $('ocr-toggle').addEventListener('click', async () => {
    if (pending) return;
    pending = true; renderOcr();
    try {
      const mode = nativeAvailable && $('ocr-mode').value === 'native' ? 'native' : 'external';
      enabled = await invoke('toggle_ocr', { enabled: !enabled, mode, windowId: mode === 'native' ? $('ocr-window').value || null : null });
      status = enabled ? ($('ocr-mode').value === 'native' ? 'waiting_game' : 'external') : 'off';
    } catch (error) { enabled = false; status = 'error'; notice(() => errorText(error)); }
    finally { pending = false; renderOcr(); }
  });
  window.__TAURI__.event.listen('ocr-echo', ({ payload }) => { if (enabled && !pending) receive(payload); })
    .catch(error => notice(() => errorText(error)));
  window.__TAURI__.event.listen('ocr-status', ({ payload }) => {
    if (!enabled || pending) return;
    status = payload.status;
    if (payload.enabled === false) enabled = false;
    renderOcr();
  }).catch(error => notice(() => errorText(error)));
  renderOcr();
}
