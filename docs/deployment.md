# 部署指南

本文介绍如何在 Linux VPS 上运行网页版。服务端只有一个二进制文件，页面资源已经编译进去，运行时不需要数据库或 Node。

部署方式有两种，选一种即可：

- **Docker Compose**：最省事，在 VPS 上构建镜像；
- **单二进制 + systemd**：不依赖 Docker。

两种方式都只在本机 `127.0.0.1:3000` 监听，由宿主机上的 Caddy 提供 HTTPS 并反向代理。

## Docker Compose

把仓库复制到 VPS，然后执行：

```bash
docker compose up -d --build
curl http://127.0.0.1:3000/api/health
```

[compose.yaml](../compose.yaml) 已配置好：

- 将宿主机 `./data` 目录只读挂载为角色数据包目录；
- 限制容器使用 2 CPU、2 GiB 内存，并使用只读文件系统；
- 使用固定的内部网段 `172.30.77.0/24`，只信任其网关 `172.30.77.1` 传来的客户端 IP。若与现有网络冲突，需同时修改网段和 `ECHO_TRUSTED_PROXIES`。

## 单二进制 + systemd

在**目标 Linux 架构**上编译（macOS 上编译的二进制不能直接在 Linux 上运行）：

```bash
cargo build --release --locked -p echo_server
```

安装二进制、数据包和服务：

```bash
sudo install -d /opt/echo-studio/data
sudo install -m 755 target/release/echo_server /opt/echo-studio/echo_server
sudo install -m 644 data/echo-data.json /opt/echo-studio/data/echo-data.json
sudo cp deploy/echo-studio.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now echo-studio
```

[echo-studio.service](../deploy/echo-studio.service) 以临时用户运行，设置 2 GiB 内存上限，并只信任本机代理。额外的环境变量可以写在 `/etc/echo-studio.env` 中。

## HTTPS

在宿主机安装 Caddy，把 [deploy/Caddyfile](../deploy/Caddyfile) 中的 `echo.example.com` 换成自己的域名，将域名的 DNS 解析指向 VPS，并开放 80 和 443 端口。Caddy 会自动申请证书。

服务只支持部署在域名的根路径下，不支持挂在子路径下。

没有域名时，可以用 SSH 端口转发在本机访问：

```bash
ssh -L 3000:127.0.0.1:3000 user@your-vps
```

## 更新角色数据

角色和权重数据放在独立的数据包中，由 `ECHO_DATA_FILE` 指定路径，两种部署方式的示例都已配置好。更新数据**无需重新编译或重启服务**：先校验新文件，再替换原文件，用户刷新页面即可获得新数据。具体步骤见[角色数据包](../data/README.md#更新网页版)。

## 配置项

所有配置都通过环境变量设置：

| 变量 | 默认值 | 说明 |
|---|---|---|
| `ECHO_BIND` | `127.0.0.1:3000` | 监听地址（容器内为 `0.0.0.0:3000`） |
| `ECHO_DATA_FILE` | 未设置 | 外部数据包路径；未设置时使用内置数据 |
| `ECHO_TRUSTED_PROXIES` | 空 | 受信任的反向代理 IP，逗号分隔；为空时忽略转发头 |
| `ECHO_ADMIN_KEY_HASHES` | 空 | 管理员访问密钥的 SHA-256 摘要，逗号分隔 |

**容量**

| 变量 | 默认值 | 说明 |
|---|---|---|
| `ECHO_CACHE_MIB` | `512` | 策略缓存预算（MiB） |
| `ECHO_CACHE_ENTRIES` | `32` | 缓存条目上限 |
| `ECHO_MAX_PENDING_BUILDS` | `8` | 排队和正在计算的不同策略总数上限 |
| `ECHO_MAX_JOBS` | `64` | 任务总数上限 |
| `ECHO_MAX_STATES` | `750000` | 重抽 / 联合求解的状态数上限 |
| `ECHO_MAX_TRANSITIONS` | `24000000` | 重抽 / 联合求解的转移数上限 |

**每 IP 配额**

| 变量 | 默认值 | 说明 |
|---|---|---|
| `ECHO_MAX_JOBS_PER_IP` | `2` | 同时未完成的任务数 |
| `ECHO_REQUESTS_PER_MINUTE` | `240` | 每分钟预览与求解请求数（轮询任务状态不计） |
| `ECHO_QUOTA_UPGRADE` | `60` | 每小时新建强化策略次数 |
| `ECHO_QUOTA_REROLL` | `12` | 每小时新建重抽策略次数 |
| `ECHO_QUOTA_JOINT` | `4` | 每小时新建联合策略次数 |
| `ECHO_MAX_IDENTITIES` | `4096` | 同时记录配额的 IP 数上限 |

默认值依据 2 CPU / 2 GiB 环境的实测结果设定，见[服务器性能测量](server-performance.md)。提高状态上限或缓存预算前，请重新测量。

## 并发与配额的工作方式

**计算调度**

- 同一时间只计算一份新策略，另有两个名额专门处理命中缓存的查询，因此一份耗时较长的计算不会阻塞其他用户的查询。
- 多人同时请求相同配置时，只计算一次。
- 缓存按 LRU 淘汰；正在使用的策略不会被淘汰。新策略放不进缓存时，请求会明确报错。
- 512 MiB 是缓存预算，不是进程内存上限。计算时的临时内存另算，由 2 GiB 的硬上限兜底。

**配额扣除**

- 只有新建策略并成功进入队列时，才扣除小时额度。命中缓存、加入相同配置的计算、修改词条或候选都不扣额度，但计入每分钟请求数。
- 已经入队的计算，即使失败或用户中途离开，也照样扣除额度；未通过输入校验或因队列已满被拒绝的请求不扣额度。
- 把某个模式的额度设为 `0`，普通用户就不能新建该模式的策略，但仍可使用已有的缓存。
- 超出限制时返回 HTTP 429 和 `Retry-After`。

**限制与隐私**

- 同一出口 IPv4 地址共用额度，IPv6 按 /64 网段合并计算。
- 配额记录只保存在内存中，重启服务后清零。多个实例之间不共享配额，不要靠多开实例来放宽限制。
- 缓存中不保存用户的声骸；声骸数据和计算结果都不写入磁盘。

## 反向代理与客户端 IP

配额按客户端 IP 计算，所以服务必须能正确识别真实 IP：

- 只把自己的反向代理加入 `ECHO_TRUSTED_PROXIES`，**不要**加入公网客户端网段；
- 服务从右向左读取转发链，遇到第一个不受信任的地址即停止，客户端无法伪造 IP 来获得新额度；
- 更换代理或 Docker 网络时，同步修改该配置。

## 管理员访问密钥

持有访问密钥的用户不受每 IP 配额限制（包括请求频率、小时额度和未完成任务数），但仍受全局队列、缓存预算和状态规模限制。

生成密钥：

```bash
/opt/echo-studio/echo_server generate-key
# Docker：
docker compose run --rm echo echo-server generate-key
```

命令会输出一个访问密钥和对应的摘要，但不会修改任何配置：

1. 私下保存访问密钥；
2. 把摘要写入 `ECHO_ADMIN_KEY_HASHES`（Compose 写在 `.env` 中，systemd 写在 `/etc/echo-studio.env` 中），**不要写入原始密钥**；
3. 重启服务。

之后在网页右上角「访问密钥」中输入密钥即可。密钥只保存在当前标签页中，通过请求头发送，不会出现在 URL 或日志里。请务必通过 HTTPS 访问。

要撤销某个密钥，删除对应的摘要并重启服务。
