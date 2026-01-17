# Confers Examples

本目录包含 confers 配置管理库的完整示例，涵盖了所有功能和特性。

## 快速开始

1. **从基础开始**: 浏览 [01-basics/](src/01-basics/) 目录学习基本配置加载
2. **逐步学习**: 按照数字顺序（01-15）浏览示例，逐步学习高级功能
3. **查看完整示例**: [15-complete/](src/15-complete/) 目录展示如何组合多个特性

## 示例目录

| 目录 | 描述 | 特性 |
|------|------|------|
| [01-basics/](src/01-basics/) | 基础功能：配置加载、默认值、环境变量 | derive |
| [02-validation/](src/02-validation/) | 配置验证功能 | validation |
| [03-formats/](src/03-formats/) | 多格式支持：TOML, JSON, YAML, INI | derive |
| [04-nested/](src/04-nested/) | 嵌套配置结构 | derive |
| [05-watch-reload/](src/05-watch-reload/) | 文件监控和热重载 | watch |
| [06-encryption/](src/06-encryption/) | 配置加密功能 | encryption |
| [07-remote/](src/07-remote/) | 远程配置（HTTP, etcd, Consul） | remote |
| [08-audit/](src/08-audit/) | 审计日志 | audit |
| [09-schema/](src/09-schema/) | Schema 生成 | schema |
| [10-diff/](src/10-diff/) | 配置差分 | - |
| [11-cli-tools/](src/11-cli-tools/) | CLI 工具使用 | cli |
| [12-advanced/](src/12-advanced/) | 高级功能 | parallel, monitoring |
| [13-wizard/](src/13-wizard/) | 交互式向导 | - |
| [14-compat/](src/14-compat/) | 兼容性（config-rs） | - |
| [15-complete/](src/15-complete/) | 完整应用示例 | validation, watch, encryption, remote |

## 运行示例

### 运行特定示例

```bash
# 运行基础示例
cargo run --bin 01-basics-minimal

# 运行需要特定特性的示例
cargo run --bin 01-basics-basic

# 运行加密示例
cargo run --bin 06-encryption-basic_encryption
```

### 编译所有示例

```bash
cargo build --bins
```

### 运行所有示例

```bash
# 编译并运行所有示例
cargo build --bins
```

## 按特性查找

### 核心功能

- **配置加载**: [01-basics/](src/01-basics/)
- **配置验证**: [02-validation/](src/02-validation/)
- **多格式支持**: [03-formats/](src/03-formats/)
- **嵌套配置**: [04-nested/](src/04-nested/)

### 高级功能

- **文件监控**: [05-watch-reload/](src/05-watch-reload/)
- **配置加密**: [06-encryption/](src/06-encryption/)
- **远程配置**: [07-remote/](src/07-remote/)
- **审计日志**: [08-audit/](src/08-audit/)

### 工具和集成

- **Schema 生成**: [09-schema/](src/09-schema/)
- **配置差分**: [10-diff/](src/10-diff/)
- **CLI 工具**: [11-cli-tools/](src/11-cli-tools/)

### 完整应用

- **生产环境**: [15-complete/15-complete-production_app.rs](src/15-complete/15-complete-production_app.rs)
- **开发环境**: [15-complete/15-complete-development_app.rs](src/15-complete/15-complete-development_app.rs)
- **多源配置**: [15-complete/15-complete-multi_source.rs](src/15-complete/15-complete-multi_source.rs)

## 前提条件

- Rust 1.75+
- 根据示例需要启用相应的 Cargo 特性

## 相关文档

- [用户指南](../docs/USER_GUIDE.md)
- [API 参考](../docs/API_REFERENCE.md)
- [FAQ](../docs/FAQ.md)

## 常见问题

### Q: 如何运行需要特定特性的示例？

A: 使用 `cargo run --bin` 命令：
```bash
cargo run --bin 01-basics-basic
```

### Q: 示例配置文件在哪里？

A: 每个示例目录都有独立的 `configs/` 子目录，包含该示例所需的配置文件。

### Q: 如何学习 confers？

A: 建议按照以下顺序学习：
1. [01-basics/](src/01-basics/) - 学习基础配置加载
2. [02-validation/](src/02-validation/) - 学习配置验证
3. [03-formats/](src/03-formats/) - 学习多格式支持
4. 继续浏览其他目录学习高级功能
5. 查看 [15-complete/](src/15-complete/) 了解完整应用

## 贡献

欢迎为示例目录贡献代码！请遵循以下规范：

1. 每个示例都应该是可独立运行的完整程序
2. 包含详细的注释和文档
3. 使用独立的配置文件
4. 在对应的 README.md 中说明示例的目的和运行方式

## 许可证

MIT License - 详见 [LICENSE](../LICENSE) 文件