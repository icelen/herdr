# Maintaining this fork

This is a personal fork of [herdrdev/herdr](https://github.com/herdrdev/herdr) that adds a native integration for the Trae (`traex`) CLI, which isn't supported upstream. herdr's agent integrations (Claude, Codex, Cursor, etc.) are hardcoded into the compiled binary — there's no config-level way to add one, so this lives as a small source patch (`Agent::Trae` in `src/detect/mod.rs`, `IntegrationTarget::Trae` wired through `src/integration/*.rs` and `src/cli/integration.rs`) sitting on top of upstream.

Trae is a literal fork of Codex CLI (same `hooks.json` schema, same `[features] hooks = true` config gate), so the integration mirrors `install_codex`, but reports full idle/working/blocked state via hooks (like Kimi/Mastracode) rather than a screen-scrape manifest, since Trae fires the same rich hook event set and has no screen manifest built yet.

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
