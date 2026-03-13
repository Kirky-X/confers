# Confers Examples

本目录包含 confers 项目的完整功能示例，展示如何在实际项目中使用 confers 的各项功能。

## 目录结构

所有示例整合为一个统一的 Rust 项目，通过 binary targets 组织：

```
examples/
├── Cargo.toml              # 项目配置（含 13 个 [[bin]] targets）
├── src/
│   ├── examples/           # 示例源码
│   │   ├── basic_usage.rs
│   │   ├── hot_reload.rs
│   │   ├── remote_consul.rs
│   │   ├── remote_etcd.rs
│   │   ├── encryption.rs
│   │   ├── key_rotation.rs
│   │   ├── migration.rs
│   │   ├── dynamic_fields.rs
│   │   ├── config_groups.rs
│   │   ├── progressive_reload.rs
│   │   ├── config_bus.rs
│   │   ├── snapshot.rs
│   │   └── full_stack.rs
│   └── main.rs            # 默认入口（显示帮助信息）
└── config/                # 配置文件目录
```

## 运行示例

```bash
# 进入 examples 目录
cd examples

# 运行指定示例
cargo run --bin basic_usage
cargo run --bin hot_reload
cargo run --bin encryption
cargo run --bin config_bus
cargo run --bin snapshot

# 或者使用快捷脚本
./run_example.sh basic_usage

# 验证所有示例编译
./verify_examples.sh
```

## 示例列表

| 示例 | 功能描述 | 所需 Features | ADR |
|------|---------|--------------|-----|
| basic_usage | 基础配置加载和使用 | toml, env | - |
| hot_reload | 热重载功能演示 | toml, watch | ADR-013 |
| remote_consul | Consul 远程配置源使用 | toml, consul | ADR-005 |
| remote_etcd | etcd 远程配置源使用 | toml, remote | ADR-005, ADR-037 |
| encryption | 敏感字段加密功能 | toml, encryption | ADR-006 |
| key_rotation | 密钥轮换功能 | toml, encryption | ADR-015 |
| migration | 配置版本迁移 | toml, migration | ADR-027 |
| dynamic_fields | DynamicField 动态字段功能 | toml, dynamic | ADR-031 |
| config_groups | 配置组功能 | toml | ADR-034 |
| progressive_reload | 渐进式重载功能 | toml, progressive-reload | ADR-036 |
| config_bus | ConfigBus 多实例广播 | toml, config-bus | ADR-035 |
| snapshot | 配置快照持久化 | toml, snapshot | ADR-033 |
| full_stack | 完整功能栈综合示例 | full | - |

## 示例说明

### basic_usage
展示最基本的配置加载功能：
- 从 TOML 文件加载配置
- 从环境变量覆盖配置
- 使用 derive 宏定义配置结构
- 访问配置值

### hot_reload
演示文件监听和热重载：
- 监听配置文件变化
- 自动重新加载配置
- 使用 watcher guard 管理生命周期
- 防抖动配置

### remote_consul
展示如何从 Consul 加载配置：
- 配置 Consul 连接
- 从 Consul KV 读取配置
- 定期轮询更新
- 认证和 TLS 配置

### remote_etcd
展示如何从 etcd 加载配置：
- etcd v3 API 连接
- TLS 安全连接
- 配置监听和自动更新
- 租约和 TTL 管理

### encryption
展示敏感字段加密：
- 使用 SecretString 保护敏感数据
- 配置加密密钥
- 加密/解密配置字段
- 防止日志泄露

### key_rotation
演示密钥轮换机制：
- 配置多个版本的密钥
- 密钥轮换策略
- 平滑迁移到新密钥
- 回滚机制

### migration
展示配置版本迁移：
- 定义配置版本
- 编写迁移函数
- 自动迁移旧配置
- 迁移注册表

### dynamic_fields
演示动态字段功能：
- 使用 DynamicField 包装字段
- 注册变更回调
- 无锁读取性能
- RAII 回调管理

### config_groups
展示配置组功能：
- 定义多个配置组
- 配置组优先级
- 按需加载配置组
- 配置组合并

### progressive_reload
演示渐进式重载：
- 金丝雀部署
- 线性推出
- 健康检查
- 自动回滚

### config_bus
演示 ConfigBus 多实例广播：
- NATS 消息总线集成
- Redis Pub/Sub 集成
- 配置变更事件广播
- 多实例配置一致性保证

### snapshot
演示配置快照持久化：
- 配置快照自动保存
- 快照时间戳命名
- 敏感字段脱敏
- 快照历史管理
- 快照对比和回溯

### full_stack
综合示例，展示所有功能：
- 多源配置
- 热重载
- 加密
- 迁移
- 动态字段
- 审计日志
- 快照

## 项目结构

统一项目结构：

```
examples/
├── Cargo.toml              # workspace + 13 个 [[bin]] targets
├── src/
│   ├── examples/           # 示例源码（13 个 .rs 文件）
│   │   ├── basic_usage.rs
│   │   ├── hot_reload.rs
│   │   └── ...
│   └── main.rs            # 默认入口
└── config/                # 配置文件目录
```

## 依赖说明

项目依赖在 Cargo.toml 中统一管理：

```toml
[dependencies]
confers = { path = "..", features = ["full"] }
```

Features 按示例分组启用，可以在 Cargo.toml 中查看具体配置。

## 外部服务

部分示例需要外部服务支持：

### Consul
```bash
# 启动 Consul
docker run -d --name consul -p 8500:8500 consul:latest

# 设置环境变量
export CONSUL_ADDRESS=http://127.0.0.1:8500
```

### etcd
```bash
# 启动 etcd
docker run -d --name etcd -p 2379:2379 -p 2380:2380 \
  quay.io/coreos/etcd:v3.5 /usr/local/bin/etcd \
  --name s1 --data-dir /etcd-data \
  --listen-client-urls http://0.0.0.0:2379 \
  --advertise-client-urls http://0.0.0.0:2379

# 设置环境变量
export ETCD_ENDPOINTS=http://127.0.0.1:2379
```

### NATS
```bash
# 启动 NATS
docker run -d --name nats -p 4222:4222 nats:latest

# 设置环境变量
export NATS_URL=nats://127.0.0.1:4222
```

## 最佳实践

1. **统一管理**：所有示例在一个项目中，便于编译和测试
2. **完整可运行**：所有示例都可以直接编译和运行
3. **详细注释**：代码中包含详细的功能说明
4. **错误处理**：展示正确的错误处理方式
5. **日志输出**：使用 tracing 展示运行状态
6. **ADR 引用**：每个示例标注对应的设计决策记录

## 扩展阅读

- [Confers API 文档](https://docs.rs/confers)
- [项目主 README](../README.md)
- [开发指南](../dev-v2.md)
- [ADR 文档](../docs/adr/)

## 贡献

欢迎贡献新的示例！请遵循以下规范：

1. 示例应该是可运行的
2. 代码注释清晰
3. 遵循 Rust 最佳实践
4. 通过 `cargo clippy` 和 `cargo fmt` 检查
5. 在 README.md 中添加示例说明
6. 标注对应的 ADR 编号
