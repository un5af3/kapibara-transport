//! Dns Resolver

use std::{net::SocketAddr, time::Duration};

use hickory_resolver::{system_conf::read_system_conf, TokioAsyncResolver};

use tokio::net::lookup_host;

use super::{option::Strategy, ResolveError, ResolveOption};

#[derive(Debug, Clone)]
pub struct DefaultResolveOption {
    timeout: Duration,
    strategy: Strategy,
}

#[derive(Debug, Clone)]
pub enum Resolver {
    Default(DefaultResolveOption),
    System(TokioAsyncResolver),
    Custom(TokioAsyncResolver),
}

impl Default for Resolver {
    fn default() -> Self {
        Self::Default(DefaultResolveOption {
            timeout: Duration::from_secs(5),
            strategy: Strategy::default(),
        })
    }
}

impl Resolver {
    pub fn new(option: ResolveOption) -> Self {
        if option.servers.is_empty() {
            #[cfg(any(unix, target_os = "windows"))]
            {
                match read_system_conf() {
                    Ok((cfg, mut opt)) => {
                        opt.timeout = option.timeout;
                        opt.ip_strategy = option.strategy.into();
                        let resolver = TokioAsyncResolver::tokio(cfg, opt);
                        Resolver::System(resolver)
                    }
                    Err(_) => Resolver::Default(DefaultResolveOption {
                        timeout: option.timeout,
                        strategy: option.strategy,
                    }),
                }
            }
            #[cfg(not(any(unix, target_os = "windows")))]
            Resolver::Default(DefaultResolveOption {
                timeout: option.timeout,
                strategy: option.strategy,
            })
        } else {
            let (cfg, opt) = option.custom_config();
            let resolver = TokioAsyncResolver::tokio(cfg, opt);
            Resolver::Custom(resolver)
        }
    }

    pub async fn resolve<S: AsRef<str> + ToString>(
        &self,
        addr: S,
        port: u16,
    ) -> Result<impl Iterator<Item = SocketAddr>, ResolveError> {
        match self {
            Self::Default(option) => {
                let result =
                    tokio::time::timeout(option.timeout, lookup_host((addr.to_string(), port)))
                        .await??;
                Ok(Resolved::Default(sort_resolved(result, option.strategy)))
            }
            Self::System(resolver) => {
                let resolver = resolver.clone();
                //let result = resolver.lookup_ip(addr.as_ref()).await?;
                let addr = addr.to_string();
                let result = tokio::spawn(async move { resolver.lookup_ip(addr).await })
                    .await
                    .map_err(|e| ResolveError::Initialize(e.to_string()))??;
                Ok(Resolved::System(
                    result.into_iter().map(move |ip| SocketAddr::new(ip, port)),
                ))
            }
            Self::Custom(resolver) => {
                let resolver = resolver.clone();
                //let result = resolver.lookup_ip(addr.as_ref()).await?;
                let addr = addr.to_string();
                let result = tokio::spawn(async move { resolver.lookup_ip(addr).await })
                    .await
                    .map_err(|e| ResolveError::Initialize(e.to_string()))??;
                Ok(Resolved::Custom(
                    result.into_iter().map(move |ip| SocketAddr::new(ip, port)),
                ))
            }
        }
    }

    pub fn block_resolve<S: AsRef<str> + ToString>(
        &self,
        addr: S,
        port: u16,
    ) -> Result<impl Iterator<Item = SocketAddr>, ResolveError> {
        tokio::task::block_in_place(move || {
            let result = if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.block_on(async move { self.resolve(addr, port).await })?
            } else {
                let mut runtime = tokio::runtime::Builder::new_current_thread();
                runtime.enable_all();
                let runtime = runtime
                    .build()
                    .map_err(|e| ResolveError::Initialize(e.to_string()))?;

                runtime.block_on(async move { self.resolve(addr, port).await })?
            };

            Ok::<_, ResolveError>(result)
        })
    }
}

