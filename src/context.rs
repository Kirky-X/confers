// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Context-aware configuration — public facade.
//!
//! Implementation lives in `crate::impl_::context`.

pub use crate::impl_::context::{
    ContextAwareField, ContextAwareFieldBuilder, ContextRule, ContextValue, EvaluationContext,
};
