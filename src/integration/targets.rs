use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use super::claude_settings::{
    install as install_claude_settings, uninstall as uninstall_claude_settings,
};
use super::command::hook_command;
#[cfg(windows)]
use super::command::powershell_encoded_hook_command;
#[cfg(not(windows))]
use super::command::shell_single_quote;
use super::config_edit::{
    build_codex_config_with_hooks, build_kimi_config_with_hooks, ensure_command_hook,
    ensure_direct_command_hook, ensure_flat_command_hook, ensure_hermes_plugin_enabled,
    ensure_hooks_object, ensure_simple_command_hook, hooks_object_if_present,
    remove_direct_hook_commands, remove_flat_command_hook, remove_hermes_plugin_enabled,
    remove_hook_commands, remove_kimi_config_block, remove_simple_command_hook,
};
use super::config_file::{check_config_targets, write_config};
use super::env::{
    antigravity_cli_dir, claude_dir, codex_dir, copilot_dir, cursor_dir, devin_dir, droid_dir,
    grok_dir, hermes_dir, hermes_plugin_dir, kilo_dir, kimi_dir, letta_dir, mastracode_dir,
    omp_extension_dir, opencode_dir, opencode_state_dir, pi_extension_dir, qodercli_dir, qwen_dir,
    trae_dir,
};
use super::file_ops::{
    make_executable, remove_dir_all_if_exists, remove_file_if_exists, remove_legacy_bash_hook_file,
};
use super::opencode_config::{
    add_cli_plugin, add_tui_plugin, remove_cli_plugin, remove_tui_plugin,
    validate_tui_plugin_config,
};
use super::types::{
    AntigravityCliInstallPaths, AntigravityCliUninstallResult, ClaudeInstallPaths,
    ClaudeUninstallResult, CodexInstallPaths, CodexUninstallResult, CopilotInstallPaths,
    CopilotUninstallResult, CursorInstallPaths, CursorUninstallResult, DevinInstallPaths,
    DevinUninstallResult, DroidInstallPaths, DroidUninstallResult, GrokInstallPaths,
    GrokUninstallResult, HermesInstallPaths, HermesUninstallResult, KiloInstallPaths,
    KiloUninstallResult, KimiInstallPaths, KimiUninstallResult, LettaInstallPaths,
    LettaUninstallResult, MastracodeInstallPaths, MastracodeUninstallResult, OmpInstallPaths,
    OmpUninstallResult, OpenCodeInstallPaths, OpenCodeUninstallResult, PiUninstallResult,
    QodercliInstallPaths, QodercliUninstallResult, QwenInstallPaths, QwenUninstallResult,
    TraeInstallPaths, TraeUninstallResult,
};
use super::{
    ANTIGRAVITY_CLI_HOOK_ASSET, ANTIGRAVITY_CLI_HOOK_BLOCK_NAME, ANTIGRAVITY_CLI_HOOK_EVENTS,
    ANTIGRAVITY_CLI_HOOK_INSTALL_NAME, ANTIGRAVITY_CLI_HOOK_TIMEOUT_SEC, CLAUDE_HOOK_ASSET,
    CLAUDE_HOOK_INSTALL_NAME, CODEX_HOOK_ASSET, CODEX_HOOK_INSTALL_NAME, COPILOT_HOOK_ASSET,
    COPILOT_HOOK_EVENTS, COPILOT_HOOK_INSTALL_NAME, COPILOT_REMOVED_LIFECYCLE_HOOK_EVENTS,
    CURSOR_HOOK_ASSET, CURSOR_HOOK_INSTALL_NAME, DEVIN_HOOK_ASSET, DEVIN_HOOK_EVENTS,
    DEVIN_HOOK_INSTALL_NAME, DEVIN_REMOVED_LIFECYCLE_HOOK_EVENTS, DROID_HOOK_ASSET,
    DROID_HOOK_EVENTS, DROID_HOOK_INSTALL_NAME, DROID_REMOVED_LIFECYCLE_HOOK_EVENTS,
    GROK_HOOK_ASSET, GROK_HOOK_CONFIG_INSTALL_NAME, GROK_HOOK_INSTALL_NAME,
    HERMES_PLUGIN_INIT_ASSET, HERMES_PLUGIN_INIT_INSTALL_NAME, HERMES_PLUGIN_MANIFEST_ASSET,
    HERMES_PLUGIN_MANIFEST_INSTALL_NAME, KILO_PLUGIN_ASSET, KILO_PLUGIN_INSTALL_NAME,
    KIMI_HOOK_ASSET, KIMI_HOOK_INSTALL_NAME, LETTA_HOOK_ASSET, LETTA_HOOK_INSTALL_NAME,
    LETTA_HOOK_TIMEOUT_MS, MASTRACODE_HOOK_ASSET, MASTRACODE_HOOK_EVENTS,
    MASTRACODE_HOOK_INSTALL_NAME, MASTRACODE_HOOK_TIMEOUT_MS, MASTRACODE_REMOVED_HOOK_EVENTS,
    OMP_EXTENSION_ASSET, OMP_EXTENSION_INSTALL_NAME, OPENCODE_PLUGIN_ASSET,
    OPENCODE_PLUGIN_INSTALL_NAME, OPENCODE_TUI_PLUGIN_ASSET, OPENCODE_TUI_PLUGIN_INSTALL_NAME,
    OPENCODE_TUI_PLUGIN_SPEC, PI_EXTENSION_ASSET, PI_EXTENSION_INSTALL_NAME, QODERCLI_HOOK_ASSET,
    QODERCLI_HOOK_EVENTS, QODERCLI_HOOK_INSTALL_NAME, QODERCLI_REMOVED_LIFECYCLE_HOOK_EVENTS,
    QWEN_HOOK_ASSET, QWEN_HOOK_EVENTS, QWEN_HOOK_INSTALL_NAME, TRAE_CLI_ENV_VAR, TRAE_CLI_NAMES,
    TRAE_HOOK_ASSET, TRAE_HOOK_EVENTS, TRAE_HOOK_INSTALL_NAME, TRAE_HOOK_TIMEOUT_SEC,
    TRAE_LEGACY_HOOK_EVENTS, TRAE_PLUGIN_DIR_NAME, TRAE_PLUGIN_MARKETPLACE, TRAE_PLUGIN_NAME,
};

fn ensure_extension_dir(dir: &Path, agent: &str) -> io::Result<()> {
    if dir.is_dir() {
        return Ok(());
    }
    if dir.parent().is_some_and(|parent| parent.is_dir()) {
        return fs::create_dir_all(dir);
    }
    Err(io::Error::other(format!(
        "{agent} extension directory not found at {}. install {agent} first",
        dir.display()
    )))
}

pub(crate) fn install_pi() -> io::Result<PathBuf> {
    let dir = pi_extension_dir()?;
    ensure_extension_dir(&dir, "pi")?;

    let path = dir.join(PI_EXTENSION_INSTALL_NAME);
    fs::write(&path, PI_EXTENSION_ASSET)?;
    Ok(path)
}

pub(crate) fn install_omp() -> io::Result<OmpInstallPaths> {
    let dir = omp_extension_dir()?;
    let pi_dir = pi_extension_dir()?;
    if dir == pi_dir {
        return Err(io::Error::other(format!(
            "Pi and OMP resolve to the same extension directory at {}; configure separate agent directories before installing OMP",
            dir.display()
        )));
    }
    ensure_extension_dir(&dir, "omp")?;

    let removed_legacy_pi_extension = remove_legacy_pi_extension_from_omp_dir(&dir)?;
    let extension_path = dir.join(OMP_EXTENSION_INSTALL_NAME);
    fs::write(&extension_path, OMP_EXTENSION_ASSET)?;
    Ok(OmpInstallPaths {
        extension_path,
        removed_legacy_pi_extension,
    })
}

pub(crate) fn remove_legacy_pi_extension_from_omp_dir(dir: &Path) -> io::Result<bool> {
    let legacy_path = dir.join(PI_EXTENSION_INSTALL_NAME);
    if !legacy_path.is_file() {
        return Ok(false);
    }

    let content = fs::read_to_string(&legacy_path)?;
    if content.contains("HERDR_INTEGRATION_ID=pi") {
        fs::remove_file(legacy_path)?;
        return Ok(true);
    }

    Ok(false)
}

pub(crate) fn install_claude() -> io::Result<ClaudeInstallPaths> {
    let dir = claude_dir()?;
    check_config_targets(&dir, &["settings.json"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "claude directory not found at {}. install claude code first",
            dir.display()
        )));
    }

    let hooks_dir = dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join(CLAUDE_HOOK_INSTALL_NAME);
    fs::write(&hook_path, CLAUDE_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    let settings_path = dir.join("settings.json");
    let existing_settings = if settings_path.is_file() {
        fs::read_to_string(&settings_path)?
    } else {
        "{}".to_string()
    };
    let updated_settings = install_claude_settings(&existing_settings, &settings_path, &hook_path)?;
    remove_legacy_bash_hook_file(&hook_path)?;

    if updated_settings != existing_settings {
        write_config(&settings_path, updated_settings)?;
    }

    Ok(ClaudeInstallPaths {
        hook_path,
        settings_path,
    })
}

