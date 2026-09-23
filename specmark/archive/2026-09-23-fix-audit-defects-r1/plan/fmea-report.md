# FMEA Failure Mode Analysis Report

Number of failure modes: 52

## Risk Priority Number Ranking

Rank | Failure Mode                                             | S  | O  | D  | RPN | Risk Level | Response Strategy  
-----|----------------------------------------------------------|----|----|----|-----|------------|-------------------------------------------
1    | crypto:encrypt宏属性假加密(密文原样进String字段)                      | 10 | 7  | 9  | 630 | Critical   | Immediate corrective action required  
2    | load:同优先级按source_id字母序默认值覆盖文件+声明顺序失效                     | 9  | 7  | 9  | 567 | Critical   | Immediate corrective action required  
3    | watch:FsWatcher单文件inode监听原子替换后静默永久失聪                     | 8  | 7  | 10 | 560 | Critical   | Immediate corrective action required  
4    | mask:builder自动快照硬编码空脱敏列表明文落盘0644                         | 9  | 6  | 10 | 540 | Critical   | Immediate corrective action required  
5    | load:EnvSource嵌套路径冲突静默丢值随env顺序漂移                         | 8  | 5  | 9  | 360 | Critical   | Immediate corrective action required  
6    | mask:CLI inspect/get无脱敏明文回显                              | 7  | 6  | 8  | 336 | Critical   | Immediate corrective action required  
7    | mask:export默认脱敏仅按值形状password=xxx直通                       | 7  | 6  | 8  | 336 | Critical   | Immediate corrective action required  
8    | crypto:弱密钥(全零/ASCII口令)静默接受+文档示范全零                        | 8  | 5  | 8  | 320 | Critical   | Immediate corrective action required  
9    | watch:事件通道满静默丢事件+回调panic杀死重载循环                           | 7  | 5  | 9  | 315 | Critical   | Immediate corrective action required  
10   | remote:Consul KV删除后空数组永久续命旧配置                            | 8  | 4  | 9  | 288 | Critical   | Immediate corrective action required  
11   | crypto:无encryption特性时enc:字段零警示doctor返回Ok                 | 8  | 4  | 9  | 288 | Critical   | Immediate corrective action required  
12   | remote:Redis总线断线静默终止无重连永久丢事件                             | 6  | 5  | 9  | 270 | Critical   | Immediate corrective action required  
13   | remote:VersionArbitratedBus发布方重启版本号重置事件全丢                | 6  | 5  | 9  | 270 | Critical   | Immediate corrective action required  
14   | crypto:envelope无密钥版本+三套格式互不兼容                            | 7  | 5  | 7  | 245 | Critical   | Immediate corrective action required  
15   | remote:SSRF黑名单缺0.0.0.0/8可打本机任意端口                         | 9  | 3  | 9  | 243 | Critical   | Immediate corrective action required  
16   | load:宏load_file_with_env注入全进程环境deny_unknown必炸            | 8  | 6  | 5  | 240 | Critical   | Immediate corrective action required  
17   | mask:ConfigValue/AnnotatedValue derive(Debug)明文+冲突报告内嵌原值 | 6  | 5  | 8  | 240 | Critical   | Immediate corrective action required  
18   | watch:宏dynamic字段x_handle每次调用返回独立handle                   | 6  | 6  | 6  | 216 | Critical   | Immediate corrective action required  
19   | crypto:Vault token缓存永不失效403不重登永久失败                       | 7  | 6  | 5  | 210 | Critical   | Immediate corrective action required  
20   | crypto:明文密钥逃逸受管类型(派生密钥裸返回等多处)                            | 6  | 5  | 7  | 210 | Critical   | Immediate corrective action required  
21   | load:null覆盖语义根级与map内不一致                                  | 6  | 4  | 8  | 192 | High       | Develop mitigation plan ASAP  
22   | remote:HTTP轮询源默认无超时且无认证支持                                | 6  | 6  | 5  | 180 | High       | Develop mitigation plan ASAP  
23   | remote:Nacos无认证+熔断try_lock争用误报                           | 6  | 5  | 6  | 180 | High       | Develop mitigation plan ASAP  
24   | watch:渐进发布candidate不可观测+rollback/migration死配置            | 6  | 5  | 6  | 180 | High       | Develop mitigation plan ASAP  
25   | remote:运行期容灾兜底缺失(熔断不回缓存/快照失败路径/无restore)                 | 7  | 5  | 5  | 175 | High       | Develop mitigation plan ASAP  
26   | load:文档特性未实现(merge_strategy/profile/RUN_ENV死属性)          | 6  | 4  | 7  | 168 | High       | Develop mitigation plan ASAP  
27   | remote:K8s REST源in-cluster无CA加载TLS必失败且无超时                | 7  | 6  | 4  | 168 | High       | Develop mitigation plan ASAP  
28   | mask:mask_value等长星号泄露精确长度                                | 4  | 5  | 8  | 160 | High       | Develop mitigation plan ASAP  
29   | remote:InvalidValue错误路径未接sanitize完整URL直通日志               | 5  | 4  | 8  | 160 | High       | Develop mitigation plan ASAP  
30   | load:env类型推断与目标字段脱钩报错key为空                               | 5  | 6  | 5  | 150 | High       | Develop mitigation plan ASAP  
31   | load:env双下划线嵌套映射文档不成立且separator不可配                       | 5  | 5  | 6  | 150 | High       | Develop mitigation plan ASAP  
32   | mask:敏感词token边界漏authorization/复数形/passwd/dsn             | 5  | 5  | 6  | 150 | High       | Develop mitigation plan ASAP  
33   | mask:审计HMAC密钥明文同存文件头可整链伪造                                | 6  | 3  | 8  | 144 | High       | Develop mitigation plan ASAP  
34   | watch:InMemoryChangeStream FIFO驱逐慢订阅者丢变更                 | 4  | 4  | 8  | 128 | High       | Develop mitigation plan ASAP  
35   | mask:AuditSink收未脱敏事件与落盘口径不一致                             | 4  | 4  | 8  | 128 | High       | Develop mitigation plan ASAP  
36   | remote:熔断器仅覆盖HTTP/Nacos其余源无                              | 4  | 4  | 8  | 128 | High       | Develop mitigation plan ASAP  
37   | load:插值无转义语法+敏感引用无告警+rename后静默失效                         | 4  | 4  | 8  | 128 | High       | Develop mitigation plan ASAP  
38   | crypto:rotate_master_key不验证旧密钥参数                         | 5  | 3  | 8  | 120 | High       | Develop mitigation plan ASAP  
39   | watch:ConfigImpl overrides TTL 5分钟静默回退                   | 4  | 3  | 9  | 108 | High       | Develop mitigation plan ASAP  
40   | crypto:keys.json/快照/审计文件默认0644无0600                      | 5  | 3  | 7  | 105 | High       | Develop mitigation plan ASAP  
41   | crypto:SecureString masked泄露前2字符+长度特征                    | 3  | 4  | 8  | 96  | Medium     | Include in monitoring and plan improvement
42   | crypto:文档env名错位CONFERS_ENCRYPTION_KEY无消费                 | 3  | 5  | 6  | 90  | Medium     | Include in monitoring and plan improvement
43   | load:MergeEngine无视priority与report_conflict结论矛盾           | 4  | 3  | 7  | 84  | Medium     | Include in monitoring and plan improvement
44   | load:宏_FILE弱化版静默跳过不回退                                    | 4  | 3  | 7  | 84  | Medium     | Include in monitoring and plan improvement
45   | watch:DynamicField并发update回调乱序无测试                        | 3  | 3  | 8  | 72  | Medium     | Include in monitoring and plan improvement
46   | remote:NATS毒消息无限即时重投                                     | 3  | 3  | 7  | 63  | Medium     | Include in monitoring and plan improvement
47   | mask:verify_audit_chain非常量时间比较                           | 3  | 2  | 9  | 54  | Medium     | Include in monitoring and plan improvement
48   | remote:快照写入非原子无tmp+rename                                | 4  | 2  | 6  | 48  | Low        | Routine monitoring  
49   | watch:AdaptiveDebouncer死代码+去抖测试#[ignore]                 | 3  | 5  | 3  | 45  | Low        | Routine monitoring  
50   | load:宏default空值/None产生难懂编译错误                             | 3  | 4  | 3  | 36  | Low        | Routine monitoring  
51   | crypto:secrecy/aes-gcm死依赖                                | 2  | 10 | 1  | 20  | Low        | Routine monitoring  
52   | crypto:KeyCachePolicy无任何缓存实现消费                           | 2  | 4  | 2  | 16  | Low        | Routine monitoring  

