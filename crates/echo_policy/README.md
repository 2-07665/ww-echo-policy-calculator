# echo_policy

计算《鸣潮》声骸的强化与重抽策略，在给定评分规则、目标分数和资源相对价值后，最小化得到一个达标成品的**期望加权净资源消耗**。

本文依次说明问题定义、求解方法和公开 API。只需强化时使用 `UpgradePolicySolver`；允许强化后重抽时使用独立的 `JointPolicySolver`；已有满词条声骸的重抽问题由 `RerollPolicySolver` 求解。

## 1. 要解决的问题

### 1.1 输入、决策和目标

输入包含：

- 评分器：将一个副词条的类型、实际数值映射为非负整数分数，声骸副词条总分为各条得分之和。
- 目标：副词条总分至少达到指定值。
- 成本模型：胚子、调谐器、经验的权重与逐等级实测经验返还；联合模式另有频整器权重。

| 求解器 | 起点 | 可选决策 | 最小化的量 |
|---|---|---|---|
| `UpgradePolicySolver` | 不断获取新胚子，每次逐条揭示副词条 | 继续强化、放弃并重开 | 得到达标成品的胚子、调谐器、经验加权净消耗期望 |
| `RerollPolicySolver` | 已经调满五条的声骸 | 锁定哪些条、重抽、接受或拒绝候选 | 当前声骸重抽到达标所需的频整器数量期望 |
| `JointPolicySolver` | 新胚子，可在调满后重抽 | 强化、放弃、保留达标成品、重抽 | 得到达标成品的上述四种资源加权净消耗期望 |

强化求解的结果是一套适用于途中状态的策略，而非预先固定的“强化到某一级再看”。每次放弃后按同一策略从新胚子开始，直到得到一个成品。返回的资源期望包含所有失败尝试。

**这是固定权重、无限次尝试下的期望成本问题。** 频整器权重越高，求解器越不愿使用；权重并不是库存数量，也不保证每次实际消耗不超过某个上限。模型不处理有限资源预算、多件装备组合、声骸获取概率或随机主词条。

### 1.2 随机模型与边界

当前实现固定采用以下模型，内置档位概率见源码 `data.rs`：

- 13 种副词条类型，一件声骸最多 5 条，类型不重复。
- 强化揭示下一条时，从尚未出现的类型中等概率选择；数值按该类型的离散概率分布产生。
- 重抽仅作用于调满的声骸。最多锁 4 条，锁定条保留类型和数值，其余位置重新生成；未锁定的原类型可以再次出现。
- 锁 0～2 条每次消耗 1 个频整器，锁 3 条消耗 2 个，锁 4 条消耗 3 个。
- 候选可以拒绝并保留原声骸，但本次频整器不返还。
- 被保留的成品总是升满并调满，即使提前达到目标分数也要计入剩余强化成本。
- 不计贝币成本。模型中的一次揭示同时包含升到下一个调谐节点和调谐该副词条。

对某条数值的评分不必单调，但必须非负、可加。评分分布不能依赖其他已有词条、抽取次数或历史操作，否则这里的状态与转移不再适用。

### 1.3 数据表示与评分单位

词条用 `(buff_index: usize, buff_value: u16)` 表示；echo 参数是这些二元组的切片。类型索引如下：

| 索引 | 类型 | 实际数值单位 |
|---:|---|---|
| 0 | 暴击率 | 0.1%，例如 6.3% 为 `63` |
| 1 | 暴击伤害 | 0.1%，例如 12.6% 为 `126` |
| 2 | 攻击百分比 | 0.1% |
| 3 | 防御百分比 | 0.1% |
| 4 | 生命百分比 | 0.1% |
| 5 | 固定攻击 | 原始整数 |
| 6 | 固定防御 | 原始整数 |
| 7 | 固定生命 | 原始整数 |
| 8 | 共鸣效率 | 0.1% |
| 9 | 普攻伤害加成 | 0.1% |
| 10 | 重击伤害加成 | 0.1% |
| 11 | 共鸣技能伤害加成 | 0.1% |
| 12 | 共鸣解放伤害加成 | 0.1% |

