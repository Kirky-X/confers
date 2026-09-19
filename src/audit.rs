// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Audit logging — public facade.
//!
//! Implementation lives in `crate::impl_::audit`.

pub use crate::impl_::audit::{
    AuditConfig, AuditConfigBuilder, AuditEvent, AuditLevel, AuditSink, AuditWriter,
    AuditWriterBuilder, verify_audit_chain,
};
