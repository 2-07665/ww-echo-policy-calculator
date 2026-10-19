# Echo Studio · 声骸策略工坊

基于 `echo_policy` 的《鸣潮》声骸策略应用。三个求解器共享评分规则，支持实际数值档位与固定词条评分：

| 模式 | 用途 | 输出 |
| --- | --- | --- |
| 强化 Upgrade | 从胚子逐条揭示，不使用频整器 | 继续 / 放弃、当前声骸达标概率、成品期望资源 |
| 重抽 Reroll | 对已有五词条声骸重抽 | 数值相关的锁法、频整器期望、单次成功率、候选接受建议 |
| 联合 Joint | 同时权衡强化、重抽与重开 | 强化 / 放弃 / 保留 / 重抽、四种资源的成品期望 |

## 运行

需要 Rust 1.96+。网页与桌面使用同一份原生 ES modules / CSS 界面，无需 Node、npm 或前端构建。Node 仅用于可选的前端检查。

**网页版**（不需要 Tauri 系统依赖）：

```bash
cargo run --release -p echo_server
# 浏览器访问 http://127.0.0.1:3000
```

**桌面版**（安装 [Tauri 系统依赖](https://v2.tauri.app/start/prerequisites/)）：

```bash
cargo run --release -p echo_calculator_app
```

桌面离线运行，通过 IPC 调用本地 Rust 求解器。网页版在服务端计算，浏览器不承担大规模状态表内存开销。使用 release 构建以获得实际求解性能。

打包桌面：

```bash
cargo install tauri-cli --version '^2.0.0' --locked
cd apps/desktop/src-tauri
cargo tauri build
# macOS 打包 .app：cargo tauri build --config tauri.macos.conf.json
```

默认配置保留 `bundle.active=false`；通用构建生成可执行文件。要生成其他平台安装包，在目标平台启用 bundle 并设置对应 targets。

## VPS 部署

仓库提供 Docker Compose 与不依赖 Docker 的单二进制部署。页面资源嵌入二进制，运行时不需额外静态文件、数据库或 Node。

### Docker Compose

将仓库复制到 VPS，执行：

```bash
docker compose up -d --build
docker compose logs -f
curl http://127.0.0.1:3000/api/health
```

默认只映射 VPS 的 `127.0.0.1:3000`。在宿主机运行 Caddy，将 [deploy/Caddyfile](deploy/Caddyfile) 中的 `echo.example.com` 替换为自己的域名；DNS 指向 VPS，开放 80/443，Caddy 提供 HTTPS 并反向代理。示例适用于独占域名根路径，不支持直接挂在 URL 子路径。

不使用域名时，可用 SSH 转发在本机访问：

```bash
ssh -L 3000:127.0.0.1:3000 user@your-vps
```

### 单二进制 + systemd

在目标 Linux 架构编译 `cargo build --release --locked -p echo_server`，将 `target/release/echo_server` 放到 `/opt/echo-studio/echo_server`。安装 [deploy/echo-studio.service](deploy/echo-studio.service) 后启动服务：

```bash
sudo cp deploy/echo-studio.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now echo-studio
```

然后使用同一份 Caddy 配置。macOS 编译的二进制不能直接复制到 Linux；Docker 构建应在 VPS 或相应 Linux 构建环境运行。

### 服务容量

- `ECHO_BIND`：默认 `127.0.0.1:3000`，容器内为 `0.0.0.0:3000`。
- `ECHO_MAX_STATES`：默认 `750000`；`ECHO_MAX_TRANSITIONS`：默认 `24000000`。这些是应用默认限制，低于核心库默认值；桌面使用同一默认限制。
- 每个服务进程同时只运行一次求解，繁忙时返回 HTTP 429，不无限排队。客户端断开连接不会中断正在运行的 Rust 计算。
- 缓存最近一份完整配置的策略。每次请求都携带完整配置，用户之间不会误用彼此的会话；交替使用不同配置会导致重算。
- Compose / systemd 示例限制 2 GiB 内存。规模限制不是严格字节上限；提高限制需同时评估机器内存，资源耗尽可能导致进程重启。
- 这是低并发计算服务，没有账户系统。需要限制访问时，可在反向代理配置认证或请求频率限制。不会把用户配置写到服务器磁盘。

## 使用与迁移

1. 选择求解模式和评分体系。预设下拉只显示当前评分体系的角色，按首次实装时间倒序排列；旁边的变体下拉显示该角色的命座、主词条或模式。上方全局搜索覆盖包含 467 个变体的内置角色预设及自定义预设，输入中英文角色名即可直接点击结果；跨体系选择会自动切换评分体系。保留线性、固定、Wuwa Echo Tool、漂泊者强化助手及 XutheringWavesUID 五套评分方式。
2. 设定目标总分，必要时展开词条权重和资源相对价值。
3. 填写已揭示词条，数值下拉框使用游戏实际档位。强化 / 联合允许 0～5 条，重抽要求完整五条。
4. 点击「求解策略」确认当前配置。之后修改当前词条会立即更新建议，清空后可继续处理新胚子；修改评分、目标或资源价值后，需要重新点击求解。
5. 重抽模式默认显示候选比较。填入完整候选后自动比较后续频整器期望；点击接受候选可将其设为当前声骸并更新下一步锁法，保留原结果时清空候选继续重抽。
6. 联合模式以强化为主，不显示重抽候选或锁法。建议重抽时，点击「转到重抽策略」，自动带入当前声骸和评分目标并计算重抽建议。

右上角使用中文 / EN 按钮切换语言，语言偏好会保存在本地。切换时保留当前配置、词条和计算结果。游戏术语与角色名按 Wiki 核对，详见 [术语来源与预设清单](apps/web/TERMINOLOGY.md)。

XutheringWavesUID 权重已按指定仓库更新，包含 61 组、278 个变体；新版数据替换原 QQ Bot 内置数据。角色排序来源见 [实装时间说明](apps/web/CHARACTER_RELEASES.md)。来源提交、转换方法及评分差异见 [权重快照说明](scripts/convert_weights/sources/README.md)。

自定义预设与最近配置保存在当前浏览器 / 桌面的本地存储。可导出 v3 JSON 并在另一端导入。旧版桌面的 `scorer-presets/*.json` 不会被删除，也不会自动迁移；先选择对应评分体系，再通过“导入预设”读取原文件（支持 grouped 和 legacy 两种格式）。同名、同评分体系的导入预设会覆盖已有项。

桌面保留 ok-ww OCR：点击“开启 OCR 自动填入”，监听 `127.0.0.1:9999`。继续使用 `apps/ocr` 的原集成任务，数据格式为 `{"buffEntries":[{"buffName":"Crit_Rate","buffValue":63}]}`。浏览器不直接监听本机 UDP。

## 计算约定

- 目标输入使用显示总分，服务先扣除主词条显示分。线性评分的内部副词条分为显示副词条分 × 100；固定评分直接使用整数。三种求解器的目标转换集中在共享服务中。
- 强化 / 联合资源卡片从新胚子起算，包含失败尝试、分解返还和成品升满的修正。经验以每个特级密音筒 5000 原始经验计。
- 重抽资源卡片从当前五词条声骸起算，仅表示未来频整器消耗。锁法概率为下一次立即达标概率。
- 联合模式的「保留概率」指按策略执行而不放弃当前声骸的概率，包含转入重抽后最终达标的情况。是否值得开始重抽由联合求解器判断，具体锁法与候选比较交由重抽模式。
- 资源权重代表相对价值，不是库存上限。精确求解超限会报错，不隐式近似。应用额外限制最高五条内部评分合计 ≤ 20000，以约束强化 DP 的评分维度。

数学模型、误差与全部 Rust API 见 [echo_policy 文档](crates/echo_policy/README.md)。

## 架构与验证

```text
apps/web/                  共享响应式界面（HTML / CSS / JS）
  api.js                   Tauri IPC / 同源 HTTP 传输选择
crates/echo_app/            DTO、校验、评分转换、三求解器适配、缓存、预设
crates/echo_policy/         数学求解核心（本次重构未修改算法）
apps/server/               Axum HTTP 服务、嵌入静态页面、计算准入限制
apps/desktop/src-tauri/     Tauri 桌面外壳、后台求解、OCR
```

API：`GET /api/health`、`GET /api/bootstrap`、`POST /api/preview`、`POST /api/solve`。请求结构见 `echo_app::SolveRequest`；每次传完整 `config`、`echo: [[类型索引, 原始数值], ...]`，重抽模式可附带 `candidate`。错误格式为 `{code, message}`。预设编辑、页面配置保存均在客户端完成。

```bash
cargo test -p echo_app -p echo_server -p echo_policy
cargo check -p echo_calculator_app
cargo test -p echo_calculator_app
bash scripts/check-ui-node.sh
node scripts/test-i18n.mjs
node scripts/test-presets.mjs
python3 scripts/convert_weights/test_xwuid_presets.py
cargo fmt --all -- --check
# 可选真实浏览器测试，需 Playwright + Chromium，另一个终端先启动 release 服务
node scripts/test-upgrade-workflow.mjs
node scripts/test-reroll-workflow.mjs
node scripts/test-joint-workflow.mjs
node scripts/test-ui.mjs
```

浏览器测试应顺序执行，覆盖三模式、即时缓存查询、无闪烁刷新、候选接受、联合到重抽的衔接、过期请求、无效输入、自定义预设持久化和手机横向溢出。`NODE_BIN` 可覆盖语法检查的 Node 路径，`PLAYWRIGHT_MODULE` 可指定已有的 Playwright 模块路径。

## 致谢

### 赞助
特别感谢 Bilibili **[@冬葳蕤](https://space.bilibili.com/58999432)** 自发组织的库洛游戏二创激励计划为本项目提供的赞助。

### 词条统计数据来源
本项目曾使用 **[@IceHe何瀚清](https://space.bilibili.com/13378662)** 统计的副词条产出数据。

### 内置评分预设来源
桌面应用内置的评分预设转换来源如下：
- Wuwa Echo Tool: https://github.com/2-07665/WuwaEchoTool
- 漂泊者强化助手（微信小程序）: `#小程序://漂泊者强化助手/FGd22Ty9ssvPcRy`
- XutheringWavesUID 权重快照（GPL-3.0）：https://github.com/IceHe/okww-xwuid-echo-score
- 原始权重资源：https://cnb.cool/loping151/XutheringWavesUID-Resources
- 旧版 WutheringWavesUID 预设来源：https://github.com/Cccc-owo/WutheringWavesUID

### 讨论与交流
- Bilibili **[@冰封_](https://space.bilibili.com/88548986)**
