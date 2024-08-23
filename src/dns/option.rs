//! dns option

use std::{net::SocketAddr, time::Duration};

use hickory_resolver::config::{
    LookupIpStrategy, NameServerConfig, Protocol as HickoryProtocol, ResolverConfig, ResolverOpts,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "snake_case")]
pub struct ResolveOption {
    pub strategy: Strategy,
    pub timeout: Duration,
    pub servers: Vec<NameServerOption>,
}

impl Default for ResolveOption {
    fn default() -> Self {
        Self {
            strategy: Strategy::default(),
            timeout: Duration::from_secs(5),
            servers: vec![],
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct NameServerOption {
    pub protocol: Protocol,
    pub address: SocketAddr,
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Tcp,
    Udp,
}

impl From<Protocol> for HickoryProtocol {
    fn from(value: Protocol) -> Self {
        match value {
            Protocol::Tcp => Self::Tcp,
            Protocol::Udp => Self::Udp,
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    Ipv4Only,
    Ipv6Only,
    Ipv4AndIpv6,
    Ipv6ThenIpv4,
    Ipv4ThenIpv6,
}

impl Default for Strategy {
    fn default() -> Self {
        Self::Ipv4ThenIpv6
    }
}

impl From<Strategy> for LookupIpStrategy {
    fn from(value: Strategy) -> Self {
        match value {
            Strategy::Ipv4Only => Self::Ipv4Only,
            Strategy::Ipv6Only => Self::Ipv6Only,
            Strategy::Ipv4AndIpv6 => Self::Ipv4AndIpv6,
            Strategy::Ipv4ThenIpv6 => Self::Ipv4thenIpv6,
            Strategy::Ipv6ThenIpv4 => Self::Ipv6thenIpv4,
        }
    }
}

impl ResolveOption {
    pub fn custom_config(&self) -> (ResolverConfig, ResolverOpts) {
        let cfg = if self.servers.is_empty() {
            ResolverConfig::default()
        } else {
            let mut tmp = ResolverConfig::new();
            for server in self.servers.iter() {
                tmp.add_name_server(NameServerConfig {
                    socket_addr: server.address,
                    protocol: server.protocol.into(),
                    trust_negative_responses: false,
                    tls_dns_name: None,
                    bind_addr: None,
                });
            }
            tmp
        };

        let mut opt = ResolverOpts::default();
        opt.ip_strategy = self.strategy.into();
        opt.timeout = self.timeout;

        (cfg, opt)
    }
}