`mask: u16` 的第 `i` 位表示第 `i` 类副词条是否存在；它不包含数值。合法声骸 mask 只使用低 13 位，置位数至多为 5。

**实际数值、显示分数、内部整数分数是三种不同的量：**

- `InternalScorer` 计算内部副词条分数 `u16`，不含主词条。
- `LinearScorer` 的内部副词条分数除以 `SCORE_MULTIPLIER = 100.0` 后得到显示副词条分数；`echo_score_display` 还会加上主词条显示分。
- `FixedScorer` 的权重本身就是内部整数分数。它的两个 `*_display` 方法也直接返回整数，不做除以 100 的换算。

| 接口 | 目标或状态分数单位 |
|---|---|
| `UpgradePolicySolver::new`、`update_target_score` | 显示副词条分数 `f64`，内部乘 100 并四舍五入 |
| `UpgradePolicySolver::get_decision`、`get_success_probability` | 内部副词条整数分数 `u16` |
| `RerollPolicySolver::set_target` | 内部副词条整数分数 `u16` |
| `JointPolicySolver::new`、`with_limits`、`update_target_score` | 内部副词条整数分数 `u16` |
| 重抽的 `*_for_echo`、`should_accept_echo`，联合的 echo 查询 | 实际数值档位，不是得分 |

若目标是 `LinearScorer` 的显示总分 `D`，应先计算 `T_display = max(0, D - scorer.main_buff_score())`。强化求解器接收 `T_display`；重抽与联合求解器接收 `round(T_display * SCORE_MULTIPLIER)`。转换前应检查有限性和 `u16` 范围，不依赖 Rust 强制转换的截断行为。

例如 `FixedScorer` 给双暴各赋 1 分，希望两条都有：强化目标传 `0.02`，重抽或联合目标传 `2`。

### 1.4 资源成本与返还

加权成本为各资源数量乘权重之和。`CostModel` 只存胚子、调谐器、经验的权重；频整器权重单独传给 `JointPolicySolver`，使用同一成本尺度。

经验以**特级密音筒等价数量**计，1 管 = 5000 原始经验，可为小数。到五个调谐节点的累计原始经验为：

```text
等级节点          5       10       15       20        25
累计经验       4500    16500    40000    79500    143000
回收返还       3000    12000    30000    59500    106500
累计净耗       1500     4500    10000    20000     36500
增量净耗       1500     3000     5500    10000     16500
```

为统一失败与成功路径，每次揭示先按“最终会分解”的净成本记账：

- 第一次揭示额外消耗 1 个胚子。
- 每条调谐器净成本为 `10 * (1 - 0.3) = 7`。
- 该阶段经验净成本为 `（本级累计净耗 - 上级累计净耗）/ 5000`，其中累计净耗为累计经验减去该等级的实测回收返还；+0 的两者均为 0。

最终成功的声骸没有分解，因此每个成品再加回一次 `15` 个调谐器和 `106500 / 5000 = 21.3` 个特级密音筒等价经验。这个修正与采用什么策略无关，可以放在求解后的期望结果中统一加入。若一次就成功且无重抽，总消耗应为 1 个胚子、50 个调谐器、28.6 个特级密音筒等价经验。

所有权重必须有限且非负，`CostModel` 的三个权重不能全为零。经验返还使用上述固定实测表，不再提供比例参数。`tuner_only()` 使用权重 `(0, 1, 0)`。频整器权重允许为零，但其余三项仍须满足 `CostModel` 的要求。

## 2. 求解方法

### 2.1 评分离散化

内置档位经过评分器映射为 `(score, probability)` 的 PMF（概率质量函数）。同一类型中得分相同的数值档位可以合并，后续只需要得分和概率。

