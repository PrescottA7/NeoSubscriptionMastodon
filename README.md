# NSM (NeoSubscriptionMastodon)

代理订阅管理服务。管理员在 Web 控制台维护用户与节点，订阅者只需拿到一条订阅链接。
同一条节点定义可以无损导出为 **V2RayN/URI、sing-box、Mihomo、Xray、Hysteria2** 五种客户端配置。

- 单个可执行文件，管理界面在编译期内嵌
- SQLite 存储，无需外部数据库
- 所有非管理路径统一返回伪装 404 页面
- 三阶段 Dockerfile + Compose 部署

## 支持的协议

| 协议 | 参数覆盖 |
| --- | --- |
| VLESS | `flow`(xtls-rprx-vision)、`packetEncoding`、VLESS Encryption、REALITY(`pbk`/`sid`/`spx`/`pqv`)、全部传输层 |
| Trojan | TLS、传输层、UDP 开关、多路复用 |
| Hysteria2 | 端口跳跃(`server_ports`/`hop_interval`/`hop_interval_max`)、Brutal 带宽、Salamander/Gecko 混淆、Realm、QUIC 窗口 |
| TUIC | 拥塞控制(cubic/new_reno/bbr)、UDP 中继模式、UDP over Stream、0-RTT、心跳、QUIC 窗口 |
| AnyTLS | 空闲会话检查/超时、最少空闲会话、客户端元数据 |

传输层支持 `tcp`(含 HTTP 伪装头)、`ws`(early data)、`grpc`(service name/mode/authority)、
`http`(H2)、`httpupgrade`、`xhttp`(mode + `extra`：`xPaddingBytes`、`scMaxEachPostBytes`、
`scMinPostsIntervalMs`、`noSSEHeader`、`XMUX`、额外请求头)。

TLS 通用参数覆盖 SNI、ALPN、uTLS 指纹、证书 SHA256 锁定、证书名称校验、TLS 版本范围。

## 快速开始

需要 Rust stable、Node.js 20+、pnpm。

```sh
cd frontend
pnpm install
pnpm run build
cd ..

cargo build --release
./target/release/nsm            # Windows: .\target\release\nsm.exe
```

首次启动会在当前目录生成带注释的 `config.yml` 与 `data/`。打开 `http://127.0.0.1:3000`
创建管理员账户。

也可以直接用 cargo 驱动前端构建（需要联网安装依赖）：

```sh
NSM_FRONTEND_INSTALL=1 cargo build --release
```

### config.yml

首次启动自动生成，带注释说明：

```yaml
# Address the HTTP server binds to.
bind_addr: 0.0.0.0:3000

# Directory holding the SQLite database, the encryption key and logs.
data_root: data

# Mark the admin session cookie as `Secure`.
# Enable ONLY when the admin UI is served over HTTPS; over plain HTTP the
# browser silently drops the cookie and login appears to fail.
cookie_secure: false

# Optional: path to a custom 404 page used to disguise the service.
# disguise_page: ./404.html
```

> **登不上先看这里**：如果通过 HTTPS 反向代理访问，把 `cookie_secure` 设为 `true`；
> 如果直接访问 HTTP 却设成了 `true`，浏览器会丢弃会话 cookie，表现为「登录成功但立刻掉回登录页」。
> 程序启动时会检测这种组合并在日志里输出警告。

### 日志

- 在终端里运行，或设置 `NSM_LOG_STDOUT=1`（`true` 亦可）时输出到 stdout（`RUST_LOG` 可调，默认 `info`，`sqlx` 降为 `warn`）。Docker 镜像默认开启，可用 `docker compose logs` 查看
- **双击启动或重定向输出时**（且未设置 `NSM_LOG_STDOUT`）：自动写入 `<data_root>/logs/nsm.log`，超过 10 MiB 轮转为 `nsm.log.1`
- 启动时会打印日志文件位置、监听地址，以及 `config.yml` 是否为本次生成
- 每个请求都会记录状态码与耗时，便于排查登录问题
- 端口被占用等致命错误会写到 stderr 并退出码 1

## Docker

```sh
docker compose up -d --build
```

服务监听 `3000`，数据保存在 `nsm-data` 卷。环境变量：

| 变量 | 说明 |
| --- | --- |
| `NSM_PORT` | 宿主机映射端口，默认 3000 |
| `NSM_COOKIE_SECURE` | `true` 时会话 cookie 带 `Secure`，需在 HTTPS 后使用 |
| `NSM_LOG_STDOUT` | `true`/`1` 时日志输出到 stdout，默认 `true`；设为 `false` 则写入 `data/logs/nsm.log` |

