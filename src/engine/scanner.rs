use std::time::Duration;
use pnet::datalink::NetworkInterface;
use std::ops::ControlFlow;

use crate::engine::models::EngineHost;
use crate::engine::sender::SenderConfig;
use crate::engine::datalink::{channel::{self, EthernetHandle}, interface::{self, NetworkInterfaceExtension}};
use crate::engine::transport::{self, UdpHandle};
use crate::engine::runner::local::LocalRunner;
use crate::utils::input::InputHandle;
use crate::utils::timing::ScanTimer;

const MAX_CHANNEL_TIME: Duration = Duration::from_millis(7_500);
const MIN_CHANNEL_TIME: Duration = Duration::from_millis(2_500);
const MAX_SILENCE: Duration = Duration::from_millis(500);

pub fn discover_lan(
    intf: NetworkInterface,
    sender_cfg: SenderConfig,
) -> anyhow::Result<Vec<EngineHost>> {
    let eth_handle: EthernetHandle = channel::start_capture(&intf)?;
    let udp_handle: UdpHandle = transport::start_capture()?;
    let input_handle: InputHandle = InputHandle::new();
    let timer = ScanTimer::new(MAX_CHANNEL_TIME, MIN_CHANNEL_TIME, MAX_SILENCE);

    let mut local_runner: LocalRunner =
        LocalRunner::new(sender_cfg, input_handle, eth_handle, udp_handle, timer)?;

    local_runner.send_discovery_packets()?;
    local_runner.start_input_listener();

    loop {
        if let ControlFlow::Break(_) = local_runner.process_packets() {
            break;
        }
    }

    Ok(local_runner.get_hosts())
}

pub fn get_prioritized_interfaces() -> anyhow::Result<Vec<NetworkInterface>> {
    interface::get_prioritized_interfaces(5)
}

pub fn print_interface_details(intf: &NetworkInterface, idx: usize) {
    intf.print_details(idx);
}
