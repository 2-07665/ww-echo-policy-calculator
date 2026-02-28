# 鸣潮声骸强化策略计算器

本项目使用动态规划计算《鸣潮》声骸强化与使用频整器重抽词条的最优决策。

仓库结构：
- `crates/echo_policy/`：核心求解器 crate（可直接命令行运行示例）。由人工编写与维护。
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

### 3. OCR 集成（Windows Only）

- OCR 方案依赖 `ok-wuthering-waves` 项目：
  https://github.com/ok-oldking/ok-wuthering-waves
- 使用方式：将 `apps/ocr/add_ocr_task.ps1` 与 `EchoOCRTask.py` 放在同一目录后运行脚本，脚本会将任务注入到本地 `ok-ww` 的任务配置中。

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
- WutheringWavesUID（GPL 许可来源，仅用于转换角色权重数据）: https://github.com/Cccc-owo/WutheringWavesUID

### 讨论与交流
- Bilibili **[@冰封_](https://space.bilibili.com/88548986)**
