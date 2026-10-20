// Game terminology and character names: see TERMINOLOGY.md. User-authored preset names stay unchanged.
export let language = 'zh-CN';
try { language = localStorage.getItem('echo-studio-language') === 'en' ? 'en' : 'zh-CN'; } catch { /* Storage is optional. */ }
export const messages = {
  'OCR 来源':'OCR source', '内置识别':'Built-in OCR', '外部 ok-ww':'External ok-ww',
  '鸣潮窗口':'Wuthering Waves window', '刷新窗口':'Refresh windows', '自动选择唯一鸣潮窗口':'Automatically select the only game window',
  '开启 OCR 自动填入':'Enable OCR autofill', 'OCR 正在切换…':'Switching OCR…', 'OCR 未开启':'OCR is off',
  '等待鸣潮窗口':'Waiting for Wuthering Waves', '发现多个鸣潮窗口，请停止后选择窗口':'Multiple game windows found. Stop OCR and select a window.',
  '游戏窗口不可用或已最小化，识别暂停':'Game window unavailable or minimized; OCR paused', '副词条已识别':'Substats recognized',
  '正在确认副词条':'Confirming Substats', '等待装备或强化页面':'Waiting for an Echo equipment or upgrade page',
  '暂不支持此分辨率比例，请使用 16:9':'Unsupported aspect ratio. Use 16:9.', '窗口捕获失败，正在重试':'Capture failed; retrying',
  'OCR 运行失败，请重新开启':'OCR failed. Start it again.', 'OCR 进程已退出，请重新开启':'OCR exited. Start it again.',
  '等待外部 OCR 结果':'Waiting for external OCR results', '当前没有已解锁的副词条':'No unlocked Substats', '词条尚未确认，继续识别':'Substats not yet confirmed; scanning',
  "角色数据": "Character data",
  "数据版本：{version}": "Data version: {version}",
  "数据更新地址": "Data update URL",
  "或导入数据包": "Or import a data pack",
  "更新数据": "Update data",
  "更新后，重新选择预设即可应用新权重。当前配置不会改变。": "Select a preset again after updating to apply its new weights. Your current configuration will stay unchanged.",
  "正在更新数据…": "Updating data\u2026",
  "数据已更新。": "Data updated.",
  "当前数据已是最新版本。": "Your data is up to date.",
  "数据包无效或版本不兼容，已保留原数据。": "The data pack is invalid or incompatible. Your previous data has been kept.",
  "无法读取或保存数据包，已保留原数据。": "Could not read or save the data pack. Your previous data has been kept.",
  "数据下载失败，请检查地址和网络；已保留原数据。": "Data download failed. Check the URL and connection. Your previous data has been kept.",

  "访问密钥":"Access key",
  "输入访问密钥":"Enter access key",
  "管理员密钥可绕过个人用量配额。密钥仅保存在当前浏览器标签页的会话中。":"An administrator key bypasses personal usage quotas. The key is stored only for this browser tab’s session.",
  "验证并使用":"Verify and use",
  "清除密钥":"Clear key",
  "当前会话已设置访问密钥":"An access key is set for this session",
  "密钥已验证，个人配额限制已解除":"Key verified. Personal quotas are bypassed.",
  "已清除访问密钥":"Access key cleared",
  "请输入访问密钥。":"Please enter an access key.",
  "访问密钥无效。":"Invalid access key.",
  "等待求解 · {seconds} 秒":"Queued · {seconds}s",
  "请在 {seconds} 秒后重试。":"Try again in {seconds} seconds.",
  "服务队列已满，请稍后重试。":"The server queue is full. Please try again shortly.",
  "请求过于频繁，请稍后重试。":"Too many requests. Please try again shortly.",
  "已达到访问配额，请稍后重试或使用访问密钥。":"Usage quota reached. Please try again later or use an access key.",
  "计算任务已过期，请重新求解。":"This job has expired. Please solve again.",
  "策略超过缓存预算或缓存正在使用，请稍后重试或简化评分。":"The policy exceeds the cache budget or the cache is in use. Try again later or simplify the scoring.",
  "求解任务异常结束，请重试。":"The solver job failed. Please try again.",

  '确认评分与目标后，点击求解策略。之后修改当前声骸或重抽候选，会自动更新建议。':'Confirm scoring and target, then solve the policy. Subsequent edits to the current Echo or reroll candidate automatically update the advice.',
  '更新中':'Updating',
  '求解策略':'Solve policy', '待求解':'Policy required',
  '确认评分与目标后，点击求解策略':'Confirm scoring and target, then solve the policy',
  '确认评分与目标后，点击求解策略。之后修改当前词条会立即更新建议。':'Confirm scoring and target, then solve the policy. Subsequent Echo edits update the recommendation immediately.',
  '设置评分与目标，填入当前词条后点击求解策略。':'Set scoring and the target, enter the current Substats, then click Solve policy.',

  '等待输入':'Waiting for input',
  '输入完整有效后，自动更新建议':'Recommendations update once all required inputs are valid',
  '展开':'Expand', '收起':'Collapse',
  'Echo Studio · 声骸策略工坊':'Echo Studio · Echo Policy Calculator',
  'Echo Studio 首页':'Echo Studio home',
  '声骸策略工坊':'Echo Policy Calculator',
  '求解模式':'Solver', '强化策略':'Upgrade policy', '重抽策略':'Reroll policy', '联合策略':'Joint policy',
  '通过强化获得达标声骸。':'Reach the target by upgrading Echoes.',
  '使用频整器重抽声骸副词条。':'Use Transducers to reroll Echo Substats.',
  '同时优化强化和重抽的资源消耗。':'Optimize resources across upgrades and rerolls.',
  '评分与目标':'Scoring and target',  '评分体系':'Scoring system',
  '线性数值评分':'Linear scoring', '固定词条评分':'Fixed stat scoring', '漂泊者强化助手':'MC Boost Assistant',
  '变体':'Variant', '先选择预设':'Select a preset first', '未实装':'Upcoming',
  '权重预设':'Weight preset', '搜索全部预设':'Search all presets',
  '找到 {count} 个预设，点击即可应用。':'{count} presets found. Click to apply.',
  '角色名或评分体系':'Resonator or scoring system',
  '默认 / 手动配置':'Default / manual configuration', '默认权重':'Default weights', '默认':'Default', '自定义':'Custom',
  '内置预设':'Built-in presets', '自定义预设':'Custom presets',
  '没有匹配的预设':'No matching presets',
  '目标总分':'Target score',
  '调整词条权重':'Stat weights', '主词条原始分':'Raw Mainstat score', '归一化总满分':'Normalized maximum score',
  '权重代表该词条最高档位的相对价值；实际数值越高，得分越高。':'Weights represent each Substat’s maximum-roll value. Higher rolls score higher.',
  '固定评分只看类型，不看数值档位。权重与目标使用整数分数。':'Fixed scoring uses Substat types only. Weights and targets must be integers.',
  'XutheringWavesUID 权重表示单位数值的价值；总分归一化为 50 分。':'XutheringWavesUID weights apply per unit of a stat. The total is normalized to 50 points.',
  '自定义预设名称':'Custom preset name', '预设名称':'Preset name', '保存':'Save', '删除':'Delete',
  '导出自定义预设':'Export custom presets', '导入预设':'Import presets',
  '资源相对价值':'Relative resource values',
  '为每单位资源设置相对价值。求解器最小化各资源「预期消耗 × 相对价值」之和；设为 0 表示不计入优化目标。':'Set a relative value per unit of each resource. The solver minimizes the sum of expected use × relative value. Set a value to 0 to exclude that resource from the objective.',
  '例如：调谐器设为 1、特级密音筒设为 10，表示 1 个特级密音筒与 10 个调谐器等价。':'For example, setting Premium Tuners to 1 and Premium Sealed Tubes to 10 values one tube as ten tuners.',
  '声骸胚子':'New Echoes', '调谐器':'Premium Tuners', '特级密音筒':'Premium Sealed Tubes', '频整器':'Transducers',
  '当前声骸':'Current Echo', '清空':'Clear', '填入示例 ↗':'Load example ↗',
  '未强化':'Unleveled', '未强化 · 0 / 5 词条':'Unleveled · 0 / 5 Substats',
  '{level} · {count} / 5 词条':'{level} · {count} / 5 Substats',
  '候选':'Candidate', '词条 {slot} 类型':'Substat {slot} type', '词条 {slot} 数值':'Substat {slot} value',
  '尚未揭示':'Not yet tuned', '选择词条':'Select Substat', '当前评分':'Current score',
  '输入已揭示的词条与实际数值':'Enter revealed Substats and their values',
  '已达到评分目标':'Target reached', '距目标 {score} 分':'{score} points below target', '请检查评分配置':'Check the scoring configuration',
  '开启 OCR 自动填入 · UDP 9999':'Enable OCR autofill · UDP 9999', 'OCR 已开启 · 点击停止':'OCR enabled · Stop',
  '比较重抽候选':'Compare a reroll result', '接受还是保留？':'Accept or keep?',
  '填入重抽得到的完整五条词条及数值，自动比较接受新结果还是保留原结果。':'Enter all five rerolled Substats and their values to automatically compare accepting the new result with keeping the original.',
  '转到重抽策略':'Go to Reroll policy',
  '先求解策略，再填入重抽候选进行比较。':'Solve the policy, then enter a reroll candidate to compare.',
  '填入完整的五条重抽候选，查看接受或保留建议。':'Enter all five candidate Substats to see whether to accept or keep.',
  '接受候选':'Accept candidate', '接受候选并继续重抽':'Accept candidate and continue rerolling',
  '清空候选':'Clear candidate', '从当前声骸复制':'Copy current Echo',
  '策略建议':'Recommended action', '策略结果':'Policy results', '等待计算':'Not calculated', '尚未计算':'Not calculated',
  '达标成品的期望消耗':'Expected resources per successful Echo', '当前声骸的重抽期望':'Expected reroll cost for this Echo',
  '到达目标所需频整器':'Transducers to target', '当前声骸评分':'Current Echo score',
  '新胚子达标概率':'Success probability for a new Echo', '加权资源期望':'Weighted expected resources',
  '个':'', '管':'', '分':'pts',
  '锁定方案':'Lock options', '候选比较':'Candidate comparison',
  '按频整器期望消耗从低到高，最多显示 5 种锁定方案。期望包含本次及后续重抽；概率为下一次重抽立即达标。':'Shows up to 5 lock options, ordered by expected Transducer cost from lowest to highest. Costs include this roll and future rerolls; probability is the chance of reaching the target on the next roll.',
  '使用说明':'Instructions',  '关闭 ×':'Close ×', '关闭说明':'Close instructions',
  "选择角色预设和变体，填写目标总分；按需调整词条权重和资源相对价值。":"Choose a Resonator preset and variant, then enter the target score. Adjust Stat weights and relative resource values as needed.",
  "填入当前声骸已揭示的词条及数值；新胚子留空。点击「求解策略」。":"Enter the revealed Substats and their values, or leave them empty for a new Echo. Click “Solve policy”.",
  "每次强化后更新词条，建议会立即刷新。建议放弃时，换一个胚子并点击「清空」；建议保留时，留下当前声骸。":"Update the Substats after each upgrade to refresh the advice immediately. If advised to discard, start a new Echo and click “Clear”. If advised to keep, keep the current Echo.",
  "修改评分、目标或资源相对价值后，重新点击「求解策略」。":"After changing scoring, the target, or relative resource values, click “Solve policy” again.",
  "选择角色预设和变体，填写目标总分，并填入当前声骸完整的五条副词条及数值。点击「求解策略」。":"Choose a Resonator preset and variant, enter the target score and all five current Echo Substats with their values, then click “Solve policy”.",
  "按建议锁定词条，在游戏中使用频整器重抽，再将完整的五条结果填入重抽候选。候选比较会自动更新。":"Apply the suggested locks and use Transducers to reroll in the game. Enter all five resulting Substats as the candidate; the comparison updates automatically.",
  "决定接受时，点击「接受候选并继续重抽」；已达标时按钮显示「接受候选」。候选会成为当前声骸，下一步建议自动更新。":"To accept, click “Accept candidate and continue rerolling”, or “Accept candidate” if the target is met. The candidate becomes the current Echo and the next recommendation updates automatically.",
  "决定保留原结果时，点击「清空候选」，继续按当前声骸的建议重抽。达标后停止重抽。":"To keep the original result, click “Clear candidate” and reroll again using the current Echo’s advice. Stop rerolling once the target is met.",
  "修改当前声骸或候选会自动更新建议；修改评分或目标后，重新点击「求解策略」。":"Editing the current Echo or candidate updates the advice automatically. After changing scoring or the target, click “Solve policy” again.",
  "选择角色预设和变体，填写目标总分，设置资源相对价值，包括频整器。":"Choose a Resonator preset and variant, enter the target score, and set relative resource values, including Transducers.",
  "按建议继续强化，每次强化后更新词条。建议放弃时，换一个胚子并点击「清空」；建议保留时，留下当前声骸。":"Follow the advice to upgrade and update the Substats after each upgrade. If advised to discard, start a new Echo and click “Clear”. If advised to keep, keep the current Echo.",
  "建议重抽时，点击「转到重抽策略」。当前声骸、评分和目标会自动带入，接着按重抽模式的步骤操作。":"When rerolling is recommended, click “Go to Reroll policy”. The current Echo, scoring and target carry over automatically. Continue with the Reroll policy steps.",
  '本地存储不可用或已满，本次配置仍可计算；请导出预设备份。':'Local storage is unavailable or full. Calculations still work; export your presets to back them up.',
  '请填写有效数值，检查目标、评分参数和资源权重。':'Enter valid numbers for the target, scoring parameters, and resource weights.',
  '待更新':'Out of date', '输入已更新':'Inputs changed',
  '正在更新建议。下方数值为上次结果，仅供参考。':'Updating the recommendation. The resource values below belong to the previous calculation.',
  '计算完成':'Calculated',
  '继续强化':'Continue upgrading', '放弃，换下一个':'Discard and start over', '保留这个声骸':'Keep this Echo', '重抽':'Reroll',
  '当前声骸保留概率':'Keep probability for this Echo',
  '新胚子保留概率':'Keep probability for a new Echo',
  '当前声骸达标概率':'Success probability for this Echo',
  '精确求解':'Exact solve', '锁定 {names}':'Lock {names}', '不锁定任何词条':'Lock no Substats',
  '本次 {cost} 个 · 单次达标 {p} · 额外期望 +{regret}':'This roll: {cost} · Success: {p} · Extra expected cost: +{regret}',
  '接受新结果 ↗':'Accept result ↗', '保留原结果 ↶':'Keep original ↶',
  '后续频整器：{from} → {to}':'Future Transducers: {from} → {to}', '候选评分：{score}':'Candidate score: {score}',
  '按后续重抽成本比较，相同成本可接受。请确认候选来自实际重抽并保留了锁定词条。':'Compares future reroll costs; equal costs can be accepted. Enter the actual result with locked Substats preserved.',
  '重抽模式需要完整的五条词条。请填写全部词条与实际数值。':'Reroll mode requires all five Substats and their values.',
  '正在求解…':'Solving…', '计算中':'Solving', '正在精确求解 · {seconds} 秒':'Solving exactly · {seconds} s',
  '未完成':'Failed', '计算失败':'Calculation failed', '本次未能得到有效策略，请查看页面上方的具体原因。':'No valid policy was produced. See the error above.',
  '请先输入自定义预设名称。':'Enter a name for the custom preset.',
  '预设文件不能超过 2 MB。':'Preset files must be no larger than 2 MB.', '无效的预设文件。':'Invalid preset file.',
  '预设格式无效。旧版预设需先选择对应评分体系再导入。':'Invalid preset format. Select the matching scoring system before importing a legacy preset.',
  '导入失败：{error}':'Import failed: {error}', 'OCR 事件连接失败：{error}':'OCR event connection failed: {error}',
  '初始化失败：{error}。请检查服务后刷新页面。':'Initialization failed: {error}. Check the service and reload.',
  '服务暂不可用（HTTP {status}）':'Service unavailable (HTTP {status})',
  '请求失败，请检查输入或重试。':'Request failed. Check your input or try again.',
  '网络连接失败，请检查求解服务。':'Connection failed. Check the solver service.',
  '求解器正在处理另一项计算，请稍后重试。':'The solver is busy. Try again shortly.',
  '精确状态空间超过服务限制。请减少非零评分词条，或使用固定评分；服务管理员也可调整规模限制。':'The exact state space exceeds the service limit. Reduce nonzero stat weights, use fixed scoring, or ask the administrator to adjust the limits.',
  '当前评分规则无法达到这个目标，请降低目标分数。':'This target cannot be reached with the current scoring rules. Lower the target.',
  '未能完成求解，请检查配置。':'The solver could not finish. Check the configuration.',
  '计算结果超出有限数值范围，请调整目标或权重。':'The result is outside the finite numerical range. Adjust the target or weights.',
  '评分参数必须是有限非负数。':'Scoring parameters must be finite and nonnegative.',
  '目标分数必须非负，且内部副词条目标不能超过 65535。':'The target must be nonnegative and the internal Substat target must not exceed 65535.',
  '固定评分的目标必须为整数。':'Fixed scoring requires an integer target.',
  '固定评分权重必须是 0～65535 的整数。':'Fixed weights must be integers from 0 to 65535.',
  '应用支持最多 20000 内部副词条分数，请降低归一化满分或固定权重。':'The app supports up to 20000 internal Substat points. Reduce the normalized maximum or fixed weights.',
  '重抽需要完整的 5 条词条；强化状态允许 0～5 条。':'Rerolling requires five Substats. Upgrade states accept zero to five.',
  '词条类型或数值档位无效。':'Invalid Substat type or roll value.', '同一声骸不能包含重复词条。':'An Echo cannot have duplicate Substat types.',
  '仅重抽模式支持候选比较。':'Candidate comparison is only available in Reroll mode.',
  '资源权重必须是有限非负数。':'Resource weights must be finite and nonnegative.',
  '求解服务需要重启。':'The solver service must be restarted.', '求解任务异常结束。':'The solver task failed unexpectedly.',
};
function lookup(table,key) { return Object.hasOwn(table,key) ? table[key] : undefined; }
export function t(key, values = {}) {
  const template = language === 'en' ? (lookup(messages,key) ?? lookup(dataDescriptions,key) ?? key) : key;
  return template.replace(/\{(\w+)\}/g, (_, k) => String(values[k] ?? `{${k}}`));
}
export function setLanguage(value) {
  language = value === 'en' ? 'en' : 'zh-CN';
  try { localStorage.setItem('echo-studio-language',language); } catch { /* Nonessential preference. */ }
}
export function translatePage() {
  document.documentElement.lang = language;
  document.querySelectorAll('[data-i18n]').forEach(e => { e.textContent = t(e.dataset.i18n); });
  document.querySelectorAll('[data-i18n-placeholder]').forEach(e => { e.placeholder = t(e.dataset.i18nPlaceholder); });
  document.querySelectorAll('[data-i18n-aria]').forEach(e => { e.setAttribute('aria-label',t(e.dataset.i18nAria)); });
}
export const statNames = ['Crit. Rate','Crit. DMG','ATK%','DEF%','HP%','ATK','DEF','HP','Energy Regen','Basic Attack DMG Bonus','Heavy Attack DMG Bonus','Resonance Skill DMG Bonus','Resonance Liberation DMG Bonus'];
export let characterNames = {};
let variantNames = {}, dataDescriptions = {};
export function setDataTranslations(data) {
  characterNames = Object.fromEntries(Object.entries(data.characters).map(([name,info]) => [name,info.en]));
  variantNames = data.variant_names;
  dataDescriptions = data.descriptions;
}
export function characterName(name) { return language === 'en' ? (lookup(characterNames,name) ?? name) : name; }
export function variantName(name) { return language === 'en' ? (lookup(variantNames,name) ?? name) : name; }
export function errorText(error) {
  if (error?.retryAfter) return `${errorText({message:error.message})} ${t('请在 {seconds} 秒后重试。', {seconds:error.retryAfter})}`;
  const message = error?.message || String(error);
  if (language !== 'en') return message;
  if (Object.hasOwn(messages,message)) return t(message);
  const prefix = message.split('（')[0];
  if (Object.hasOwn(messages,prefix)) return t(prefix) + (message.includes('（') ? ` (${message.split('（').slice(1).join('（').replace(/）$/, '')})` : '');
  if (/\p{Script=Han}/u.test(message)) return t('请求失败，请检查输入或重试。');
  return message;
}