`LinearScorer` 按最大档位的相对权重分配分数：先对权重最高的五类按最大余数法分配内部整数分数预算，余数并列时按类型索引决定；其余类型的最高分不超过这五类中的最低最高分。单条得分随后按 `最高分 * 实际值 / 最大档位值` 取整。这样显示评分与求解使用同一份离散结果，不会因独立舍入导致理论满分目标不可达。

“精确求解”针对的是这些离散得分和给定概率，不代表无量化误差的连续评分，也不代表使用有理数算术；期望值计算使用 `f64`。

### 2.2 不考虑重抽：分数状态 DP 与比值优化

强化状态压缩为 `(mask, score)`。将来会抽到什么类型只由 mask 决定；成功条件只看总分，因此无需保留每条具体数值。

设一轮从新胚子开始、至成功或放弃结束的成功概率为 `p`，期望加权净成本为 `c`，则不断重开的成品成本为：

```text
J = c / p + 成功返还修正
```

外层用 Dinkelbach 比值迭代找最优策略。对固定参数 `lambda`，内层从五条终态反向计算：

```text
V_lambda(full) = B                       若分数达标
                0                       否则
V_lambda(s)    = max(0, E[V_lambda(next)] - lambda * reveal_cost(s))
```

放弃的价值为 0；选择继续后按未出现类型及其分数 PMF 求期望。实现用 `B = 1000`。推导出策略后重新计算其 `p`、`c`，更新 `lambda = B * p / c`。内部归一化成本权重，收敛条件按最小一轮成本约束相对残差，避免同比缩放资源权重改变有效误差尺度。

缓存按 mask 和整数总分组织，并利用最大可能剩余得分剪枝及继续强化的分数阈值。最终成本从实际策略的资源期望计算，而不是直接把尚有误差的 lambda 取倒数。

### 2.3 考虑数值的重抽：状态压缩与阈值顺序求解

重抽时，哪条值得锁定取决于各条得分；仅保留总分会丢失信息。状态保留每种有效类型及其得分档位，并做两种等价合并：

1. 同一类型的相同得分档位合并概率。
2. 所有档位都得零分的类型只保留数量。锁定这种类型不会优于解锁它，因此无需枚举它们的锁定选择。

**有效类型某次得零分仍保留类型身份**，因为它占用了一个不能再次出现的类型，并可能在重抽后获得非零分。

令 `R(s)` 为当前满词条状态达到目标的最小期望频整器消耗，达标状态为 0。对锁定配置 `L`（含锁定类型及其得分），候选为 `Y`，单次消耗为 `a(L)`：

```text
R(s) = min_L { a(L) + E[min(R(s), R(Y)) | L] }
```

内部的 min 表示拒绝较差的候选，保留原状态。对一个锁定配置，可以把上式改写成阈值 `r_L` 的方程：

```text
E[max(r_L - R(Y), 0) | L] = a(L)
R(s) = min_{L compatible with s} r_L
```

比阈值更差的候选总能拒绝，因此确定阈值只需要已知的较低成本状态。算法从达标状态开始，用优先队列按阈值从小到大确定其余状态的成本，类似 Dijkstra 的定标过程，而非反复做全局价值迭代。

条件概率通过锁定事件归一化得到。设有效类型数为 `m`、零分类型数为 `z = 13-m`，某个满词条压缩状态有 `j` 条有效类型，锁定了 `k` 条：

```text
P0(state) = C(z, 5-j) / C(13, 5) * product(各有效词条得分的概率)
P0(lock)  = C(13-k, 5-k) / C(13, 5) * product(各锁定得分的概率)
P(state | lock) = P0(state) / P0(lock)   若兼容，否则为 0
```

累计已定标的兼容状态的概率质量 `M` 和概率加权成本 `S`，候选阈值为 `(a(L) * P0(lock) + S) / M`。每个满词条状态只定标一次，存储的是状态与锁定配置的关联，不构建状态之间的稠密转移矩阵。

