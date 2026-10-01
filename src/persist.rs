//! Session persistence — save/restore workspaces, layouts, and working directories.
//!
//! Stored at `~/.config/herdr/session.json`.
//! Optional pane screen history is stored separately at `session-history.json`.
//! Installed plugins are persisted separately at `plugins.json`.

mod io;
pub mod plugin_registry;
mod restore;
mod snapshot;
mod writer;

pub use self::io::{clear_history, load, load_history};
pub use self::restore::restore;
#[cfg(unix)]
pub use self::restore::{handoff_pane_aliases, restore_handoff};
pub use self::snapshot::{
    apply_codex_worktree_sessions, capture, capture_history, CodexPane, DirectionSnapshot,
    LayoutSnapshot, SessionHistorySnapshot, SessionSnapshot, TabSnapshot, WorkspaceSnapshot,
};
pub(crate) use self::writer::SessionWriter;