pub enum Resolved<A, B, C>
where
    A: Iterator<Item = SocketAddr>,
    B: Iterator<Item = SocketAddr>,
    C: Iterator<Item = SocketAddr>,
{
    Default(A),
    System(B),
    Custom(C),
}

impl<A, B, C> Iterator for Resolved<A, B, C>
where
    A: Iterator<Item = SocketAddr>,
    B: Iterator<Item = SocketAddr>,
    C: Iterator<Item = SocketAddr>,
{
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Default(s) => s.next(),
            Self::System(s) => s.next(),
            Self::Custom(s) => s.next(),
        }
    }
}

pub enum SortedResolved<A, B, C, D, E>
where
    A: Iterator<Item = SocketAddr>,
    B: Iterator<Item = SocketAddr>,
    C: Iterator<Item = SocketAddr>,
    D: Iterator<Item = SocketAddr>,
    E: Iterator<Item = SocketAddr>,
{
    Ipv4AndIpv6(A),
    Ipv4Only(B),
    Ipv6Only(C),
    Ipv4thenIpv6(D),
    Ipv6thenIpv4(E),
}

impl<A, B, C, D, E> Iterator for SortedResolved<A, B, C, D, E>
where
    A: Iterator<Item = SocketAddr>,
    B: Iterator<Item = SocketAddr>,
    C: Iterator<Item = SocketAddr>,
    D: Iterator<Item = SocketAddr>,
    E: Iterator<Item = SocketAddr>,
{
    type Item = SocketAddr;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Ipv4AndIpv6(s) => s.next(),
            Self::Ipv4Only(s) => s.next(),
            Self::Ipv6Only(s) => s.next(),
            Self::Ipv4thenIpv6(s) => s.next(),
            Self::Ipv6thenIpv4(s) => s.next(),
        }
    }
}

pub fn sort_resolved(
    result: impl Iterator<Item = SocketAddr>,
    strategy: Strategy,
) -> impl Iterator<Item = SocketAddr> {
    match strategy {
        Strategy::Ipv4AndIpv6 => SortedResolved::Ipv4AndIpv6(result),
        Strategy::Ipv4Only => SortedResolved::Ipv4Only(result.filter(|s| s.is_ipv4())),
        Strategy::Ipv6Only => SortedResolved::Ipv6Only(result.filter(|s| s.is_ipv6())),
        Strategy::Ipv4ThenIpv6 => {
            let (ipv4_addrs, ipv6_addrs): (Vec<_>, Vec<_>) = result.partition(|s| s.is_ipv4());
            SortedResolved::Ipv4thenIpv6(ipv4_addrs.into_iter().chain(ipv6_addrs))
        }
        Strategy::Ipv6ThenIpv4 => {
            let (ipv6_addrs, ipv4_addrs): (Vec<_>, Vec<_>) = result.partition(|s| s.is_ipv6());
            SortedResolved::Ipv6thenIpv4(ipv6_addrs.into_iter().chain(ipv4_addrs))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::dns::option::{NameServerOption, Protocol, Strategy};

    #[tokio::test]
    async fn test_dns_resolve() -> Result<(), ResolveError> {
        let mut dns_option = ResolveOption::default();
        dns_option.strategy = Strategy::Ipv4ThenIpv6;
        dns_option.servers = vec![NameServerOption {
            address: "8.8.8.8:53".parse().unwrap(),
            protocol: Protocol::Udp,
        }];

        let resolver = Resolver::new(dns_option.clone());
        let result: Vec<_> = tokio::runtime::Handle::current()
            .spawn_blocking(move || resolver.block_resolve("bing.com", 443))
            .await
            .unwrap()?
            .collect();
        println!("{:?}", result);

        let resolver = Resolver::new(dns_option);
        let result = resolver.resolve("bing.com", 443).await?.collect::<Vec<_>>();
        println!("{:?}", result);

        Ok(())
    }
}
