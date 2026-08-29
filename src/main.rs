#![forbid(unsafe_code)]

use happy_wakey_sidecar::{Config, HappyWakeyProbe};
use ores_otel_sidecar::{runtime, SidecarConfig, SidecarIdentity};

const SERVICE: &str = "happy-wakey-sidecar";
const BIND_ENV: &str = "HAPPY_WAKEY_SIDECAR_BIND";

fn main() {
    if let Err(error) = run() {
        eprintln!(
            "{{\"service\":\"happy-wakey-sidecar\",\"severity\":\"fatal\",\"operation\":\"configure\",\"outcome\":\"rejected\",\"error_class\":\"{}\"}}",
            error.class()
        );
        std::process::exit(1);
    }
}

fn run() -> Result<(), StartupError> {
    let config = Config::from_env().map_err(|_| StartupError::Configuration)?;
    let probe = HappyWakeyProbe::start(&config).map_err(|_| StartupError::ProbeThread)?;
    let sidecar = SidecarConfig::from_bind_with(
        SidecarIdentity::new(SERVICE, BIND_ENV),
        &config.bind,
        false,
        probe,
    )
    .map_err(|_| StartupError::BindPolicy)?;
    runtime::run(&sidecar);
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum StartupError {
    Configuration,
    ProbeThread,
    BindPolicy,
}

impl StartupError {
    const fn class(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::ProbeThread => "probe_thread",
            Self::BindPolicy => "bind_policy",
        }
    }
}
