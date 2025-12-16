use std::{net::IpAddr, sync::mpsc, thread};
use pnet::{
    packet::{
        Packet,
        ip::IpNextHeaderProtocols,
        udp::UdpPacket
    }, 
    transport::{
        self, 
        TransportChannelType, 
        TransportProtocol, 
        TransportReceiver, 
        TransportSender
    }
};

use crate::net::packets::{dns, udp};

const TRANSPORT_BUFFER_SIZE: usize = 4096;
const CHANNEL_TYPE_UDP: TransportChannelType = TransportChannelType::Layer4(
    TransportProtocol::Ipv4(IpNextHeaderProtocols::Udp)
);

pub struct UdpHandle {
    pub tx: TransportSender,
    pub rx: mpsc::Receiver<Vec<u8>>,
}

pub fn start_capture() -> anyhow::Result<UdpHandle> {
    let (tx, rx_socket) = open_udp_channel()?;
    let (queue_tx, queue_rx) = mpsc::channel();

    spawn_udp_listener(queue_tx, rx_socket);

    Ok(UdpHandle { tx, rx: queue_rx })
}

pub fn open_udp_channel() -> anyhow::Result<(TransportSender, TransportReceiver)> {
    let (tx, rx) = transport::transport_channel(TRANSPORT_BUFFER_SIZE, CHANNEL_TYPE_UDP)?;
    Ok((tx, rx))
}

pub fn spawn_udp_listener(udp_tx: mpsc::Sender<Vec<u8>>, mut udp_rx: TransportReceiver) {
    thread::spawn(move || {
        let mut udp_iterator = pnet::transport::udp_packet_iter(&mut udp_rx);
        loop {
            if let Ok((udp_packet, _)) = udp_iterator.next() {
                if udp_tx.send(udp_packet.packet().to_vec()).is_err() {
                    break;
                }
            }
        }
    });
}

pub fn send_dns_query<F>(dns_packet_creator_fn: F, id: u16, target_addr: &IpAddr, udp_tx: &mut TransportSender)
where 
    F: Fn(&IpAddr, u16) -> anyhow::Result<Vec<u8>>
{
    let Ok(bytes) = dns_packet_creator_fn(target_addr, id) else { return };
    let Ok((dst_addr, dst_port)) = dns::get_dns_server_socket_addr(&target_addr) else { return };
    let src_port = rand::random_range(50_000..u16::max_value());
    let Ok(udp_bytes) = udp::create_packet(src_port, dst_port, bytes) else { return };
    let Some(udp_pkt) = UdpPacket::new(&udp_bytes) else { return };
    let _ = udp_tx.send_to(udp_pkt, dst_addr);
}