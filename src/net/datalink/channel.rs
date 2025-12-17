use crate::net::protocol;
use crate::net::sender::SenderConfig;
use crate::utils::print;
use anyhow::{self, Context};
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const READ_TIMEOUT_MS: u64 = 50;

pub struct EthernetHandle {
    pub tx: Box<dyn DataLinkSender>, 
    pub rx: mpsc::Receiver<Vec<u8>>,
}

pub fn start_capture(intf: &NetworkInterface) -> anyhow::Result<EthernetHandle> {
    let (tx, rx_socket) = open_eth_channel(intf, datalink::channel)?;
    let (queue_tx, queue_rx) = mpsc::channel();
    spawn_eth_listener(queue_tx, rx_socket);
    Ok(EthernetHandle {
        tx,
        rx: queue_rx,
    })
}

pub fn send_packets(tx: &mut Box<dyn DataLinkSender>, sender_cfg: &SenderConfig) -> anyhow::Result<()> {
    let packets: Vec<Vec<u8>> = protocol::create_packets(sender_cfg)?;
    for packet in packets {
        tx.send_to(&packet, None);
    }
    Ok(())
}

pub fn open_eth_channel<F>(
    intf: &NetworkInterface,
    channel_opener: F,
) -> anyhow::Result<(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>)>
where F: FnOnce(&NetworkInterface, Config) -> std::io::Result<datalink::Channel>,
{
    let ch: Channel =
        channel_opener(intf, get_config()).with_context(|| format!("opening on {}", intf.name))?;
    match ch {
        Channel::Ethernet(tx, rx) => {
            print::print_status("Connection established successfully");
            Ok((tx, rx))
        }
        _ => anyhow::bail!("non-ethernet channel for {}", intf.name),
    }
}

pub fn spawn_eth_listener(eth_tx: mpsc::Sender<Vec<u8>>, eth_rx: Box<dyn DataLinkReceiver>) {
    thread::spawn(move || {
        let mut eth_iter = eth_rx;
        loop {
            if let Ok(frame) = eth_iter.next() {
                if eth_tx.send(frame.to_vec()).is_err() {
                    break;
                }
            }
        }
    });
}

fn get_config() -> Config {
    Config {
        read_timeout: Some(Duration::from_millis(READ_TIMEOUT_MS)),
        ..Default::default()
    }
}