镜像以非 root 用户运行，内置 `HEALTHCHECK`。

## 订阅

用户只是订阅者，**不持有代理凭据**。每个节点有自己的账号表，所以同一个人在不同节点上可以有
完全不同的 UUID 和密码——这正是真实代理服务端的形态。凭据保存在 `node_credentials(node_id, user_id, uuid, password, email, enabled)`：

- `uuid` 用于 VLESS 和 TUIC
- `password` 用于 Trojan、Hysteria2、TUIC、AnyTLS（加密存储）
- `email` 是客户端显示的账号标签，默认 `备注@节点名`
- 一行凭据**同时就是授权**：没有这一行的用户不会收到该节点，不存在"看得见但用不了"的中间态

在节点编辑页的「用户凭据」卡片里维护。UUID / 密码留空会在保存时自动生成，也可以把服务端已有的值
粘回来覆盖；移除某一行会立即让该用户失去这个节点。

订阅链接形如 `https://<域名>/<前缀>/<令牌>`，默认前缀 `/s`，可在设置中修改。
令牌只存哈希，明文仅在创建用户或重置链接时显示一次，重置会立刻使旧链接失效。

## 节点配置

一个节点由**两半**组成，分开存储是有意为之：

| | 存在哪 | 描述什么 | 谁需要 |
| --- | --- | --- | --- |
| 客户端参数 `params` | `nodes.params` | 客户端怎么连：协议、TLS、REALITY 公钥、传输方式 | 订阅、客户端配置导出 |
| 服务端参数 `server_params` | `nodes.server_params` | 这台机器怎么跑：监听、证书、REALITY **私钥**、回落 | `xray -c` / `hysteria2 -c` |

分开的直接原因是 REALITY：服务端持有 `privateKey`，客户端只应拿到由它推导出的 `publicKey`。
把两者塞进一个结构里，迟早会把私钥写进订阅。

服务端配置在节点编辑页的「服务端配置」卡片里维护，支持：

- 监听地址 / 端口 / 入站标签 / 流量嗅探
- 传输方式（tcp / ws / grpc / httpupgrade / xhttp）与对应的 path、gRPC 服务名
- 证书文件路径、ALPN（REALITY 不需要证书）
- REALITY：私钥、`serverNames`、`target`、`shortIds`、`maxTimeDiff`、ML-DSA-65 seed
- 回落（fallbacks）

### 导出可运行的配置

```
GET /api/nodes/{id}/server-config            # JSON，内联返回
GET /api/nodes/{id}/server-config?download=1 # 附件下载
```

`xray` 目标产出完整 JSON（`inbounds` + `freedom` 出站 + 路由），`clients` 数组由该节点的
`node_credentials` 生成，只包含已启用的账号。`hysteria2` 目标产出 YAML，账号写进 `auth.userpass`。

导出结果已用真实二进制验证：`xray run -test` 返回 `Configuration OK.`，`hysteria2 server` 正常启动。

### 格式协商

| 方式 | 说明 |
| --- | --- |
| `?target=sing-box` | 完整 sing-box 配置 |
| `?target=mihomo` | 完整 Mihomo 配置 |
| `?target=xray` | 完整 Xray 配置 |
| `?target=hysteria2` | Hysteria2 客户端配置 |
| `?target=uri` | 订阅链接列表（默认经过 Base64 编码） |
| 不指定 | 按 `User-Agent` 判断客户端，无法判断时用设置中的默认格式 |
| `&download=1` | 以附件形式返回 |

常见别名同样可用：`clash`、`clash-meta` → mihomo，`v2rayn`、`shadowrocket` → uri，`hy2` → hysteria2。

## 伪装

订阅端点的所有失败路径——令牌不存在、用户被禁用、没有可用节点、数据库异常——都返回
`404.html`，并移除 `X-Powered-By`、`Server` 等指纹头。非管理路径的任意请求同样落到该页面，
管理 API 之外没有可探测的应用端点。

可在设置页替换伪装页面 HTML，或通过 `failure_delay_ms` 为失败响应增加延迟。

