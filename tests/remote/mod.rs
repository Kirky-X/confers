// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Remote configuration source & bus integration tests —
//! HTTP polled sources, etcd, consul, NATS bus, and Redis bus.

#[path = "../common.rs"]
pub mod common;

mod bus;
mod consul;
mod etcd;
mod remote;