pub(crate) fn install_codex() -> io::Result<CodexInstallPaths> {
    let dir = codex_dir()?;
    check_config_targets(&dir, &["hooks.json", "config.toml"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "codex config directory not found at {}. install codex first",
            dir.display()
        )));
    }

    let hook_path = dir.join(CODEX_HOOK_INSTALL_NAME);
    fs::write(&hook_path, CODEX_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    let hooks_path = dir.join("hooks.json");
    let mut hooks_file = if hooks_path.is_file() {
        serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?).map_err(|err| {
            io::Error::other(format!("failed to parse {}: {err}", hooks_path.display()))
        })?
    } else {
        json!({})
    };

    let hooks = ensure_hooks_object(
        &mut hooks_file,
        &hooks_path,
        "codex hooks file",
        "codex hooks file hooks",
    )?;
    remove_hook_commands(hooks, "PermissionRequest", &hook_path, Some("blocked"))?;
    remove_hook_commands(hooks, "SessionStart", &hook_path, Some("idle"))?;
    remove_hook_commands(hooks, "UserPromptSubmit", &hook_path, Some("working"))?;
    remove_hook_commands(hooks, "PreToolUse", &hook_path, Some("working"))?;
    remove_hook_commands(hooks, "Stop", &hook_path, Some("idle"))?;
    remove_hook_commands(hooks, "SessionStart", &hook_path, Some("session"))?;
    ensure_command_hook(
        hooks,
        "SessionStart",
        hook_command(&hook_path, Some("session")),
        10,
        None,
    )?;
    remove_legacy_bash_hook_file(&hook_path)?;

    write_config(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;

    let config_path = dir.join("config.toml");
    let existing_config = if config_path.is_file() {
        fs::read_to_string(&config_path)?
    } else {
        String::new()
    };
    let new_config = build_codex_config_with_hooks(&existing_config);
    if new_config != existing_config {
        write_config(&config_path, new_config)?;
    }

    Ok(CodexInstallPaths {
        hook_path,
        hooks_path,
        config_path,
    })
}

pub(crate) fn install_kimi() -> io::Result<KimiInstallPaths> {
    let dir = kimi_dir()?;
    check_config_targets(&dir, &["config.toml"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "kimi code config directory not found at {}. install kimi code first",
            dir.display()
        )));
    }

    let hooks_dir = dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join(KIMI_HOOK_INSTALL_NAME);
    fs::write(&hook_path, KIMI_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    let config_path = dir.join("config.toml");
    let existing_config = if config_path.is_file() {
        fs::read_to_string(&config_path)?
    } else {
        String::new()
    };
    let new_config = build_kimi_config_with_hooks(&existing_config, &hook_path);
    if new_config != existing_config {
        write_config(&config_path, new_config)?;
    }
    remove_legacy_bash_hook_file(&hook_path)?;

    Ok(KimiInstallPaths {
        hook_path,
        config_path,
    })
}

pub(crate) fn install_copilot() -> io::Result<CopilotInstallPaths> {
    let dir = copilot_dir()?;
    check_config_targets(&dir, &["settings.json"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "copilot config directory not found at {}. install github copilot cli first",
            dir.display()
        )));
    }

    let hooks_dir = dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join(COPILOT_HOOK_INSTALL_NAME);
    fs::write(&hook_path, COPILOT_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    let settings_path = dir.join("settings.json");
    let mut settings = if settings_path.is_file() {
        serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?).map_err(|err| {
            io::Error::other(format!(
                "failed to parse {}: {err}",
                settings_path.display()
            ))
        })?
    } else {
        json!({})
    };

    let hooks = ensure_hooks_object(
        &mut settings,
        &settings_path,
        "copilot settings",
        "copilot settings hooks",
    )?;
    let command = hook_command(&hook_path, None);
    for event in COPILOT_REMOVED_LIFECYCLE_HOOK_EVENTS {
        remove_direct_hook_commands(hooks, event, &hook_path, None)?;
    }
    for event in COPILOT_HOOK_EVENTS {
        remove_direct_hook_commands(hooks, event, &hook_path, None)?;
    }
    for event in COPILOT_HOOK_EVENTS {
        ensure_direct_command_hook(hooks, event, command.clone(), 10, None)?;
    }
    remove_legacy_bash_hook_file(&hook_path)?;

    write_config(&settings_path, serde_json::to_string_pretty(&settings)?)?;

    Ok(CopilotInstallPaths {
        hook_path,
        settings_path,
    })
}

pub(crate) fn install_devin() -> io::Result<DevinInstallPaths> {
    let dir = devin_dir()?;
    check_config_targets(&dir, &["config.json"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "devin config directory not found at {}. install devin cli first",
            dir.display()
        )));
    }

    let hook_path = dir.join(DEVIN_HOOK_INSTALL_NAME);
    fs::write(&hook_path, DEVIN_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    let settings_path = dir.join("config.json");
    let mut settings = if settings_path.is_file() {
        serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?).map_err(|err| {
            io::Error::other(format!(
                "failed to parse {}: {err}",
                settings_path.display()
            ))
        })?
    } else {
        json!({})
    };

    let hooks = ensure_hooks_object(
        &mut settings,
        &settings_path,
        "devin settings",
        "devin settings hooks",
    )?;
    for (event, action) in DEVIN_REMOVED_LIFECYCLE_HOOK_EVENTS {
        remove_hook_commands(hooks, event, &hook_path, Some(action))?;
    }
    for (event, action) in DEVIN_HOOK_EVENTS {
        remove_hook_commands(hooks, event, &hook_path, Some(action))?;
    }
    for (event, action) in DEVIN_HOOK_EVENTS {
        ensure_command_hook(
            hooks,
            event,
            hook_command(&hook_path, Some(action)),
            10,
            None,
        )?;
    }
    remove_legacy_bash_hook_file(&hook_path)?;

    write_config(&settings_path, serde_json::to_string_pretty(&settings)?)?;

    Ok(DevinInstallPaths {
        hook_path,
        settings_path,
    })
}

pub(crate) fn install_droid() -> io::Result<DroidInstallPaths> {
    let dir = droid_dir()?;
    check_config_targets(&dir, &["settings.json", "hooks.json"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "droid config directory not found at {}. install droid first",
            dir.display()
        )));
    }

    let hooks_dir = dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join(DROID_HOOK_INSTALL_NAME);
    fs::write(&hook_path, DROID_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    let settings_path = dir.join("settings.json");
    let mut settings = if settings_path.is_file() {
        serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?).map_err(|err| {
            io::Error::other(format!(
                "failed to parse {}: {err}",
                settings_path.display()
            ))
        })?
    } else {
        json!({})
    };

    let hooks = ensure_hooks_object(
        &mut settings,
        &settings_path,
        "droid settings",
        "droid settings hooks",
    )?;
    remove_hook_commands(hooks, "SessionStart", &hook_path, None)?;
    for (event, action) in DROID_REMOVED_LIFECYCLE_HOOK_EVENTS {
        remove_hook_commands(hooks, event, &hook_path, Some(action))?;
    }
    for (event, action) in DROID_HOOK_EVENTS {
        remove_hook_commands(hooks, event, &hook_path, Some(action))?;
    }
    for (event, action) in DROID_HOOK_EVENTS {
        ensure_command_hook(
            hooks,
            event,
            hook_command(&hook_path, Some(action)),
            10,
            None,
        )?;
    }
    remove_legacy_bash_hook_file(&hook_path)?;

    write_config(&settings_path, serde_json::to_string_pretty(&settings)?)?;

    let hooks_path = dir.join("hooks.json");
    let mut updated_legacy_hooks = false;
    if hooks_path.is_file() {
        let mut hooks_file = serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?)
            .map_err(|err| {
                io::Error::other(format!("failed to parse {}: {err}", hooks_path.display()))
            })?;
        if let Some(hooks) = hooks_object_if_present(
            &mut hooks_file,
            &hooks_path,
            "droid hooks file",
            "droid hooks file hooks",
        )? {
            updated_legacy_hooks = remove_hook_commands(hooks, "SessionStart", &hook_path, None)?;
            for (event, action) in DROID_REMOVED_LIFECYCLE_HOOK_EVENTS {
                updated_legacy_hooks |=
                    remove_hook_commands(hooks, event, &hook_path, Some(action))?;
            }
            for (event, action) in DROID_HOOK_EVENTS {
                updated_legacy_hooks |=
                    remove_hook_commands(hooks, event, &hook_path, Some(action))?;
            }
        }
        if updated_legacy_hooks {
            write_config(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;
        }
    }

    Ok(DroidInstallPaths {
        hook_path,
        hooks_path,
        settings_path,
        updated_legacy_hooks,
    })
}

