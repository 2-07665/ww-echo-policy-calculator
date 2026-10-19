# 角色数据包

`echo-data.json` 是网页端和桌面端共用的发布文件，包含五套评分体系的权重、角色中英文名称、首次实装日期、变体名称和预设说明。程序仍内置一份相同数据作为首次运行及故障时的备用。

## 更新网页端

启动服务器时设置：

```sh
ECHO_DATA_FILE=/opt/echo-studio/data/echo-data.json /opt/echo-studio/echo_server
```

Docker Compose 已将宿主机 `./data` 目录只读挂载到 `/data`。挂载整个目录，避免单文件 bind mount 在原子替换后仍指向旧文件。systemd 示例使用 `/opt/echo-studio/data/echo-data.json`，该文件需允许服务用户读取。

后续发布只需用**已编译的同一程序**校验新包，再在同一目录原子替换文件：

```sh
/opt/echo-studio/echo_server validate-data /opt/echo-studio/data/echo-data.next.json
mv /opt/echo-studio/data/echo-data.next.json /opt/echo-studio/data/echo-data.json
```

校验失败时不要执行替换。服务器每次收到数据读取请求时检查文件，最多每秒加载一次；无须重新编译、重启或清空求解缓存。用户刷新页面或在“角色数据”中点击“更新数据”即可加载。`GET /api/data-pack` 提供当前有效数据包下载；`GET /api/bootstrap` 返回界面需要的数据和版本、内容摘要及加载状态。这两个接口均禁止 HTTP 缓存，没有公开上传/写入接口。

损坏、不完整或不兼容的文件不会替换进程中最近一次有效数据，并在“角色数据”里提示失败。若服务器刚启动时外部文件就不可读或无效，则使用程序内置数据。未配置 `ECHO_DATA_FILE` 时始终使用内置数据。

## 更新桌面端

打开“角色数据”，选择以下一种方式：

- 在线更新：默认从本仓库 `main/data/echo-data.json` 下载；也可填写自己服务器的 `https://你的域名/api/data-pack` 地址。
- 离线更新：导入下载好的 `echo-data.json`。

默认更新地址读取仓库已发布的数据包；尚未发布的本地数据可通过导入测试。下载仅接受 HTTPS，限制重定向次数、30 秒总超时和 8 MiB 大小。下载后先校验，再原子替换缓存；失败时继续使用原数据。应用启动只读取缓存，不依赖网络。缓存位于 Tauri 应用数据目录的 `echo-data.json`（macOS 为 `~/Library/Application Support/com.echo.calculator/echo-data.json`）。在线更新地址保存在本地，不携带求解服务的访问密钥。

两端更新都保留正在使用的权重、目标、声骸和已求得的策略。数据内容变化后，当前内置预设选择变为“默认 / 手动配置”；重新选择预设才会应用新权重。自定义预设不被覆盖。

## 维护与发布

1. 使用 `scripts/convert_weights/` 下相应转换脚本更新权重源文件。
2. 在 `data/catalog.json` 补充新角色的英文名、实装日期、变体完整名称的英文翻译及预设说明；参考现有 Wiki 和上游来源。日期未知时可以留空，排序会置后。
3. 生成发布文件，使用新的数据版本号：

   ```sh
   python3 scripts/build-data-pack.py --version 2026.10.20.1
   python3 scripts/test-data-pack.py
   node scripts/test-presets.mjs
   node scripts/test-i18n.mjs
   target/release/echo_server validate-data data/echo-data.json
   ```

4. 发布 `data/echo-data.json` 到仓库或服务器，桌面用户即可下载或导入。没有算法/界面变动时，无需编译应用。

生成器从现有发布文件保留预设 ID；新预设采用评分体系、角色名、变体名的稳定摘要，源文件重排不会使旧 ID 指向其他角色。不要删除旧发布文件后重新生成。上游自带 ID 时保留该 ID。`--check --version <当前版本>` 可检查发布文件与源数据是否一致。

`schema_version` 表示应用支持的数据结构版本，目前为 `1`；普通数据更新只修改 `version`。内容摘要能识别同版本号下的内容变化，但正式发布仍应使用不同版本号。只允许已有的五套评分算法和 13 个词条 ID，首个变体必须提供完整权重，后续变体可继承首个变体；角色信息、翻译、唯一 ID、日期、有限非负数值和文件大小都会校验。新增评分算法、词条种类或不兼容格式仍需更新应用。

来源和许可证见 [权重快照说明](../scripts/convert_weights/sources/README.md)，角色与术语来源见 [实装时间](../apps/web/CHARACTER_RELEASES.md) 和 [翻译记录](../apps/web/TERMINOLOGY.md)。
