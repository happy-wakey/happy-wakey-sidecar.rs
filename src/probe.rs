#![forbid(unsafe_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ores_otel_sidecar::{ProductProbe, SidecarOverrides};
use serde_json::json;

use crate::config::{Config, ProbeEndpoint, ProductKind};
use crate::lifecycle::{Event, Machine, Snapshot};

const IO_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_STATUS_LINE_BYTES: u64 = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProbeError {
    #[error("product probe transport failed")]
    Transport,
    #[error("product probe returned an invalid response")]
    InvalidResponse,
    #[error("product probe returned a non-ready status")]
    NotReady,
}

#[derive(Clone)]
pub struct HappyWakeyProbe {
    machine: Arc<Mutex<Machine>>,
    product_kind: ProductKind,
}

impl HappyWakeyProbe {
    /// Start one sequential product-probe worker and its readiness reducer.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the named worker thread cannot be spawned.
    pub fn start(config: &Config) -> Result<Self, std::io::Error> {
        let mut machine = Machine::new(config.success_threshold, config.failure_threshold);
        machine
            .apply(Event::Configured)
            .map_err(|_| std::io::Error::other("invalid initial sidecar transition"))?;
        let probe = Self {
            machine: Arc::new(Mutex::new(machine)),
            product_kind: config.product_kind,
        };
        let worker_probe = probe.clone();
        let endpoint = config.product_probe.clone();
        let interval = config.interval;
        thread::Builder::new()
            .name("happy-wakey-sidecar-probe".to_owned())
            .spawn(move || loop {
                let event = if probe_once(&endpoint).is_ok() {
                    Event::ProbeSucceeded
                } else {
                    Event::ProbeFailed
                };
                if worker_probe.apply(event).is_err() {
                    return;
                }
                thread::sleep(interval);
            })?;
        Ok(probe)
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        self.machine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .snapshot()
    }

    fn apply(&self, event: Event) -> Result<(), ()> {
        self.machine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .apply(event)
            .map(|_| ())
            .map_err(|_| ())
    }
}

impl ProductProbe for HappyWakeyProbe {
    fn extra_health(&self) -> Option<serde_json::Value> {
        let snapshot = self.snapshot();
        Some(json!({
            "contract": "happy-wakey.sidecar-health.v1",
            "product_kind": self.product_kind.as_str(),
            "phase": snapshot.phase.as_str(),
            "ready": matches!(snapshot.phase, crate::lifecycle::Phase::Ready),
            "ready_certified": snapshot.ready_certified,
            "consecutive_successes": snapshot.consecutive_successes,
            "consecutive_failures": snapshot.consecutive_failures,
            "probe_generation": snapshot.probe_generation,
            "auth_authority": "shared-auth",
            "sync_authority": "opto-sync"
        }))
    }

    fn ready(&self) -> bool {
        self.machine
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_ready()
    }
}

impl SidecarOverrides for HappyWakeyProbe {
    fn allow_non_loopback(&self, _from_env: bool) -> bool {
        false
    }
}

/// Perform one bounded HTTP/1.x health probe against a validated loopback endpoint.
///
/// # Errors
///
/// Returns [`ProbeError`] on transport failure, an invalid response, or any
/// status other than HTTP 200.
pub fn probe_once(endpoint: &ProbeEndpoint) -> Result<(), ProbeError> {
    let mut stream = TcpStream::connect_timeout(&endpoint.address, IO_TIMEOUT)
        .map_err(|_| ProbeError::Transport)?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|_| ProbeError::Transport)?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|_| ProbeError::Transport)?;
    write!(
        stream,
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: happy-wakey-sidecar/0.1\r\n\r\n",
        endpoint.path, endpoint.host_header
    )
    .map_err(|_| ProbeError::Transport)?;
    stream.flush().map_err(|_| ProbeError::Transport)?;

    let reader = BufReader::new(stream);
    let mut limited = reader.take(MAX_STATUS_LINE_BYTES);
    let mut status_line = String::new();
    limited
        .read_line(&mut status_line)
        .map_err(|_| ProbeError::Transport)?;
    if !status_line.ends_with("\r\n") {
        return Err(ProbeError::InvalidResponse);
    }
    let mut fields = status_line.trim_end().split_ascii_whitespace();
    let protocol = fields.next().ok_or(ProbeError::InvalidResponse)?;
    let status = fields.next().ok_or(ProbeError::InvalidResponse)?;
    if !matches!(protocol, "HTTP/1.0" | "HTTP/1.1") {
        return Err(ProbeError::InvalidResponse);
    }
    if status == "200" {
        Ok(())
    } else {
        Err(ProbeError::NotReady)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, TcpListener};

    use super::*;

    #[test]
    fn bounded_loopback_probe_accepts_only_http_200() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = [0_u8; 512];
            let _ = socket.read(&mut request).unwrap();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .unwrap();
        });
        let endpoint = ProbeEndpoint {
            address,
            host_header: format!("127.0.0.1:{}", address.port()),
            path: "/healthz".to_owned(),
        };
        assert_eq!(probe_once(&endpoint), Ok(()));
        server.join().unwrap();

        let closed = ProbeEndpoint {
            address: std::net::SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9),
            ..endpoint
        };
        assert_eq!(probe_once(&closed), Err(ProbeError::Transport));
    }
}