重抽表只依赖评分器、目标和固定的锁定消耗规则；不依赖其他资源的权重。接受规则是候选的 `R` 不大于当前的 `R`，不必要求候选总分更高。

### 2.4 联合求解：将重抽表作为强化的终态

联合强化状态为 `(已揭示条数, 各有效类型及其得分)`。永久零分类型只需要数量；下一条某个未出现有效类型的概率为 `1/(13-n)`，零分类型的概率为剩余零分类型数除以 `13-n`。

先独立求得 `R(s)`。设重新开始的最优剩余净成本为 `J`，频整器权重为 `w_f`，则未达标满词条状态的成本为：

```text
min(J, w_f * R(s))
```

这里可以直接截断，因为重抽能拒绝候选并保留当前声骸：决定重抽后只需要接受不更差的重抽状态，不必承诺接受会导致更高成本的候选。

联合求解仍用反向 DP 加外层比值迭代，但成功奖励取 `B = 1`，满词条终态变成：

```text
V_lambda(full) = 1                                    若已达标
                max(0, 1 - lambda * w_f * R(full))    否则
```

选择重抽的终态，最终成功概率为 1，期望频整器消耗为 `R(full)`；选择放弃则两者都为 0。每个强化状态同时累计成功概率和三种后续资源消耗，外层按四种资源的加权成本更新比值。

不能沿用“剩余强化无论如何都达不到目标就放弃”的剪枝，因为升满后仍能重抽。原有强化求解器因此独立保留；高频整器权重下联合策略可能不使用重抽，但仍有联合状态空间的开销。

### 2.5 规模与数值限制

若有效类型 `i` 有 `b_i` 个不同得分档位，满词条状态数为：

```text
N = sum_{U subset of effective types, |U| <= 5, 5-|U| <= z}
        product_{i in U} b_i
```

每个状态至多关联 31 个可锁定配置。若关联数为 `E`，满词条数为 `N`，锁定配置数为 `A`，联合部分状态数为 `P`，主要存储量随 `N + A + E + P` 增长；其中关联索引本身占 `4 * E` 字节，另外还需状态、概率、期望、优先队列和分配器开销。联合迭代每轮遍历部分状态及其下一条可能结果，耗时还受外层迭代次数影响。

`SolverLimits` 默认 `max_states = 2_500_000`、`max_transitions = 80_000_000`。前者分别检查满词条数、锁定配置数，以及联合求解的部分状态总数；后者限制满词条状态与锁定配置的关联数。不是总字节数上限，也不会自动切换为近似算法。

超限会返回错误；自定义概率若使联合状态概率或锁定事件概率进入 `f64` 非正规数范围，也会返回 `NumericalFailure`，避免相对精度损失放大成错误的期望。内置游戏概率不触发这个数值保护。

## 3. Public API

对外接口均由 crate 根模块重新导出。`data`、`scoring` 等模块本身是私有的；例如源码中的 `convert_display_to_internal` 不是外部可调用接口。

### 3.1 评分器

