//! Audit logging — public facade.
//!
//! Implementation lives in `crate::impl_::audit`.

pub use crate::impl_::audit::{
    AuditConfig, AuditConfigBuilder, AuditEvent, AuditLevel, AuditWriter, AuditWriterBuilder,
};
