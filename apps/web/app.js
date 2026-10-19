import { buildPresetList, importPresetList, upcoming } from './preset-data.js';
import { t, language, setLanguage, translatePage, statNames, characterNames, characterName, variantName, errorText } from './i18n.js';
import { api, desktop } from './api.js';
const $ = id => document.getElementById(id);
const escape = value => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const format = (value, digits = 2) => Number(value).toLocaleString(language, { maximumFractionDigits: digits, minimumFractionDigits: digits });
const percent = v => `${format(v * 100)}%`;
const modes = {
  upgrade: ['强化策略', '通过强化获得达标声骸。'],
  reroll: ['重抽策略', '使用频整器重抽声骸副词条。'],
  joint: ['联合策略', '同时优化强化和重抽的资源消耗。'],
};
const scorerDefaults = {
  linear: { weights: [100,100,70,0,0,36,0,0,40,0,0,0,0], target: 70, main_score: 0, max_score: 100 },
  wuwa_echo_tool: { weights: [100,100,70,0,0,36,0,0,40,0,0,0,0], target: 70, main_score: 0, max_score: 100 },
  fixed: { weights: [3,3,1,0,0,0,0,0,1,1,0,0,0], target: 7, main_score: 0, max_score: 100 },
  mc_boost_assistant: { weights: [1,1,.7,0,0,.36,0,0,.25,0,0,0,0], target: 95, main_score: 0, max_score: 120 },
  qq_bot: { weights: [2,1,1.1,0,0,.1,0,0,.2,0,0,0,0], target: 35, main_score: 0, max_score: 50 },
};
let confirmedPolicyConfig = null;
let renderedRerollInput = null;
let quietSolve = false, refreshTimer;
let boot, config, echo = [], candidate = [], presets = [], custom = [], revision = 0, previewRevision = 0, timer, autoSolvePending = false, busy = false, solved = false, ocr = false, lastResult = null, lastPreview = null, selectedPreset = '', resultState = 'empty';
function setText(element, value) {
  if (element.textContent === value) return;
  if (element.childNodes.length === 1 && element.firstChild.nodeType === Node.TEXT_NODE) element.firstChild.nodeValue = value;
  else element.textContent = value;
}
function setHTML(element, markup) {
  if (element.innerHTML !== markup) element.innerHTML = markup;
}
function finishRefresh() {
  clearTimeout(refreshTimer);
  refreshTimer = undefined;
  document.querySelector('.results-column').setAttribute('aria-busy', 'false');
}
function statLabel(stat) { return language === 'en' ? statNames[boot.stats.indexOf(stat)] : stat.label; }
let noticeContent = null;
function notice(content = null) { noticeContent = content; renderNotice(); }
function renderNotice() {
  const message = typeof noticeContent === 'function' ? noticeContent() : (noticeContent || '');
  $('notice').textContent = message; $('notice').hidden = !message;
}
function saveStorage(key, value) {
  try { localStorage.setItem(key, JSON.stringify(value)); }
  catch { notice(() => t('本地存储不可用或已满，本次配置仍可计算；请导出预设备份。')); }
}
function readStorage(key, fallback) { try { return JSON.parse(localStorage.getItem(key)) ?? fallback; } catch { return fallback; } }
function number(id) {
  const el = $(id);
  if (el.value.trim() === '' || !el.checkValidity()) throw new Error('请填写有效数值，检查目标、评分参数和资源权重。');
  return Number(el.value);
}
function collect() {
  return { ...config, scoring: $('scoring').value, target: number('target'), main_score: number('main-score'), max_score: number('max-score'),
    weights: Array.from(document.querySelectorAll('[data-weight]'), e => number(e.id)),
    costs: config.mode === 'reroll' ? config.costs : Array.from(document.querySelectorAll('[data-cost]'), e => number(e.id)), refund: config.mode === 'reroll' ? config.refund : number('refund') };
}
function payload() {
  return { config: collect(), echo, candidate: config.mode === 'reroll' && echo.length === 5 && candidate.length === 5 ? candidate : null };
}
let computeHint = ['确认评分与目标后，点击求解策略', {}];
function setComputeHint(key, values = {}) {
  computeHint = [key, values];
  $('compute-hint').textContent = t(key, values);
  $('compute-hint').hidden = !key;
}
function dirty(kind = 'config') {
  if (!['echo', 'candidate', 'accepted-candidate'].includes(kind)) confirmedPolicyConfig = null;
  const refreshing = ((kind === 'echo' && config.mode !== 'reroll' && policyMatches())
    || (['echo', 'candidate', 'accepted-candidate'].includes(kind) && config.mode === 'reroll' && policyMatches()))
    && solved && ['ready', 'refreshing'].includes(resultState);
  resultState = refreshing ? 'refreshing' : solved ? 'stale' : 'empty';
  renderCandidateAction();
  renderRerollTransfer();
  revision++;
  previewRevision++;
  if (refreshing) {
    document.querySelector('.results-column').setAttribute('aria-busy', 'true');
    if (refreshTimer === undefined) refreshTimer = setTimeout(() => {
      if (resultState === 'refreshing') setText($('result-status'), t('更新中'));
    }, 250);
  } else {
    finishRefresh();
  }
  if (solved && !refreshing) {
    document.querySelector('.results-column').classList.add('stale');
    $('result-status').textContent = t('待更新');
    renderPending();
    $('locks-panel').hidden = true;
  }
  notice();
  try { config = collect(); saveStorage('echo-studio-session-v3', { config, echo, selectedPreset }); } catch { /* Keep the last valid session. */ }
  clearTimeout(timer);
  // Cached policy queries return the score and recommendation together.
  if (!refreshing) timer = setTimeout(updatePreview, 160);
  scheduleSolve();
}
function needsPolicy() {
  finishRefresh();
  resultState = 'needs-policy';
  $('result-status').textContent = t('待求解');
  setComputeHint('确认评分与目标后，点击求解策略');
  renderPending();
}
function policyMatches() {
  try { return confirmedPolicyConfig !== null && JSON.stringify(collect()) === confirmedPolicyConfig; }
  catch { return false; }
}
function rerollInputKey(c = collect(), current = echo) {
  return JSON.stringify({ config: c, echo: current });
}
function renderCandidateAction() {
  const button = $('accept-candidate');
  button.hidden = config.mode !== 'reroll' || !lastResult?.comparison || candidate.length !== 5 || !['ready', 'refreshing'].includes(resultState);
  button.disabled = busy || resultState !== 'ready' || candidate.length !== 5 || !policyMatches();
  setText(button, t(lastResult?.comparison?.candidate_cost === 0 ? '接受候选' : '接受候选并继续重抽'));
}
function renderComparison() {
  $('comparison-panel').hidden = config.mode !== 'reroll';
  if (config.mode !== 'reroll') return;
  const c = lastResult?.mode === 'reroll' && ['ready', 'refreshing'].includes(resultState) && candidate.length === 5 ? lastResult.comparison : null;
  if (c) {
    setHTML($('comparison'), `<div class="comparison-result">${c.accept ? t('接受新结果 ↗') : t('保留原结果 ↶')}</div><div class="compare-values">${t('后续频整器：{from} → {to}',{from:format(c.current_cost),to:format(c.candidate_cost)})}<br>${t('候选评分：{score}',{score:format(c.candidate_score)})}</div><p class="field-hint">${t('按后续重抽成本比较，相同成本可接受。请确认候选来自实际重抽并保留了锁定词条。')}</p>`);
  } else {
    setHTML($('comparison'), `<p class="field-hint">${t(!policyMatches() ? '先求解策略，再填入重抽候选进行比较。' : '填入完整的五条重抽候选，查看接受或保留建议。')}</p>`);
  }
  renderCandidateAction();
}
function renderRerollTransfer() {
  const button = $('go-reroll');
  button.hidden = config.mode !== 'joint' || lastResult?.mode !== 'joint' || lastResult.decision !== 'reroll' || !['ready', 'refreshing'].includes(resultState);
  button.disabled = busy || resultState !== 'ready' || !policyMatches();
}
function acceptCandidate() {
  if (config.mode !== 'reroll' || busy || resultState !== 'ready' || !lastResult?.comparison || candidate.length !== 5 || !policyMatches()) return;
  echo = structuredClone(candidate);
  candidate = [];
  // Accepting a real reroll continues the confirmed session with a new baseline.
  renderSlots('echo');
  renderSlots('candidate');
  renderComparison();
  dirty('accepted-candidate');
}
// All modes confirm scoring; Echo and candidate edits query the cached policy.
// Keep at most one request in flight and a single latest-input follow-up.
function scheduleSolve() {
  autoSolvePending = false;
  if (!policyMatches()) { needsPolicy(); return; }
  setComputeHint('');
  if (config.mode === 'reroll' && candidate.length !== 5 && renderedRerollInput === rerollInputKey() && ['ready', 'refreshing'].includes(resultState)) {
    finishRefresh();
    // Partial candidates leave the already-rendered baseline result intact.
    lastResult = { ...lastResult, comparison: null };
    resultState = 'ready';
    setText($('result-status'), t('计算完成'));
    renderComparison();
    return;
  }
  autoSolvePending = true;
  if (!busy) void solve(true);
}
function renderSolveButton() {
  $('solve').classList.add('primary');
  $('solve').classList.remove('secondary');
  setText($('solve').firstElementChild, t(busy && !quietSolve ? '正在求解…' : '求解策略'));
}
async function updatePreview() {
  const token = ++previewRevision;
  try {
    config = collect();
    saveStorage('echo-studio-session-v3', { config, echo, selectedPreset });
    const result = await api('preview', { config, echo });
    if (token !== previewRevision) return;
    renderPreview(result);
  } catch (error) {
    if (token !== previewRevision) return;
    $('current-score').textContent = '—';
    $('max-label').textContent = '/ —';
    $('score-note').textContent = t('请检查评分配置');
  }
}
function renderPreview(result) {
  lastPreview = result;
  setText($('current-score'), format(result.score));
  setText($('max-label'), `/ ${format(result.maximum, config.scoring === 'fixed' ? 0 : 2)}`);
  setText($('score-note'), result.score >= config.target ? t('已达到评分目标') : t('距目标 {score} 分', {score:format(config.target - result.score)}));
}
function valueLabel(index, value) { return boot.stats[index].percent ? `${(value/10).toFixed(1)}%` : String(value); }
function renderSlots(kind) {
  const entries = kind === 'echo' ? echo : candidate;
  const root = $(kind === 'echo' ? 'echo-slots' : 'candidate-slots');
  root.innerHTML = Array.from({ length: 5 }, (_, slot) => {
    const entry = entries[slot];
    const used = entries.map(e => e[0]);
    const options = boot.stats.map((s, i) => `<option value="${i}" ${entry?.[0] === i ? 'selected' : ''} ${used.includes(i) && entry?.[0] !== i ? 'disabled' : ''}>${escape(statLabel(s))}</option>`).join('');
    return `<div class="slot ${entry ? 'filled' : ''}"><span class="slot-index">${slot + 1}</span><select aria-label="${t(kind === 'echo' ? '当前声骸' : '候选')} ${t('词条 {slot} 类型', {slot:slot+1})}" data-slot="${slot}" data-kind="${kind}" data-field="type" ${slot > entries.length ? 'disabled' : ''}><option value="">${slot > entries.length ? t('尚未揭示') : t('选择词条')}</option>${options}</select><select aria-label="${t(kind === 'echo' ? '当前声骸' : '候选')} ${t('词条 {slot} 数值', {slot:slot+1})}" data-slot="${slot}" data-kind="${kind}" data-field="value" ${entry ? '' : 'disabled'}>${entry ? boot.stats[entry[0]].values.map(v => `<option value="${v}" ${entry[1] === v ? 'selected' : ''}>${valueLabel(entry[0],v)}</option>`).join('') : '<option>—</option>'}</select></div>`;
  }).join('');
  $('echo-stage').textContent = t('{level} · {count} / 5 词条', {level:echo.length ? `+${echo.length * 5}` : t('未强化'), count:echo.length});
  $('candidate-panel').hidden = config.mode !== 'reroll';
}
function renderConfig() {
  $('scoring').value = config.scoring;
  $('target').value = config.target;
  $('target').step = config.scoring === 'fixed' ? '1' : '0.1';
  $('main-score').value = config.main_score;
  $('max-score').value = config.max_score;
  $('refund').value = config.refund;
  document.querySelectorAll('[data-cost]').forEach(e => { e.id = `cost-${e.dataset.cost}`; e.value = config.costs[Number(e.dataset.cost)]; });
  $('weights').innerHTML = boot.stats.map((s,i) => `<label>${escape(statLabel(s))}<input id="weight-${i}" data-weight="${i}" type="number" min="0" step="${config.scoring === 'fixed' ? '1' : 'any'}" value="${config.weights[i]}" required></label>`).join('');
  $('normalization').hidden = ['fixed','mc_boost_assistant'].includes(config.scoring);
  $('max-score-field').hidden = config.scoring === 'qq_bot';
  renderWeightHint();
}
function renderWeightHint() {
  $('weight-hint').textContent = config.scoring === 'fixed' ? t('固定评分只看类型，不看数值档位。权重与目标使用整数分数。') : config.scoring === 'qq_bot' ? t('XutheringWavesUID 权重表示单位数值的价值；总分归一化为 50 分。') : t('权重代表该词条最高档位的相对价值；实际数值越高，得分越高。');
}
function renderMode() {
  const text = modes[config.mode];
  ['mode-title','mode-description'].forEach((id,i) => $(id).textContent = t(text[i]));
  document.querySelectorAll('[data-mode]').forEach(e => {
    e.classList.toggle('active', e.dataset.mode === config.mode);
    e.setAttribute('aria-pressed', String(e.dataset.mode === config.mode));
  });
  $('cost-details').hidden = config.mode === 'reroll';
  $('frequency-field').hidden = config.mode !== 'joint';
  $('candidate-panel').hidden = config.mode !== 'reroll';
  $('resource-title').textContent = config.mode === 'reroll' ? t('当前声骸的重抽期望') : t('达标成品的期望消耗');
  renderResources(lastResult);
  renderComparison();
  renderRerollTransfer();
  renderSolveButton();
}
function presetConfig(p) {
  const defaults = scorerDefaults[p.scoring];
  return { ...config, ...defaults, scoring:p.scoring,
    weights: boot.stats.map(s => Number(p.weights?.[s.id] ?? 0)),
    main_score: p.mainBuffScore ?? defaults.main_score,
    max_score: p.normalizedMaxScore ?? defaults.max_score,
    target: p.target ?? defaults.target,
  };
}
function buildPresets() {
  presets = buildPresetList(boot.presets, custom);
}
function presetCharacterLabel(p) {
  const label = characterName(p.character);
  return upcoming(p.character) ? `${label} · ${t('未实装')}` : label;
}
const scoringNames = {linear:'线性数值评分', fixed:'固定词条评分', wuwa_echo_tool:'Wuwa Echo Tool', mc_boost_assistant:'漂泊者强化助手', qq_bot:'XutheringWavesUID'};
function renderPresets(selected = selectedPreset) {
  selectedPreset = selected;
  buildPresets();
  const items = presets.filter(p => p.scoring === config.scoring);
  const groups = [...new Map(items.map(p => [p.groupId, p])).values()];
  let options = `<option value="">${t('默认 / 手动配置')}</option>`;
  for (const builtin of [true, false]) {
    const group = groups.filter(p => p.builtin === builtin);
    if (group.length) options += `<optgroup label="${t(builtin ? '内置预设' : '自定义预设')}">${group.map(p => `<option value="${escape(p.groupId)}">${escape(p.builtin ? presetCharacterLabel(p) : p.name)}</option>`).join('')}</optgroup>`;
  }
  const current = items.find(p => p.id === selectedPreset);
  $('preset').innerHTML = options;
  $('preset').value = current?.groupId || '';
  const variants = current ? items.filter(p => p.groupId === current.groupId) : [];
  $('preset-variant').innerHTML = variants.length ? variants.map(p => `<option value="${escape(p.id)}">${escape(variantName(p.variant))}</option>`).join('') : `<option value="">${t('先选择预设')}</option>`;
  $('preset-variant').value = current?.id || '';
  $('preset-variant').disabled = variants.length <= 1;
  if (!current) selectedPreset = '';
  $('delete-preset').hidden = !current || current.builtin;
  $('preset-intro').textContent = current?.presetIntro ? t(current.presetIntro) : '';
  $('preset-intro').hidden = !current?.presetIntro;
  renderPresetSearch();
}
function renderPresetSearch() {
  const query = $('preset-search').value.trim().toLowerCase();
  // One search result per preset; variants are selected in their own control.
  const seen = new Set();
  const groups = presets.filter(p => !seen.has(p.groupId) && seen.add(p.groupId));
  const matches = query ? groups.filter(p => [p.builtin ? p.character : p.name,characterNames[p.character],t(scoringNames[p.scoring]),p.scoring,p.scoring === 'qq_bot' ? 'QQ Bot XWUID XW-UID' : ''].join(' ').toLowerCase().includes(query)) : [];
  $('preset-count').hidden = !query;
  $('preset-results').hidden = !query || !matches.length;
  $('preset-count').textContent = !query ? '' : matches.length ? t('找到 {count} 个预设，点击即可应用。', {count:matches.length}) : t('没有匹配的预设');
  $('preset-results').innerHTML = matches.map(p => `<button type="button" data-preset-id="${escape(p.id)}"><span>${escape(p.builtin ? presetCharacterLabel(p) : p.name)}</span><small>${escape(t(scoringNames[p.scoring]))}${p.builtin ? '' : ` · ${t('自定义预设')}`}</small></button>`).join('');
}
function applyPreset(id) {
  selectedPreset = id;
  const p = presets.find(p => p.id === id);
  config = p ? presetConfig(p) : { ...config, ...scorerDefaults[config.scoring] };
  renderConfig(); renderPresets(); dirty();
}
function validCustom(p) {
  return p && typeof p.name === 'string' && p.name.trim().length > 0 && p.name.length <= 60 && Object.hasOwn(scorerDefaults, p.scoring)
    && (p.presetIntro == null || typeof p.presetIntro === 'string')
    && p.weights && boot.stats.every(s => Number.isFinite(p.weights[s.id]) && p.weights[s.id] >= 0)
    && [p.mainBuffScore,p.normalizedMaxScore,p.target].every(v => v === undefined || Number.isFinite(v));
}
function describeLock(mask) {
  const labels = boot.stats.filter((_,i) => (mask & (1 << i)) !== 0).map(s => statLabel(s));
  return labels.length ? t('锁定 {names}', {names:labels.join(language === 'en' ? ', ' : '、')}) : t('不锁定任何词条');
}
function renderResources(result) {
  const r = result?.mode === config.mode ? result : null;
  const resource = (label, value, unit = '个', formatter = format) => `<div><span>${t(label)}</span><div class="resource-value"><strong>${value == null ? '—' : formatter(value)}</strong>${unit ? `<small>${t(unit)}</small>` : ''}</div></div>`;
  if (config.mode === 'reroll') {
    setHTML($('resources'), resource('到达目标所需频整器', r?.frequencies) + resource('当前声骸评分', r?.preview.score, '分'));
  } else {
    const values = r?.resources;
    setHTML($('resources'), resource('声骸胚子', values?.echoes) + resource('调谐器', values?.tuners) + resource('特级密音筒', values?.tubes)
      + (config.mode === 'joint' ? resource('频整器', values?.frequencies) : '')
      + resource(config.mode === 'joint' ? '新胚子保留概率' : '新胚子达标概率', values?.attempt_probability, '', percent)
      + resource('加权资源期望', values?.weighted, ''));
  }
}
function renderResult(r) {
  finishRefresh();
  lastResult = r;
  resultState = 'ready';
  renderPreview(r.preview);
  solved = true;
  document.querySelector('.results-column').classList.remove('stale');
  setText($('result-status'), t('计算完成'));
  const probabilityLabel = r.mode === 'joint' ? '当前声骸保留概率' : '当前声骸达标概率';
  const action = { upgrade: t('继续强化'), abandon: t('放弃，换下一个'), keep: t('保留这个声骸'), reroll: t('重抽') }[r.decision];
  const locks = r.mode === 'reroll' && r.decision === 'reroll' ? `<p>${escape(describeLock(r.lock_mask ?? 0))}</p>` : '';
  const heading = $('decision-content').querySelector('h2');
  const probability = $('decision-content').querySelector('.decision-detail strong');
  if (r.mode !== 'reroll' && heading && probability && !$('decision-content').querySelector('p')) {
    setText(heading, action);
    setText(probability.previousElementSibling, t(probabilityLabel));
    setText(probability, percent(r.current_probability));
  } else {
    setHTML($('decision-content'), `<h2>${action}</h2>${locks}${r.current_probability !== null ? `<div class="decision-detail"><span>${t(probabilityLabel)}</span><strong>${percent(r.current_probability)}</strong></div>` : ''}`);
  }
  renderResources(r);
  $('locks-panel').hidden = config.mode !== 'reroll' || !r.locks.length;
  setHTML($('locks'), r.locks.map((l,i) => `<div class="lock-row"><div><span><span class="lock-rank">${i+1}</span>${escape(describeLock(l.mask))}</span><strong>${format(l.expected)} ${t('个')}</strong></div><p>${t('本次 {cost} 个 · 单次达标 {p} · 额外期望 +{regret}', {cost:l.cost,p:percent(l.probability),regret:format(l.regret)})}</p></div>`).join(''));
  renderComparison();
  setComputeHint('');
  renderCandidateAction();
  renderRerollTransfer();
}
async function solve(automatic = false) {
  if (busy) return;
  autoSolvePending = false;
  if (automatic && !policyMatches()) { needsPolicy(); return; }
  let request;
  try {
    request = structuredClone(payload());
    if (config.mode === 'reroll' && echo.length !== 5) throw new Error('重抽模式需要完整的五条词条。请填写全部词条与实际数值。');
  } catch (e) {
    if (automatic) {
      resultState = 'incomplete';
      $('result-status').textContent = t('等待输入');
      setComputeHint('输入完整有效后，自动更新建议');
      renderPending();
    } else notice(() => errorText(e));
    return;
  }
  notice();
  if (!automatic) confirmedPolicyConfig = JSON.stringify(request.config);
  clearTimeout(timer);
  previewRevision++;
  const token = revision;
  quietSolve = automatic && resultState === 'refreshing';
  resultState = quietSolve ? 'refreshing' : 'busy';
  busy = true;
  renderCandidateAction();
  renderRerollTransfer();
  $('solve').setAttribute('aria-disabled', 'true');
  if (!quietSolve) {
    $('solve').disabled = true;
    renderSolveButton();
    $('result-status').textContent = t('计算中');
  }
  const start = Date.now();
  const ticker = quietSolve ? null : setInterval(() => { if (token === revision) setComputeHint('正在精确求解 · {seconds} 秒', {seconds:Math.floor((Date.now()-start)/1000)}); }, 1000);
  try {
    const result = await api('solve',request);
    if (token !== revision) return;
    renderedRerollInput = request.config.mode === 'reroll' ? rerollInputKey(request.config, request.echo) : null;
    previewRevision++;
    renderResult(result);
  } catch (error) {
    // A failed obsolete query must not discard the latest queued Echo edit.
    if (token !== revision) return;
    confirmedPolicyConfig = null;
    autoSolvePending = false;
    finishRefresh();
    notice(() => errorText(error));
    resultState = 'error';
    document.querySelector('.results-column').classList.add('stale');
    $('result-status').textContent = t('未完成');
    renderPending();
    $('locks-panel').hidden = true;
  } finally {
    clearInterval(ticker);
    busy = false;
    quietSolve = false;
    $('solve').setAttribute('aria-disabled', 'false');
    $('solve').disabled = false;
    renderSolveButton();
    renderCandidateAction();
    renderRerollTransfer();
    if (autoSolvePending) void solve(true);
    else if (token !== revision && !policyMatches()) needsPolicy();
  }
}
function renderPending() {
  renderRerollTransfer();
  renderComparison();
  const title = resultState === 'needs-policy' ? t('待求解') : resultState === 'incomplete' ? t('等待输入') : resultState === 'error' ? t('计算失败') : resultState === 'stale' ? t('输入已更新') : t('尚未计算');
  const body = resultState === 'needs-policy' ? (config.mode === 'reroll' ? t('确认评分与目标后，点击求解策略。之后修改当前声骸或重抽候选，会自动更新建议。') : t('确认评分与目标后，点击求解策略。之后修改当前词条会立即更新建议。')) : resultState === 'incomplete' ? t('输入完整有效后，自动更新建议') : resultState === 'error' ? t('本次未能得到有效策略，请查看页面上方的具体原因。') : resultState === 'stale' ? t('正在更新建议。下方数值为上次结果，仅供参考。') : t('设置评分与目标，填入当前词条后点击求解策略。');
  $('decision-content').innerHTML = `<h2>${title}</h2><p>${body}</p>`;
}
function renderLanguageButtons() {
  document.querySelectorAll('[data-language]').forEach(button => button.setAttribute('aria-pressed', String(button.dataset.language === language)));
}
function changeLanguage(value) {
  const currentState = resultState;
  const currentPreview = lastPreview;
  const currentHint = computeHint;
  setLanguage(value);
  renderLanguageButtons();
  translatePage();
  // Keep partially edited numeric fields; only relabel the existing weight inputs.
  document.querySelectorAll('[data-weight]').forEach((input,i) => { input.parentElement.firstChild.textContent = statLabel(boot.stats[i]); });
  renderMode(); renderPresets(); renderSlots('echo'); renderSlots('candidate');
  renderWeightHint();
  if (lastResult) renderResult(lastResult);
  if (currentPreview) renderPreview(currentPreview);
  resultState = currentState;
  if (currentState === 'refreshing') {
    document.querySelector('.results-column').setAttribute('aria-busy', 'true');
    setText($('result-status'), t('更新中'));
  } else if (currentState !== 'ready') {
    document.querySelector('.results-column').classList.toggle('stale', Boolean(lastResult));
    renderPending();
    $('result-status').textContent = t(currentState === 'busy' ? '计算中' : currentState === 'stale' ? '待更新' : currentState === 'error' ? '未完成' : currentState === 'incomplete' ? '等待输入' : currentState === 'needs-policy' ? '待求解' : '等待计算');
    $('locks-panel').hidden = true; renderComparison();
  }
  renderSolveButton();
  renderCandidateAction();
  renderRerollTransfer();
  renderComparison();
  $('ocr-toggle').textContent = t(ocr ? 'OCR 已开启 · 点击停止' : '开启 OCR 自动填入 · UDP 9999');
  setComputeHint(...currentHint);
  renderNotice();
}
function setup() {
  $('language').addEventListener('click', e => {
    const value = e.target.closest('[data-language]')?.dataset.language;
    if (value && value !== language) changeLanguage(value);
  });
  $('modes').addEventListener('click', e => {
    const mode = e.target.closest('[data-mode]')?.dataset.mode;
    if (!mode || mode === config.mode) return;
    config.mode = mode;
    renderMode(); dirty(); notice();
  });
  $('scoring').addEventListener('change', () => {
    config = { ...config, scoring: $('scoring').value, ...scorerDefaults[$('scoring').value] };
    selectedPreset = ''; renderConfig(); renderPresets(); dirty();
  });
  $('preset-search').addEventListener('input', renderPresetSearch);
  $('preset-search').addEventListener('keydown', e => {
    if (e.key === 'ArrowDown' && !$('preset-results').hidden) {
      e.preventDefault(); $('preset-results').querySelector('button')?.focus();
    }
    if (e.key === 'Escape') { e.preventDefault(); $('preset-search').value = ''; renderPresetSearch(); }
  });
  $('preset-results').addEventListener('click', e => {
    const id = e.target.closest('[data-preset-id]')?.dataset.presetId;
    if (!id) return;
    $('preset-search').value = '';
    applyPreset(id);
    $('preset').focus();
  });
  $('preset-results').addEventListener('keydown', e => {
    const button = e.target.closest('[data-preset-id]');
    if (!button) return;
    if (e.key === 'ArrowDown') { e.preventDefault(); (button.nextElementSibling || button).focus(); }
    if (e.key === 'ArrowUp') { e.preventDefault(); (button.previousElementSibling || $('preset-search')).focus(); }
    if (e.key === 'Escape') { e.preventDefault(); $('preset-search').value = ''; renderPresetSearch(); $('preset-search').focus(); }
  });
  $('preset').addEventListener('change', () => applyPreset(presets.find(p => p.groupId === $('preset').value)?.id || ''));
  $('preset-variant').addEventListener('change', () => applyPreset($('preset-variant').value));
  document.querySelector('.workspace').addEventListener('input', e => {
    if (e.target.matches('input[type=number]')) {
      if (e.target.matches('[data-weight],#main-score,#max-score')) { selectedPreset = ''; renderPresets(); }
      dirty();
    }
  });
  document.querySelector('.workspace').addEventListener('change', e => {
    const el = e.target;
    if (!el.matches('[data-slot]')) return;
    const list = el.dataset.kind === 'echo' ? echo : candidate;
    const slot = Number(el.dataset.slot);
    if (el.dataset.field === 'type') {
      if (el.value === '') list.splice(slot,1);
      else list[slot] = [Number(el.value), boot.stats[Number(el.value)].values[0]];
    } else list[slot][1] = Number(el.value);
    const field = el.dataset.field;
    if (field === 'type') renderSlots(el.dataset.kind);
    dirty(el.dataset.kind);
    document.querySelector(`[data-kind="${el.dataset.kind}"][data-slot="${slot}"][data-field="${field}"]`)?.focus();
  });
  $('clear-echo').addEventListener('click', () => { echo = []; renderSlots('echo'); dirty('echo'); });
  $('sample').addEventListener('click', () => {
    echo = config.mode === 'reroll' ? [[0,63],[1,126],[2,64],[8,68],[9,64]] : [[0,75],[1,150]];
    renderSlots('echo'); dirty('echo');
  });
  $('copy-candidate').addEventListener('click', () => { candidate = structuredClone(echo); renderSlots('candidate'); dirty('candidate'); });
  $('clear-candidate').addEventListener('click', () => { candidate = []; renderSlots('candidate'); dirty('candidate'); });
  $('accept-candidate').addEventListener('click', acceptCandidate);
  $('go-reroll').addEventListener('click', () => {
    if (busy || resultState !== 'ready' || config.mode !== 'joint' || lastResult?.mode !== 'joint' || lastResult.decision !== 'reroll' || !policyMatches()) return;
    config = { ...collect(), mode: 'reroll' };
    candidate = [];
    renderMode(); renderSlots('candidate');
    dirty();
    void solve();
  });
  $('solve').addEventListener('click', () => solve());
  $('guide-open').addEventListener('click', () => $('guide').showModal());
  $('guide-close').addEventListener('click', () => $('guide').close());
  $('save-preset').addEventListener('click', async () => {
    try {
      const name = $('preset-name').value.trim();
      if (!name) throw new Error('请先输入自定义预设名称。');
      const c = collect();
      await api('preview', {config:c,echo:[]});
      const p = { name, scoring: c.scoring, weights: Object.fromEntries(boot.stats.map((s,i) => [s.id,c.weights[i]])), mainBuffScore: c.main_score, normalizedMaxScore: c.max_score, target: c.target };
      const index = custom.findIndex(v => v.name === name && v.scoring === c.scoring);
      if (index >= 0) custom[index] = p; else custom.push(p);
      saveStorage('echo-studio-presets-v3',custom);
      selectedPreset = `custom:${c.scoring}:${name}`;
      renderPresets();
      saveStorage('echo-studio-session-v3', {config:c,echo,selectedPreset});
      $('delete-preset').hidden = false;
      $('preset-name').value = '';
    } catch (e) { notice(() => errorText(e)); }
  });
  $('delete-preset').addEventListener('click', () => {
    const p = presets.find(p => p.id === selectedPreset);
    if (!p || p.builtin) return;
    custom = custom.filter(v => v.name !== p.name || v.scoring !== p.scoring);
    saveStorage('echo-studio-presets-v3',custom); renderPresets();
  });
  $('export-presets').addEventListener('click', () => {
    const url = URL.createObjectURL(new Blob([JSON.stringify({ version:3, presets:custom },null,2)], {type:'application/json'}));
    const a = document.createElement('a'); a.href = url; a.download = 'echo-studio-presets.json'; a.click(); setTimeout(() => URL.revokeObjectURL(url),1000);
  });
  $('import-presets').addEventListener('change', async e => {
    try {
      const file = e.target.files[0]; if (!file) return;
      if (file.size > 2_000_000) throw new Error('预设文件不能超过 2 MB。');
      const data = JSON.parse(await file.text());
      if (!Array.isArray(data.presets) || data.presets.length > 500) throw new Error('无效的预设文件。');
      // Read v3 exports and the previous desktop's grouped / legacy scorer JSON.
      const incoming = importPresetList(data, config.scoring, scorerDefaults[config.scoring].weights, boot.stats.map(s => s.id));
      if (incoming.length > 500 || !incoming.every(validCustom)) throw new Error('预设格式无效。旧版预设需先选择对应评分体系再导入。');
      for (const p of incoming) { const i = custom.findIndex(v => v.name === p.name && v.scoring === p.scoring); if (i >= 0) custom[i] = p; else custom.push(p); }
      saveStorage('echo-studio-presets-v3',custom); renderPresets(); notice();
    } catch (error) { notice(() => t('导入失败：{error}', {error:errorText(error)})); }
    finally { e.target.value = ''; }
  });
  if (desktop) {
    $('ocr-toggle').hidden = false;
    $('ocr-toggle').addEventListener('click', async () => {
      try {
        const state = await window.__TAURI__.core.invoke('toggle_ocr', { enabled: !ocr }); ocr = state;
        $('ocr-toggle').textContent = ocr ? t('OCR 已开启 · 点击停止') : t('开启 OCR 自动填入 · UDP 9999');
      } catch (e) { notice(() => errorText(e)); }
    });
    window.__TAURI__.event.listen('ocr-echo', ({ payload }) => { echo = payload; renderSlots('echo'); dirty('echo'); }).catch(e => notice(() => t('OCR 事件连接失败：{error}', {error:errorText(e)})));
  }
}
async function init() {
  translatePage();
  renderLanguageButtons();
  try {
    boot = await api('bootstrap');
    config = structuredClone(boot.defaults);
    const saved = readStorage('echo-studio-session-v3',null);
    if (saved?.config && Object.hasOwn(modes,saved.config.mode) && Object.hasOwn(scorerDefaults,saved.config.scoring)) {
      try { await api('preview', { config:saved.config, echo:saved.echo }); config = saved.config; echo = saved.echo; selectedPreset = saved.selectedPreset || ''; } catch { /* Ignore invalid / older local sessions. */ }
    }
    const stored = readStorage('echo-studio-presets-v3',[]);
    custom = Array.isArray(stored) ? stored.filter(validCustom) : [];
    renderConfig(); renderPresets(); renderMode(); renderSlots('echo'); renderSlots('candidate'); setup();
    $('solve').disabled = false;
    await updatePreview();
    scheduleSolve();
  } catch (error) { notice(() => t('初始化失败：{error}。请检查服务后刷新页面。', {error:errorText(error)})); }
}
init();
