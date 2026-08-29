# Happy Wakey sidecar agent policy

Read the root policy at `ORESoftware/my-ai/AGENTS.md` before modifying this
repository. This repository is the product-specific sidecar named by that
fleet policy.

- `src/lifecycle.rs` is the sole readiness authority. Do not derive readiness
  independently in handlers, probes, or manifests.
- Inherit generic probe serving and stderr telemetry from
  `ores-otel/ores-otel-sidecar.rs`; make product policy explicit here.
- Bind to loopback. Product overrides must never enable non-loopback binds.
- Keep stdout unused. Diagnostics go to structured stderr through the inherited
  Ores runtime.
- Never accept, persist, log, or forward Shared Auth credentials, bearer tokens,
  Opto Sync write tokens, Bluetooth payloads, or user data.
- `.cli-flags.toml` is the environment/flag authority. Generated bindings and
  schema must be regenerated together.
- `formal/sidecar_lifecycle.qnt` and Rust reducer traces must stay aligned.
- Use exact immutable Git revisions and public, credential-free build inputs.
- Work on a normal feature branch and PR. Do not rebase, force-push, reset, or
  create worktrees. Preserve unrelated state.
- Prove local gates, hosted current-SHA gates, and deployed runtime separately.