pub(crate) fn install_opencode() -> io::Result<OpenCodeInstallPaths> {
    let dir = opencode_dir()?;
    check_config_targets(&dir, &["tui.jsonc", "tui.json", "cli.json"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "opencode config directory not found at {}. install opencode first",
            dir.display()
        )));
    }

    validate_tui_plugin_config(&dir)?;
    let plugins_dir = dir.join("plugins");
    fs::create_dir_all(&plugins_dir)?;

    let plugin_path = plugins_dir.join(OPENCODE_PLUGIN_INSTALL_NAME);
    fs::write(&plugin_path, OPENCODE_PLUGIN_ASSET)?;
    let tui_plugin_path = dir.join(OPENCODE_TUI_PLUGIN_INSTALL_NAME);
    fs::write(&tui_plugin_path, OPENCODE_TUI_PLUGIN_ASSET)?;
    let tui_config_path = add_tui_plugin(&dir, OPENCODE_TUI_PLUGIN_SPEC)?;
    let v2_dir = dir.join(super::OPENCODE_V2_TUI_PLUGIN_DIR);
    fs::create_dir_all(&v2_dir)?;
    fs::write(v2_dir.join("tui.js"), super::OPENCODE_V2_TUI_PLUGIN_ASSET)?;
    let cli_config_path = add_cli_plugin(
        &dir,
        &opencode_state_dir()?,
        super::OPENCODE_V2_TUI_PLUGIN_SPEC,
    )?;

    Ok(OpenCodeInstallPaths {
        plugin_path,
        tui_plugin_path,
        tui_config_path,
        cli_config_path,
    })
}

pub(crate) fn install_kilo() -> io::Result<KiloInstallPaths> {
    let dir = kilo_dir()?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "kilo config directory not found at {}. install kilo first",
            dir.display()
        )));
    }

    let plugins_dir = dir.join("plugin");
    fs::create_dir_all(&plugins_dir)?;

    let plugin_path = plugins_dir.join(KILO_PLUGIN_INSTALL_NAME);
    fs::write(&plugin_path, KILO_PLUGIN_ASSET)?;

    Ok(KiloInstallPaths { plugin_path })
}

pub(crate) fn install_hermes() -> io::Result<HermesInstallPaths> {
    let dir = hermes_dir()?;
    check_config_targets(&dir, &["config.yaml"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "hermes config directory not found at {}. install hermes agent first",
            dir.display()
        )));
    }

    let plugin_dir = hermes_plugin_dir()?;
    fs::create_dir_all(&plugin_dir)?;
    fs::write(
        plugin_dir.join(HERMES_PLUGIN_MANIFEST_INSTALL_NAME),
        HERMES_PLUGIN_MANIFEST_ASSET,
    )?;
    fs::write(
        plugin_dir.join(HERMES_PLUGIN_INIT_INSTALL_NAME),
        HERMES_PLUGIN_INIT_ASSET,
    )?;

    let config_path = dir.join("config.yaml");
    let existing_config = if config_path.is_file() {
        fs::read_to_string(&config_path)?
    } else {
        String::new()
    };
    let new_config = ensure_hermes_plugin_enabled(&existing_config);
    if new_config != existing_config {
        write_config(&config_path, new_config)?;
    }

    Ok(HermesInstallPaths {
        plugin_dir,
        config_path,
    })
}

pub(crate) fn uninstall_pi() -> io::Result<PiUninstallResult> {
    let extension_path = pi_extension_dir()?.join(PI_EXTENSION_INSTALL_NAME);
    let removed_extension = remove_file_if_exists(&extension_path)?;

    Ok(PiUninstallResult {
        extension_path,
        removed_extension,
    })
}

pub(crate) fn uninstall_omp() -> io::Result<OmpUninstallResult> {
    let extension_path = omp_extension_dir()?.join(OMP_EXTENSION_INSTALL_NAME);
    let removed_extension = remove_file_if_exists(&extension_path)?;

    Ok(OmpUninstallResult {
        extension_path,
        removed_extension,
    })
}

pub(crate) fn uninstall_claude() -> io::Result<ClaudeUninstallResult> {
    let dir = claude_dir()?;
    check_config_targets(&dir, &["settings.json"])?;
    let hook_path = dir.join("hooks").join(CLAUDE_HOOK_INSTALL_NAME);
    let settings_path = dir.join("settings.json");
    let mut updated_settings = false;

    if settings_path.is_file() {
        let existing_settings = fs::read_to_string(&settings_path)?;
        let new_settings =
            uninstall_claude_settings(&existing_settings, &settings_path, &hook_path)?;
        updated_settings = new_settings != existing_settings;
        if updated_settings {
            write_config(&settings_path, new_settings)?;
        }
    }

    let removed_hook_file =
        remove_file_if_exists(&hook_path)? | remove_legacy_bash_hook_file(&hook_path)?;

    Ok(ClaudeUninstallResult {
        hook_path,
        settings_path,
        removed_hook_file,
        updated_settings,
    })
}

pub(crate) fn uninstall_codex() -> io::Result<CodexUninstallResult> {
    let codex_dir = codex_dir()?;
    check_config_targets(&codex_dir, &["hooks.json"])?;
    let hook_path = codex_dir.join(CODEX_HOOK_INSTALL_NAME);
    let hooks_path = codex_dir.join("hooks.json");
    let config_path = codex_dir.join("config.toml");
    let mut updated_hooks = false;

    if hooks_path.is_file() {
        let mut hooks_file = serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?)
            .map_err(|err| {
                io::Error::other(format!("failed to parse {}: {err}", hooks_path.display()))
            })?;

        if let Some(hooks) = hooks_object_if_present(
            &mut hooks_file,
            &hooks_path,
            "codex hooks file",
            "codex hooks file hooks",
        )? {
            updated_hooks |= remove_hook_commands(hooks, "SessionStart", &hook_path, Some("idle"))?;
            updated_hooks |=
                remove_hook_commands(hooks, "SessionStart", &hook_path, Some("session"))?;
            updated_hooks |=
                remove_hook_commands(hooks, "UserPromptSubmit", &hook_path, Some("working"))?;
            updated_hooks |=
                remove_hook_commands(hooks, "PreToolUse", &hook_path, Some("working"))?;
            updated_hooks |=
                remove_hook_commands(hooks, "PermissionRequest", &hook_path, Some("blocked"))?;
            updated_hooks |= remove_hook_commands(hooks, "Stop", &hook_path, Some("idle"))?;
        }

        if updated_hooks {
            write_config(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;
        }
    }

    let removed_hook_file =
        remove_file_if_exists(&hook_path)? | remove_legacy_bash_hook_file(&hook_path)?;

    Ok(CodexUninstallResult {
        hook_path,
        hooks_path,
        config_path,
        removed_hook_file,
        updated_hooks,
    })
}

pub(crate) fn uninstall_kimi() -> io::Result<KimiUninstallResult> {
    let kimi_dir = kimi_dir()?;
    check_config_targets(&kimi_dir, &["config.toml"])?;
    let hook_path = kimi_dir.join("hooks").join(KIMI_HOOK_INSTALL_NAME);
    let config_path = kimi_dir.join("config.toml");
    let mut updated_config = false;

    if config_path.is_file() {
        let existing_config = fs::read_to_string(&config_path)?;
        let new_config = remove_kimi_config_block(&existing_config);
        if new_config != existing_config {
            write_config(&config_path, new_config)?;
            updated_config = true;
        }
    }

    let removed_hook_file =
        remove_file_if_exists(&hook_path)? | remove_legacy_bash_hook_file(&hook_path)?;

    Ok(KimiUninstallResult {
        hook_path,
        config_path,
        removed_hook_file,
        updated_config,
    })
}

pub(crate) fn uninstall_copilot() -> io::Result<CopilotUninstallResult> {
    let copilot_dir = copilot_dir()?;
    check_config_targets(&copilot_dir, &["settings.json"])?;
    let hook_path = copilot_dir.join("hooks").join(COPILOT_HOOK_INSTALL_NAME);
    let settings_path = copilot_dir.join("settings.json");
    let mut updated_settings = false;

    if settings_path.is_file() {
        let mut settings = serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?)
            .map_err(|err| {
                io::Error::other(format!(
                    "failed to parse {}: {err}",
                    settings_path.display()
                ))
            })?;

        if let Some(hooks) = hooks_object_if_present(
            &mut settings,
            &settings_path,
            "copilot settings",
            "copilot settings hooks",
        )? {
            for event in COPILOT_HOOK_EVENTS {
                updated_settings |= remove_direct_hook_commands(hooks, event, &hook_path, None)?;
            }
            for event in COPILOT_REMOVED_LIFECYCLE_HOOK_EVENTS {
                updated_settings |= remove_direct_hook_commands(hooks, event, &hook_path, None)?;
            }
        }

        if updated_settings {
            write_config(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        }
    }

    let removed_hook_file =
        remove_file_if_exists(&hook_path)? | remove_legacy_bash_hook_file(&hook_path)?;

    Ok(CopilotUninstallResult {
        hook_path,
        settings_path,
        removed_hook_file,
        updated_settings,
    })
}

