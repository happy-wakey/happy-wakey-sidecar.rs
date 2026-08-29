// Finite model of the Happy Wakey sidecar readiness reducer.
module sidecar_lifecycle {
  type Phase = Booting | Probing | Ready | Degraded | Draining

  type State = {
    phase: Phase,
    ready_certified: bool,
    successes: int,
    failures: int,
    generation: int,
  }

  var s: State

  val success_threshold = 2
  val failure_threshold = 2

  action init = {
    s' = { phase: Booting, ready_certified: false, successes: 0, failures: 0, generation: 0 }
  }

  action configured = all {
    s.phase == Booting,
    s' = { ...s, phase: Probing },
  }

  action probe_succeeded = all {
    s.phase == Probing or s.phase == Ready or s.phase == Degraded,
    s' = {
      phase: if (s.successes + 1 >= success_threshold) Ready else Probing,
      ready_certified: s.ready_certified or s.successes + 1 >= success_threshold,
      successes: s.successes + 1,
      failures: 0,
      generation: s.generation + 1,
    },
  }

  action probe_failed = all {
    s.phase == Probing or s.phase == Ready or s.phase == Degraded,
    s' = {
      phase: if (s.failures + 1 >= failure_threshold) Degraded else s.phase,
      ready_certified: s.ready_certified and s.failures + 1 < failure_threshold,
      successes: 0,
      failures: s.failures + 1,
      generation: s.generation + 1,
    },
  }

  action shutdown = all {
    s.phase != Draining,
    s' = { ...s, phase: Draining, ready_certified: false, successes: 0, failures: 0 },
  }

  action idle = s' = s

  action step = any { configured, probe_succeeded, probe_failed, shutdown, idle }

  val counters_nonnegative = and {
    s.successes >= 0,
    s.failures >= 0,
    s.generation >= 0,
  }

  val ready_requires_certification = s.phase != Ready or s.ready_certified
  val degraded_requires_threshold = s.phase != Degraded or s.failures >= failure_threshold
  val exclusive_probe_streak = s.successes == 0 or s.failures == 0

  val sidecar_safety = and {
    counters_nonnegative,
    ready_requires_certification,
    degraded_requires_threshold,
    exclusive_probe_streak,
  }
}
