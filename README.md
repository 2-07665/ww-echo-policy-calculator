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

仓库提供 Docker Compose 与不依赖 Docker 的单二进制部署。页面资源嵌入二进制，角色和权重可从外部数据包加载；运行时不需数据库或 Node。

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
sudo install -d /opt/echo-studio/data
sudo install -m 644 data/echo-data.json /opt/echo-studio/data/echo-data.json
sudo cp deploy/echo-studio.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now echo-studio
```

然后使用同一份 Caddy 配置。macOS 编译的二进制不能直接复制到 Linux；Docker 构建应在 VPS 或相应 Linux 构建环境运行。

### 角色和权重更新

角色名称、英文翻译、实装日期及所有预设权重放在独立的 [data/echo-data.json](data/echo-data.json) 中。完成这次应用升级后，后续普通数据更新无需重新编译：

- 网页端通过 `ECHO_DATA_FILE` 指定数据包路径。校验并原子替换文件后，用户刷新页面或点击“角色数据 → 更新数据”即可加载，无须重启服务。Compose 和 systemd 示例已配置该路径。
- 桌面端在“角色数据”中在线更新或导入 JSON，校验成功后缓存到本地，离线启动仍可使用。
- 无效更新保留原数据；更新不会修改当前计算配置，重新选择预设才应用新权重。

数据格式、发布步骤和故障回退见 [数据包维护说明](data/README.md)。

### 多人访问、容量与配额

服务器按完整配置共享策略，当前声骸和候选仍由每次请求独立传入。相同配置同时请求只构建一次；冷求解串行，缓存查询有独立的两个工作名额。后台任务不依赖 HTTP 连接，断开浏览器不会提前释放计算名额。

`POST /api/solve` 验证后返回 HTTP 202 和 `{id,status}`；浏览器自动轮询 `GET /api/jobs/{id}`，状态依次为 `queued` / `building` / `querying` / `done` 或 `failed`。结果只向提交者 IP 返回，最多保留 60 秒；任务表满时可提前淘汰已完成结果。网页显示排队状态，桌面 IPC 不受影响。

| 环境变量 | 默认 | 含义 |
| --- | --- | --- |
| `ECHO_BIND` | `127.0.0.1:3000` | HTTP 监听地址；容器内为 `0.0.0.0:3000` |
| `ECHO_CACHE_MIB` | `512` | 策略缓存容量，按实际分配容量加 25% 余量计入 |
| `ECHO_CACHE_ENTRIES` | `32` | 缓存条目上限；空闲条目按 LRU 淘汰，30 分钟无访问后可清理 |
| `ECHO_MAX_PENDING_BUILDS` | `8` | 正在构建及排队的不同策略总数 |
| `ECHO_MAX_JOBS` | `64` | 活跃任务与暂存结果的总上限 |
| `ECHO_MAX_JOBS_PER_IP` | `2` | 每个普通访问 IP 的未完成任务上限（IPv6 按 /64 合并） |
| `ECHO_REQUESTS_PER_MINUTE` | `240` | 每 IP 每分钟的预览与求解请求总量；状态轮询不扣额度 |
| `ECHO_QUOTA_UPGRADE` | `60` | 每 IP 每小时新建强化策略的次数 |
| `ECHO_QUOTA_REROLL` | `12` | 每 IP 每小时新建重抽策略的次数 |
| `ECHO_QUOTA_JOINT` | `4` | 每 IP 每小时新建联合策略的次数 |
| `ECHO_MAX_IDENTITIES` | `4096` | 配额记录上限；满时拒绝新身份，闲置一小时后清理 |
| `ECHO_MAX_STATES` | `750000` | 重抽／联合状态上限 |
| `ECHO_MAX_TRANSITIONS` | `24000000` | 重抽／联合转移上限 |
| `ECHO_TRUSTED_PROXIES` | 空 | 可提供真实客户端 IP 的代理地址，逗号分隔；默认忽略转发头 |
| `ECHO_ADMIN_KEY_HASHES` | 空 | 允许绕过个人配额的密钥 SHA-256 摘要，逗号分隔 |

小时额度只在需要新建策略、且成功进入队列时扣除。缓存命中、加入同配置的正在进行的求解、修改当前词条和候选均不重复扣小时额度，但仍受请求频率限制。已入队的求解即使失败或用户断开也计费；未通过输入验证及队列满的请求不扣求解额度。某模式额度设为 `0` 时，普通访问不能新建该模式策略，仍可查询共享缓存。限额返回 HTTP 429、错误码和 `Retry-After`，网页显示等待时间。

配额为单进程内存状态，服务重启后重置；同一出口 IPv4 共享额度，IPv6 按 /64 共享额度。不要部署多个独立实例来规避这些限制；多实例部署需另加共享配额存储。缓存不保存用户声骸，任务的声骸输入与结果不写磁盘。

Compose / systemd 仍设置 **2 GiB 硬内存上限**。512 MiB 是保留策略的预算，不是进程 RSS 上限；临时构建缓冲区、查询与分配器另需空间。正在使用的策略不会被淘汰；新策略放不下会明确报错。状态规模检查与一次只构建一份策略共同限制峰值，硬上限负责兜底；提高状态限制或缓存预算前需重新测量。参考 [VPS 测量记录](docs/server-performance.md)。

### 管理员访问密钥

在服务器运行以下命令生成随机密钥与对应摘要（不会自动更改配置）：

```sh
./echo_server generate-key
# Docker:
docker compose run --rm echo echo-server generate-key
```

保存输出中的访问密钥，将摘要配置为 `ECHO_ADMIN_KEY_HASHES`。Compose 可在本地 `.env` 中设置；systemd 可写入 `/etc/echo-studio.env`。配置文件只保存摘要，**不要填原始密钥**。重启服务后，在网页右上角「访问密钥」中验证并使用。密钥只放在当前标签页的 sessionStorage，通过 Authorization 请求头发送；不进入 URL、普通配置导出或服务日志。公开访问使用 HTTPS。

有效密钥绕过每 IP 请求频率、小时求解额度和个人未完成任务数限制；仍遵守全局队列、任务数量、缓存预算、状态规模和单次冷构建约束。可配置多个摘要；删除某个摘要并重启即可撤销该密钥。网页「清除密钥」恢复普通配额。

反向代理必须正确识别 IP：systemd 示例只信任本机 Caddy；Compose 示例只信任其固定网桥网关 `172.30.77.1`，并仅将端口发布到宿主机 loopback。更换代理或 Docker 网络时同步修改信任地址。转发链从右向左处理，遇到首个不受信任地址即停止，客户端伪造的前缀无法获得新额度。不要把公网客户端网段加入信任名单。

多人服务的独立 HTTP 验证：`python3 scripts/test-server-http.py target/release/echo_server`。浏览器密钥流程：`node scripts/test-access-key.mjs`（需 Playwright，与下方浏览器测试使用相同环境配置）。两者均自动生成临时测试密钥，不输出密钥，不修改现有服务。

## 使用与迁移

1. 选择求解模式和评分体系。预设下拉只显示当前评分体系的角色，按首次实装时间倒序排列；旁边的变体下拉显示该角色的命座、主词条或模式。上方全局搜索覆盖包含 578 个变体的内置角色预设及自定义预设，输入中英文角色名即可直接点击结果；跨体系选择会自动切换评分体系。保留线性、固定、Wuwa Echo Tool、漂泊者强化助手及 XutheringWavesUID 五套评分方式。
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
- 强化 / 联合资源卡片从新胚子起算，包含失败尝试、分解返还和成品升满的修正。经验以每个特级密音筒 5000 原始经验计；+5/+10/+15/+20/+25 的累计消耗为 4500/16500/40000/79500/143000，回收返还为 3000/12000/30000/59500/106500，按实测表固定计算，不再设置返还比例。
- 重抽资源卡片从当前五词条声骸起算，仅表示未来频整器消耗。锁法概率为下一次立即达标概率。
- 联合模式的「保留概率」指按策略执行而不放弃当前声骸的概率，包含转入重抽后最终达标的情况。是否值得开始重抽由联合求解器判断，具体锁法与候选比较交由重抽模式。
- 资源权重代表相对价值，不是库存上限。精确求解超限会报错，不隐式近似。应用额外限制最高五条内部评分合计 ≤ 20000，以约束强化 DP 的评分维度。

数学模型、误差与全部 Rust API 见 [echo_policy 文档](crates/echo_policy/README.md)。

## 架构与验证

```text
apps/web/                  共享响应式界面（HTML / CSS / JS）
  api.js                   Tauri IPC / 同源 HTTP 传输选择
