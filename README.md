# Happy Wakey sidecar

`happy-wakey-sidecar` is the product-specific, fail-closed Kubernetes sidecar
for Happy Wakey API and web pods. It inherits the probe server, structured
stderr telemetry, loopback-only bind policy, and exec-probe behavior from
[`ores-otel-sidecar`](https://github.com/ores-otel/ores-otel-sidecar.rs) at an
immutable Git revision.

The sidecar owns no user data, credentials, or business decisions. It verifies
that one local product process is healthy and that the configured Shared Auth
and Opto Sync authorities satisfy the Happy Wakey transport policy. It then
publishes a bounded product snapshot through the inherited `/healthz`,
`/readyz`, and `/metrics` endpoints on loopback only.

## Authority and readiness

The Rust reducer in `src/lifecycle.rs` is the sole operational authority:

```text
Booting --Configured--> Probing --success threshold--> Ready
                              \--failure threshold--> Degraded
Ready --failure threshold--> Degraded --success threshold--> Ready
any live state --Shutdown--> Draining
```

Invalid transitions do not mutate state. A single sequential worker performs
probes, so stale asynchronous completions cannot overtake newer results.
Readiness requires all of the following:

1. a syntactically valid, credential-free local HTTP probe URL;
2. a successful HTTP 200 response from the loopback product process;
3. validated Shared Auth and Opto Sync authority URLs;
4. the configured consecutive-success threshold.

`/healthz` reports that the sidecar process is alive. `/readyz` reports the
reducer's capability. Kubernetes uses the inherited same-container exec probe
for liveness; the sidecar does not add a Pod readiness probe that could remove
the product from its Service independently of the product container.

## Configuration

`.cli-flags.toml` is the flags-2-env authority. The committed Rust, Dart,
TypeScript, and JSON Schema files under `generated/` are derived from it and
checked in CI.

| Environment | Default | Meaning |
| --- | --- | --- |
| `HAPPY_WAKEY_SIDECAR_BIND` | `127.0.0.1:9090` | inherited diagnostic listener |
| `HAPPY_WAKEY_SIDECAR_PRODUCT_KIND` | required | `api` or `web` |
| `HAPPY_WAKEY_SIDECAR_PRODUCT_PROBE_URL` | required | loopback HTTP health endpoint |
| `HAPPY_WAKEY_SHARED_AUTH_BASE_URL` | required | Shared Auth authority |
| `HAPPY_WAKEY_OPTO_SYNC_BASE_URL` | required | Opto Sync authority |
| `HAPPY_WAKEY_SIDECAR_INTERVAL_MS` | `2000` | probe interval, 250–60000 ms |
| `HAPPY_WAKEY_SIDECAR_SUCCESS_THRESHOLD` | `2` | successes required for Ready, 1–10 |
| `HAPPY_WAKEY_SIDECAR_FAILURE_THRESHOLD` | `3` | failures required for Degraded, 1–10 |

Remote authorities require HTTPS. Cleartext HTTP is accepted only for loopback
and Kubernetes `.svc` names. URL userinfo, query strings, and fragments are
rejected. No Shared Auth service credential or Opto Sync write token belongs in
this process.

## Verification

```bash
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
npx --yes @informalsystems/quint@0.32.0 typecheck formal/sidecar_lifecycle.qnt
npx --yes @informalsystems/quint@0.32.0 run formal/sidecar_lifecycle.qnt --invariant sidecar_safety --max-samples 10000 --max-steps 30
```

The formal model and deterministic Rust traces prove the bounded reducer
properties only. Hosted CI does not prove a deployed cluster, live Shared Auth
or Opto Sync service, or Bluetooth hardware behavior.
