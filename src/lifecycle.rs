#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Booting,
    Probing,
    Ready,
    Degraded,
    Draining,
}

impl Phase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Booting => "booting",
            Self::Probing => "probing",
            Self::Ready => "ready",
            Self::Degraded => "degraded",
            Self::Draining => "draining",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Configured,
    ProbeSucceeded,
    ProbeFailed,
    Shutdown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub phase: Phase,
    pub ready_certified: bool,
    pub consecutive_successes: u8,
    pub consecutive_failures: u8,
    pub probe_generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("event {event:?} is invalid while sidecar is {phase:?}")]
pub struct TransitionError {
    pub phase: Phase,
    pub event: Event,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Machine {
    snapshot: Snapshot,
    success_threshold: u8,
    failure_threshold: u8,
}

impl Machine {
    #[must_use]
    pub const fn new(success_threshold: u8, failure_threshold: u8) -> Self {
        Self {
            snapshot: Snapshot {
                phase: Phase::Booting,
                ready_certified: false,
                consecutive_successes: 0,
                consecutive_failures: 0,
                probe_generation: 0,
            },
            success_threshold,
            failure_threshold,
        }
    }

    #[must_use]
    pub const fn snapshot(&self) -> Snapshot {
        self.snapshot
    }

    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self.snapshot.phase, Phase::Ready)
    }

    /// Apply one event through the sole readiness transition authority.
    ///
    /// # Errors
    ///
    /// Returns [`TransitionError`] for events invalid in the current phase.
    /// Rejection leaves the snapshot unchanged.
    pub fn apply(&mut self, event: Event) -> Result<Snapshot, TransitionError> {
        let before = self.snapshot;
        let next = match (before.phase, event) {
            (Phase::Booting, Event::Configured) => Snapshot {
                phase: Phase::Probing,
                ..before
            },
            (Phase::Probing | Phase::Ready | Phase::Degraded, Event::ProbeSucceeded) => {
                let successes = before.consecutive_successes.saturating_add(1);
                Snapshot {
                    phase: if successes >= self.success_threshold {
                        Phase::Ready
                    } else {
                        Phase::Probing
                    },
                    ready_certified: before.ready_certified || successes >= self.success_threshold,
                    consecutive_successes: successes,
                    consecutive_failures: 0,
                    probe_generation: before.probe_generation.saturating_add(1),
                }
            }
            (Phase::Probing | Phase::Ready | Phase::Degraded, Event::ProbeFailed) => {
                let failures = before.consecutive_failures.saturating_add(1);
                Snapshot {
                    phase: if failures >= self.failure_threshold {
                        Phase::Degraded
                    } else {
                        before.phase
                    },
                    ready_certified: before.ready_certified && failures < self.failure_threshold,
                    consecutive_successes: 0,
                    consecutive_failures: failures,
                    probe_generation: before.probe_generation.saturating_add(1),
                }
            }
            (Phase::Booting | Phase::Probing | Phase::Ready | Phase::Degraded, Event::Shutdown) => {
                Snapshot {
                    phase: Phase::Draining,
                    ready_certified: false,
                    consecutive_successes: 0,
                    consecutive_failures: 0,
                    probe_generation: before.probe_generation,
                }
            }
            (phase, event) => return Err(TransitionError { phase, event }),
        };
        self.snapshot = next;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_trace_reaches_ready_degrades_and_recovers() {
        let mut machine = Machine::new(2, 2);
        assert_eq!(
            machine.apply(Event::Configured).unwrap().phase,
            Phase::Probing
        );
        assert!(!machine.is_ready());
        machine.apply(Event::ProbeSucceeded).unwrap();
        assert_eq!(
            machine.apply(Event::ProbeSucceeded).unwrap().phase,
            Phase::Ready
        );
        assert!(machine.snapshot().ready_certified);
        machine.apply(Event::ProbeFailed).unwrap();
        assert_eq!(
            machine.apply(Event::ProbeFailed).unwrap().phase,
            Phase::Degraded
        );
        assert!(!machine.snapshot().ready_certified);
        machine.apply(Event::ProbeSucceeded).unwrap();
        assert_eq!(
            machine.apply(Event::ProbeSucceeded).unwrap().phase,
            Phase::Ready
        );
        assert_eq!(machine.snapshot().probe_generation, 6);
    }

    #[test]
    fn invalid_transition_is_rejected_without_mutation() {
        let mut machine = Machine::new(1, 1);
        let before = machine.snapshot();
        assert!(machine.apply(Event::ProbeSucceeded).is_err());
        assert_eq!(machine.snapshot(), before);
        machine.apply(Event::Shutdown).unwrap();
        let draining = machine.snapshot();
        assert!(machine.apply(Event::ProbeSucceeded).is_err());
        assert_eq!(machine.snapshot(), draining);
    }
}
