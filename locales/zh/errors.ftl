# confers 错误目录(zh)。与 en/errors.ftl 键一一对应。
#
# 除 ConfigError 变体键外,此处还保存组件消息键: error-* 为在构造点本地化、
# 内嵌进 ConfigError 字段的消息片段及 CryptoError 变体键, log-* 为运维日志行键。

error-file-not-found = 配置文件未找到: { $filename }
error-parse-error = 解析 { $format } 失败: { $message }
error-parse-error-at = 解析 { $format } 失败(位置 { $location }): { $message }
error-validation-failed = 字段 '{ $field }' 校验失败: { $message }(规则: { $rule })
error-schema-validation-failed = schema 校验发现 { $count } 个错误
error-decryption-failed = 解密失败。
error-remote-unavailable = 远程配置源不可用
error-version-mismatch = 配置版本不匹配: 期望 { $expected },实际 { $found }
error-migration-failed = 从 v{ $from } 迁移到 v{ $to } 失败: { $reason }
error-module-not-found = 组 '{ $group }' 中未找到模块 '{ $module }'
error-reload-rolled-back = 配置重载已回滚: { $reason }
error-io = IO 错误: { $message }
error-invalid-value = 配置项 '{ $key }' 的值无效: { $message }
error-source-chain = 配置源链错误: { $message }
error-timeout = 操作在 { $duration_ms }ms 后超时
error-size-limit-exceeded = 配置大小超出限制: { $actual } 字节(上限: { $limit })
error-interpolation = 变量 '{ $variable }' 插值错误: { $message }
error-key = 加密密钥错误。
error-circular-reference = 检测到循环引用: { $path }
error-lock-poisoned = 资源 '{ $resource }' 的锁已中毒
error-multi-source = 多个配置源加载失败
error-multi-source-detail = 多个配置源加载失败: { $failed }/{ $total }
error-concurrency-conflict = 配置键 '{ $key }' 发生并发冲突: { $message }
error-key-rotation-failed = 密钥轮换从 '{ $from_version }' 到 '{ $to_version }' 失败: { $reason }
error-watcher = 配置监视器错误: { $message }
error-override-blocked = 配置键 '{ $key }' 的覆盖被阻止: { $reason }
error-health-check-failed = 健康检查失败: { $reason }

# --- 组件消息片段(内嵌进 ConfigError 字段) ---

error-env-path-conflict = 变量映射到配置路径 '{ $path }',与 { $existing } 在 '{ $at }' 处冲突; 请重命名其中一个冲突的变量
error-conflict-existing-nested-map = 由其他变量构建的嵌套映射
error-conflict-existing-scalar = 由其他变量设置的标量值
error-env-file-unreadable = 无法读取 { $var } 引用的文件
error-spawn-blocking-failed = spawn_blocking: { $message }
error-reload-precommit-validation-failed = 提交前校验失败: { $reason }
error-reload-linear-step-rolled-back = 线性步骤 { $step }: { $reason }
error-reload-linear-step-failed = 线性步骤 { $step } 失败: { $reason }

error-path-empty = 文件路径不能为空
error-path-cannot-resolve = 无法解析文件路径
error-path-access-denied = 不允许访问 { $path }
error-path-credential-dir-denied = 不允许访问凭据目录(.ssh、.aws、.gnupg、.kube、.gcloud、.env)
error-path-not-regular-file = 只能读取常规文件
error-path-extension-denied = 文件扩展名 { $ext } 不在允许范围内

error-decrypt-key-derivation-failed = 密钥派生失败
error-decrypt-payload-not-base64 = 载荷不是有效的 base64
error-decrypt-payload-too-short = 载荷太短,不足以容纳 nonce
error-decrypt-open-failed = 解密失败
error-decrypt-plaintext-not-utf8 = 明文不是有效的 UTF-8
error-decrypt-malformed-envelope = 信封格式损坏
error-decrypt-master-key-missing = 主密钥缺失

error-key-plaintext-not-utf8 = 明文不是有效的 UTF-8: { $message }
error-key-store-temp-open-failed = 打开临时密钥库失败: { $message }
error-key-rotation-verification-failed = 主密钥轮换被拒绝: 旧主密钥校验失败
error-key-file-weak-material = 密钥文件包含弱密钥材料(常量字节); 请改用随机生成的密钥

error-crypto-encryption-failed = 加密失败
error-crypto-decryption-failed = 解密失败
error-crypto-invalid-key-length = 密钥长度无效: XChaCha20-Poly1305 要求恰好 32 字节,实际为 { $actual } 字节
error-crypto-weak-key = 弱密钥已被拒绝(全零等常量字节密钥材料不安全)
error-crypto-key-not-found = 未找到密钥
error-crypto-legacy-decryption-failed = 旧版解密失败(AES-256-GCM)

error-redis-pubsub-connect-timeout = redis_pubsub: 连接超时 (10s)
error-redis-pubsub-connect-failed = redis_pubsub: { $message }
error-redis-subscribe-failed = redis_subscribe: { $message }
error-vault-token-not-provided = 未提供 Vault token
error-vault-request-failed = vault_request: { $message }

error-k8s-ca-read-failed = 读取 Kubernetes CA 文件 '{ $path }' 失败: { $message }
error-k8s-ca-no-pem-block = '{ $path }' 中的 Kubernetes CA 证书无效: 未找到 PEM 证书块
error-k8s-ca-invalid = '{ $path }' 中的 Kubernetes CA 证书无效: { $message }
error-k8s-client-build-failed = 构建 k8s HTTP 客户端失败: { $message }
error-k8s-circuit-breaker-lock-poisoned = k8s 熔断器锁已中毒

error-nacos-auth-password-missing = nacos 认证需要同时提供用户名和密码; 缺少密码
error-nacos-auth-username-missing = nacos 认证需要同时提供用户名和密码; 缺少用户名
error-nacos-login-request-failed = nacos 登录请求失败
error-nacos-login-rejected = nacos 登录被状态码 { $status } 拒绝
error-nacos-login-body-unreadable = nacos 登录返回了无法解析的响应体
error-nacos-login-no-token = nacos 登录响应中没有 accessToken
error-nacos-request-failed = nacos 请求失败: { $message }
error-nacos-unauthorized = nacos 返回 401 Unauthorized (accessToken 被拒绝)
error-nacos-circuit-breaker-open = nacos 配置源熔断器已打开

# --- 变更流错误 (ChangeStreamError) ---

error-stream-lagged = 订阅者已落后: { $from } 之前的版本已被逐出
error-stream-version-not-found = 版本 { $version } 未在此流上发布过

# --- 运维日志行 ---

log-redis-pubsub-connection-lost = Redis 配置总线 pubsub 连接丢失(通道 '{ $channel }'); 将在 { $backoff } 后重连
log-redis-pubsub-reconnect-failed = Redis 配置总线 pubsub 重连失败(通道 '{ $channel }'): { $message }; 将在 { $backoff } 后重试
log-fs-watch-failed = 监视 { $path } 失败: { $message }
log-fs-watcher-parent-removed = 文件监视器: 被监视的父目录 { $path } 已被移除; 内核监视已失效且无法重新建立
log-fs-watcher-channel-full = 文件监视器事件通道已满; 已丢弃 { $path } 的变更事件(累计丢弃: { $total })
log-reload-validation-commit-anyway = 重载校验失败但 rollback_on_validation_failure 已禁用; 仍将提交: { $reason }
log-post-commit-migration-failed = 提交后迁移 { $from }→{ $to } 失败(配置保持已提交状态): { $message }