| 类型 / 方法 | 用途与约定 |
|---|---|
| `InternalScorer::buff_score_internal(index, value)` | 实现非负整数副词条评分；输入为实际数值 |
| `InternalScorer::echo_score_internal(echo)` | 默认验证最多 5 条且类型不重复，再求各条分数之和；不含主词条 |
| `InternalScorer::build_score_pmfs()` | 默认使用内置数值概率，按得分合并，返回 13 个类型的 PMF |
| `FixedScorer::new(weights)` | 13 个 `u16` 类型权重，忽略数值对得分的影响；不能全零，最高五项之和不得超过 `u16::MAX` |
| `FixedScorer::max_score()` | 最佳五种类型的总分 |
| `FixedScorer::{buff_score_display, echo_score_display}` | 返回固定权重整数分数，与内部单位相同 |
| `FixedScorer::build_from_buff_selection()` | **尚未实现，调用会 panic**；当前请使用 `new` |
| `LinearScorer::new(weights, main_buff_score, normalized_max_score)` | 权重表示各类型最大档位的相对得分；主词条参数是归一化前的分数，不是归一化后的显示分 |
| `LinearScorer::default(weights)` | 主词条 0 分、目标归一化总满分 100；这是构造方法，不是 `Default` trait |
| `LinearScorer::qq_bot_scorer(weights, main_buff_score)` | 将单位数值权重乘类型最大值，百分比类型另乘 0.1，再按总满分 50 归一化 |
| `LinearScorer::mc_boost_assistant_scorer(weights)` | 副词条分数预算 70，固定主词条显示分 50，总满分标称 120 |
| `LinearScorer::{main_buff_score, normalized_max_score}()` | 读取归一化后的主词条分和标称总满分；离散取整可能产生细小显示差异 |
| `LinearScorer::buff_score_display(index, value)` | 单条内部得分除以 100 |
| `LinearScorer::echo_score_display(echo)` | 主词条显示分加内部副词条总分除以 100 |
| `SCORE_MULTIPLIER` | 显示副词条分数与内部整数分数的比例，当前为 `100.0` |

自定义评分器必须能对所有内置合法数值档位成功评分，否则默认 PMF 构建会 panic。任意五种不同类型的分数之和应能用 `u16` 表示；默认 `echo_score_internal` 本身不做溢出保护。若覆盖 `build_score_pmfs`，需提供 13 个非空 PMF，概率有限、非负且每种类型概率和为 1（校验容差 `1e-9`），并与实际数值评分保持一致。求解器按可加评分工作，不会使用自定义的非加性 `echo_score_internal` 来改变 DP 目标。

### 3.2 成本与 mask 工具

| 类型 / 方法 | 用途与约定 |
|---|---|
| `CostModel::new(w_echo, w_tuner, w_exp)` | 构造三种资源的相对价值模型；约束见 1.4 |
| `CostModel::tuner_only()` | 只优化调谐器成本，经验统计仍使用实测表 |
| `CostModel::update_weights(Option, Option, Option)` | `None` 保持该项原值；全部验证后才更新 |
| `CostModel::tuner_cost()` | 单条未加权净成本 7，不是原始支付数量 10 |
| `CostModel::exp_cost(slot)` | 第 `slot` 次揭示的未加权净经验增量，`slot` 必须在 `0..5` |
| `CostModel::full_upgrade_exp_cost(current_slot)` | 已开 `current_slot` 条后升满的剩余净经验，参数范围 `0..=5` |
| `CostModel::weighted_reveal_cost(slot)` | 本次揭示的加权净成本；`slot=0` 包含胚子，范围 `0..5` |
| `CostModel::success_additional_tuner_cost()` | 保留成品时加回的 15 个调谐器 |
| `CostModel::success_additional_exp_cost()` | 保留成品时加回的经验返还部分 |
| `CostModel::weighted_success_additional_cost()` | 上述成功修正的加权和 |
| `bits_to_mask(bits)` | 前 13 个元素中值恰为 1 的位置置位；其他值忽略，缺失位置为 0，超长部分忽略 |
| `mask_to_bits(mask)` | 返回低 13 位对应的 `[u8; 13]`，忽略更高位 |

带 slot 的成本方法不返回 `Result`，越界输入可能 panic；两个 mask 转换工具也不承担声骸合法性校验。`CostModel` 是 `Copy`，修改外部模型不会更新已经构造的求解器。

### 3.3 强化求解器

常规调用顺序是 `new → lambda_search → 查询策略/期望`。

