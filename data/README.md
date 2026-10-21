# 角色数据包

[echo-data.json](echo-data.json) 包含全部角色预设：五套评分体系的权重、角色中英文名、首次实装日期、变体名称和说明。网页版和桌面版共用这个文件，程序内也内置一份，供首次运行或外部数据损坏时使用。

数据包可以独立于程序更新。只要不新增评分算法或词条种类，更新角色和权重都不需要重新编译。

## 更新网页版

服务器通过 `ECHO_DATA_FILE` 读取数据包；[部署指南](../docs/deployment.md)中的 Docker Compose 和 systemd 示例都已配置好。

发布新数据时，先用**正在运行的同一个程序**校验新文件，校验通过后再在同一目录下替换原文件：

```bash
/opt/echo-studio/echo_server validate-data /opt/echo-studio/data/echo-data.next.json
mv /opt/echo-studio/data/echo-data.next.json /opt/echo-studio/data/echo-data.json
```

用户刷新页面后即可获得新数据，无需重启服务。

- 服务器收到请求时检查数据文件，最多每秒重新加载一次。
- 新文件无效时，服务器继续使用上一份有效数据，失败原因可以在 `GET /api/bootstrap` 返回的加载状态 `warning` 中查看。如果启动时外部文件就不可用，则使用内置数据。
- Docker Compose 挂载的是整个 `data/` 目录而不是单个文件，这样替换文件后容器能看到新文件。
- `GET /api/data-pack` 可以下载当前生效的数据包，不存在上传或写入接口。

## 更新桌面版

在应用的「角色数据」中选择一种方式：

- **在线更新**：默认从本仓库 `main` 分支的 `data/echo-data.json` 下载，也可以填写自己服务器的 `https://<域名>/api/data-pack`。
- **导入文件**：选择下载好的 `echo-data.json`，适合离线使用或测试尚未发布的数据。

下载只接受 HTTPS，文件最大 8 MiB，超时 30 秒。新数据校验通过后才会替换本地缓存，失败时继续使用原数据。应用启动时只读取缓存，不需要联网。缓存位于 Tauri 应用数据目录，在 macOS 上为 `~/Library/Application Support/com.echo.calculator/echo-data.json`。

## 更新后的行为

- 当前的权重、目标和声骸都会保留。桌面版连已求得的策略也会保留；网页版刷新后需要重新求解。
- 数据有变化时，预设选择会变为“默认 / 手动配置”，需要新权重时重新选择预设即可。
- 自定义预设不受影响。

## 发布新数据

1. 用 `scripts/convert_weights/` 中的脚本更新权重源文件，参见[权重快照说明](../scripts/convert_weights/sources/README.md)。
2. 在 [catalog.json](catalog.json) 中补充新角色的英文名、实装日期、变体英文名和说明。日期未知时可以留空，该角色会排在已知日期的角色之后。
3. 用新的版本号生成数据包并运行检查：

   ```bash
   python3 scripts/build-data-pack.py --version 2026.10.20.1
   python3 scripts/test-data-pack.py
   node scripts/test-presets.mjs
   node scripts/test-i18n.mjs
   target/release/echo_server validate-data data/echo-data.json
   ```

4. 把 `data/echo-data.json` 发布到仓库或服务器。

`python3 scripts/build-data-pack.py --check --version <当前版本>` 可以检查发布文件是否与源数据一致。

### 注意事项

- **预设 ID 要保持稳定。** 生成器会沿用现有发布文件中的 ID，新增预设的 ID 由评分体系、角色名和变体名计算得出，调整源文件顺序不会改变已有 ID。所以**不要删掉旧的发布文件再重新生成**。
- **版本号**：普通的数据更新只修改 `version`。`schema_version`（当前为 `1`）表示数据结构的版本，只有数据格式变化时才需要修改，同时也要更新程序。
- **校验范围**：只允许已有的五套评分算法和 13 种词条。每个角色的第一个变体必须给出完整权重，后续变体可以沿用第一个变体的权重。角色信息、翻译、ID 唯一性、日期、数值和文件大小都会被校验。

角色排序和术语翻译的依据见[实装时间](../apps/web/CHARACTER_RELEASES.md)和[术语对照](../apps/web/TERMINOLOGY.md)。
