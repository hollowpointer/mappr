use std::{sync::mpsc, thread};

use pnet::{
    packet::{
        ip::IpNextHeaderProtocols,
        Packet
    }, 
    transport::{
        self, 
        TransportChannelType, 
        TransportProtocol, 
        TransportReceiver, 
        TransportSender
    }
};

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

const TRANSPORT_BUFFER_SIZE: usize = 4096;
const CHANNEL_TYPE_UDP: TransportChannelType = TransportChannelType::Layer4(
    TransportProtocol::Ipv4(IpNextHeaderProtocols::Udp)
);

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