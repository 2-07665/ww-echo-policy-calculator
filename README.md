# 鸣潮声骸强化策略计算器

本项目使用动态规划计算《鸣潮》声骸强化开词条与使用频整器重抽词条的最优策略，以便在强化前计算期望资源消耗，以及在强化时实时辅助决策。

仓库结构：
- `crates/echo_policy/`：核心求解器 crate（可直接命令行运行示例）。
- `crates/echo_policy/src/bin/target_score_sweep.rs`：用于批量扫描 `target_score` 并输出 Mathematica / Wolfram Language 数据的辅助 CLI。由 Codex 管理。
- `apps/desktop/`：桌面应用（Tauri 2，前端资源已内置，无需 Node/npm）。由 Codex 管理。

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

### 3. 批量扫描 `target_score`

该入口用于固定评分器和成本模型后，批量扫描一组 `target_score`，输出适合 Mathematica / Wolfram Language 读取的结果。

```bash
cargo run --release --manifest-path crates/echo_policy/Cargo.toml --bin target_score_sweep -- \
  crates/echo_policy/examples/target_score_sweep.json \
  crates/echo_policy/examples/output.wl
```

### 4. OCR 集成（Windows Only）

- OCR 方案依赖 `ok-wuthering-waves` 项目：
  https://github.com/ok-oldking/ok-wuthering-waves
- 使用方式：双击运行 `将OCR任务加入ok-ww.cmd`。启动 `ok-ww` 后运行 `Echo OCR` 任务。

### 5. 核心库：独立强化与联合求解

不考虑重抽时，继续使用 `UpgradePolicySolver`，其接口与求解方法保持不变。
考虑重抽时，使用 `JointPolicySolver`：它把频整器作为第四种加权资源，先求解重抽，再反向求解强化决策。
这些接口目前仅在核心库中提供，桌面应用尚未接入联合模式。

`RerollPolicySolver` 现在支持任意实现 `InternalScorer` 的非负、可加评分器。
原有的 `new(weights)` 和 mask 查询仍可用于按词条类型评分；数值评分使用
`with_scorer(&scorer)` 和带 `_for_echo` 的查询方法：

```rust
use echo_policy::{CostModel, JointPolicySolver, LinearScorer, RerollPolicySolver};

let weights = [1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0];
let scorer = LinearScorer::default(weights).unwrap();
let mut reroll = RerollPolicySolver::with_scorer(&scorer).unwrap();
reroll.set_target(8000).unwrap(); // 内部副词条分数：主词条为 0 时，8000 对应显示分 80
reroll.solve().unwrap();

// 词条序号与 scorer 一致；数值使用游戏原始档位单位，不能传入评分。
// 例如暴击率 6.3% 为 63，暴击伤害 12.6% 为 126。
let echo = [(0, 63), (1, 126), (2, 64), (8, 68), (9, 64)];
let frequency_cost = reroll.expected_lock_cost_for_echo(&echo).unwrap();
let locks = reroll.lock_choices_for_echo(&echo, 3).unwrap();

// 复用已求好的重抽表。此处调谐器权重为 1，频整器权重为 1000。
let mut joint = JointPolicySolver::from_reroll(reroll, CostModel::tuner_only(), 1000.0).unwrap();
joint.derive_policy(1e-9, 100).unwrap();
let decision = joint.get_decision(&[(0, 63), (1, 126)]).unwrap();
let resources = joint.calculate_expected_resources().unwrap();
let weighted_cost = joint.weighted_expected_cost().unwrap();
```

也可以直接用 `JointPolicySolver::new(&scorer, cost_model, frequency_weight, target_score)` 创建。
`update_cost_model` 更改资源权重后，需要重新 `derive_policy`，但会复用重抽表；
`update_target_score` 则会使重抽表一并失效。
联合决策包含 `Upgrade`、`Abandon`、`Keep` 和 `Reroll { lock_mask }`。
重抽结果的接受判断可以通过 `joint.reroll_solver().should_accept_echo(...)` 查询。

注意单位与模型边界：

- 重抽与联合求解的目标均为 **内部副词条分数**；`LinearScorer` 的显示目标应先减去主词条分数，再乘 `SCORE_MULTIPLIER` 并四舍五入。现有 `UpgradePolicySolver` 的目标参数仍使用显示副词条分数。
- Echo 查询接受 0～5 条互不重复的真实词条及合法数值档位；重抽查询必须恰好有 5 条。只有 mask、没有数值的查询遇到数值相关评分时会返回 `ValuesRequired`。
- 资源结果表示获得一个达标成品的期望净消耗，计入分解返还与保留成品的返还修正。经验单位为每管 5000 经验的特级密音筒。
- 频整器权重表达其相对稀缺程度，**不表示库存硬上限**。模型允许反复重抽、拒绝新结果并保留原结果；达标成品总是升满、调满。
- 精确求解仍有状态空间限制。默认最多 250 万个状态、8000 万个状态与锁定组合关联；超限返回错误，不做隐式近似。可通过 `SolverLimits` 和对应的 `with_limits` / `with_scorer_and_limits` 显式调整，内存消耗也会随之增加。
- 自定义评分概率若小到使联合状态概率进入 `f64` 的非正规数范围，会返回 `NumericalFailure`，避免概率舍入放大为错误的期望消耗；实际游戏概率不受影响。
- 新重抽算法不使用迭代收敛。旧 `derive_policy(tol, max_iter)` 接口保留参数校验；`max_iter > 0` 时完成精确求解，`tol` 不再决定重抽精度。联合求解的 `tol` / `max_iter` 仍控制外层迭代。

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

## 致谢

### 赞助
特别感谢 Bilibili **[@冬葳蕤](https://space.bilibili.com/58999432)** 自发组织的库洛游戏二创激励计划为本项目提供的赞助。

### 词条统计数据来源
本项目曾使用 **[@IceHe何瀚清](https://space.bilibili.com/13378662)** 统计的副词条产出数据。

### 内置评分预设来源
桌面应用内置的评分预设转换来源如下：
- Wuwa Echo Tool: https://github.com/2-07665/WuwaEchoTool
- 漂泊者强化助手（微信小程序）: `#小程序://漂泊者强化助手/FGd22Ty9ssvPcRy`
- WutheringWavesUID（GPL 许可来源）: https://github.com/Cccc-owo/WutheringWavesUID

### 讨论与交流
- Bilibili **[@冰封_](https://space.bilibili.com/88548986)**
