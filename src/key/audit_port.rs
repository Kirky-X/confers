// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 轮换事件出口端口（对象安全）。
//!
//! `KeyManager::rotate_key` 与 `KeyRotationService::execute_rotation` 经此
//! 端口发射 `AuditEvent::KeyRotation`；默认 [`NoopRotationAudit`] 零行为。
//! 端口签名引用 [`AuditEvent`]，而该类型仅在 `audit` feature 下编译，
//! 因此整个模块随 `audit` 门控——未启用 audit 时轮换行为与现状完全一致。

use crate::impl_::audit::AuditEvent;

/// 轮换事件出口端口（对象安全，可经 `Arc<dyn RotationAuditPort>` 注入）。
pub trait RotationAuditPort: Send + Sync {
    /// 记录一条轮换相关审计事件。实现方自行决定容错策略，不得 panic。
    fn record(&self, event: AuditEvent);
}

/// 零行为端口：默认注入项，保证未配置审计时轮换零行为变化。
pub struct NoopRotationAudit;

impl RotationAuditPort for NoopRotationAudit {
    fn record(&self, _event: AuditEvent) {}
}

/// 经 [`AuditWriter`] 落链的端口实现：事件走 `AuditWriter::write`
/// （本地 HMAC 链 + 转发全部已注入 `AuditSink`）。
///
/// 容错：写入 `Err`（如 `log_dir` 未配置写 durable 事件）仅记 debug 日志，
/// 不上抛、不 panic——与 `log_key_access` 等 helper 的容错风格一致，
/// 轮换本身不因审计落链失败而中断。
pub struct AuditWriterPort {
    writer: std::sync::Arc<crate::impl_::audit::AuditWriter>,
}

impl AuditWriterPort {
    pub fn new(writer: std::sync::Arc<crate::impl_::audit::AuditWriter>) -> Self {
        Self { writer }
    }
}

impl RotationAuditPort for AuditWriterPort {
    fn record(&self, event: AuditEvent) {
        if let Err(e) = self.writer.write(event) {
            // log/tracing 均为可选 feature，统一走 crate 内 telemetry 通道。
            crate::telemetry::warn(
                "confers.key.rotation_audit_dropped",
                &[("reason", &e.to_string())],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    #[test]
    fn test_noop_rotation_audit_record_does_not_panic() {
        let port = NoopRotationAudit;
        port.record(AuditEvent::KeyRotation {
            key_id: "k".to_string(),
            old_version: "v1".to_string(),
            new_version: "v2".to_string(),
            fields_reencrypted: 0,
            timestamp: OffsetDateTime::now_utc(),
        });
    }

    #[test]
    fn test_audit_writer_port_swallows_write_errors() {
        // enabled 但未配置 log_dir：durable 事件写入返回 Err，端口只告警不 panic。
        let writer = crate::impl_::audit::AuditWriter::builder()
            .enabled(true)
            .build();
        let port = AuditWriterPort::new(std::sync::Arc::new(writer));
        port.record(AuditEvent::KeyRotation {
            key_id: "k".to_string(),
            old_version: "v1".to_string(),
            new_version: "v2".to_string(),
            fields_reencrypted: 0,
            timestamp: OffsetDateTime::now_utc(),
        });

        // disabled writer：write 直接 Ok，端口同样无事。
        let disabled = crate::impl_::audit::AuditWriter::new();
        let port = AuditWriterPort::new(std::sync::Arc::new(disabled));
        port.record(AuditEvent::KeyRotation {
            key_id: "k".to_string(),
            old_version: "v1".to_string(),
            new_version: "v2".to_string(),
            fields_reencrypted: 0,
            timestamp: OffsetDateTime::now_utc(),
        });
    }
}