## 管理员接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `GET` | `/api/auth/setup-status` | 是否需要初始化 |
| `POST` | `/api/auth/setup` | 创建管理员 |
| `POST` | `/api/auth/login` / `/api/auth/logout` | 登录 / 登出 |
| `POST` | `/api/auth/password` | 修改密码（注销所有会话） |
| `GET/POST` | `/api/users` | 用户列表 / 创建 |
| `PATCH/DELETE` | `/api/users/{id}` | 修改 / 删除 |
| `POST` | `/api/users/{id}/reset-link` | 重置订阅令牌 |
| `GET` | `/api/users/{id}/credentials` | 该用户在所有节点上的凭据 |
| `GET/POST` | `/api/nodes` | 节点列表 / 创建 |
| `GET/PUT/DELETE` | `/api/nodes/{id}` | 节点详情 / 更新 / 删除 |
| `GET` | `/api/nodes/{id}/credentials` | 该节点的凭据列表 |
| `GET` | `/api/nodes/{id}/suggested-credentials` | 为每个用户预生成一组值 |
| `POST` | `/api/nodes/{id}/credentials/{user_id}/rotate` | 重新生成某用户的 UUID 与密码 |
| `GET` | `/api/nodes/{id}/render?core=…` | 单节点客户端配置导出 |
| `GET` | `/api/nodes/{id}/server-config` | 单节点**服务端**配置导出（可 `&download=1`） |
| `GET` | `/api/render?core=…` | 全量节点配置导出 |
| `GET/PUT` | `/api/settings` | 读取 / 更新设置 |
| `PUT` | `/api/settings/disguise-page` | 更新伪装页面 |

管理接口使用 HttpOnly 会话 cookie；令牌只在数据库里以 SHA-256 保存。

### 错误响应

所有出错路径都返回同一种 JSON 结构：

```json
{
  "error": "端口必须是 1-65535 之间的整数（共 3 项）",
  "details": [
    { "field": "port", "message": "端口必须是 1-65535 之间的整数" },
    { "field": "name", "message": "节点名称不能为空" }
  ],
  "request_id": "3f9c1a0b"
}
```

| 字段 | 说明 |
| --- | --- |
| `error` | 始终可以展示给用户。校验失败时取第一条错误的摘要 |
| `details` | 逐字段错误，`field` 为点号路径（如 `params.tls.reality.public_key`），前端据此把提示挂到对应输入框 |
| `request_id` | 仅 500 响应出现，用于在服务端日志中定位真正的失败原因 |

规则：

- **422** 表示请求格式正确但内容不合法，`details` 会一次性列出全部问题，前端与后端使用同一套规则（[validate.rs](./src/validate.rs) 与 [validation.ts](./frontend/src/validation.ts)）。
- **4xx** 的 `error` 是服务端明确选择的消息（如「用户 3 不存在」），可以原样展示。
- **500** 的 `error` 固定为「服务器内部错误，请稍后重试」，真实原因（SQL、路径等）只写入日志，避免通过错误信息泄露内部结构。

## 数据目录

```
data/
├─ nsm.sqlite3          # 用户、节点、节点凭据、会话、设置
├─ encryption.key       # 加密节点密码的 32 字节密钥
└─ logs/nsm.log         # 非终端且未设置 NSM_LOG_STDOUT 时写入，超过 10 MiB 轮转
```

`encryption.key` 与数据库必须一起备份，否则 `node_credentials` 中保存的密码无法解密。

### 从旧版本升级

早期版本的 `users` 表带有全局 `uuid` / `password`，并靠 `node_users` 表控制可见性。
启动时会自动迁移：按旧的可见关系为每个 `(节点, 用户)` 生成一行 `node_credentials`，
复制原来的凭据，然后删除 `node_users`。旧的用户列会被保留但不再读取。

迁移只在 `node_users` 存在时执行一次，日志里会出现 `migrated legacy credentials`。

## 目录结构

```
.
├─ Cargo.toml / build.rs      # 单一 crate，build.rs 构建并内嵌前端
├─ 404.html                   # 默认伪装页面
├─ src/
│  ├─ protocol/               # 协议强类型模型（shared + kinds）
│  ├─ render/                 # uri / sing-box / mihomo / xray / hysteria2 渲染器
│  ├─ routes/                 # 管理 API 与订阅端点
│  ├─ db/                     # sea-orm 实体与数据访问
│  ├─ auth.rs crypto.rs       # 会话与凭据加密
│  ├─ disguise.rs             # 伪装响应
│  └─ state.rs config.rs      # 应用状态与配置
├─ tests/api.rs               # 端到端测试
└─ frontend/                  # React + MUI + Redux 管理界面
```

## 开发

```sh
cargo test            # 单元测试 + 端到端测试
cargo clippy --all-targets
cd frontend && pnpm lint && pnpm test
```

前端开发服务器（`pnpm dev`，端口 5173）会把 `/api` 代理到 `127.0.0.1:3000`。
