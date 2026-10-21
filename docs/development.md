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

默认配置下 `bundle.active = false`，只生成可执行文件。macOS 例外：Tauri 会自动合并 `tauri.macos.conf.json`，直接生成 `.app`。不要新增 `tauri.windows.conf.json` 或 `tauri.linux.conf.json`，它们同样会被自动合并，改变本地构建结果。

需要安装包时，叠加 CI 使用的 [tauri.release.conf.json](../apps/desktop/src-tauri/tauri.release.conf.json)（只开启 bundle），再用 `--bundles` 选择格式：

```bash
cargo tauri build --config tauri.release.conf.json --bundles app,dmg   # macOS
cargo tauri build --config tauri.release.conf.json --bundles nsis      # Windows
cargo tauri build --config tauri.release.conf.json --bundles deb,appimage  # Linux
```

产物在仓库根目录的 `target/release/bundle/` 下；指定 `--target` 时在 `target/<目标三元组>/release/bundle/` 下。

## CI 与发布

[desktop-release.yml](../.github/workflows/desktop-release.yml) 负责桌面版的检查和发布。发布时在以下平台用 release 模式打包：

| 平台 | 产物 |
|---|---|
| Windows x64 | NSIS 安装程序、免安装的 `.exe` |
| macOS Apple Silicon | `.dmg` |
| macOS Intel | `.dmg`（在 Apple Silicon runner 上交叉编译） |
| Linux x64 | `.deb`、`.AppImage`（在 Ubuntu 22.04 上构建，兼容较旧的发行版） |

- **PR 和推送到 `main`**：在 Windows、macOS、Linux 上运行 `cargo check -p echo_calculator_app --all-targets`，不打包。只改文档或部署文件时不触发。
- **在 Actions 页面手动运行**：除检查外，还会打包全部平台，产物保存在该次运行的 Artifacts 中，保留 14 天，不发布 Release。适合在打 tag 前确认打包正常。
- **推送 `v*` tag**：构建全部平台，创建同名 GitHub Release，上传安装包和 `SHA256SUMS.txt`。tag 必须与 `tauri.conf.json` 中的版本一致，否则直接失败；带 `-` 的 tag（如 `v3.1.0-beta.1`）发布为预发布版本。

发布新版本：

1. 同步修改 `apps/desktop/src-tauri/tauri.conf.json` 和 `apps/desktop/src-tauri/Cargo.toml` 中的版本号，提交并推送。
2. 打 tag 并推送：

   ```bash
   git tag v3.0.1
   git push origin v3.0.1
   ```

3. 等待工作流完成，在 Releases 页面检查产物，按需编辑说明。

打包直接调用 `cargo tauri build`，与本地打包命令一致，不需要 Node；tauri-cli 优先下载预编译版本，Apple Silicon 上没有预编译版本，会从源码安装。发布用 runner 自带的 `gh` 完成。没有使用 `tauri-apps/tauri-action`，因为它主要面向 Node 前端项目，并按自己的规则命名和上传产物；直接调用命令更容易在本地复现。

### 签名与公证

默认不签名：macOS 版使用 ad-hoc 签名，Windows 版没有签名。用户首次打开时会看到系统警告：

- **macOS**：提示“已损坏”或“无法验证开发者”时，把应用拖进“应用程序”后运行 `xattr -cr "/Applications/Echo Studio.app"`，或在“系统设置 → 隐私与安全性”中选择“仍要打开”。
- **Windows**：SmartScreen 提示时，点击“更多信息 → 仍要运行”。

有证书后，在仓库的 Settings → Secrets and variables → Actions 中添加以下 secrets，工作流会自动使用；未配置的 secrets 会被跳过。

| Secret | 说明 |
|---|---|
| `APPLE_CERTIFICATE` | Developer ID Application 证书导出的 `.p12`，Base64 编码 |
| `APPLE_CERTIFICATE_PASSWORD` | 导出 `.p12` 时设置的密码 |
| `APPLE_SIGNING_IDENTITY` | 签名身份，如 `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID`、`APPLE_PASSWORD`、`APPLE_TEAM_ID` | 公证用的 Apple ID、App 专用密码和团队 ID；三项都配置时才会公证 |
| `WINDOWS_CERTIFICATE` | Windows 代码签名证书 `.pfx`，Base64 编码 |
| `WINDOWS_CERTIFICATE_PASSWORD` | `.pfx` 的密码 |

Base64 编码可以用 `base64 -i certificate.p12 | pbcopy`（macOS）或 `[Convert]::ToBase64String([IO.File]::ReadAllBytes("certificate.pfx"))`（PowerShell）。Windows 签名使用 DigiCert 的时间戳服务；EV 证书或云端签名服务无法导出 `.pfx`，需要另行修改工作流。