pub(crate) fn uninstall_devin() -> io::Result<DevinUninstallResult> {
    let devin_dir = devin_dir()?;
    check_config_targets(&devin_dir, &["config.json"])?;
    let hook_path = devin_dir.join(DEVIN_HOOK_INSTALL_NAME);
    let settings_path = devin_dir.join("config.json");
    let mut updated_settings = false;

    if settings_path.is_file() {
        let mut settings = serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?)
            .map_err(|err| {
                io::Error::other(format!(
                    "failed to parse {}: {err}",
                    settings_path.display()
                ))
            })?;

        if let Some(hooks) = hooks_object_if_present(
            &mut settings,
            &settings_path,
            "devin settings",
            "devin settings hooks",
        )? {
            for (event, action) in DEVIN_REMOVED_LIFECYCLE_HOOK_EVENTS {
                updated_settings |= remove_hook_commands(hooks, event, &hook_path, Some(action))?;
            }
            for (event, action) in DEVIN_HOOK_EVENTS {
                updated_settings |= remove_hook_commands(hooks, event, &hook_path, Some(action))?;
            }
        }

        if updated_settings {
            write_config(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        }
    }

    let removed_hook_file =
        remove_file_if_exists(&hook_path)? | remove_legacy_bash_hook_file(&hook_path)?;

    Ok(DevinUninstallResult {
        hook_path,
        settings_path,
        removed_hook_file,
        updated_settings,
    })
}

pub(crate) fn uninstall_droid() -> io::Result<DroidUninstallResult> {
    let droid_dir = droid_dir()?;
    check_config_targets(&droid_dir, &["settings.json", "hooks.json"])?;
    let hook_path = droid_dir.join("hooks").join(DROID_HOOK_INSTALL_NAME);
    let hooks_path = droid_dir.join("hooks.json");
    let settings_path = droid_dir.join("settings.json");
    let mut updated_hooks = false;
    let mut updated_settings = false;
    if hooks_path.is_file() {
        let mut hooks_file = serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?)
            .map_err(|err| {
                io::Error::other(format!("failed to parse {}: {err}", hooks_path.display()))
            })?;

        if let Some(hooks) = hooks_object_if_present(
            &mut hooks_file,
            &hooks_path,
            "droid hooks file",
            "droid hooks file hooks",
        )? {
            updated_hooks |= remove_hook_commands(hooks, "SessionStart", &hook_path, None)?;
            for (event, action) in DROID_REMOVED_LIFECYCLE_HOOK_EVENTS {
                updated_hooks |= remove_hook_commands(hooks, event, &hook_path, Some(action))?;
            }
            for (event, action) in DROID_HOOK_EVENTS {
                updated_hooks |= remove_hook_commands(hooks, event, &hook_path, Some(action))?;
            }
        }

        if updated_hooks {
            write_config(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;
        }
    }

    if settings_path.is_file() {
        let mut settings = serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?)
            .map_err(|err| {
                io::Error::other(format!(
                    "failed to parse {}: {err}",
                    settings_path.display()
                ))
            })?;
        if let Some(hooks) = hooks_object_if_present(
            &mut settings,
            &settings_path,
            "droid settings",
            "droid settings hooks",
        )? {
            updated_settings = remove_hook_commands(hooks, "SessionStart", &hook_path, None)?;
            for (event, action) in DROID_REMOVED_LIFECYCLE_HOOK_EVENTS {
                updated_settings |= remove_hook_commands(hooks, event, &hook_path, Some(action))?;
            }
            for (event, action) in DROID_HOOK_EVENTS {
                updated_settings |= remove_hook_commands(hooks, event, &hook_path, Some(action))?;
            }
        }

        if updated_settings {
            write_config(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        }
    }

    let removed_hook_file =
        remove_file_if_exists(&hook_path)? | remove_legacy_bash_hook_file(&hook_path)?;

    Ok(DroidUninstallResult {
        hook_path,
        hooks_path,
        settings_path,
        removed_hook_file,
        updated_hooks,
        updated_settings,
    })
}

pub(crate) fn uninstall_opencode() -> io::Result<OpenCodeUninstallResult> {
    let dir = opencode_dir()?;
    check_config_targets(&dir, &["tui.jsonc", "tui.json", "cli.json"])?;
    let plugin_path = dir.join("plugins").join(OPENCODE_PLUGIN_INSTALL_NAME);
    let tui_plugin_path = dir.join(OPENCODE_TUI_PLUGIN_INSTALL_NAME);
    let mut errors = Vec::new();
    remove_cli_plugin(&dir, super::OPENCODE_V2_TUI_PLUGIN_SPEC).unwrap_or_else(|err| {
        errors.push(err.to_string());
        false
    });
    let v2_dir = dir.join(super::OPENCODE_V2_TUI_PLUGIN_DIR);
    remove_dir_all_if_exists(&v2_dir).unwrap_or_else(|err| {
        errors.push(format!("failed to remove {}: {err}", v2_dir.display()));
        false
    });
    let updated_tui_configs =
        remove_tui_plugin(&dir, OPENCODE_TUI_PLUGIN_SPEC).unwrap_or_else(|err| {
            errors.push(err.to_string());
            Vec::new()
        });
    let removed_plugin = remove_file_if_exists(&plugin_path).unwrap_or_else(|err| {
        errors.push(format!("failed to remove {}: {err}", plugin_path.display()));
        false
    });
    let removed_tui_plugin = remove_file_if_exists(&tui_plugin_path).unwrap_or_else(|err| {
        errors.push(format!(
            "failed to remove {}: {err}",
            tui_plugin_path.display()
        ));
        false
    });
    if !errors.is_empty() {
        return Err(io::Error::other(errors.join("; ")));
    }

    Ok(OpenCodeUninstallResult {
        plugin_path,
        tui_plugin_path,
        removed_plugin,
        removed_tui_plugin,
        updated_tui_configs,
    })
}

pub(crate) fn uninstall_kilo() -> io::Result<KiloUninstallResult> {
    let plugin_path = kilo_dir()?.join("plugin").join(KILO_PLUGIN_INSTALL_NAME);
    let removed_plugin = remove_file_if_exists(&plugin_path)?;

    Ok(KiloUninstallResult {
        plugin_path,
        removed_plugin,
    })
}

pub(crate) fn uninstall_hermes() -> io::Result<HermesUninstallResult> {
    let dir = hermes_dir()?;
    check_config_targets(&dir, &["config.yaml"])?;
    let plugin_dir = hermes_plugin_dir()?;
    let config_path = dir.join("config.yaml");

    let removed_plugin_dir = remove_dir_all_if_exists(&plugin_dir)?;
    let mut updated_config = false;
    if config_path.is_file() {
        let existing_config = fs::read_to_string(&config_path)?;
        let new_config = remove_hermes_plugin_enabled(&existing_config);
        if new_config != existing_config {
            write_config(&config_path, new_config)?;
            updated_config = true;
        }
    }

    Ok(HermesUninstallResult {
        plugin_dir,
        config_path,
        removed_plugin_dir,
        updated_config,
    })
}

pub(crate) fn install_qodercli() -> io::Result<QodercliInstallPaths> {
    let dir = qodercli_dir()?;
    check_config_targets(&dir, &["settings.json"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "qodercli config directory not found at {}. install qodercli first",
            dir.display()
        )));
    }

    let hooks_dir = dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join(QODERCLI_HOOK_INSTALL_NAME);
    fs::write(&hook_path, QODERCLI_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    // Register the hook in ~/.qoder/settings.json. The schema mirrors claude
    // settings.json (per https://docs.qoder.com/zh/cli/hooks): a top-level
    // `hooks` object keyed by event name, each entry holding a matcher + a
    // list of `{type: "command", command, timeout?}` invocations. The hook
    // script reads the event payload from stdin via `hook_event_name`.
    let settings_path = dir.join("settings.json");
    let mut settings = if settings_path.is_file() {
        serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?).map_err(|err| {
            io::Error::other(format!(
                "failed to parse {}: {err}",
                settings_path.display()
            ))
        })?
    } else {
        json!({})
    };

    let hooks = ensure_hooks_object(
        &mut settings,
        &settings_path,
        "qodercli settings",
        "qodercli settings hooks",
    )?;
    for (event, action) in QODERCLI_REMOVED_LIFECYCLE_HOOK_EVENTS {
        remove_hook_commands(hooks, event, &hook_path, Some(action))?;
    }
    for (event, action) in QODERCLI_HOOK_EVENTS {
        remove_hook_commands(hooks, event, &hook_path, Some(action))?;
    }
    for (event, action) in QODERCLI_HOOK_EVENTS {
        ensure_command_hook(
            hooks,
            event,
            hook_command(&hook_path, Some(action)),
            10,
            Some("*"),
        )?;
    }
    remove_legacy_bash_hook_file(&hook_path)?;

    write_config(&settings_path, serde_json::to_string_pretty(&settings)?)?;

    Ok(QodercliInstallPaths {
        hook_path,
        settings_path,
    })
}

