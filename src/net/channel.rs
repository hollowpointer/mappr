use anyhow::{Context, Result};
use pnet::datalink;
use pnet::datalink::{Channel, Config, DataLinkReceiver, DataLinkSender, NetworkInterface};

pub fn open_ethernet_channel(intf: &NetworkInterface, cfg: &Config)
                             -> Result<(Box<dyn DataLinkSender>, Box<dyn DataLinkReceiver>)> {
    let ch = datalink::channel(intf, *cfg)
        .with_context(|| format!("opening on {}", intf.name))?;
    match ch {
        Channel::Ethernet(tx, rx) => Ok((tx, rx)),
        _ => anyhow::bail!("non-ethernet channel for {}", intf.name),
    }
}