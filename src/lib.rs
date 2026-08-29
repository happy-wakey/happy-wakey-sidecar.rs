#![forbid(unsafe_code)]

pub mod config;
#[path = "../generated/rust/env.rs"]
pub mod generated_env;
pub mod lifecycle;
pub mod probe;

pub use config::{Config, ConfigError, ProductKind};
pub use lifecycle::{Event, Machine, Phase, Snapshot, TransitionError};
pub use probe::{HappyWakeyProbe, ProbeError};