data/                      独立发布包、角色译名和实装日期
crates/echo_app/            DTO、数据包校验/缓存、评分转换、三求解器适配
crates/echo_policy/         数学求解核心与固定资源成本表
apps/server/               Axum HTTP 服务、嵌入静态页面、计算准入限制
apps/desktop/src-tauri/     Tauri 桌面外壳、后台求解、OCR
```

API：`GET /api/health`、`GET /api/bootstrap`、`GET /api/data-pack`、`POST /api/preview`、`POST /api/solve`。请求结构见 `echo_app::SolveRequest`；每次传完整 `config`、`echo: [[类型索引, 原始数值], ...]`，重抽模式可附带 `candidate`。错误格式为 `{code, message}`。预设编辑、页面配置保存均在客户端完成。

```bash
cargo test -p echo_app -p echo_server -p echo_policy
cargo check -p echo_calculator_app
cargo test -p echo_calculator_app
bash scripts/check-ui-node.sh
node scripts/test-i18n.mjs
python3 scripts/test-data-pack.py
node scripts/test-presets.mjs
python3 scripts/convert_weights/test_xwuid_presets.py
cargo fmt --all -- --check
# 可选真实浏览器测试，需 Playwright + Chromium，另一个终端先启动 release 服务
node scripts/test-upgrade-workflow.mjs
node scripts/test-reroll-workflow.mjs
node scripts/test-joint-workflow.mjs
node scripts/test-ui.mjs
node scripts/test-data-updates.mjs
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
