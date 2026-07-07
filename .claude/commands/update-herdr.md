Update this herdr fork from upstream and rebuild it, following FORK_MAINTENANCE.md in this repo. Do this:

1. `git fetch upstream` then `git merge upstream/master`. If there are conflicts, they're most likely in `src/integration/registry.rs` (`integration_specs()`), the `Agent`/`IntegrationTarget` enums in `src/detect/mod.rs` / `src/api/schema/integrations.rs`, or `src/integration/config_edit.rs` — resolve by re-applying the Trae wiring, not by dropping it. `docs/next/api/herdr-api.schema.json` is a generated snapshot — don't hand-merge it, just regenerate afterward.
2. `export ZIG=~/.local/share/zig-0.15.2/zig` (adjust if the pinned zig lives elsewhere) and run `cargo test`.
3. If `generated_protocol_schema_artifact_is_current` fails, regenerate it: `HERDR_UPDATE_API_SCHEMA=1 cargo test generated_protocol_schema_artifact_is_current`, then re-run the full suite.
4. If anything else fails, stop and report it — don't push past a real test failure.
5. `cargo build --release`, then `cp target/release/herdr ~/bin/herdr`.
6. Stop here and ask before doing anything further. Restarting the herdr server (`herdr server stop && herdr`) disconnects every pane in the current session, so confirm timing with me before running it. Same for `herdr integration install trae` if the integration version bumped — check first, don't run blindly.
7. If the merge went cleanly and tests pass, ask whether to push the updated `master` to `origin` (the fork).
