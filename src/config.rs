#![forbid(unsafe_code)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use url::Url;

pub const BIND_ENV: &str = "HAPPY_WAKEY_SIDECAR_BIND";
pub const PRODUCT_KIND_ENV: &str = "HAPPY_WAKEY_SIDECAR_PRODUCT_KIND";
pub const PRODUCT_PROBE_URL_ENV: &str = "HAPPY_WAKEY_SIDECAR_PRODUCT_PROBE_URL";
pub const SHARED_AUTH_BASE_URL_ENV: &str = "HAPPY_WAKEY_SHARED_AUTH_BASE_URL";
pub const OPTO_SYNC_BASE_URL_ENV: &str = "HAPPY_WAKEY_OPTO_SYNC_BASE_URL";
pub const INTERVAL_MS_ENV: &str = "HAPPY_WAKEY_SIDECAR_INTERVAL_MS";
pub const SUCCESS_THRESHOLD_ENV: &str = "HAPPY_WAKEY_SIDECAR_SUCCESS_THRESHOLD";
pub const FAILURE_THRESHOLD_ENV: &str = "HAPPY_WAKEY_SIDECAR_FAILURE_THRESHOLD";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductKind {
    Api,
    Web,
}

impl ProductKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Web => "web",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeEndpoint {
    pub address: SocketAddr,
    pub host_header: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub bind: String,
    pub product_kind: ProductKind,
    pub product_probe: ProbeEndpoint,
    pub interval: Duration,
    pub success_threshold: u8,
    pub failure_threshold: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ConfigError {
    #[error("required sidecar configuration is missing")]
    Missing,
    #[error("sidecar configuration is invalid")]
    Invalid,
}

impl Config {
    /// Load and validate the complete fail-closed process configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when a required value is missing or any value
    /// violates the transport, kind, or numeric bounds.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Load configuration from an explicit lookup for deterministic consumers.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] under the same conditions as [`Self::from_env`].
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let bind = optional(&lookup, BIND_ENV).unwrap_or_else(|| "127.0.0.1:9090".to_owned());
        let product_kind = match required(&lookup, PRODUCT_KIND_ENV)?.as_str() {
            "api" => ProductKind::Api,
            "web" => ProductKind::Web,
            _ => return Err(ConfigError::Invalid),
        };
        let product_probe = parse_product_probe(&required(&lookup, PRODUCT_PROBE_URL_ENV)?)?;
        validate_authority_url(&required(&lookup, SHARED_AUTH_BASE_URL_ENV)?)?;
        validate_authority_url(&required(&lookup, OPTO_SYNC_BASE_URL_ENV)?)?;
        let interval_ms = bounded_u64(optional(&lookup, INTERVAL_MS_ENV), 2_000, 250, 60_000)?;
        let success_threshold = bounded_u8(optional(&lookup, SUCCESS_THRESHOLD_ENV), 2, 1, 10)?;
        let failure_threshold = bounded_u8(optional(&lookup, FAILURE_THRESHOLD_ENV), 3, 1, 10)?;
        Ok(Self {
            bind,
            product_kind,
            product_probe,
            interval: Duration::from_millis(interval_ms),
            success_threshold,
            failure_threshold,
        })
    }
}

fn required(lookup: &impl Fn(&str) -> Option<String>, key: &str) -> Result<String, ConfigError> {
    optional(lookup, key).ok_or(ConfigError::Missing)
}

