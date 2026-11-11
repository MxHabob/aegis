use std::{
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct VpnDiagnostic {
    pub supported: bool,
    pub likely_interfaces: Vec<String>,
    pub note: String,
}

pub fn diagnose_os_vpn(timeout: Duration) -> VpnDiagnostic {
    #[cfg(windows)]
    let (program, args) = (
        std::env::var_os("WINDIR")
            .map(|root| std::path::PathBuf::from(root).join("System32\\netsh.exe"))
            .unwrap_or_else(|| std::path::PathBuf::from("C:\\Windows\\System32\\netsh.exe")),
        vec!["interface", "show", "interface"],
    );
    #[cfg(target_os = "linux")]
    let (program, args) = (
        std::path::PathBuf::from("/usr/sbin/ip"),
        vec!["-brief", "link"],
    );
    #[cfg(target_os = "macos")]
    let (program, args) = (std::path::PathBuf::from("/sbin/ifconfig"), vec![]);
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        let _ = timeout;
        return VpnDiagnostic {
            supported: false,
            likely_interfaces: Vec::new(),
            note: "VPN interface detection is not implemented for this operating system. Traffic routing is not verified.".into(),
        };
    }

    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    {
        let mut child = match Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                return VpnDiagnostic {
                    supported: false,
                    likely_interfaces: Vec::new(),
                    note: format!("Could not query OS network interfaces: {error}"),
                };
            }
        };

        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        return VpnDiagnostic {
                            supported: false,
                            likely_interfaces: Vec::new(),
                            note: "The OS network interface query failed. Traffic routing is not verified.".into(),
                        };
                    }
                    break;
                }
                Ok(None) if start.elapsed() < timeout => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return VpnDiagnostic {
                        supported: false,
                        likely_interfaces: Vec::new(),
                        note: "The OS network interface query timed out. Traffic routing is not verified.".into(),
                    };
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return VpnDiagnostic {
                        supported: false,
                        likely_interfaces: Vec::new(),
                        note: format!("Could not inspect OS network interfaces: {error}"),
                    };
                }
            }
        }

        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(error) => {
                return VpnDiagnostic {
                    supported: false,
                    likely_interfaces: Vec::new(),
                    note: format!("Could not read OS network interface output: {error}"),
                };
            }
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let likely_interfaces = text
            .lines()
            .filter(|line| {
                let line = line.to_ascii_lowercase();
                [
                    "vpn",
                    "wireguard",
                    "openvpn",
                    "tailscale",
                    "zerotier",
                    "tun",
                    "tap",
                ]
                .iter()
                .any(|marker| line.contains(marker))
            })
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        VpnDiagnostic {
            supported: true,
            note: if likely_interfaces.is_empty() {
                "No likely VPN interface name was found. This heuristic does not prove that no VPN is active, and traffic routing is not verified.".into()
            } else {
                "Likely VPN interface names were found. This is only a name-based hint; active status and traffic routing are not verified.".into()
            },
            likely_interfaces,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vpn_diagnostic_has_explicit_routing_caveat() {
        let result = diagnose_os_vpn(Duration::from_secs(3));
        assert!(result.note.contains("routing is not verified"));
    }
}
