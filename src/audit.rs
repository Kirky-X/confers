// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Audit logging — public facade.
//!
//! Implementation lives in `crate::impl_::audit`.

pub use crate::impl_::audit::{
    verify_audit_chain, AuditConfig, AuditConfigBuilder, AuditEvent, AuditLevel, AuditSink,
    AuditWriter, AuditWriterBuilder,
};
