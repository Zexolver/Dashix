use network_interface::{Addr, NetworkInterface, NetworkInterfaceConfig};
use serde::Serialize;

/// A network interface as presented to the UI: no manual MAC/IP entry ever
/// required, the user just picks one of these from a dropdown.
#[derive(Debug, Clone, Serialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub mac: Option<String>,
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
}

pub fn scan_interfaces() -> anyhow::Result<Vec<InterfaceInfo>> {
    let interfaces =
        NetworkInterface::show().map_err(|e| anyhow::anyhow!("enumerating interfaces: {e}"))?;

    let mut result = Vec::with_capacity(interfaces.len());
    for itf in interfaces {
        let mut ipv4 = Vec::new();
        let mut ipv6 = Vec::new();
        for addr in &itf.addr {
            match addr {
                Addr::V4(a) => ipv4.push(a.ip.to_string()),
                Addr::V6(a) => ipv6.push(a.ip.to_string()),
            }
        }
        result.push(InterfaceInfo {
            name: itf.name,
            mac: itf.mac_addr,
            ipv4,
            ipv6,
        });
    }
    Ok(result)
}

/// First IPv4/IPv6 address currently bound to a named interface, used by the
/// dynv6 sync loop to figure out what to publish without the user ever typing
/// an address in.
pub fn primary_addrs(name: &str) -> anyhow::Result<(Option<String>, Option<String>)> {
    let itf = scan_interfaces()?.into_iter().find(|i| i.name == name);
    Ok(match itf {
        Some(i) => (i.ipv4.into_iter().next(), i.ipv6.into_iter().next()),
        None => (None, None),
    })
}
