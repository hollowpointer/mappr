use mappr_common::network::host::Host;
use mappr_common::network::range::IpCollection;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::scanner::increment_host_count;

pub async fn handshake_range_discovery<F, Fut>(
    targets: IpCollection,
    mut prober: F,
) -> anyhow::Result<Vec<Host>>
where
    F: FnMut(IpAddr) -> Fut,
    Fut: Future<Output = anyhow::Result<Option<Host>>>,
{
    let mut result: Vec<Host> = Vec::new();
    for target in targets {
        if let Some(found) = prober(target).await? { result.push(found);
        }
    }
    Ok(result)
}

pub async fn handshake_probe(addr: IpAddr) -> anyhow::Result<Option<Host>> {
    let socket_addr: SocketAddr = SocketAddr::new(addr, 443);
    let probe_timeout: Duration = Duration::from_millis(100);

    match timeout(probe_timeout, TcpStream::connect(socket_addr)).await {
        Ok(Ok(_)) | Ok(Err(_)) => {
            increment_host_count();
            Ok(Some(Host::new(addr)))
        },
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
    use mappr_common::network::host::Host;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    #[ignore]
    async fn handshake_probe_should_find_known_open_port() {
        let ip: IpAddr = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        let result: Option<Host> = handshake_probe(ip).await.unwrap();
        assert!(result.is_some());
    }

    #[tokio::test]
    #[ignore]
    async fn handshake_probe_should_timeout_on_unreachable_ip() {
        let ip: IpAddr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1));
        let result: Option<Host> = handshake_probe(ip).await.unwrap();
        assert!(result.is_none());
    }
}