| 方法 | 行为 |
|---|---|
| `new(&scorer, target_score_display, cost_model)` | 创建 PMF 与缓存；此时尚未求策略。有限的非正目标按 0 处理 |
| `lambda_search(tol, max_iter)` | 求最优策略并缓存资源期望；返回原始成本尺度下的 lambda，奖励系数为 1000 |
| `derive_policy_at_lambda(lambda)` | 高级入口：按给定 lambda 求策略，不进行外层优化，也不自动填充资源期望缓存。调用方应提供有限非负值；此接口不验证参数 |
| `get_decision(mask, score)` | `true` 表示继续揭示下一条。空声骸返回 true，已满五条返回 false；满五条的 false 不表示一定不达标 |
| `get_success_probability(mask, score)` | 当前声骸严格执行策略、放弃前成功的条件概率，不是无限重开最终成功的概率 |
| `calculate_expected_resources()` | 计算/复用资源缓存，返回 `ExpectedUpgradeCost` |
| `weighted_expected_cost()` | 当前策略得到一个成品的期望加权净成本，包含成功修正 |
| `update_target_score(display_score)` | 成功更新目标后清空策略和资源缓存，需要再次求解 |
| `cost_model()`、`is_policy_derived()` | 只读查看成本模型与策略是否可用 |

`get_decision` 是分数阈值查询，不完整验证 mask 与得分是否可达；调用方应使用同一评分器从实际声骸计算内部得分。`get_success_probability` 也不能用作通用输入校验。常规路径先成功执行 `lambda_search` 即可查询；显式 lambda 路径建议先调用 `calculate_expected_resources`，否则需要缓存的概率查询会返回 `ExpectedResourcesNotComputed`。显式 lambda 可能产生成功概率为零的策略，此时期望成品消耗可能为无穷大。

`ExpectedUpgradeCost` 提供 `success_probability()`、`echo_per_success()`、`tuner_per_success()`、`exp_per_success()`。其中成功概率为一轮新胚子尝试的概率，胚子期望为它的倒数；后两项已经加入成功返还修正，不要再手动加一次。

更换评分器或成本模型需重建 `UpgradePolicySolver`；它没有对应更新接口。

### 3.4 重抽求解器

调用顺序是 `new / with_scorer → set_target → solve → 查询`。

| 方法 | 行为 |
|---|---|
| `new(weights: [u16; 13])` | 兼容类型评分入口，内部创建 `FixedScorer` |
| `with_scorer(&scorer)` | 使用给定评分器，采用默认规模限制 |
| `with_scorer_and_limits(&scorer, limits)` | 显式指定精确状态规模上限 |
| `set_target(internal_score)` | 设置内部副词条目标，并使已求策略失效；目标不变也会失效 |
| `solve()` | 用有限次定标求解，不需要迭代容差 |
| `derive_policy(tol, max_iter)` | 兼容入口：校验有限正 tol；`max_iter=0` 报错，其余调用 `solve`。这两个参数不再控制重抽求解精度和定标次数 |
| `is_target_set()`、`is_policy_derived()` | 查看目标和策略状态 |
| `state_count()` | 满词条压缩状态数，不含锁定配置和联合部分状态 |

查询分两组，语义相同：

| mask 查询 | 实际词条查询 | 返回含义 |
|---|---|---|
| `expected_lock_cost(mask)` | `expected_lock_cost_for_echo(echo)` | `R(s)`：到达目标的期望频整器消耗，已达标为 0 |
| `best_lock_choices(mask)` | `best_lock_choices_for_echo(echo)` | 一个最优锁定 mask；已达标为 `None` |
| `lock_choices(mask, top_k)` | `lock_choices_for_echo(echo, top_k)` | 按期望成本排序的 `Vec<LockChoice>`；`top_k=0` 返回全部保留的锁定选项，已达标为空 |
| `best_lock_success_probability(mask)` | `best_lock_success_probability_for_echo(echo)` | 推荐锁法下一次立即达标的概率；已达标为 `None` |
| `should_accept(baseline_mask, candidate_mask)` | `should_accept_echo(baseline, candidate)` | 候选期望后续成本不高于原状态时返回 true，包括相等 |

