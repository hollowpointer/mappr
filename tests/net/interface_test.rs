use pnet::datalink::{MacAddr, NetworkInterface};
use mappr::cmd::Target;
use mappr::net::interface;
use super::util::{ni, v4, v6};

/*************************************************************
                       Tests for LAN
**************************************************************/

fn assert_lan_selected(expected: &NetworkInterface, interfaces: Vec<NetworkInterface>) {
    let got = interface::select(Target::LAN, &interfaces);
    assert_eq!(
        got.as_ref(), Some(expected),
        "LAN select mismatch.\nexpected: {:?}\n     got: {:?}\nfrom set: {:#?}",
        expected, got, interfaces);
}

#[test]
fn lan_ignores_down_interfaces() {
    let got = interface::select(Target::LAN, &vec![enp9s0_down()]);
    assert_eq!(None, got);
}

#[test]
fn lan_ignores_virtuals_and_bridges() {
    let got = interface::select(Target::LAN, &vec![docker0(), br0(), veth1234()]);
    assert!(got.is_none());
}

#[test]
fn lan_ignores_ipv6_link_local_only() {
    let got = interface::select(Target::LAN, &vec![veth1234(), ipv6leakintrf0()]);
    assert!(got.is_none());
}

#[test]
fn lan_selects_enp9s0() {
    assert_lan_selected(&enp9s0(), iface_all());
}

#[test]
fn lan_selects_eth1() {
    assert_lan_selected(&eth1(), vec![lo(), ipv6leakintrf0(), docker0(), eth1(), veth1234(), br0()]);
}

#[test]
fn lan_selects_wlan0() {
    assert_lan_selected(&wlan0(), vec![ipv6leakintrf0(), lo(), br0(), veth1234(), wlan0()])
}

#[test]
fn lan_prefers_wired_over_wifi() {
    let got = interface::select(Target::LAN, &vec![wlan0(), enp9s0()]);
    assert_eq!(Some(enp9s0()), got);
}

/*************************************************************
                  Mock interfaces for testing
**************************************************************/

fn iface_all() -> Vec<NetworkInterface> {
    vec![lo(),
         enp9s0(),
         tun0(),
         ipv6leakintrf0(),
         wlan0(),
         eth1(),
         docker0(),
         veth1234(),
         br0()
    ]
}

fn lo() -> NetworkInterface {
    ni(
        "lo",
        1,
        Some(MacAddr::new(0, 0, 0, 0, 0, 0)),
        &[v4(127, 0, 0, 1, 8), v6("::1", 128)],
        65609,
    )
}

fn enp9s0() -> NetworkInterface {
    ni(
        "enp9s0",
        2,
        Some(MacAddr::new(0xa8, 0xa1, 0x59, 0x13, 0x41, 0x46)),
        &[
            v4(192, 168, 0, 32, 24),
            v6("2a02:908:8c1:b880::b054", 128),
            v6("2a02:908:8c1:b880:97f7:c408:8dff:b5bf", 64),
            v6("fe80::b3dd:5c39:7c29:48b6", 64),
        ],
        69699,
    )
}

fn enp9s0_down() -> NetworkInterface {
    let mut nic = enp9s0();
    // Mask out the UP flag if your ni() encodes it in flags.
    nic.flags &= !libc::IFF_UP as u32;
    nic
}


fn tun0() -> NetworkInterface {
    ni(
        "tun0",
        5,
        None,
        &[v4(10, 96, 0, 57, 16), v6("fe80::c137:8964:5a63:efde", 64)],
        69841,
    )
}

fn ipv6leakintrf0() -> NetworkInterface {
    ni(
        "ipv6leakintrf0",
        6,
        Some(MacAddr::new(0xd2, 0x25, 0xd4, 0x9f, 0x18, 0xfd)),
        &[v6("fdeb:446c:912d:8da::", 64), v6("fe80::7f87:ff4a:9ad8:d2f0", 64)],
        65731,
    )
}

fn wlan0() -> NetworkInterface {
    ni(
        "wlan0",
        3,
        Some(MacAddr::new(0x34, 0xcf, 0xf6, 0x9a, 0x11, 0x22)),
        &[
            v4(192, 168, 1, 42, 24),
            v6("fe80::36cf:f6ff:fe9a:1122", 64),
        ],
        69699,
    )
}

fn eth1() -> NetworkInterface {
    ni(
        "eth1",
        4,
        Some(MacAddr::new(0x52, 0x54, 0x00, 0x12, 0x34, 0x56)),
        &[v4(10, 0, 0, 15, 24)],
        69699,
    )
}

fn docker0() -> NetworkInterface {
    ni(
        "docker0",
        7,
        Some(MacAddr::new(0x02, 0x42, 0xac, 0x11, 0x00, 0x01)),
        &[v4(172, 17, 0, 1, 16)],
        69699,
    )
}

fn veth1234() -> NetworkInterface {
    ni(
        "veth1234",
        8,
        Some(MacAddr::new(0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f)),
        &[v6("fe80::1a2b:3cff:fe4d:5e6f", 64)],
        69699,
    )
}

fn br0() -> NetworkInterface {
    ni(
        "br0",
        9,
        Some(MacAddr::new(0xde, 0xad, 0xbe, 0xef, 0x00, 0x01)),
        &[
            v4(192, 168, 100, 1, 24),
            v6("fd00:dead:beef::1", 64),
        ],
        69699,
    )
}