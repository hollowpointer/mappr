use tokio::net::TcpStream;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tokio::time::timeout;
use crate::host::ExternalHost;
use crate::net::range::{ip_iter, Ipv4Range};

pub async fn handshake_range_discovery(ipv4range: Ipv4Range) -> anyhow::Result<Vec<ExternalHost>> {
    let mut result: Vec<ExternalHost> = Vec::new();
    for ip in ip_iter(&ipv4range) {
        if let Some(found) = handshake_probe(IpAddr::V4(ip)).await? { 
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