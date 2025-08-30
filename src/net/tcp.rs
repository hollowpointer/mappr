use tokio::net::TcpStream;

use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;
use tokio::time::timeout;
use crate::net::range::ip_iter;

pub async fn handshake_discovery(start: Ipv4Addr, end: Ipv4Addr) -> anyhow::Result<Vec<Ipv4Addr>> {
    let mut result: Vec<Ipv4Addr> = Vec::new();
    for ip in ip_iter((start, end)) {
        if let Some(found) = handshake_probe(ip).await? {
            result.push(found);
        }
    }
    Ok(result)
}

async fn handshake_probe(addr: Ipv4Addr) -> anyhow::Result<Option<Ipv4Addr>> {
    let sa = SocketAddrV4::new(addr, 443);

    match timeout(Duration::from_millis(100), TcpStream::connect(sa)).await {
        Ok(Ok(_)) | Ok(Err(_)) => Ok(Some(addr)),
        Err(_elapsed) => Ok(None),
    }
}