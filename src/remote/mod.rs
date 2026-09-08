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
    /// 探测方向：对一个已关闭的本地端口发起请求。直连时连接必然立即失败
    /// （返回 Err）；若请求反而"成功"，唯一合理的解释是代理代答了这次
    /// 连接——即制造了一次 false positive 的连接成功（对不存在的服务端口
    /// 返回了响应）。本函数据此判定存在拦截代理并返回 true，相关测试应跳过
    /// （CI 无代理不受影响）。
    ///
    /// 注意误检方向：代理若以"让请求失败/超时"而非"代答"的方式介入，本
    /// 探测无法识别（返回 false）；此时依赖"期望 connection refused"的
    /// 断言仍然成立，只是测试路径可能并非直连。
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
