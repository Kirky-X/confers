// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Remote configuration source & bus integration tests —
//! HTTP polled sources, etcd, consul, NATS bus, and Redis bus.

#[path = "../common.rs"]
pub mod common;

mod bus;
mod consul;
mod etcd;
mod remote;