所有重抽查询要求已经求解，且输入为五条。实际词条输入必须类型不重复、数值属于内置真实档位；mask 查询无法指定有多种得分的已有类型，此时返回 `ValuesRequired`。类型得分只有一个档位时可以只给 mask。

`LockChoice` 的公开字段：

- `lock_mask`：锁定哪些**类型**，沿用当前 echo 中这些类型的数值。
- `expected_cost`：先执行这次锁法，之后按最优策略行动的总频整器期望，包含本次消耗以及拒绝候选的可能性。
- `regret`：该期望比返回列表最优项高多少；不是目标分数差。
- `success_probability`：这次重抽立即达标的概率，不是最终成功概率。

锁法列表省略了被支配的永久零分类型锁定选项。同成本的多个最优锁法中，单项推荐与列表首项不保证相同；浮点累加也可能产生微小差异。接受查询只比较两份合法状态的价值，不验证候选是否确实保留了某次锁定的词条。

### 3.5 联合求解器

| 方法 | 行为 |
|---|---|
| `new(&scorer, cost_model, frequency_weight, internal_target)` | 创建带默认规模限制的联合求解器；频整器权重显式传入 |
| `with_limits(&scorer, cost_model, frequency_weight, internal_target, limits)` | 显式指定规模限制 |
| `from_reroll(reroll, cost_model, frequency_weight)` | 接管已设置目标的重抽求解器；可以已求解，也可以尚未求解 |
| `derive_policy(tol, max_iter)` | 必要时先求重抽表，再做联合比值迭代；要求有限正 tol 与正迭代预算 |
| `get_decision(echo)` | 对 0～5 条实际词条返回 `JointDecision`，空声骸代表开始新一轮 |
| `get_success_probability(echo)` | 当前声骸执行策略、放弃前成功的概率，包含可能的多次重抽 |
| `calculate_expected_resources()` | 返回 `ExpectedJointCost`，此方法只读 |
| `weighted_expected_cost()` | 四种资源的期望加权净成本，包含成功修正 |
| `update_cost_model(cost_model, frequency_weight)` | 使联合策略失效但保留重抽表；再次 `derive_policy` 后可查询 |
| `update_target_score(internal_target)` | 使联合策略及重抽策略都失效，需要重新求解 |
| `cost_model()`、`frequency_weight()`、`is_policy_derived()` | 查看配置及策略状态 |
| `reroll_solver()` | 只读访问内部重抽表，用于锁法列表、接受候选等查询 |

`JointDecision` 包含：`Upgrade`（再揭示一条）、`Abandon`（放弃重开）、`Keep`（满词条且达标）、`Reroll { lock_mask }`（使用指定锁法重抽）。不能用联合求解器的内部重抽表单独决定是否值得开始重抽：重抽表只优化频整器数量，`get_decision` 才会比较放弃重开的代价。

`ExpectedJointCost` 有与 `ExpectedUpgradeCost` 相同的四个 getter，另加 `frequency_per_success()`。推荐重抽的满词条状态条件成功概率为 1，但可能消耗很多次频整器；成品频整器期望同样包含全部尝试的影响。

评分器变化时需要重建联合求解器与重抽表。资源权重变化不必重算重抽表，但不代表只需把旧联合策略的成本线性换算；最优强化决策也可能变化。

### 3.6 错误处理与生命周期

公开错误类型为 `ScorerError`、`CostModelError`、`UpgradePolicySolverError`、`RerollPolicySolverError`、`JointPolicySolverError`，支持 `Debug`，目前未实现 `Display` / `std::error::Error`。联合求解中的重抽错误包装在 `JointPolicySolverError::Reroll` 中。

主要错误类别：

