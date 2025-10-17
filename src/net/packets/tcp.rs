use tokio::net::TcpStream;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use crate::host::Host;
use crate::net::range::{ip_iter, Ipv4Range};

pub async fn handshake_range_discovery(ipv4range: Arc<Ipv4Range>) -> anyhow::Result<Vec<Host>> {
    let mut result: Vec<Host> = Vec::new();
    for ip in ip_iter(&ipv4range) {
        if let Some(found) = handshake_probe(ip).await? { result.push(found); }
    }
    Ok(result)
}

async fn handshake_probe(addr: Ipv4Addr) -> anyhow::Result<Option<Host>> {
    let sa = SocketAddrV4::new(addr, 443);
    let mut host: Host = Host::default();
    match timeout(Duration::from_millis(100), TcpStream::connect(sa)).await {
        Ok(Ok(_)) | Ok(Err(_)) => {
            host.set_ipv4(*sa.ip());
            Ok(Some(host))
        },

        Err(_elapsed) => Ok(None),
    }
}