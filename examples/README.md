# Confers 示例索引

本目录是 confers 的示例子 crate（包名 `confers-examples`，为 workspace 成员）。全部 21 个示例以 `[[bin]]` 二进制目标的形式组织在 `src/examples/` 下，依赖 `confers = { path = "..", features = ["full"] }`，覆盖库的全部主要功能。

## 环境要求

- Rust 1.97.1+（与 workspace 的 `rust-version` 基线一致）
- 从仓库根目录运行命令（`confers-examples` 是 workspace 成员，无需进入子目录）

## 运行示例

```bash
# 在仓库根目录运行指定示例（统一格式）
cargo run -p confers-examples --bin <示例名>

# 也可以进入本目录后运行
cd examples
cargo run --bin <示例名>

# 快捷脚本：运行单个示例
./run_example.sh basic_usage

# 验证全部示例可编译
./verify_examples.sh
```

## 示例索引

### 快速上手

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `basic_usage` | 基础配置加载：TOML 文件 + 环境变量覆盖 + derive 宏定义配置结构 | `cargo run -p confers-examples --bin basic_usage` |
| `interpolation` | 配置字符串插值：`${VAR}` 语法、`${VAR:default}` 默认值与插值跟踪 | `cargo run -p confers-examples --bin interpolation` |
| `validation` | 基于 garde 的字段级配置校验：内置规则、自定义逻辑与错误处理 | `cargo run -p confers-examples --bin validation` |
| `cli_integration` | `#[derive(ConfigClap)]` 生成 CLI 参数并与配置文件合并（试试追加 `-- --help`） | `cargo run -p confers-examples --bin cli_integration` |
| `json_schema` | `#[derive(ConfigSchema)]` 生成 JSON Schema 与 TypeScript 类型定义 | `cargo run -p confers-examples --bin json_schema` |

### 热重载与渐进发布

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `hot_reload` | 文件监听与热重载：`FsWatcher` 监听变更、防抖与自动重载 | `cargo run -p confers-examples --bin hot_reload` |
| `progressive_reload` | 渐进式重载：`ProgressiveReloader` 健康检查与自动回滚 | `cargo run -p confers-examples --bin progressive_reload` |

### 安全与加密

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `encryption` | 敏感字段加密：XChaCha20-Poly1305 加解密、密钥派生与防日志泄露 | `cargo run -p confers-examples --bin encryption` |
| `key_rotation` | 密钥安全轮转：多版本密钥共存与平滑迁移 | `cargo run -p confers-examples --bin key_rotation` |
| `security` | 安全模块：`EncryptionPrefix` 加密前缀识别与环境变量安全校验 | `cargo run -p confers-examples --bin security` |

### 动态与上下文感知配置

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `dynamic_fields` | `DynamicField` 动态字段：运行时更新、变更回调与无锁读取 | `cargo run -p confers-examples --bin dynamic_fields` |
| `config_groups` | 配置组：分组定义、优先级与按需加载合并 | `cargo run -p confers-examples --bin config_groups` |
| `context_aware` | 上下文感知配置：按 plan/environment/region 解析不同配置值 | `cargo run -p confers-examples --bin context_aware` |

### 迁移、快照与审计

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `migration` | 配置版本迁移：定义版本、编写迁移函数并自动升级旧配置 | `cargo run -p confers-examples --bin migration` |
| `snapshot` | 配置快照持久化：`SnapshotManager` 自动保存、脱敏与历史回溯 | `cargo run -p confers-examples --bin snapshot` |
| `audit` | 审计日志：`AuditConfig` / `AuditWriter` 记录加载、密钥访问与解密事件 | `cargo run -p confers-examples --bin audit` |

### 远程配置

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `remote_consul` | 从 Consul KV 加载配置并定期轮询更新 | `cargo run -p confers-examples --bin remote_consul` |
| `remote_etcd` | 从 etcd KV 加载配置（支持 TLS 与异步构建） | `cargo run -p confers-examples --bin remote_etcd` |

### 消息总线、模块化与综合示例

| 示例 | 说明 | 运行命令 |
|------|------|----------|
| `config_bus` | ConfigBus 配置变更事件广播：`BusBuilder` + `InMemoryBus` 多订阅者演示 | `cargo run -p confers-examples --bin config_bus` |
| `modules_demo` | 模块化配置注册表：`ModuleRegistry` 注册配置组与 profile 切换 | `cargo run -p confers-examples --bin modules_demo` |
| `full_stack` | 综合示例：多来源、热重载、加密、迁移、动态字段、审计与快照 | `cargo run -p confers-examples --bin full_stack` |

## 外部服务

只有两个远程示例依赖外部服务，且未启动服务时示例会优雅降级并提示（使用代码内默认地址，也可用环境变量覆盖）：

### Consul（`remote_consul`）

```bash
# 启动 Consul
docker run -d --name consul -p 8500:8500 consul:latest

# 可选：覆盖默认地址（默认 127.0.0.1:8500）
export CONSUL_ADDRESS=127.0.0.1:8500
```

### etcd（`remote_etcd`）

```bash
# 启动 etcd
docker run -d --name etcd -p 2379:2379 \
  quay.io/coreos/etcd:v3.5 /usr/local/bin/etcd \
  --name s1 --data-dir /etcd-data \
  --listen-client-urls http://0.0.0.0:2379 \
  --advertise-client-urls http://0.0.0.0:2379

# 可选：覆盖默认端点（默认 127.0.0.1:2379）
export ETCD_ENDPOINT=127.0.0.1:2379
```

## 相关文档

- [项目主 README](../README.md)
- [用户指南](../docs/USER_GUIDE.md)
- [API 参考](../docs/API_REFERENCE.md)
- [常见问题 FAQ](../docs/FAQ.md)
- [更新日志](../docs/CHANGELOG.md)
- [安全设计](../docs/SECURITY.md)
- [贡献指南](../docs/CONTRIBUTING.md)

## 贡献新示例

1. 在 `src/examples/` 下新增示例源文件，并在 `Cargo.toml` 中登记对应的 `[[bin]]` 目标
2. 示例必须可直接编译运行，代码注释清晰
3. 通过 `cargo clippy` 与 `cargo fmt` 检查
4. 在本 README 的对应主题分组中补充条目
