# Maintaining this fork

This is a personal fork of [ogulcancelik/herdr](https://github.com/ogulcancelik/herdr) that adds a native integration for the Trae (`traex`) CLI, which isn't supported upstream. herdr's agent integrations (Claude, Codex, Cursor, etc.) are hardcoded into the compiled binary — there's no config-level way to add one, so this lives as a small source patch (`Agent::Trae` in `src/detect/mod.rs`, `IntegrationTarget::Trae` wired through `src/integration/*.rs` and `src/cli/integration.rs`) sitting on top of upstream.

Trae is a literal fork of Codex CLI (same `hooks.json` schema, same `[features] hooks = true` config gate), so the integration mirrors `install_codex`, but reports full idle/working/blocked state via hooks (like Kimi/Mastracode) rather than a screen-scrape manifest, since Trae fires the same rich hook event set and has no screen manifest built yet.

## Remotes

- `origin` → this fork (`git@github.com:icelen/herdr.git`)
- `upstream` → `https://github.com/ogulcancelik/herdr.git`

## Toolchain requirements

- Rust ≥ 1.88 (some deps require it; homebrew's rust may lag behind — `brew upgrade rust` if `cargo build` complains about `rustc` version).
- **zig 0.15.2 exactly**, to build the vendored `libghostty-vt` dependency. Homebrew's zig (0.16+) has breaking API changes (`std.Build.Dir.readFileAlloc` signature, etc.) and will fail with a compile error inside `build.zig`. Get the pinned version from ziglang.org if you don't already have it:
  ```
  curl -sL -o /tmp/zig.tar.xz "https://ziglang.org/download/0.15.2/zig-aarch64-macos-0.15.2.tar.xz"
  tar -xf /tmp/zig.tar.xz -C ~/.local/share/
  ```
  (swap the URL's arch/OS suffix as needed; use `zig-x86_64-...` etc.)

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
export ZIG=~/.local/share/zig-0.15.2/zig   # adjust path if installed elsewhere
cargo test
```

If `generated_protocol_schema_artifact_is_current` fails after a merge, regenerate the snapshot instead of hand-editing it:

```sh
HERDR_UPDATE_API_SCHEMA=1 cargo test generated_protocol_schema_artifact_is_current
```

Then build and install:

```sh
cargo build --release
cp target/release/herdr ~/bin/herdr
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

The zero-maintenance alternative is opening this patch as a PR against `ogulcancelik/herdr` instead of hand-merging forever. Not done yet, but the branch is ready for it whenever.
