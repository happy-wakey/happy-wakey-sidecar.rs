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

## Repository-local Git worktrees

- Create or use a Git worktree only when the human operator explicitly authorizes it for the current task. Concurrency or a dirty checkout is not permission by itself.
- Put every authorized worktree at `<repository-root>/tmp/worktrees/<name>`; from the repository root, use `./tmp/worktrees/<name>`. Never place worktrees beside repositories or organization directories.
- Keep `tmp`, `temp`, `tmp/worktrees`, and `temp/worktrees` ignored in the repository-root `.gitignore`. Do not commit files from those directories.
- Relocate or remove a worktree only when the operator explicitly requests it. Before removal, preserve and publish intended changes, verify its commit is represented on the target branch, and confirm there are no tracked, untracked, ignored-sensitive, or in-use files that must survive. Remove it with `git worktree remove <path>` without `--force`; never delete a worktree directory with `rm`.
