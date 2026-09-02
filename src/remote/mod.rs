// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Remote configuration sources.

pub(crate) mod circuit_breaker;
pub(crate) mod common;
mod interval;

#[cfg(feature = "consul")]
pub(crate) mod consul;
#[cfg(feature = "etcd")]
pub(crate) mod etcd;
pub(crate) mod poll;

pub use interval::PollInterval;

#[cfg(all(test, feature = "remote"))]
pub(crate) mod test_support {
    /// 检测本机是否存在拦截 127.0.0.1 流量的代理（Windows 系统代理等）。
    ///
    /// 对一个已关闭的本地端口发起请求：直连必然立即失败；若收到任何
    /// HTTP 响应，说明流量被代理代答——此时"期望 connection refused"
    /// 的 localhost 网络断言不可靠，相关测试应跳过（CI 无代理不受影响）。
    pub(crate) async fn localhost_proxy_intercept() -> bool {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let port = listener.local_addr().expect("local_addr").port();
        drop(listener);
        reqwest::get(format!("http://127.0.0.1:{port}/"))
            .await
            .is_ok()
    }
}

#[cfg(feature = "consul")]
pub use consul::{ConsulSource, ConsulSourceBuilder, ConsulTlsConfig};
#[cfg(feature = "etcd")]
pub use etcd::{EtcdSource, EtcdSourceBuilder, EtcdTlsConfig};
pub use poll::{HttpPolledSource, HttpPolledSourceBuilder, PolledSource};
