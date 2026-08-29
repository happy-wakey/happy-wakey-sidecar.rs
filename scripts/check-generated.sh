#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tool="${FLAGS2ENV_BIN:-${1:-}}"
mode="${2:-check}"

if [[ -z "$tool" || ! -x "$tool" ]]; then
  echo "usage: FLAGS2ENV_BIN=/absolute/path/to/flags2env $0 [tool] [check|update]" >&2
  exit 2
fi

"$tool" audit "$root/.cli-flags.toml"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

generate() {
  local language="$1"
  local output="$2"
  "$tool" generate "$language" "$root/.cli-flags.toml" \
    --name HappyWakeySidecarEnv > "$output"
}

generate rust "$tmp/env.rs"
generate dart "$tmp/env.dart"
generate typescript "$tmp/env.ts"
generate json-schema "$tmp/env.schema.json"

if [[ "$mode" == "update" ]]; then
  install -m 0444 "$tmp/env.rs" "$root/generated/rust/env.rs"
  install -m 0444 "$tmp/env.dart" "$root/generated/dart/env.dart"
  install -m 0444 "$tmp/env.ts" "$root/generated/typescript/env.ts"
  install -m 0444 "$tmp/env.schema.json" "$root/generated/json-schema/env.schema.json"
  exit 0
fi

diff -u "$root/generated/rust/env.rs" "$tmp/env.rs"
diff -u "$root/generated/dart/env.dart" "$tmp/env.dart"
diff -u "$root/generated/typescript/env.ts" "$tmp/env.ts"
diff -u "$root/generated/json-schema/env.schema.json" "$tmp/env.schema.json"
