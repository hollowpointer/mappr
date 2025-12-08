use crate::net::packets;
use crate::net::sender::SenderConfig;
use crate::print;
use anyhow::{self, Context};
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};
use std::time::Duration;

const READ_TIMEOUT_MS: u64 = 50; 

pub fn send_packets(mut tx: Box<dyn DataLinkSender>, sender_cfg: &SenderConfig) -> anyhow::Result<()> {
    let packets: Vec<Vec<u8>> = packets::create_packets(sender_cfg)?;
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

fn get_config() -> Config {
    Config {
        read_timeout: Some(Duration::from_millis(READ_TIMEOUT_MS)),
        ..Default::default()
    }
}