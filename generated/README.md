# Generated contracts

These files are generated from the root `.cli-flags.toml` by the pinned
`ORESoftware/flags-2-env` toolchain. They are frozen outputs; change the TOML
authority and run `scripts/check-generated.sh --update` rather than editing an
individual language.

- `rust/env.rs`: compile-checked by the sidecar crate
- `dart/env.dart`: consumed by Flutter deployment tooling
- `typescript/env.ts`: consumed by infrastructure and external E2E tooling
- `json-schema/env.schema.json`: runtime-neutral post-coercion shape

Fields without TOML defaults are optional in flags-2-env's post-coercion type.
The Rust `Config` boundary additionally requires the product kind, product
probe URL, Shared Auth URL, and Opto Sync URL before the process can start.
