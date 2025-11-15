use tokio::net::TcpStream;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::time::timeout;
use crate::host::ExternalHost;
use crate::net::range::{ip_iter, Ipv4Range};
use std::future::Future;

pub async fn handshake_range_discovery<F, Fut>( 
    ipv4_range: Ipv4Range,
    mut prober: F,
) -> anyhow::Result<Vec<ExternalHost>>
where
    F: FnMut(IpAddr) -> Fut, 
    Fut: Future<Output = anyhow::Result<Option<ExternalHost>>>
{
    let mut result: Vec<ExternalHost> = Vec::new();
    for ip in ip_iter(&ipv4_range) {
        if let Some(found) = prober(IpAddr::V4(ip)).await? { 
            result.push(found); 
        }
    }
    Ok(result)
}

pub async fn handshake_probe(addr: IpAddr) -> anyhow::Result<Option<ExternalHost>> {
    let socket_addr: SocketAddr = SocketAddr::new(addr, 443);
    let probe_timeout: Duration = Duration::from_millis(100);

    match timeout(probe_timeout, TcpStream::connect(socket_addr)).await {
        Ok(Ok(_)) | Ok(Err(_)) => { Ok(Some(ExternalHost::from(addr))) },
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
    use crate::net::range::Ipv4Range;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn handshake_range_discovery_should_collect_found_hosts() {
        let range = Ipv4Range::new(Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 2));
        let mock_prober = |ip: IpAddr| async move {
            if ip == IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)) {
                Ok(Some(ExternalHost::from(ip)))
            } else {
                Ok(None)
            }
        };
        let results: Vec<ExternalHost> = handshake_range_discovery(range, mock_prober).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].ips.contains(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))), true);
    }

    #[tokio::test]
    async fn handshake_range_discovery_returns_empty_vec_when_none_found() {
        let range = Ipv4Range::new(Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 2));
        let mock_prober = |_: IpAddr| async move {
            Ok(None)
        };
        let results: Vec<ExternalHost> = handshake_range_discovery(range, mock_prober).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn handshake_range_discovery_handles_empty_range() {
        let range = Ipv4Range::new(Ipv4Addr::new(10, 0, 0, 5), Ipv4Addr::new(10, 0, 0, 4));
        let mock_prober = |ip: IpAddr| async move {
            panic!("Prober should not be called for an empty range, but was called for {}", ip);
            #[allow(unreachable_code)]
            Ok(None)
        };
        let results: Vec<ExternalHost> = handshake_range_discovery(range, mock_prober).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn handshake_range_discovery_propagates_prober_error() {
        let range = Ipv4Range::new(Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 2));
        let mock_prober = |_: IpAddr| async move {
            Err(anyhow::anyhow!("Network subsystem failure!"))
        };
        let result = handshake_range_discovery(range, mock_prober).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.to_string(), "Network subsystem failure!");
        }
    }

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