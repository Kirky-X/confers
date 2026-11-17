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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_noop_rotation_audit_record_does_not_panic() {
        let port = NoopRotationAudit;
        port.record(AuditEvent::KeyRotation {
            key_id: "k".to_string(),
            old_version: "v1".to_string(),
            new_version: "v2".to_string(),
            fields_reencrypted: 0,
            timestamp: Utc::now(),
        });
    }
}
