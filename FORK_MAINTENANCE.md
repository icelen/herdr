# Maintaining this fork

This is a personal fork of [herdrdev/herdr](https://github.com/herdrdev/herdr) that adds a native integration for the Trae (`traex`) CLI, which isn't supported upstream. herdr's agent integrations (Claude, Codex, Cursor, etc.) are hardcoded into the compiled binary — there's no config-level way to add one, so this lives as a small source patch (`Agent::Trae` in `src/detect/mod.rs`, `IntegrationTarget::Trae` wired through `src/integration/*.rs` and `src/cli/integration.rs`) sitting on top of upstream.

Trae is a fork of Codex CLI, but current Trae (traecli 0.207+) no longer runs hooks from the legacy user-level `~/.trae/hooks.json`, and only runs plugin hooks it has a trusted hash for. So `herdr integration install trae` (integration v3):

- writes a local Trae plugin to `~/.trae/herdr-plugin/` (`.codex-plugin/plugin.json`, `hooks.json`, `herdr-agent-state.sh`) and registers it with `traex plugin install --type local … --name herdr` (becomes `herdr@local`; override the CLI with `HERDR_TRAE_CLI`);
- records one `[hooks.state."herdr@local:hooks.json:<event>:0:0"] trusted_hash = "sha256:…"` per hook in `~/.trae/traecli.toml`. The hash is SHA-256 of the canonical JSON `{"event_name","hooks":[{"async":false,"command","timeout","type"}]}`; golden values Trae accepted live are pinned in `trae_trusted_hash_matches_hashes_trae_accepted`. If Trae changes this scheme, hooks silently stop firing and state falls back to the screen manifest (`src/detect/manifests/trae.toml`);
- removes the v1-v2 legacy `~/.trae/hooks.json` entries and script.

Hooks report idle/working/blocked (like Kimi/Mastracode); screen detection keeps running alongside as a fallback.

Trae is also registered as an official resume source in `src/agent_resume.rs` (`is_official_agent_source`, `plan`, `persisted_session_from_launch_args`): the session id the hook reports on `SessionStart` is kept, and Herdr restores the pane with `traex resume <id>`. Being official also means Herdr ignores the hook's `release` report and ends the Trae session when the `traex` process exits, like its built-in integrations.

Like Codex, traex can run sessions on a shared app-server daemon (`traex dashboard`; `daemon_auto_start` is off by default, so only when a daemon is already running). The daemon runs every session's hooks with the environment of the pane that started it, so the hook (integration v4) exits when `traex app-server` is among its three nearest ancestor processes. Both the Codex and Trae guards use `ps -ww`: without it macOS truncates long command lines, and the daemon's ` app-server` sits after a long install path.

## Codex idle detection

Upstream (v0.9.1, #4563) stopped inferring Codex `idle` from the screen, so finished Codex panes stay `unknown` and sort last. The fork adds two rules to `src/detect/manifests/codex.toml` (mirrored in `distribution/agent-detection/codex.toml`) that read Codex's `run-state` terminal-title item: `Ready` → idle, `Working`/`Thinking` → working. `Action Required` → blocked is upstream's. The title comes from the Codex TUI in each pane, so it works with Codex's shared app-server daemon (`codex agents`, `codex queue`).

Upstream v0.9.3 (#4756) also reports Codex `working`/`idle` from hooks, but only reliably with `codex --no-daemon`: the shared daemon runs every session's hooks with the environment of the pane that started it, so `HERDR_PANE_ID` points at the wrong pane. The fork's `src/integration/assets/codex/herdr-agent-state.sh` exits when its parent process is `codex app-server` (checked with `ps -ww`), so daemon sessions rely on the title rules instead (the Windows `.ps1` hook has no such guard). The guard doesn't bump `CODEX_INTEGRATION_VERSION`, to avoid colliding with a future upstream version; reinstall the Codex integration after changing the script.

It needs `run-state` in `~/.codex/config.toml`. Without it Codex falls back to upstream behavior:

```toml
[tui]
terminal_title = ["activity", "run-state", "project-name"]
```

Set this in the file, not with `codex -c`: any `-c` override makes Codex run without the shared daemon.

Because daemon sessions can't report their session id to a pane, the Codex hook also records each main-thread session (subagent threads are skipped) in the session's worktree: `herdr-codex-session` under `git rev-parse --git-path` (for a linked worktree, `.git/worktrees/<name>/`, so it stays out of `git status` and goes away with `git worktree remove`). On every session save, `apply_codex_worktree_sessions` (`src/persist/snapshot.rs`, called from the save thread in `src/app/session.rs`) replaces the saved session of each pane running Codex with its worktree's record (`codex_worktree_session` in `src/agent_resume.rs`), so restarts resume the latest session even after `/new`, `/resume`, or a fresh `codex`. This assumes one resume-worthy Codex session per worktree; the last one started wins.

The daemon also runs every session's shell commands with the environment of the pane that started it, so `$HERDR_WORKSPACE_ID`, `$HERDR_TAB_ID` and `$HERDR_PANE_ID` inside a daemon-hosted Codex session name that pane, not the session's own. The same save pass writes the pane's real ids to `herdr-pane-ids` beside `herdr-codex-session` (`write_codex_worktree_env`, rewritten only when they change); an agent loads them with `. "$(git rev-parse --git-path herdr-pane-ids)"`, and the `herdr-workspace-labels` plugin (`herdr-label`, `herdr-env`, `herdr-phase`) reads it to pick the workspace. It is not named `herdr-env` because that is the label command. The file is refreshed only while a pane in that worktree is running Codex.

The Codex installer keeps existing hook entries in place (Codex trusts hooks by their position in `hooks.json`) and sets the `Interrupt` hook's timeout to 3s in place, since Codex clamps it and otherwise warns on every start.

`read_remote_manifest` in `src/detect/manifest.rs` ignores the downloaded Codex manifest (outside tests) so a newer upstream catalog manifest can't replace the fork's rules. Upstream Codex rule changes therefore only arrive through merges. When merging, keep both rules and a `version` at least as new as upstream's.

## Remotes

- `origin` → this fork (`git@github.com:icelen/herdr.git`)
- `upstream` → `https://github.com/herdrdev/herdr.git`

## Toolchain requirements

- Rust via **rustup** (`brew install rustup`, keg-only: put `/opt/homebrew/opt/rustup/bin` first on `PATH`). rustup honors `rust-toolchain.toml`, so the pinned toolchain and clippy are picked up automatically. Don't use Homebrew's `rust` formula: it's usually newer than the pin, and its clippy flags lints upstream hasn't adopted, so `just ci` fails.
- **Zig 0.16.0** (upstream bumped from 0.15.2 in v0.9.1; `build.rs` and `vendor/libghostty-vt/build.zig.zon` say which version is required). Homebrew's default formula matches: `brew install zig`. `build.rs` uses `zig` from `PATH` when `ZIG` is unset. When switching zig versions, `rm -rf vendor/libghostty-vt/.zig-cache` first, since it caches the SDK path.
  - Zig 0.16's built-in HTTP client can fail with `TlsInitializationFailed` when fetching libghostty-vt's packages here, even though `curl` downloads the same URLs fine. Seed zig's package cache by hand: download each URL named in the error with `curl -L -o <file> <url>`, then run `zig fetch <file>` **from inside `vendor/libghostty-vt/`** (0.16 requires a `build.zig` in the working directory). The printed hash should match the one in `build.zig.zon`.
- `just` and `cargo-nextest` (`brew install just cargo-nextest`). Plain `cargo test` runs everything in one process and can die with SIGPIPE; use nextest.

## Syncing with upstream

The Trae patch is a single commit on top of upstream, so a plain merge only needs one conflict-resolution pass:

```sh
git fetch upstream
git merge upstream/master
```

Likely conflict hot spots (because they're exactly what the patch touches):

- `src/integration/registry.rs` — the `integration_specs()` array (size + entries)
- `src/detect/mod.rs` / `src/api/schema/integrations.rs` — the `Agent` / `IntegrationTarget` enums
- `src/integration/config_edit.rs` — the hook-editing helper functions
- `docs/next/api/herdr-api.schema.json` — a generated snapshot, safe to regenerate rather than hand-merge (see below)

Prefer rebasing instead? `git rebase upstream/master` works too (still just one round of conflicts since it's one commit), but you'll need to force-push to `origin` afterward since it rewrites history.

## Building and installing

```sh
just ci 'not binary(live_handoff)'   # clippy + nextest + maintenance tests, same filter upstream CI uses on macOS
```

If your global git config sets `diff.external` (e.g. difftastic), `scripts/test_release.py` fails because it builds real patches with `git diff`. Run the suite with `GIT_CONFIG_GLOBAL=/dev/null` to isolate it.

Merge gotcha: `Agent::ALL` and `Agent::SCREEN_MANIFEST_AGENTS` in `src/detect/mod.rs` have explicit array lengths. When upstream adds an agent, git merges the entries cleanly but keeps one side's length, so the build fails with "expected an array with a size of N". Set the length to the real count.

`live_handoff` tests are Linux-only (upstream CI excludes them on macOS too); two of them fail on macOS regardless of the Trae patch.

If `generated_protocol_schema_artifact_is_current` fails after a merge, regenerate the snapshot instead of hand-editing it:

```sh
HERDR_UPDATE_API_SCHEMA=1 cargo test generated_protocol_schema_artifact_is_current
```

Then build and install:

```sh
cargo build --release --locked
cp target/release/herdr ~/bin/herdr.new && mv ~/bin/herdr.new ~/bin/herdr   # rename, don't overwrite the running binary in place
```

Restarting the server to pick up the new binary disconnects every pane in your current session — do it when that's convenient, not mid-task:

```sh
herdr server stop
herdr
```

If `TRAE_INTEGRATION_VERSION` changed (or hooks look stale), reinstall the integration:

```sh
herdr integration install trae
```

## Going fully upstream

The zero-maintenance alternative is opening this patch as a PR against `herdrdev/herdr` instead of hand-merging forever. Not done yet, but the branch is ready for it whenever.
