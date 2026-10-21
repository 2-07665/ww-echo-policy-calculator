# 开发说明

## 项目结构

求解算法由独立的 [echo_policy](https://github.com/2-07665/echo-policy) crate 实现，本仓库只负责界面、服务端和应用适配，通过 Git 依赖引用 echo_policy 的固定提交（见 [crates/echo_app/Cargo.toml](../crates/echo_app/Cargo.toml)）。

```text
crates/echo_app/          共享应用层：请求/响应结构、配置校验、评分换算、
                          调用三个求解器、数据包加载与缓存
apps/server/              网页版服务端（Axum），内嵌静态页面，负责排队与配额
apps/desktop/src-tauri/   桌面版外壳（Tauri），本地求解、数据更新、OCR
apps/web/                 网页版和桌面版共用的界面（原生 HTML / CSS / JS）
apps/ocr/                 ok-ww 的 OCR 集成任务
data/                     角色数据包及其源目录
scripts/                  数据生成、权重转换和测试脚本
```

网页版和桌面版共用同一套界面代码。[apps/web/api.js](../apps/web/api.js) 判断当前运行环境：在桌面版中通过 Tauri IPC 调用本地求解器，在浏览器中请求同源的 HTTP 接口。其余界面代码完全一致。

所有评分换算和求解调用都集中在 `echo_app` 中。服务端和桌面端只是适配层，**不要在其中另写评分或求解逻辑**。界面输入的目标是显示总分，`echo_app` 负责将其换算为 echo_policy 使用的内部分。

### 桌面端

- [main.rs](../apps/desktop/src-tauri/src/main.rs) 提供 `bootstrap`、`preview`、`solve` 三个命令，均直接转交给 `echo_app`。
- 求解在后台线程中进行，不会阻塞界面。求解器只能同时处理一个请求，此时再有调用会直接报错，而不是排队等待。
- [ocr.rs](../apps/desktop/src-tauri/src/ocr.rs) 在本机 UDP 9999 端口接收 OCR 消息。
- 产品标识符沿用旧版，因此 webview 的本地存储（自定义预设等）可以保留下来。
- `gen/schemas/` 由 Tauri 构建时自动生成，不纳入版本控制；需要维护的权限配置是 `capabilities/default.json`。

## HTTP 接口

| 接口 | 说明 |
|---|---|
| `GET /api/health` | 健康检查，返回版本号 |
| `GET /api/bootstrap` | 界面初始化数据：角色数据、默认配置、数据包加载状态 |
| `GET /api/data-pack` | 下载当前生效的数据包 |
| `POST /api/preview` | 计算当前声骸的评分，不求解策略 |
| `POST /api/solve` | 提交求解任务，返回 HTTP 202 与 `{id, status}` |
| `GET /api/jobs/{id}` | 查询任务状态：`queued` → `building` → `querying` → `done` 或 `failed` |
| `POST /api/auth` | 校验访问密钥 |

请求体结构见 `echo_app::SolveRequest`：每次都携带完整的 `config`，以及 `echo: [[类型索引, 原始数值], ...]`；重抽模式还可以附带 `candidate`。出错时返回 `{code, message}`。

任务结果只返回给提交者的 IP，最多保留 60 秒。后台计算不依赖 HTTP 连接，浏览器断开后计算仍会继续，占用的计算名额也不会提前释放。预设编辑和配置保存都在客户端完成，服务端不存储用户数据。

## 测试

日常检查：

```bash
cargo fmt --all -- --check
cargo test -p echo_app -p echo_server
cargo check -p echo_calculator_app
cargo test -p echo_calculator_app
bash scripts/check-ui-node.sh
node scripts/test-i18n.mjs
node scripts/test-presets.mjs
python3 scripts/test-data-pack.py
python3 scripts/convert_weights/test_xwuid_presets.py
python3 scripts/convert_weights/test_wuwa_echo_tool_presets.py
```

`scripts/check-ui-node.sh` 只检查 JS 语法；可以用 `NODE_BIN` 指定 Node 路径。

**浏览器端到端测试**需要 Playwright 和 Chromium。先在另一个终端启动 release 版服务，然后**依次**运行：

```bash
node scripts/test-upgrade-workflow.mjs
node scripts/test-reroll-workflow.mjs
node scripts/test-joint-workflow.mjs
node scripts/test-ui.mjs
node scripts/test-data-updates.mjs
```

默认连接 `http://127.0.0.1:3000`，可以用 `ECHO_URL` 修改地址，用 `PLAYWRIGHT_MODULE` 指定已安装的 Playwright 模块路径。

**多用户与访问密钥测试**会自行启动临时服务、生成临时密钥，不会影响正在运行的服务。需要先构建 release 版服务端：

```bash
cargo build --release -p echo_server
python3 scripts/test-server-http.py target/release/echo_server
node scripts/test-access-key.mjs
```

**性能测量**：`cargo run --release -p echo_app --example profile -- <模式> <有效权重数>`，例如 `-- joint 7`。结果记录见[服务器性能测量](server-performance.md)。

## 打包桌面版

```bash
cargo install tauri-cli --version '^2.0.0' --locked
cd apps/desktop/src-tauri
cargo tauri build
```

默认配置下 `bundle.active = false`，只生成可执行文件。在 macOS 上生成 `.app`：

```bash
cargo tauri build --config tauri.macos.conf.json
```

其他平台的安装包需要在对应平台上启用 bundle，并设置相应的 targets。