fn optional(lookup: &impl Fn(&str) -> Option<String>, key: &str) -> Option<String> {
    lookup(key)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn bounded_u64(
    raw: Option<String>,
    default: u64,
    minimum: u64,
    maximum: u64,
) -> Result<u64, ConfigError> {
    let value = raw
        .map(|value| value.parse().map_err(|_| ConfigError::Invalid))
        .transpose()?
        .unwrap_or(default);
    (minimum..=maximum)
        .contains(&value)
        .then_some(value)
        .ok_or(ConfigError::Invalid)
}

fn bounded_u8(
    raw: Option<String>,
    default: u8,
    minimum: u8,
    maximum: u8,
) -> Result<u8, ConfigError> {
    let value = raw
        .map(|value| value.parse().map_err(|_| ConfigError::Invalid))
        .transpose()?
        .unwrap_or(default);
    (minimum..=maximum)
        .contains(&value)
        .then_some(value)
        .ok_or(ConfigError::Invalid)
}

fn parse_product_probe(raw: &str) -> Result<ProbeEndpoint, ConfigError> {
    let url = Url::parse(raw).map_err(|_| ConfigError::Invalid)?;
    if url.scheme() != "http"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ConfigError::Invalid);
    }
    let host = url.host_str().ok_or(ConfigError::Invalid)?;
    let ip = if host.eq_ignore_ascii_case("localhost") {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    } else {
        host.parse::<IpAddr>().map_err(|_| ConfigError::Invalid)?
    };
    if !ip.is_loopback() {
        return Err(ConfigError::Invalid);
    }
    let port = url.port_or_known_default().ok_or(ConfigError::Invalid)?;
    let path = match url.path() {
        "" | "/" => "/healthz".to_owned(),
        value if value.len() <= 256 => value.to_owned(),
        _ => return Err(ConfigError::Invalid),
    };
    Ok(ProbeEndpoint {
        address: SocketAddr::new(ip, port),
        host_header: format!("{host}:{port}"),
        path,
    })
}

fn validate_authority_url(raw: &str) -> Result<(), ConfigError> {
    let url = Url::parse(raw).map_err(|_| ConfigError::Invalid)?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ConfigError::Invalid);
    }
    let host = url
        .host_str()
        .ok_or(ConfigError::Invalid)?
        .to_ascii_lowercase();
    let secure = url.scheme() == "https";
    let internal_cleartext = url.scheme() == "http"
        && (host == "localhost"
            || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
            || host.strip_suffix(".svc").is_some()
            || host.ends_with(".svc.cluster.local"));
    (secure || internal_cleartext)
        .then_some(())
        .ok_or(ConfigError::Invalid)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn valid() -> BTreeMap<String, String> {
        BTreeMap::from([
            (PRODUCT_KIND_ENV.to_owned(), "api".to_owned()),
            (
                PRODUCT_PROBE_URL_ENV.to_owned(),
                "http://127.0.0.1:8080/healthz".to_owned(),
            ),
            (
                SHARED_AUTH_BASE_URL_ENV.to_owned(),
                "https://auth.example.test".to_owned(),
            ),
            (
                OPTO_SYNC_BASE_URL_ENV.to_owned(),
                "http://opto-sync.sync.svc".to_owned(),
            ),
        ])
    }

    #[test]
    fn complete_configuration_is_bounded_and_fail_closed() {
        let values = valid();
        let config = Config::from_lookup(|key| values.get(key).cloned()).unwrap();
        assert_eq!(config.product_kind, ProductKind::Api);
        assert!(config.product_probe.address.ip().is_loopback());
        assert_eq!(config.interval, Duration::from_secs(2));
    }

    #[test]
    fn rejects_public_probe_credentials_and_cleartext_authorities() {
        let mut values = valid();
        values.insert(
            PRODUCT_PROBE_URL_ENV.to_owned(),
            "http://192.0.2.10:8080/healthz".to_owned(),
        );
        assert!(Config::from_lookup(|key| values.get(key).cloned()).is_err());

        let mut values = valid();
        values.insert(
            SHARED_AUTH_BASE_URL_ENV.to_owned(),
            "https://user:secret@auth.example.test".to_owned(),
        );
        assert!(Config::from_lookup(|key| values.get(key).cloned()).is_err());

        let mut values = valid();
        values.insert(
            OPTO_SYNC_BASE_URL_ENV.to_owned(),
            "http://sync.example.test".to_owned(),
        );
        assert!(Config::from_lookup(|key| values.get(key).cloned()).is_err());
    }
}