pub(crate) fn install_qwen() -> io::Result<QwenInstallPaths> {
    let dir = qwen_dir()?;
    check_config_targets(&dir, &["settings.json"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "qwen code config directory not found at {}. install qwen code first",
            dir.display()
        )));
    }

    let hooks_dir = dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join(QWEN_HOOK_INSTALL_NAME);
    fs::write(&hook_path, QWEN_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    let settings_path = dir.join("settings.json");
    let mut settings = if settings_path.is_file() {
        serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?).map_err(|err| {
            io::Error::other(format!(
                "failed to parse {}: {err}",
                settings_path.display()
            ))
        })?
    } else {
        json!({})
    };

    let hooks = ensure_hooks_object(
        &mut settings,
        &settings_path,
        "qwen settings",
        "qwen settings hooks",
    )?;
    for (event, action) in QWEN_HOOK_EVENTS {
        remove_hook_commands(hooks, event, &hook_path, Some(action))?;
        ensure_command_hook(
            hooks,
            event,
            hook_command(&hook_path, Some(action)),
            10_000,
            Some("*"),
        )?;
    }

    write_config(&settings_path, serde_json::to_string_pretty(&settings)?)?;

    Ok(QwenInstallPaths {
        hook_path,
        settings_path,
    })
}

fn letta_install_artifact_path(path: &Path, role: &str) -> io::Result<PathBuf> {
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::other(format!("invalid install path: {}", path.display())))?;
    let mut artifact_name = file_name.to_os_string();
    artifact_name.push(format!(".herdr-install-{}-{role}", std::process::id()));
    Ok(path.with_file_name(artifact_name))
}

pub(super) fn prepare_letta_install_file(
    target: &Path,
    contents: &[u8],
    executable: bool,
    preserve_permissions: bool,
) -> io::Result<(PathBuf, PathBuf)> {
    if target.try_exists()? && !target.is_file() {
        return Err(io::Error::other(format!(
            "install target is not a file: {}",
            target.display()
        )));
    }

    let staged = letta_install_artifact_path(target, "staged")?;
    let backup = letta_install_artifact_path(target, "backup")?;
    if staged.try_exists()? || backup.try_exists()? {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("stale install artifact exists for {}", target.display()),
        ));
    }

    let prepare_result = (|| {
        fs::write(&staged, contents)?;
        if preserve_permissions && target.is_file() {
            fs::set_permissions(&staged, fs::metadata(target)?.permissions())?;
        }
        if executable {
            make_executable(&staged)?;
        }
        Ok(())
    })();
    if let Err(err) = prepare_result {
        let _ = remove_file_if_exists(&staged);
        return Err(err);
    }

    Ok((staged, backup))
}

fn combine_letta_install_errors(primary: io::Error, rollback: io::Result<()>) -> io::Error {
    match rollback {
        Ok(()) => primary,
        Err(rollback_err) => io::Error::new(
            primary.kind(),
            format!("{primary}; rollback failed: {rollback_err}"),
        ),
    }
}

pub(super) fn publish_letta_install_file(
    target: &Path,
    staged: &Path,
    backup: &Path,
) -> io::Result<bool> {
    let had_original = target.try_exists()?;
    if had_original {
        fs::rename(target, backup)?;
    }

    if let Err(err) = fs::rename(staged, target) {
        let rollback = if had_original {
            fs::rename(backup, target)
        } else {
            Ok(())
        };
        return Err(combine_letta_install_errors(err, rollback));
    }

    Ok(had_original)
}

pub(super) fn rollback_letta_install_file(
    target: &Path,
    backup: &Path,
    had_original: bool,
) -> io::Result<()> {
    remove_file_if_exists(target)?;
    if had_original {
        fs::rename(backup, target)?;
    }
    Ok(())
}

fn cleanup_letta_install_artifact(path: &Path) {
    if let Err(err) = remove_file_if_exists(path) {
        tracing::warn!(path = %path.display(), %err, "failed to remove Letta install artifact");
    }
}

fn ensure_letta_session_hook(hooks: &mut Map<String, Value>, command: String) -> io::Result<()> {
    let entries = hooks
        .entry("SessionStart".to_string())
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| io::Error::other("hook entries for SessionStart must be an array"))?;

    entries.push(json!({
        "hooks": [{
            "type": "command",
            "command": command,
            "timeout": LETTA_HOOK_TIMEOUT_MS,
            "quiet": true,
        }],
    }));
    Ok(())
}

pub(crate) fn install_letta() -> io::Result<LettaInstallPaths> {
    let dir = letta_dir()?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "letta code config directory not found at {}. install letta code first",
            dir.display()
        )));
    }

    let hooks_dir = dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join(LETTA_HOOK_INSTALL_NAME);

    let settings_path = dir.join("settings.json");
    let mut settings = if settings_path.is_file() {
        serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?).map_err(|err| {
            io::Error::other(format!(
                "failed to parse {}: {err}",
                settings_path.display()
            ))
        })?
    } else {
        json!({})
    };

    let hooks = ensure_hooks_object(
        &mut settings,
        &settings_path,
        "letta settings",
        "letta settings hooks",
    )?;
    remove_hook_commands(hooks, "SessionStart", &hook_path, Some("session"))?;
    ensure_letta_session_hook(hooks, hook_command(&hook_path, Some("session")))?;

    let settings_contents = serde_json::to_string_pretty(&settings)?;
    let (hook_staged, hook_backup) =
        prepare_letta_install_file(&hook_path, LETTA_HOOK_ASSET.as_bytes(), true, false)?;
    let (settings_staged, settings_backup) =
        match prepare_letta_install_file(&settings_path, settings_contents.as_bytes(), false, true)
        {
            Ok(paths) => paths,
            Err(err) => {
                cleanup_letta_install_artifact(&hook_staged);
                return Err(err);
            }
        };

    let hook_had_original = match publish_letta_install_file(&hook_path, &hook_staged, &hook_backup)
    {
        Ok(had_original) => had_original,
        Err(err) => {
            cleanup_letta_install_artifact(&hook_staged);
            cleanup_letta_install_artifact(&settings_staged);
            return Err(err);
        }
    };

    let settings_had_original =
        match publish_letta_install_file(&settings_path, &settings_staged, &settings_backup) {
            Ok(had_original) => had_original,
            Err(err) => {
                let err = combine_letta_install_errors(
                    err,
                    rollback_letta_install_file(&hook_path, &hook_backup, hook_had_original),
                );
                cleanup_letta_install_artifact(&settings_staged);
                return Err(err);
            }
        };

    if hook_had_original {
        cleanup_letta_install_artifact(&hook_backup);
    }
    if settings_had_original {
        cleanup_letta_install_artifact(&settings_backup);
    }

    Ok(LettaInstallPaths {
        hook_path,
        settings_path,
    })
}

pub(crate) fn install_cursor() -> io::Result<CursorInstallPaths> {
    let dir = cursor_dir()?;
    check_config_targets(&dir, &["hooks.json"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "cursor config directory not found at {}. install cursor agent cli first",
            dir.display()
        )));
    }

    let hook_path = dir.join(CURSOR_HOOK_INSTALL_NAME);
    fs::write(&hook_path, CURSOR_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    let hooks_path = dir.join("hooks.json");
    let mut hooks_file = if hooks_path.is_file() {
        serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?).map_err(|err| {
            io::Error::other(format!("failed to parse {}: {err}", hooks_path.display()))
        })?
    } else {
        json!({ "version": 1 })
    };

    if hooks_file.get("version").is_none() {
        hooks_file
            .as_object_mut()
            .ok_or_else(|| {
                io::Error::other(format!(
                    "cursor hooks file at {} must be a JSON object",
                    hooks_path.display()
                ))
            })?
            .insert("version".to_string(), json!(1));
    }

    let hooks = ensure_hooks_object(
        &mut hooks_file,
        &hooks_path,
        "cursor hooks file",
        "cursor hooks file hooks",
    )?;
    let session_command = hook_command(&hook_path, Some("session"));
    remove_simple_command_hook(hooks, "beforeSubmitPrompt", &session_command)?;
    remove_simple_command_hook(hooks, "beforeShellExecution", &session_command)?;
    remove_simple_command_hook(hooks, "beforeMCPExecution", &session_command)?;
    remove_simple_command_hook(hooks, "stop", &session_command)?;
    remove_simple_command_hook(hooks, "sessionEnd", &session_command)?;
    ensure_simple_command_hook(hooks, "sessionStart", session_command)?;

    write_config(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;

    Ok(CursorInstallPaths {
        hook_path,
        hooks_path,
    })
}