## Failure Modes Requiring Priority Action

- **crypto:encrypt宏属性假加密(密文原样进String字段)** (RPN=630, Critical) → Immediate corrective action required
- **load:同优先级按source_id字母序默认值覆盖文件+声明顺序失效** (RPN=567, Critical) → Immediate corrective action required
- **watch:FsWatcher单文件inode监听原子替换后静默永久失聪** (RPN=560, Critical) → Immediate corrective action required
- **mask:builder自动快照硬编码空脱敏列表明文落盘0644** (RPN=540, Critical) → Immediate corrective action required
- **load:EnvSource嵌套路径冲突静默丢值随env顺序漂移** (RPN=360, Critical) → Immediate corrective action required
- **mask:CLI inspect/get无脱敏明文回显** (RPN=336, Critical) → Immediate corrective action required
- **mask:export默认脱敏仅按值形状password=xxx直通** (RPN=336, Critical) → Immediate corrective action required
- **crypto:弱密钥(全零/ASCII口令)静默接受+文档示范全零** (RPN=320, Critical) → Immediate corrective action required
- **watch:事件通道满静默丢事件+回调panic杀死重载循环** (RPN=315, Critical) → Immediate corrective action required
- **remote:Consul KV删除后空数组永久续命旧配置** (RPN=288, Critical) → Immediate corrective action required
- **crypto:无encryption特性时enc:字段零警示doctor返回Ok** (RPN=288, Critical) → Immediate corrective action required
- **remote:Redis总线断线静默终止无重连永久丢事件** (RPN=270, Critical) → Immediate corrective action required
- **remote:VersionArbitratedBus发布方重启版本号重置事件全丢** (RPN=270, Critical) → Immediate corrective action required
- **crypto:envelope无密钥版本+三套格式互不兼容** (RPN=245, Critical) → Immediate corrective action required
- **remote:SSRF黑名单缺0.0.0.0/8可打本机任意端口** (RPN=243, Critical) → Immediate corrective action required
- **load:宏load_file_with_env注入全进程环境deny_unknown必炸** (RPN=240, Critical) → Immediate corrective action required
- **mask:ConfigValue/AnnotatedValue derive(Debug)明文+冲突报告内嵌原值** (RPN=240, Critical) → Immediate corrective action required
- **watch:宏dynamic字段x_handle每次调用返回独立handle** (RPN=216, Critical) → Immediate corrective action required
- **crypto:Vault token缓存永不失效403不重登永久失败** (RPN=210, Critical) → Immediate corrective action required
- **crypto:明文密钥逃逸受管类型(派生密钥裸返回等多处)** (RPN=210, Critical) → Immediate corrective action required
- **load:null覆盖语义根级与map内不一致** (RPN=192, High) → Develop mitigation plan ASAP
- **remote:HTTP轮询源默认无超时且无认证支持** (RPN=180, High) → Develop mitigation plan ASAP
- **remote:Nacos无认证+熔断try_lock争用误报** (RPN=180, High) → Develop mitigation plan ASAP
- **watch:渐进发布candidate不可观测+rollback/migration死配置** (RPN=180, High) → Develop mitigation plan ASAP
- **remote:运行期容灾兜底缺失(熔断不回缓存/快照失败路径/无restore)** (RPN=175, High) → Develop mitigation plan ASAP
- **load:文档特性未实现(merge_strategy/profile/RUN_ENV死属性)** (RPN=168, High) → Develop mitigation plan ASAP
- **remote:K8s REST源in-cluster无CA加载TLS必失败且无超时** (RPN=168, High) → Develop mitigation plan ASAP
- **mask:mask_value等长星号泄露精确长度** (RPN=160, High) → Develop mitigation plan ASAP
- **remote:InvalidValue错误路径未接sanitize完整URL直通日志** (RPN=160, High) → Develop mitigation plan ASAP
- **load:env类型推断与目标字段脱钩报错key为空** (RPN=150, High) → Develop mitigation plan ASAP
- **load:env双下划线嵌套映射文档不成立且separator不可配** (RPN=150, High) → Develop mitigation plan ASAP
- **mask:敏感词token边界漏authorization/复数形/passwd/dsn** (RPN=150, High) → Develop mitigation plan ASAP
- **mask:审计HMAC密钥明文同存文件头可整链伪造** (RPN=144, High) → Develop mitigation plan ASAP
- **watch:InMemoryChangeStream FIFO驱逐慢订阅者丢变更** (RPN=128, High) → Develop mitigation plan ASAP
- **mask:AuditSink收未脱敏事件与落盘口径不一致** (RPN=128, High) → Develop mitigation plan ASAP
- **remote:熔断器仅覆盖HTTP/Nacos其余源无** (RPN=128, High) → Develop mitigation plan ASAP
- **load:插值无转义语法+敏感引用无告警+rename后静默失效** (RPN=128, High) → Develop mitigation plan ASAP
- **crypto:rotate_master_key不验证旧密钥参数** (RPN=120, High) → Develop mitigation plan ASAP
- **watch:ConfigImpl overrides TTL 5分钟静默回退** (RPN=108, High) → Develop mitigation plan ASAP
- **crypto:keys.json/快照/审计文件默认0644无0600** (RPN=105, High) → Develop mitigation plan ASAP