- 配置/输入：非法权重、非法评分范围、非法 PMF、`InvalidMask`、`InvalidEcho`、`InvalidScore`、`ValuesRequired`。
- 调用顺序：`TargetNotSet`、`PolicyNotDerived`、`ExpectedResourcesNotComputed`。
- 不可达目标：`TargetScoreImpossible`。这是基于评分分布的可达性，不是成功概率低或成本高。
- 规模/表示范围：`StateSpaceTooLarge`、`TransitionSpaceTooLarge`、`StateEncodingOverflow`、`ScoreRangeOverflow`、`NumericalFailure`。
- 外层优化：`InvalidTolerance`、迭代预算耗尽等。重抽兼容接口保留旧名称 `FailedtoConvergeWithinMaxIter`，其零预算错误不代表新算法使用价值迭代。

目标或成本更新验证失败时，原有效配置保留；更新成功时按上述规则失效。求解失败后不要使用旧结果，处理错误并检查 `is_policy_derived()`。增大迭代次数不能修复非法输入、状态超限或浮点表示范围问题。

### 3.7 使用示例

下面是完整 Rust 程序，展示同一目标的三种调用方式。示例采用文本代码块，未加入 crate 的测试或 doctest。

```text
use echo_policy::{
    CostModel, InternalScorer, JointPolicySolver, LinearScorer,
    RerollPolicySolver, SCORE_MULTIPLIER, UpgradePolicySolver,
};

fn main() {
    let weights = [1., 1., 1., 0., 0., 0., 0., 0., 1., 1., 0., 0., 0.];
    let scorer = LinearScorer::default(weights).unwrap();
    let costs = CostModel::tuner_only();
    let target_total: f64 = 80.0;
    let target_display = (target_total - scorer.main_buff_score()).max(0.0);
    let target_internal = (target_display * SCORE_MULTIPLIER).round() as u16;

    // 1. 禁用重抽：沿用总分状态强化求解器。
    let mut upgrade = UpgradePolicySolver::new(&scorer, target_display, costs).unwrap();
    upgrade.lambda_search(1e-9, 100).unwrap();
    let partial = [(0, 63), (1, 126)];
    let mask = (1u16 << 0) | (1u16 << 1);
    let score = scorer.echo_score_internal(&partial).unwrap();
    println!("upgrade: {}", upgrade.get_decision(mask, score).unwrap());
    println!("tuners: {}", upgrade.calculate_expected_resources().unwrap().tuner_per_success());

    // 2. 已有满词条声骸：查询到达目标的频整器期望和推荐锁法。
    let mut reroll = RerollPolicySolver::with_scorer(&scorer).unwrap();
    reroll.set_target(target_internal).unwrap();
    reroll.solve().unwrap();
    let full = [(0, 63), (1, 126), (2, 64), (8, 68), (9, 64)];
    println!("frequencies: {}", reroll.expected_lock_cost_for_echo(&full).unwrap());
    println!("locks: {:?}", reroll.lock_choices_for_echo(&full, 3).unwrap());

    // 3. 联合模式：1 个频整器与 1000 个调谐器具有相同成本。
    let mut joint = JointPolicySolver::from_reroll(reroll, costs, 1000.0).unwrap();
    joint.derive_policy(1e-9, 100).unwrap();
    println!("joint decision: {:?}", joint.get_decision(&partial).unwrap());
    println!("weighted cost: {}", joint.weighted_expected_cost().unwrap());
    println!("resources: {:?}", joint.calculate_expected_resources().unwrap());

    // 只改变资源相对价值：重抽表可以复用，联合策略需要重新计算。
    joint.update_cost_model(costs, 2000.0).unwrap();
    joint.derive_policy(1e-9, 100).unwrap();
}
```

在仓库根目录运行 `cargo doc -p echo_policy --no-deps --open` 可阅读生成的 API 文档。正文说明当前实现的模型和行为，不会自动跟随游戏版本更新。