pub(crate) fn uninstall_qodercli() -> io::Result<QodercliUninstallResult> {
    let dir = qodercli_dir()?;
    check_config_targets(&dir, &["settings.json"])?;
    let hook_path = dir.join("hooks").join(QODERCLI_HOOK_INSTALL_NAME);
    let settings_path = dir.join("settings.json");
    let mut updated_settings = false;

    if settings_path.is_file() {
        let mut settings = serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?)
            .map_err(|err| {
                io::Error::other(format!(
                    "failed to parse {}: {err}",
                    settings_path.display()
                ))
            })?;

        if let Some(hooks) = hooks_object_if_present(
            &mut settings,
            &settings_path,
            "qodercli settings",
            "qodercli settings hooks",
        )? {
            for (event, action) in QODERCLI_REMOVED_LIFECYCLE_HOOK_EVENTS {
                updated_settings |= remove_hook_commands(hooks, event, &hook_path, Some(action))?;
            }
            for (event, action) in QODERCLI_HOOK_EVENTS {
                updated_settings |= remove_hook_commands(hooks, event, &hook_path, Some(action))?;
            }
        }

        if updated_settings {
            write_config(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        }
    }

    let removed_hook_file =
        remove_file_if_exists(&hook_path)? | remove_legacy_bash_hook_file(&hook_path)?;

    Ok(QodercliUninstallResult {
        hook_path,
        settings_path,
        removed_hook_file,
        updated_settings,
    })
}

pub(crate) fn uninstall_qwen() -> io::Result<QwenUninstallResult> {
    let dir = qwen_dir()?;
    check_config_targets(&dir, &["settings.json"])?;
    let hook_path = dir.join("hooks").join(QWEN_HOOK_INSTALL_NAME);
    let settings_path = dir.join("settings.json");
    let mut updated_settings = false;

    if settings_path.is_file() {
        let mut settings = serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?)
            .map_err(|err| {
                io::Error::other(format!(
                    "failed to parse {}: {err}",
                    settings_path.display()
                ))
            })?;

        if let Some(hooks) = hooks_object_if_present(
            &mut settings,
            &settings_path,
            "qwen settings",
            "qwen settings hooks",
        )? {
            for (event, action) in QWEN_HOOK_EVENTS {
                updated_settings |= remove_hook_commands(hooks, event, &hook_path, Some(action))?;
            }
        }

        if updated_settings {
            write_config(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        }
    }

    let removed_hook_file = remove_file_if_exists(&hook_path)?;

    Ok(QwenUninstallResult {
        hook_path,
        settings_path,
        removed_hook_file,
        updated_settings,
    })
}

pub(crate) fn uninstall_letta() -> io::Result<LettaUninstallResult> {
    let dir = letta_dir()?;
    let hook_path = dir.join("hooks").join(LETTA_HOOK_INSTALL_NAME);
    let settings_path = dir.join("settings.json");
    let mut updated_settings = false;

    if settings_path.is_file() {
        let mut settings = serde_json::from_str::<Value>(&fs::read_to_string(&settings_path)?)
            .map_err(|err| {
                io::Error::other(format!(
                    "failed to parse {}: {err}",
                    settings_path.display()
                ))
            })?;

        if let Some(hooks) = hooks_object_if_present(
            &mut settings,
            &settings_path,
            "letta settings",
            "letta settings hooks",
        )? {
            updated_settings |=
                remove_hook_commands(hooks, "SessionStart", &hook_path, Some("session"))?;
        }

        if updated_settings {
            fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;
        }
    }

    let removed_hook_file = remove_file_if_exists(&hook_path)?;

    Ok(LettaUninstallResult {
        hook_path,
        settings_path,
        removed_hook_file,
        updated_settings,
    })
}

pub(crate) fn uninstall_cursor() -> io::Result<CursorUninstallResult> {
    let cursor_home = cursor_dir()?;
    check_config_targets(&cursor_home, &["hooks.json"])?;
    let hook_path = cursor_home.join(CURSOR_HOOK_INSTALL_NAME);
    let hooks_path = cursor_home.join("hooks.json");
    let mut updated_hooks = false;

    if hooks_path.is_file() {
        let mut hooks_file = serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?)
            .map_err(|err| {
                io::Error::other(format!("failed to parse {}: {err}", hooks_path.display()))
            })?;

        if let Some(hooks) = hooks_object_if_present(
            &mut hooks_file,
            &hooks_path,
            "cursor hooks file",
            "cursor hooks file hooks",
        )? {
            let session_command = hook_command(&hook_path, Some("session"));
            updated_hooks |= remove_simple_command_hook(hooks, "sessionStart", &session_command)?;
            updated_hooks |=
                remove_simple_command_hook(hooks, "beforeSubmitPrompt", &session_command)?;
            updated_hooks |=
                remove_simple_command_hook(hooks, "beforeShellExecution", &session_command)?;
            updated_hooks |=
                remove_simple_command_hook(hooks, "beforeMCPExecution", &session_command)?;
            updated_hooks |= remove_simple_command_hook(hooks, "stop", &session_command)?;
            updated_hooks |= remove_simple_command_hook(hooks, "sessionEnd", &session_command)?;
        }

        if updated_hooks {
            write_config(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;
        }
    }

    let removed_hook_file = remove_file_if_exists(&hook_path)?;

    Ok(CursorUninstallResult {
        hook_path,
        hooks_path,
        removed_hook_file,
        updated_hooks,
    })
}

pub(crate) fn mastracode_hook_command(hook_path: &Path, action: &str) -> String {
    #[cfg(windows)]
    {
        powershell_encoded_hook_command(hook_path, action)
    }
    #[cfg(not(windows))]
    {
        hook_command(hook_path, Some(action))
    }
}

pub(crate) fn install_mastracode() -> io::Result<MastracodeInstallPaths> {
    let mastracode_home = mastracode_dir()?;
    check_config_targets(&mastracode_home, &["hooks.json"])?;
    let hook_dir = mastracode_home.join("hooks");
    fs::create_dir_all(&hook_dir)?;

    let hook_path = hook_dir.join(MASTRACODE_HOOK_INSTALL_NAME);
    fs::write(&hook_path, MASTRACODE_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    let hooks_path = mastracode_home.join("hooks.json");
    let mut hooks_file = if hooks_path.is_file() {
        serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?).map_err(|err| {
            io::Error::other(format!("failed to parse {}: {err}", hooks_path.display()))
        })?
    } else {
        json!({})
    };

    let hooks = hooks_file.as_object_mut().ok_or_else(|| {
        io::Error::other(format!(
            "mastracode hooks file at {} must be a JSON object",
            hooks_path.display()
        ))
    })?;

    for (event, action) in MASTRACODE_REMOVED_HOOK_EVENTS {
        remove_flat_command_hook(hooks, event, &hook_command(&hook_path, Some(action)))?;
        remove_flat_command_hook(hooks, event, &mastracode_hook_command(&hook_path, action))?;
    }
    for (event, action) in MASTRACODE_HOOK_EVENTS {
        remove_flat_command_hook(hooks, event, &hook_command(&hook_path, Some(action)))?;
        ensure_flat_command_hook(
            hooks,
            event,
            mastracode_hook_command(&hook_path, action),
            MASTRACODE_HOOK_TIMEOUT_MS,
        )?;
    }

    write_config(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;

    Ok(MastracodeInstallPaths {
        hook_path,
        hooks_path,
    })
}

pub(crate) fn uninstall_mastracode() -> io::Result<MastracodeUninstallResult> {
    let mastracode_home = mastracode_dir()?;
    check_config_targets(&mastracode_home, &["hooks.json"])?;
    let hook_path = mastracode_home
        .join("hooks")
        .join(MASTRACODE_HOOK_INSTALL_NAME);
    let hooks_path = mastracode_home.join("hooks.json");
    let mut updated_hooks = false;

    if hooks_path.is_file() {
        let mut hooks_file = serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?)
            .map_err(|err| {
                io::Error::other(format!("failed to parse {}: {err}", hooks_path.display()))
            })?;
        let hooks = hooks_file.as_object_mut().ok_or_else(|| {
            io::Error::other(format!(
                "mastracode hooks file at {} must be a JSON object",
                hooks_path.display()
            ))
        })?;

        for (event, action) in MASTRACODE_HOOK_EVENTS
            .into_iter()
            .chain(MASTRACODE_REMOVED_HOOK_EVENTS)
        {
            updated_hooks |=
                remove_flat_command_hook(hooks, event, &hook_command(&hook_path, Some(action)))?;
            updated_hooks |= remove_flat_command_hook(
                hooks,
                event,
                &mastracode_hook_command(&hook_path, action),
            )?;
        }

        if updated_hooks {
            write_config(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;
        }
    }

    let removed_hook_file = remove_file_if_exists(&hook_path)?;

    Ok(MastracodeUninstallResult {
        hook_path,
        hooks_path,
        removed_hook_file,
        updated_hooks,
    })
}

pub(crate) fn install_antigravity_cli() -> io::Result<AntigravityCliInstallPaths> {
    let dir = antigravity_cli_dir()?;
    check_config_targets(&dir, &["hooks.json"])?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "antigravity cli config directory not found at {}. install antigravity cli first",
            dir.display()
        )));
    }

    let hooks_dir = dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join(ANTIGRAVITY_CLI_HOOK_INSTALL_NAME);
    fs::write(&hook_path, ANTIGRAVITY_CLI_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    let hooks_path = dir.join("hooks.json");
    let mut hooks_file = if hooks_path.is_file() {
        serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?).map_err(|err| {
            io::Error::other(format!("failed to parse {}: {err}", hooks_path.display()))
        })?
    } else {
        json!({})
    };

    let hooks = hooks_file.as_object_mut().ok_or_else(|| {
        io::Error::other(format!(
            "antigravity cli hooks file at {} must be a JSON object",
            hooks_path.display()
        ))
    })?;

    // The Herdr block is Herdr-owned, so rewrite it wholesale and leave every
    // other named hook untouched.
    hooks.insert(
        ANTIGRAVITY_CLI_HOOK_BLOCK_NAME.to_string(),
        antigravity_cli_hook_block(&hook_path),
    );

    write_config(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;

    Ok(AntigravityCliInstallPaths {
        hook_path,
        hooks_path,
    })
}

