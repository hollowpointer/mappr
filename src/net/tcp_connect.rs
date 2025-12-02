use crate::host::ExternalHost;
use std::collections::HashSet;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

pub async fn handshake_range_discovery<F, Fut>(
    ip_addrs: HashSet<IpAddr>,
    mut prober: F,
) -> anyhow::Result<Vec<ExternalHost>>
where
    F: FnMut(IpAddr) -> Fut,
    Fut: Future<Output = anyhow::Result<Option<ExternalHost>>>,
{
    let mut result: Vec<ExternalHost> = Vec::new();
    for ip in ip_addrs {
        if let Some(found) = prober(ip).await? {
            result.push(found);
        }
    }
    Ok(result)
}

pub async fn handshake_probe(addr: IpAddr) -> anyhow::Result<Option<ExternalHost>> {
    let socket_addr: SocketAddr = SocketAddr::new(addr, 443);
    let probe_timeout: Duration = Duration::from_millis(100);

    match timeout(probe_timeout, TcpStream::connect(socket_addr)).await {
        Ok(Ok(_)) | Ok(Err(_)) => Ok(Some(ExternalHost::from(addr))),
        Err(_elapsed) => Ok(None),
    }
}

// ╔════════════════════════════════════════════╗
// ║ ████████╗███████╗███████╗████████╗███████╗ ║
// ║ ╚══██╔══╝██╔════╝██╔════╝╚══██╔══╝██╔════╝ ║
// ║    ██║   █████╗  ███████╗   ██║   ███████╗ ║
// ║    ██║   ██╔══╝  ╚════██║   ██║   ╚════██║ ║
// ║    ██║   ███████╗███████║   ██║   ███████║ ║
// ║    ╚═╝   ╚══════╝╚══════╝   ╚═╝   ╚══════╝ ║
// ╚════════════════════════════════════════════╝

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::ExternalHost;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    #[ignore]
    async fn handshake_probe_should_find_known_open_port() {
        let ip: IpAddr = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        let result: Option<ExternalHost> = handshake_probe(ip).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    #[ignore]
    async fn handshake_probe_should_timeout_on_unreachable_ip() {
        let ip: IpAddr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1));
        let result: Option<ExternalHost> = handshake_probe(ip).await.unwrap();
        assert!(result.is_none());
    }
}
