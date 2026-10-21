# Echo Studio · 声骸策略工坊

《鸣潮》声骸培养策略计算器，[echo_policy](https://github.com/2-07665/echo-policy) 求解库的图形前端。

选好角色的评分预设、设定目标分数，它会告诉你：这件声骸还值不值得继续强化，满词条后该锁哪几条重抽，以及平均要花多少资源才能得到一件达标声骸。

提供**网页版**和**桌面版**，两者界面相同。

## 功能

| 模式 | 适用场景 | 给出的建议 |
|---|---|---|
| 强化 | 从胚子开始逐条开词条，不用频整器 | 继续还是放弃，平均每件成品的资源消耗 |
| 重抽 | 手上已有一件满词条声骸 | 锁哪几条，新结果要不要接受，平均要多少频整器 |
| 联合 | 强化和重抽都考虑 | 强化、放弃、保留还是重抽，平均每件成品的资源消耗 |

- 内置五套评分体系的数百个角色预设，支持中英文角色名搜索，也可以自己调整权重、保存自定义预设。
- 中文 / English 界面。
- 桌面版可离线使用，并支持通过 ok-ww OCR 自动填入词条。

## 快速开始

需要 Rust 1.96 或更高版本。界面是原生 HTML/JS，不需要 Node 或前端构建。

**网页版**

```bash
cargo run --release -p echo_server
```

然后在浏览器打开 <http://127.0.0.1:3000>。

**桌面版**（需先安装 [Tauri 系统依赖](https://v2.tauri.app/start/prerequisites/)）

```bash
cargo run --release -p echo_calculator_app
```

请始终使用 `--release`，调试构建的求解速度会慢很多。

## 文档

| 文档 | 内容 |
|---|---|
| [使用说明](docs/usage.md) | 各模式的操作流程、结果的含义、预设与 OCR |
| [部署指南](docs/deployment.md) | 在 VPS 上运行网页版：Docker / systemd、HTTPS、配额与访问密钥 |
| [角色数据包](data/README.md) | 不重新编译就更新角色和权重数据 |
| [开发说明](docs/development.md) | 项目结构、HTTP 接口、测试与桌面打包 |
| [服务器性能测量](docs/server-performance.md) | 默认容量参数的依据 |

求解算法和数学模型见 [echo_policy 文档](https://github.com/2-07665/echo-policy#readme)。

## 致谢

**赞助**：感谢 Bilibili [@冬葳蕤](https://space.bilibili.com/58999432) 自发组织的库洛游戏二创激励计划为本项目提供赞助。

**词条统计数据**：本项目曾使用 [@IceHe何瀚清](https://space.bilibili.com/13378662) 统计的副词条产出数据。

**评分预设来源**：

- Wuwa Echo Tool：<https://github.com/2-07665/WuwaEchoTool>
- 漂泊者强化助手（微信小程序）：`#小程序://漂泊者强化助手/FGd22Ty9ssvPcRy`
- XutheringWavesUID 权重快照（GPL-3.0）：<https://github.com/IceHe/okww-xwuid-echo-score>
- XutheringWavesUID 原始权重资源：<https://cnb.cool/loping151/XutheringWavesUID-Resources>
- 旧版 WutheringWavesUID 预设：<https://github.com/Cccc-owo/WutheringWavesUID>

**讨论与交流**：Bilibili [@冰封_](https://space.bilibili.com/88548986)

## 许可证

[LICENSE](LICENSE)