pub(crate) fn antigravity_cli_hook_command(hook_path: &Path, action: &str) -> String {
    #[cfg(windows)]
    {
        powershell_encoded_hook_command(hook_path, action)
    }
    #[cfg(not(windows))]
    {
        hook_command(hook_path, Some(action))
    }
}

/// Builds the Herdr-owned `hooks.json` block for Antigravity CLI.
///
/// Every event Herdr registers takes a flat handler list; the `matcher`/`hooks`
/// group is only valid for the tool events, which Herdr does not use.
fn antigravity_cli_hook_block(hook_path: &Path) -> Value {
    let mut block = Map::new();
    for (event, action) in ANTIGRAVITY_CLI_HOOK_EVENTS {
        let handler = json!({
            "type": "command",
            "command": antigravity_cli_hook_command(hook_path, action),
            "timeout": ANTIGRAVITY_CLI_HOOK_TIMEOUT_SEC,
        });
        block.insert(event.to_string(), json!([handler]));
    }
    Value::Object(block)
}

pub(crate) fn uninstall_antigravity_cli() -> io::Result<AntigravityCliUninstallResult> {
    let dir = antigravity_cli_dir()?;
    check_config_targets(&dir, &["hooks.json"])?;
    let hook_path = dir.join("hooks").join(ANTIGRAVITY_CLI_HOOK_INSTALL_NAME);
    let hooks_path = dir.join("hooks.json");
    let mut updated_hooks = false;

    if hooks_path.is_file() {
        let mut hooks_file = serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?)
            .map_err(|err| {
                io::Error::other(format!("failed to parse {}: {err}", hooks_path.display()))
            })?;

        let hooks = hooks_file.as_object_mut().ok_or_else(|| {
            io::Error::other(format!(
                "antigravity cli hooks file at {} must be a JSON object",
                hooks_path.display()
            ))
        })?;

        updated_hooks = hooks.remove(ANTIGRAVITY_CLI_HOOK_BLOCK_NAME).is_some();

        if updated_hooks {
            write_config(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;
        }
    }

    let removed_hook_file = remove_file_if_exists(&hook_path)?;

    Ok(AntigravityCliUninstallResult {
        hook_path,
        hooks_path,
        removed_hook_file,
        updated_hooks,
    })
}

/// The complete Herdr-owned Grok hook config. Installation and status share
/// this value so any config drift is reported as outdated.
fn grok_hook_command(hook_path: &Path) -> String {
    #[cfg(windows)]
    {
        hook_command(hook_path, Some("session"))
    }
    #[cfg(not(windows))]
    {
        format!(
            "sh {} session",
            shell_single_quote(&hook_path.display().to_string())
        )
    }
}

pub(crate) fn grok_hook_config(hook_path: &Path) -> Value {
    let session_command = grok_hook_command(hook_path);
    json!({
        "hooks": {
            "SessionStart": [
                {
                    "hooks": [
                        {
                            "type": "command",
                            "command": session_command,
                            "timeout": 10,
                        }
                    ]
                }
            ]
        }
    })
}

pub(crate) fn install_grok() -> io::Result<GrokInstallPaths> {
    let dir = grok_dir()?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "grok config directory not found at {}. install grok cli first",
            dir.display()
        )));
    }

    // Grok merges every `~/.grok/hooks/*.json`, so herdr owns a dedicated
    // config file and never edits the user's other hooks. The hook script and
    // its config live side by side under `hooks/`.
    let hooks_dir = dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join(GROK_HOOK_INSTALL_NAME);
    fs::write(&hook_path, GROK_HOOK_ASSET)?;
    make_executable(&hook_path)?;

    let config_path = hooks_dir.join(GROK_HOOK_CONFIG_INSTALL_NAME);
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&grok_hook_config(&hook_path))?,
    )?;

    Ok(GrokInstallPaths {
        hook_path,
        config_path,
    })
}

pub(crate) fn uninstall_grok() -> io::Result<GrokUninstallResult> {
    let hooks_dir = grok_dir()?.join("hooks");
    let hook_path = hooks_dir.join(GROK_HOOK_INSTALL_NAME);
    let config_path = hooks_dir.join(GROK_HOOK_CONFIG_INSTALL_NAME);

    // herdr owns both files outright, so removal is a straight delete.
    let removed_config_file = remove_file_if_exists(&config_path)?;
    let removed_hook_file = remove_file_if_exists(&hook_path)?;

    Ok(GrokUninstallResult {
        hook_path,
        config_path,
        removed_hook_file,
        removed_config_file,
    })
}

pub(crate) fn install_trae() -> io::Result<TraeInstallPaths> {
    let dir = trae_dir()?;
    if !dir.is_dir() {
        return Err(io::Error::other(format!(
            "trae config directory not found at {}. install trae (traex) first",
            dir.display()
        )));
    }

    remove_trae_legacy_hooks(&dir)?;

    let plugin_dir = dir.join(TRAE_PLUGIN_DIR_NAME);
    let manifest_dir = plugin_dir.join(".codex-plugin");
    fs::create_dir_all(&manifest_dir)?;
    let hook_path = plugin_dir.join(TRAE_HOOK_INSTALL_NAME);
    fs::write(&hook_path, TRAE_HOOK_ASSET)?;
    make_executable(&hook_path)?;
    fs::write(
        plugin_dir.join("hooks.json"),
        serde_json::to_string_pretty(&trae_plugin_hooks())?,
    )?;
    fs::write(
        manifest_dir.join("plugin.json"),
        serde_json::to_string_pretty(&json!({
            "name": TRAE_PLUGIN_NAME,
            "version": format!("{}.0.0", super::TRAE_INTEGRATION_VERSION),
            "description": "Report Trae agent state to herdr",
            "hooks": "./hooks.json",
            "interface": {
                "displayName": "herdr",
                "shortDescription": "Report Trae agent state to herdr",
                "category": "Developer Tools"
            }
        }))?,
    )?;

    // Reinstalling the same local plugin refreshes Trae's cached copy.
    let plugin_dir_arg = plugin_dir.to_string_lossy().into_owned();
    let (cli, output) = run_trae_cli(&[
        "plugin",
        "install",
        "--type",
        "local",
        &plugin_dir_arg,
        "--name",
        TRAE_PLUGIN_NAME,
        "--yes",
    ])?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "`{cli} plugin install` failed: {}",
            command_output_summary(&output)
        )));
    }

    // Trae skips plugin hooks without a trusted hash, so record one per hook.
    let config_path = dir.join("traecli.toml");
    let existing_config = if config_path.is_file() {
        fs::read_to_string(&config_path)?
    } else {
        String::new()
    };
    let new_config = with_trae_trust_entries(&build_codex_config_with_hooks(&existing_config));
    toml::from_str::<toml::Value>(&new_config).map_err(|err| {
        io::Error::other(format!(
            "refusing to write {}: result would not be valid TOML: {err}",
            config_path.display()
        ))
    })?;
    if new_config != existing_config {
        fs::write(&config_path, new_config)?;
    }

    Ok(TraeInstallPaths {
        plugin_dir,
        hook_path,
        config_path,
        cli,
    })
}

