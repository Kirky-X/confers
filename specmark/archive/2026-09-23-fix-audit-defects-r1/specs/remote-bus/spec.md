# Spec — remote-bus

> Delta spec for change `fix-audit-defects-r1`. 覆盖此变更引入/修改的远程配置源、SSRF 防护与消息总线行为需求。

## Requirements

### R-remote-001: SSRF 黑名单完备
IP 黑名单包含 `0.0.0.0/8`、IPv6 unspecified `::/128`、NAT64 `64:ff9b::/96`，与 `security/rules/ssrf.rs` 共用单一来源实现。
**验收标准：**
- `https://0.0.0.0:PORT`、十进制 `https://0`、`::` 均被拒绝
- 既有 rebinding/重定向防护测试不回归

### R-remote-002: Consul 删除语义
recurse 查询空数组表示该前缀 KV 已删除：推进 last_index 并产出空配置集；仅当 X-Consul-Index 与上次一致才视为无变化返回缓存。
**验收标准：**
- mock 删除 KV 后 poll 返回空配置且 index 前进
- 正常阻塞查询（非空）行为不回归

### R-remote-003: K8s in-cluster 可用
REST 源 in-cluster 默认路径加载 SA ca.crt 并应用默认超时。
**验收标准：**
- client 构造断言含根证书与超时（connect 10s/total 30s）

### R-remote-004: HTTP 源默认超时与认证头
builder 未设置时应用默认超时；`with_header` 注入认证头且值不出现在 Debug/日志。
**验收标准：**
- 默认构造的 client 超时断言；自定义 header 出现在请求（mock 断言）

### R-remote-005: 容灾兜底可选项
`stale_on_error(true)` 时熔断打开/请求失败返回 cached 并附 warning 标记；默认 false 保持 fail-loud。build 失败路径尝试写快照；CLI 提供 `snapshot restore`。
**验收标准：**
- 默认行为既有 fail-loud 测试不回归
- stale_on_error 开启后失败返回旧值+标记
- 失败路径产生快照文件；restore 子命令可加载合法快照、拒绝损坏快照

### R-remote-006: Redis 总线断线重连
订阅流意外终止后自动重连（指数退避 1s→30s）并产生 warn 日志。
**验收标准：**
- mock 断线后重连成功且后续消息可达

### R-remote-007: 总线版本 epoch
事件携带发布方 epoch，仲裁器按 (publisher_id, epoch) 去重；发布方重启（新 epoch、seq 重置）后事件被接受。
**验收标准：**
- 新 epoch seq=1 事件在旧轨 seq 高位之后仍投递

### R-remote-008: Nacos 认证与熔断公平性
username/password 登录换 accessToken 附带请求，401 重登一次；熔断器 try_lock 争用记 unknown 不计失败不误报 open。
**验收标准：**
- mock 登录后请求带 token；token 失效自动重登一次
- 并发争用下熔断器不进入 open

### R-remote-009: 熔断全覆盖与毒消息上限
etcd/Consul/K8s REST 源接入熔断器；NATS consumer max_deliver=5 且反序列化失败 Nak 带 5s 延迟。
**验收标准：**
- 每源连续失败达到阈值后 poll 快速失败（open）
- NATS 毒消息在第 5 次投递后不再即时重投

### R-remote-010: 快照原子写
快照落盘 tmp+rename，不存在半截目标文件窗口。
**验收标准：**
- 写入中断（tmp 存在）时目标路径无残留；成功后 tmp 清理

## Constraints
- 默认行为向后兼容（stale_on_error 默认 false；fail-loud 测试不修改断言语义）。
- 远程源请求头中的凭据不得进入错误信息与日志。

## Out of Scope
- Redis Streams 升级、长连接推送、服务端灰度 API。
