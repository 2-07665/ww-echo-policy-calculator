# ⚠️ Archived Project

This project is no longer maintained and is kept for reference only.  

# 鸣潮声骸强化策略计算器

本项目使用动态规划计算《鸣潮》声骸强化与使用频整器重抽词条的最优决策。

仓库结构：
- `crates/echo_policy/`：核心求解器 crate（可直接命令行运行示例）。核心库与主 CLI 由人工编写与维护。
- `crates/echo_policy/src/bin/target_score_sweep.rs`：用于批量扫描 `target_score` 并输出 Mathematica / Wolfram Language 数据的辅助 CLI。该入口由 Codex 管理与维护。
- `apps/desktop/`：桌面应用（Tauri 2，前端资源已内置，无需 Node/npm）。当前由 Codex 全量管理与维护。

## 环境要求

- Rust（Edition 2024）
- 桌面应用需要安装 Tauri 系统依赖（按操作系统）：[Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)

## 使用方法

### 1. 运行核心求解器示例（CLI）

```bash
cargo run --release --manifest-path crates/echo_policy/Cargo.toml --bin cli
```

### 2. 运行桌面应用（Tauri）

```bash
cargo run --release --manifest-path apps/desktop/src-tauri/Cargo.toml
```

### 3. 批量扫描 `target_score`（Codex-managed helper）

该入口用于固定评分器和成本模型后，批量扫描一组 `target_score`，输出适合 Mathematica / Wolfram Language 读取的结果。

说明：
- 该工具是辅助脚本式入口，由 Codex 管理与维护。
- 示例配置见 [`crates/echo_policy/examples/target_score_sweep.json`](crates/echo_policy/examples/target_score_sweep.json)。

```bash
cargo run --release --manifest-path crates/echo_policy/Cargo.toml --bin target_score_sweep -- \
  crates/echo_policy/examples/target_score_sweep.json \
  crates/echo_policy/examples/output.wl
```

### 4. OCR 集成（Windows Only）

- OCR 方案依赖 `ok-wuthering-waves` 项目：
  https://github.com/ok-oldking/ok-wuthering-waves
- 使用方式：双击运行 `将OCR任务加入ok-ww.cmd`。启动 `ok-ww` 后运行 `Echo OCR` 任务。

## 构建

先安装 Tauri CLI：

```bash
cargo install tauri-cli --version "^2.0.0"
```

再执行：

```bash
cd apps/desktop/src-tauri
cargo tauri build
```

## 致谢（Acknowledgements）

### 赞助
特别感谢 Bilibili **[@冬葳蕤](https://space.bilibili.com/58999432)** 自发组织的库洛游戏二创激励计划为本项目提供的赞助。

### 词条统计数据来源
本项目使用的副词条产出统计数据来自：
- Bilibili **[@IceHe何瀚清](https://space.bilibili.com/13378662)**

### 内置评分预设来源
桌面应用内置的评分预设转换来源如下：
- Wuwa Echo Tool: https://github.com/2-07665/WuwaEchoTool
- 漂泊者强化助手（微信小程序）: `#小程序://漂泊者强化助手/FGd22Ty9ssvPcRy`
- WutheringWavesUID（GPL 许可来源）: https://github.com/Cccc-owo/WutheringWavesUID

### 讨论与交流
- Bilibili **[@冰封_](https://space.bilibili.com/88548986)**