pub(crate) fn uninstall_trae() -> io::Result<TraeUninstallResult> {
    let dir = trae_dir()?;
    let plugin_dir = dir.join(TRAE_PLUGIN_DIR_NAME);
    let config_path = dir.join("traecli.toml");
    let read_config = || -> io::Result<String> {
        if config_path.is_file() {
            fs::read_to_string(&config_path)
        } else {
            Ok(String::new())
        }
    };

    let mut unregistered_plugin = false;
    let mut unregister_warning = None;
    let plugin_table = format!("[plugins.\"{}\"]", trae_plugin_id());
    if read_config()?
        .lines()
        .any(|line| line.trim() == plugin_table)
    {
        match run_trae_cli(&["plugin", "uninstall", &trae_plugin_id()]) {
            Ok((_, output)) if output.status.success() => unregistered_plugin = true,
            Ok((cli, output)) => {
                unregister_warning = Some(format!(
                    "`{cli} plugin uninstall {}` failed: {}",
                    trae_plugin_id(),
                    command_output_summary(&output)
                ));
            }
            Err(err) => unregister_warning = Some(err.to_string()),
        }
    }

    let existing_config = read_config()?;
    let stripped_config = without_trae_trust_entries(&existing_config);
    let removed_trust_entries = stripped_config != existing_config;
    if removed_trust_entries {
        fs::write(&config_path, stripped_config)?;
    }

    let removed_plugin_dir = remove_dir_all_if_exists(&plugin_dir)?;
    let removed_legacy_hooks = remove_trae_legacy_hooks(&dir)?;

    Ok(TraeUninstallResult {
        plugin_dir,
        config_path,
        unregistered_plugin,
        removed_trust_entries,
        removed_plugin_dir,
        removed_legacy_hooks,
        unregister_warning,
    })
}

fn trae_plugin_id() -> String {
    format!("{TRAE_PLUGIN_NAME}@{TRAE_PLUGIN_MARKETPLACE}")
}

/// The command Trae runs for one hook. `__PLUGIN_DIR__` is expanded by Trae to
/// the installed plugin root.
pub(super) fn trae_plugin_hook_command(action: &str) -> String {
    format!("sh \"__PLUGIN_DIR__/{TRAE_HOOK_INSTALL_NAME}\" {action}")
}

pub(super) fn trae_plugin_hooks() -> Value {
    let mut hooks = Map::new();
    for (event, action) in TRAE_HOOK_EVENTS {
        hooks.insert(
            event.to_string(),
            json!([{
                "hooks": [{
                    "type": "command",
                    "command": trae_plugin_hook_command(action),
                    "timeout": TRAE_HOOK_TIMEOUT_SEC
                }]
            }]),
        );
    }
    json!({ "hooks": hooks })
}

/// `PostToolUseFailure` -> `post_tool_use_failure`, the event name Trae uses in
/// hook trust keys.
pub(super) fn trae_event_snake_case(event: &str) -> String {
    let mut snake = String::with_capacity(event.len() + 4);
    for (index, ch) in event.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                snake.push('_');
            }
            snake.push(ch.to_ascii_lowercase());
        } else {
            snake.push(ch);
        }
    }
    snake
}

pub(super) fn trae_trust_key(event: &str) -> String {
    format!(
        "{}:hooks.json:{}:0:0",
        trae_plugin_id(),
        trae_event_snake_case(event)
    )
}

/// Trae trusts a hook by the SHA-256 of a canonical (sorted-key, compact) JSON
/// identity of its event and handler. Verified against hashes Trae itself
/// recorded for marketplace plugins.
pub(super) fn trae_trusted_hash(event: &str, action: &str) -> String {
    use sha2::{Digest, Sha256};

    // Keys are inserted in sorted order so the encoding is canonical whether or
    // not serde_json preserves insertion order.
    let mut handler = Map::new();
    handler.insert("async".to_string(), json!(false));
    handler.insert(
        "command".to_string(),
        json!(trae_plugin_hook_command(action)),
    );
    handler.insert("timeout".to_string(), json!(TRAE_HOOK_TIMEOUT_SEC));
    handler.insert("type".to_string(), json!("command"));
    let mut identity = Map::new();
    identity.insert(
        "event_name".to_string(),
        json!(trae_event_snake_case(event)),
    );
    identity.insert("hooks".to_string(), json!([Value::Object(handler)]));

    let digest = Sha256::digest(Value::Object(identity).to_string().as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{hex}")
}

fn is_trae_trust_header(line: &str) -> bool {
    line.trim()
        .starts_with(&format!("[hooks.state.\"{}:", trae_plugin_id()))
}

/// Removes every herdr hook trust table, leaving the rest of the file as-is.
pub(super) fn without_trae_trust_entries(config: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut skipping = false;
    for line in config.lines() {
        if is_trae_trust_header(line) {
            skipping = true;
            continue;
        }
        if skipping && line.trim_start().starts_with('[') {
            skipping = false;
        }
        if !skipping {
            kept.push(line);
        }
    }
    let body = kept.join("\n");
    let body = body.trim_end();
    if body.is_empty() {
        String::new()
    } else {
        format!("{body}\n")
    }
}

pub(super) fn with_trae_trust_entries(config: &str) -> String {
    let mut out = without_trae_trust_entries(config);
    for (event, action) in TRAE_HOOK_EVENTS {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!(
            "[hooks.state.\"{}\"]\ntrusted_hash = \"{}\"\n",
            trae_trust_key(event),
            trae_trusted_hash(event, action)
        ));
    }
    out
}

/// Runs the Trae CLI (`HERDR_TRAE_CLI`, else the first of traex/traecli/trae-cli
/// found on PATH or in ~/.local/bin). Returns the program used and its output.
fn run_trae_cli(args: &[&str]) -> io::Result<(String, std::process::Output)> {
    let mut candidates: Vec<String> = Vec::new();
    if let Some(cli) = std::env::var_os(TRAE_CLI_ENV_VAR).filter(|value| !value.is_empty()) {
        candidates.push(cli.to_string_lossy().into_owned());
    } else {
        candidates.extend(TRAE_CLI_NAMES.iter().map(|name| name.to_string()));
        if let Ok(home) = super::env::home_dir() {
            candidates.extend(TRAE_CLI_NAMES.iter().map(|name| {
                home.join(".local")
                    .join("bin")
                    .join(name)
                    .to_string_lossy()
                    .into_owned()
            }));
        }
    }

    for candidate in &candidates {
        match crate::noninteractive_process::command(candidate)
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
        {
            Ok(output) => return Ok((candidate.clone(), output)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(io::Error::other(format!(
                    "failed to run {candidate}: {err}"
                )))
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "trae cli not found (looked for {}); install trae first or set {TRAE_CLI_ENV_VAR}",
            TRAE_CLI_NAMES.join(", ")
        ),
    ))
}

fn command_output_summary(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let text = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    let tail: Vec<&str> = text.lines().rev().take(5).collect();
    let tail: Vec<&str> = tail.into_iter().rev().collect();
    format!("{} ({})", tail.join(" | "), output.status)
}

/// Removes what integration versions 1-2 installed: herdr entries in the
/// legacy ~/.trae/hooks.json and the ~/.trae/herdr-agent-state.sh script.
fn remove_trae_legacy_hooks(dir: &Path) -> io::Result<bool> {
    let legacy_hook_path = dir.join(TRAE_HOOK_INSTALL_NAME);
    let hooks_path = dir.join("hooks.json");
    let mut removed = false;

    if hooks_path.is_file() {
        let mut hooks_file = serde_json::from_str::<Value>(&fs::read_to_string(&hooks_path)?)
            .map_err(|err| {
                io::Error::other(format!("failed to parse {}: {err}", hooks_path.display()))
            })?;
        let mut updated = false;
        if let Some(hooks) = hooks_object_if_present(
            &mut hooks_file,
            &hooks_path,
            "trae hooks file",
            "trae hooks file hooks",
        )? {
            for (event, action) in TRAE_LEGACY_HOOK_EVENTS {
                updated |= remove_hook_commands(hooks, event, &legacy_hook_path, Some(action))?;
            }
        }
        if updated {
            fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_file)?)?;
            removed = true;
        }
    }

    removed |= remove_file_if_exists(&legacy_hook_path)?;
    removed |= remove_legacy_bash_hook_file(&legacy_hook_path)?;
    Ok(removed)
}
