use super::*;

// Codex is only a registry key here; behavior tests supply synthetic rules.
fn remote_manifest(version: &str, state: &str, contains: &str) -> String {
    format!(
        r#"
id = "codex"
version = "{version}"
min_engine_version = 1
updated_at = "2026-06-10T12:00:00Z"

[[rules]]
id = "test"
state = "{state}"
contains = ["{contains}"]
"#
    )
}

fn local_manifest(state: &str, contains: &str) -> String {
    format!(
        r#"
id = "codex"

[[rules]]
id = "test"
state = "{state}"
contains = ["{contains}"]
"#
    )
}

fn rules_manifest(rules: &str) -> String {
    format!(
        r#"
id = "codex"

{rules}
"#
    )
}

fn with_manifest_dirs<T>(name: &str, f: impl FnOnce() -> T) -> T {
    let _guard = crate::config::test_config_env_lock().lock().unwrap();
    let old_config = std::env::var_os("XDG_CONFIG_HOME");
    let old_state = std::env::var_os("XDG_STATE_HOME");
    let base = std::env::temp_dir().join(format!(
        "herdr-manifest-loader-{name}-{}",
        std::process::id()
    ));
    let config_dir = base.join("config");
    let state_dir = base.join("state");
    let _ = std::fs::remove_dir_all(&base);
    std::env::set_var("XDG_CONFIG_HOME", &config_dir);
    std::env::set_var("XDG_STATE_HOME", &state_dir);
    reload_manifests();
    let result = f();
    match old_config {
        Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    match old_state {
        Some(value) => std::env::set_var("XDG_STATE_HOME", value),
        None => std::env::remove_var("XDG_STATE_HOME"),
    }
    reload_manifests();
    let _ = std::fs::remove_dir_all(&base);
    result
}

fn write_remote_codex(content: &str) {
    let path = crate::detect::manifest_update::remote_manifest_path(Agent::Codex);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
    reload_manifests();
}

fn write_remote_codex_without_reload(content: &str) {
    let path = crate::detect::manifest_update::remote_manifest_path(Agent::Codex);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

fn write_local_codex(content: &str) {
    let path = override_path(Agent::Codex).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
    reload_manifests();
}

#[test]
fn codex_no_match_is_unknown_without_changing_other_agents() {
    with_manifest_dirs("no-match", || {
        write_local_codex(&local_manifest("working", "active-marker"));
        let explain = explain(Agent::Codex, "unmatched-marker");

        assert_eq!(explain.state, AgentState::Unknown);
        assert!(!explain.visible_idle);
        assert_eq!(
            explain.fallback_reason.as_deref(),
            Some("codex_state_ambiguous")
        );
        let other = fallback_explain(Some(Agent::Pi), None, false);
        assert_eq!(other.state, AgentState::Idle);
        assert_eq!(
            other.fallback_reason.as_deref(),
            Some(DEFAULT_KNOWN_AGENT_IDLE_FALLBACK)
        );
    });
}

#[test]
fn rule_semantics_apply_gates_priority_and_line_regex() {
    with_manifest_dirs("rule-semantics", || {
        write_local_codex(&rules_manifest(
            r#"
[[rules]]
id = "low_contains"
state = "idle"
priority = 1
contains = ["match"]

[[rules]]
id = "high_nested_gates"
state = "working"
priority = 10
contains = ["match"]
all = [
  { any = [{ regex = ["w[io]n"] }, { contains = ["fallback"] }] },
]
not = [
  { contains = ["blocked"] },
]

[[rules]]
id = "line_regex"
state = "blocked"
priority = 20
line_regex = ["^exact line$"]
"#,
        ));

        let high = explain(Agent::Codex, "match win");
        assert_eq!(high.state, AgentState::Working);
        assert_eq!(
            high.matched_rule.as_ref().map(|rule| rule.id.as_str()),
            Some("high_nested_gates")
        );

        let not_gate = explain(Agent::Codex, "match win blocked");
        assert_eq!(not_gate.state, AgentState::Idle);
        assert_eq!(
            not_gate.matched_rule.as_ref().map(|rule| rule.id.as_str()),
            Some("low_contains")
        );

        let line = explain(Agent::Codex, "before\nexact line\nafter");
        assert_eq!(line.state, AgentState::Blocked);
        assert_eq!(
            line.matched_rule.as_ref().map(|rule| rule.id.as_str()),
            Some("line_regex")
        );
    });
}

#[test]
fn remote_manifest_loads_between_local_override_and_bundled() {
    with_manifest_dirs("remote-source", || {
        write_remote_codex(&remote_manifest("9999.01.01.1", "blocked", "remote-ready"));

        let explain = explain(Agent::Codex, "remote-ready");

        assert_eq!(explain.state, AgentState::Blocked);
        assert!(matches!(
            explain.source,
            Some(ManifestSource::Remote { .. })
        ));
        assert_eq!(explain.manifest_version.as_deref(), Some("9999.01.01.1"));
        assert_eq!(
            explain.cached_remote_version.as_deref(),
            Some("9999.01.01.1")
        );
    });
}

#[test]
fn fallback_explain_preserves_active_manifest_version() {
    with_manifest_dirs("fallback-version", || {
        write_remote_codex(&remote_manifest("9999.01.01.1", "blocked", "remote-ready"));

        let explain = explain(Agent::Codex, "ordinary prompt text");

        assert_eq!(explain.state, AgentState::Unknown);
        assert_eq!(
            explain.fallback_reason.as_deref(),
            Some("codex_state_ambiguous")
        );
        assert_eq!(explain.manifest_version.as_deref(), Some("9999.01.01.1"));
        assert!(matches!(
            explain.source,
            Some(ManifestSource::Remote { .. })
        ));
    });
}

#[test]
fn older_cached_remote_manifest_does_not_shadow_newer_bundled_manifest() {
    with_manifest_dirs("older-remote-bundled-fallback", || {
        write_remote_codex(&remote_manifest("2026.06.10.0", "blocked", "remote-ready"));

        let explain = explain(Agent::Codex, "remote-ready");

        assert!(matches!(explain.source, Some(ManifestSource::Bundled)));
        assert_eq!(
            explain.cached_remote_version.as_deref(),
            Some("2026.06.10.0")
        );
        assert!(explain
            .warning
            .as_deref()
            .is_some_and(|warning| warning.contains("older than bundled")));
    });
}

#[test]
fn local_override_shadows_cached_remote_manifest() {
    with_manifest_dirs("local-shadows-remote", || {
        write_remote_codex(&remote_manifest("9999.01.01.1", "blocked", "remote-ready"));
        write_local_codex(&local_manifest("idle", "local-ready"));

        let explain = explain(Agent::Codex, "local-ready");

        assert_eq!(explain.state, AgentState::Idle);
        assert!(matches!(explain.source, Some(ManifestSource::Override(_))));
        assert!(explain.local_override_shadowing_remote);
        assert_eq!(
            explain.cached_remote_version.as_deref(),
            Some("9999.01.01.1")
        );
    });
}

#[test]
fn invalid_local_override_falls_back_to_cached_remote_manifest() {
    with_manifest_dirs("invalid-local-remote-fallback", || {
        write_remote_codex(&remote_manifest("9999.01.01.1", "blocked", "remote-ready"));
        write_local_codex("id = ");

        let explain = explain(Agent::Codex, "remote-ready");

        assert_eq!(explain.state, AgentState::Blocked);
        assert!(matches!(
            explain.source,
            Some(ManifestSource::Remote { .. })
        ));
        assert!(explain.warning.is_some());
    });
}

#[test]
fn detection_uses_cached_manifest_until_explicit_reload() {
    with_manifest_dirs("cache-boundary", || {
        write_remote_codex(&remote_manifest("9999.01.01.1", "blocked", "cached-ready"));

        let cached = explain(Agent::Codex, "cached-ready");
        assert_eq!(cached.state, AgentState::Blocked);
        assert!(matches!(cached.source, Some(ManifestSource::Remote { .. })));
        assert_eq!(
            cached.matched_rule.as_ref().map(|rule| rule.id.as_str()),
            Some("test")
        );

        write_remote_codex_without_reload(&remote_manifest("9999.01.01.2", "working", "new-ready"));

        let unchanged = explain(Agent::Codex, "new-ready");
        assert_eq!(unchanged.state, AgentState::Unknown);
        assert_eq!(
            unchanged.fallback_reason.as_deref(),
            Some("codex_state_ambiguous")
        );
        assert_eq!(
            unchanged.cached_remote_version.as_deref(),
            Some("9999.01.01.1")
        );

        reload_manifests();

        let reloaded = explain(Agent::Codex, "new-ready");
        assert_eq!(reloaded.state, AgentState::Working);
        assert_eq!(
            reloaded.cached_remote_version.as_deref(),
            Some("9999.01.01.2")
        );
        assert_eq!(
            reloaded.matched_rule.as_ref().map(|rule| rule.id.as_str()),
            Some("test")
        );
    });
}

#[test]
fn compiled_rules_are_shared_until_manifest_reload() {
    with_manifest_dirs("shared-compiled-rules", || {
        write_remote_codex(&format!(
            "{}\nregex = ['^cached-[a-z]+$']\n",
            remote_manifest("9999.01.01.1", "blocked", "cached-ready")
        ));
        let first = load_manifest(Agent::Codex).unwrap();
        let second = load_manifest(Agent::Codex).unwrap();
        assert!(!first.compiled_rules.is_empty());
        assert_eq!(
            first.compiled_rules.as_ptr(),
            second.compiled_rules.as_ptr(),
            "cached loads must retain the same compiled rules and regex search caches"
        );

        write_remote_codex_without_reload(&format!(
            "{}\nregex = ['^new-[a-z]+$']\n",
            remote_manifest("9999.01.01.2", "working", "new-ready")
        ));
        let unchanged = load_manifest(Agent::Codex).unwrap();
        assert_eq!(
            first.compiled_rules.as_ptr(),
            unchanged.compiled_rules.as_ptr()
        );

        reload_manifests_for_agents(&[Agent::Codex]);
        let reloaded = load_manifest(Agent::Codex).unwrap();
        let shared_reload = load_manifest(Agent::Codex).unwrap();
        assert_ne!(
            first.compiled_rules.as_ptr(),
            reloaded.compiled_rules.as_ptr()
        );
        assert_eq!(
            reloaded.compiled_rules.as_ptr(),
            shared_reload.compiled_rules.as_ptr()
        );
        assert!(compiled_rule_matches(
            &first.compiled_rules[0],
            "cached-ready"
        ));
        assert!(!compiled_rule_matches(
            &first.compiled_rules[0],
            "new-ready"
        ));
        assert_eq!(
            explain(Agent::Codex, "new-ready").state,
            AgentState::Working
        );

        std::thread::scope(|scope| {
            for _ in 0..4 {
                let reloaded = &reloaded;
                scope.spawn(move || {
                    let loaded = load_manifest(Agent::Codex).unwrap();
                    assert_eq!(
                        loaded.compiled_rules.as_ptr(),
                        reloaded.compiled_rules.as_ptr()
                    );
                    for _ in 0..8 {
                        assert_eq!(detect(Agent::Codex, "new-ready").state, AgentState::Working);
                    }
                });
            }
        });
    });
}

#[test]
fn osc_regions_use_separate_inputs_and_share_rule_priority() {
    with_manifest_dirs("osc-regions", || {
        write_local_codex(&rules_manifest(
            r#"
[[rules]]
id = "screen"
state = "idle"
priority = 10
region = "whole_recent"
visible_idle = true
contains = ["screen-marker"]

[[rules]]
id = "title"
state = "working"
priority = 20
region = "osc_title"
visible_working = true
regex = ['^title-marker$']

[[rules]]
id = "progress"
state = "blocked"
priority = 30
region = "osc_progress"
visible_blocker = true
regex = ['^progress-marker$']
"#,
        ));
        for (screen, title, progress, state, rule) in [
            ("screen-marker", "", "", AgentState::Idle, "screen"),
            (
                "screen-marker",
                "title-marker",
                "",
                AgentState::Working,
                "title",
            ),
            (
                "screen-marker",
                "title-marker",
                "progress-marker",
                AgentState::Blocked,
                "progress",
            ),
            (
                "screen-marker title-marker progress-marker",
                "",
                "",
                AgentState::Idle,
                "screen",
            ),
        ] {
            let input = DetectionInput {
                screen,
                osc_title: title,
                osc_progress: progress,
            };
            let result = explain_with_input(Agent::Codex, input);
            assert_eq!(result.state, state);
            assert_eq!(
                result
                    .matched_rule
                    .as_ref()
                    .map(|matched| matched.id.as_str()),
                Some(rule)
            );
            let detection =
                crate::detect::detect_agent_with_osc(Some(Agent::Codex), screen, title, progress);
            assert_eq!(detection.state, state);
            assert_eq!(detection.visible_idle, state == AgentState::Idle);
            assert_eq!(detection.visible_working, state == AgentState::Working);
            assert_eq!(detection.visible_blocker, state == AgentState::Blocked);
        }
        let swapped = explain_with_input(
            Agent::Codex,
            DetectionInput {
                screen: "",
                osc_title: "progress-marker",
                osc_progress: "title-marker",
            },
        );
        assert!(swapped.matched_rule.is_none());
    });
}

#[test]
fn skip_rule_suppresses_state_update_without_visible_state_evidence() {
    with_manifest_dirs("skip-rule", || {
        write_local_codex(&rules_manifest(
            r#"
[[rules]]
id = "activity"
state = "working"
priority = 10
visible_working = true
contains = ["activity-marker"]

[[rules]]
id = "overlay"
state = "unknown"
priority = 20
skip_state_update = true
contains = ["overlay-marker"]
"#,
        ));
        let screen = "activity-marker overlay-marker";
        let result = explain(Agent::Codex, screen);
        assert_eq!(result.state, AgentState::Unknown);
        assert!(result.skip_state_update);
        assert_eq!(
            result.skipped_update_reason.as_deref(),
            Some("matched_rule:overlay")
        );
        assert!(!result.visible_idle);
        assert!(!result.visible_working);
        assert!(!result.visible_blocker);
        assert!(detect(Agent::Codex, screen).skip_state_update);
    });
}

#[test]
fn screen_regions_extract_structure_without_classifying_agent_state() {
    for (screen, spec, expected) in [
        ("old\n\nnew\n", "bottom_lines(2)", "\nnew\n"),
        (
            "before\n› input\nafter\n",
            "after_last_prompt_marker",
            "after\n",
        ),
        (
            "before\n› input\nafter\n",
            "before_current_prompt_marker",
            "before\n",
        ),
        (
            "before\n› input\nafter\n",
            "whole_recent_without_current_prompt_marker",
            "",
        ),
        (
            "no marker\n",
            "whole_recent_without_current_prompt_marker",
            "no marker\n",
        ),
        (
            "• old\n■ latest\n› input\n",
            "current_prompt_block_marker",
            "■ latest",
        ),
        (
            "• old\n■ latest\n› input\n",
            "after_current_prompt_block_marker",
            "■ latest\n› input\n",
        ),
        ("› old\n• new\n", "current_prompt_block_marker", ""),
        (
            "above\n\n───\nbody\n───\nfooter\n",
            "above_prompt_box",
            "above\n\n",
        ),
        (
            "above\n\n───\nbody\n───\nfooter\n",
            "last_non_empty_above_prompt_box",
            "above",
        ),
        (
            "above\n───\nbody\n───\nfooter\n",
            "prompt_box_body",
            "body\n",
        ),
        (
            "above\n───\nbody\n───\nfooter\n",
            "after_last_horizontal_rule",
            "footer\n",
        ),
    ] {
        assert_eq!(
            region(
                DetectionInput {
                    screen,
                    osc_title: "",
                    osc_progress: ""
                },
                spec
            ),
            expected,
            "region={spec}"
        );
    }
}

#[test]
fn all_bundled_manifests_parse_and_validate() {
    for agent in Agent::SCREEN_MANIFEST_AGENTS {
        assert!(
            bundled_manifest(agent).is_some(),
            "missing bundled manifest for {}",
            agent_label(agent)
        );
    }
}

#[test]
fn trae_manifest_detects_idle_working_and_blocked_states() {
    // Captured from a real idle `traecli` pane via `herdr agent read <pane>
    // --source detection --format text`.
    let idle = explain(
        Agent::Trae,
        "╭─────────────────────────────────────────────────────────╮\n│ ▄▄▄▄▄▄▄                                                 │\n│ █ ◆ ◆ █  TRAE CLI Next (v0.200.16)                      │\n│  ▀▀▀▀▀▀                                                 │\n│ Good morning, lei.cen                                   │\n│                                                         │\n│ model:     GPT-5.5 (MAX) high   /model to change        │\n│ directory: ~/go/src/code.byted.org/tiktok/bric_monorepo │\n╰─────────────────────────────────────────────────────────╯\n\nToken usage: total=49,001 input=47,411 (+ 133,120 cached) output=1,590 (reasoning 876)\n\n▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄\n❯ Summarize recent commits\n▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀\n  GPT-5.5 (MAX) high · Context 100% left · ~/go/src/code.byted.org/tiktok/bric_monorepo · master … ◐ Workspace Edit (shift+tab to cycle)\n",
    );
    assert_eq!(idle.state, AgentState::Idle);
    assert_eq!(
        idle.matched_rule.as_ref().map(|rule| rule.id.as_str()),
        Some("no_activity_idle")
    );
    assert!(idle.visible_idle);

    // Captured from a real pane mid-turn (running a background shell command).
    let working = explain(
        Agent::Trae,
        "◆ Running sleep 60 && gdpa-cli run env --session-id sess_20260709_114459_c77957f1 --input '{\"action\": \"dsl_status\"}' 2>&1 | tail -50\n  └ (22s · timeout 2m)\n    (ctrl+b ctrl+b (twice) to run in background)\n\n◈ Running command… (3m 27s • ↑ 4.05K tokens • esc to interrupt) · 1 shell running… · /ps to manage\n▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄\n❯ Improve documentation in @filename\n▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀\n  openrouter-3o (MAX) xhigh · Context 82% left · ~/go/src/code.byted.org/bric_webarch/faraday_admin · feat/captcha-decisi… ▰ Full Access\n",
    );
    assert_eq!(working.state, AgentState::Working);
    assert_eq!(
        working.matched_rule.as_ref().map(|rule| rule.id.as_str()),
        Some("turn_in_progress_working")
    );
    assert!(working.visible_working);

    // Captured from a real pane waiting on a shell-command approval prompt.
    let command_permission_prompt = explain(
        Agent::Trae,
        "◆ Running bazelisk test //shared/plugin/newton/action:action_test\n\n──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────\n\n  Would you like to run the following command?\n\n  Reason: Do you want me to run the targeted Bazel test for the merged Newton action package before committing the merge?\n\n  $ bazelisk test //shared/plugin/newton/action:action_test\n\n❯ 1. Yes, proceed (y)\n  2. Yes, switch this session to auto mode (r)\n  3. Yes, switch this session to full access mode (f)\n  4. No, and tell TRAE CLI what to do differently (esc)\n\n  Press enter to confirm or esc to cancel\n",
    );
    assert_eq!(command_permission_prompt.state, AgentState::Blocked);
    assert_eq!(
        command_permission_prompt
            .matched_rule
            .as_ref()
            .map(|rule| rule.id.as_str()),
        Some("permission_prompt_blocked")
    );
    assert!(command_permission_prompt.visible_blocker);

    // Captured from a real pane waiting on an MCP tool-call approval prompt.
    let mcp_permission_prompt = explain(
        Agent::Trae,
        "◆ Calling\n  └ lark-docs.import_markdown_to_lark({\"filePath\":\"/path/to/file.md\",\"title\":\"Title\"})\n\n──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────\n\n  Field 1/1\n  Allow the lark-docs MCP server to run tool \"import_markdown_to_lark\"?\n\n  filePath: /path/to/file.md\n  title: Title\n\n  ❯ 1. Allow                   Run the tool and continue.\n    2. Allow for this session  Run the tool and remember this choice for this session.\n    3. Always allow            Run the tool and remember this choice for future tool calls.\n    4. Cancel                  Cancel this tool call\n  enter to submit | esc to cancel\n",
    );
    assert_eq!(mcp_permission_prompt.state, AgentState::Blocked);
    assert_eq!(
        mcp_permission_prompt
            .matched_rule
            .as_ref()
            .map(|rule| rule.id.as_str()),
        Some("permission_prompt_blocked")
    );
    assert!(mcp_permission_prompt.visible_blocker);

    // Captured from a real pane waiting on Trae's native structured-question
    // UI. Its footer also contains "esc to interrupt" (shared with the
    // working-turn footer), so this must be classified via
    // "enter to submit answer", not misread as still working.
    let structured_question_prompt = explain(
        Agent::Trae,
        "──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────\n\n  Question 1/1 (1 unanswered)\n  Which would you like?\n\n  ❯ 1. Coffee             A hot, caffeinated pick-me-up.\n    2. Tea                A milder, soothing alternative.\n    3. None of the above  Optionally, add details in notes (tab).\n\n  tab to add notes | enter to submit answer | esc to interrupt\n",
    );
    assert_eq!(structured_question_prompt.state, AgentState::Blocked);
    assert_eq!(
        structured_question_prompt
            .matched_rule
            .as_ref()
            .map(|rule| rule.id.as_str()),
        Some("permission_prompt_blocked")
    );
    assert!(structured_question_prompt.visible_blocker);
}

#[test]
fn manifest_validation_rejects_unknown_fields_empty_rules_invalid_regions_and_regexes() {
    assert!(parse_manifest(
        r#"
id = "codex"

[[rules]]
id = "typo"
state = "working"
contain = ["Working"]
"#
    )
    .is_err());
    assert!(parse_manifest(
        r#"
id = "codex"

[[rules]]
id = "empty"
state = "working"
"#
    )
    .is_err());
    assert!(parse_manifest(
        r#"
id = "codex"

[[rules]]
id = "bad_region"
state = "working"
region = "after_last_promt_marker"
contains = ["Working"]
"#
    )
    .is_err());
    assert!(parse_manifest(
        r#"
id = "codex"

[[rules]]
id = "bad_regex"
state = "working"
regex = ["["]
"#
    )
    .is_err());
    assert!(parse_manifest(
        r#"
id = "codex"

[[rules]]
id = "bad_nested_regex"
state = "working"
any = [{ line_regex = ["["] }]
"#
    )
    .is_err());
}

#[test]
fn manifest_validation_keeps_skip_rules_neutral() {
    assert!(parse_manifest(
        r#"
id = "codex"

[[rules]]
id = "bad_skip_state"
state = "idle"
skip_state_update = true
contains = ["menu"]
"#
    )
    .is_err());
    assert!(parse_manifest(
        r#"
id = "codex"

[[rules]]
id = "bad_skip_visible"
state = "unknown"
skip_state_update = true
visible_blocker = true
contains = ["menu"]
"#
    )
    .is_err());
}

#[test]
fn manifest_validation_rejects_excessive_rule_count() {
    let mut manifest = String::from(
        r#"
id = "codex"
"#,
    );
    for index in 0..129 {
        manifest.push_str(&format!(
            r#"
[[rules]]
id = "rule_{index}"
state = "idle"
contains = ["ready"]
"#
        ));
    }
    assert!(parse_manifest(&manifest).is_err());
}

#[test]
fn manifest_validation_rejects_excessive_gate_depth() {
    let manifest = r#"
id = "codex"

[[rules]]
id = "deep"
state = "idle"
contains = ["ready"]
all = [
  { contains = ["1"], all = [
    { contains = ["2"], all = [
      { contains = ["3"], all = [
        { contains = ["4"], all = [
          { contains = ["5"], all = [
            { contains = ["6"], all = [
              { contains = ["7"], all = [
                { contains = ["8"], all = [
                  { contains = ["9"] },
                ] },
              ] },
            ] },
          ] },
        ] },
      ] },
    ] },
  ] },
]
"#;
    assert!(parse_manifest(manifest).is_err());
}

#[test]
fn manifest_validation_rejects_excessive_matchers() {
    let matchers = (0..33)
        .map(|index| format!(r#""m{index}""#))
        .collect::<Vec<_>>()
        .join(", ");
    let manifest = format!(
        r#"
id = "codex"

[[rules]]
id = "many"
state = "idle"
contains = [{matchers}]
"#
    );
    assert!(parse_manifest(&manifest).is_err());
}

#[test]
fn bottom_non_empty_lines_uses_bottom_occurrence_for_repeated_text() {
    let content = "marker\nold\n\nmiddle\nmarker\nnew\n";
    assert_eq!(
        region(
            DetectionInput {
                screen: content,
                osc_title: "",
                osc_progress: ""
            },
            "bottom_non_empty_lines(2)"
        ),
        "marker\nnew\n"
    );
}

#[test]
fn top_non_empty_lines_uses_top_occurrence_for_repeated_text() {
    let content = "\nmarker\nold\n\nmiddle\nmarker\nnew\n";
    assert_eq!(
        region(
            DetectionInput {
                screen: content,
                osc_title: "",
                osc_progress: ""
            },
            "top_non_empty_lines(2)"
        ),
        "\nmarker\nold\n"
    );
}

#[test]
fn top_non_empty_lines_requires_a_canonical_positive_bounded_count() {
    let name = "top_non_empty_lines";
    assert!(validate_region_name(&format!("{name}(1)")).is_ok());
    assert!(validate_region_name(&format!("{name}({})", u16::MAX)).is_ok());
    for count in ["0", "01", "+1", "65536", "999999999999999999999999"] {
        assert!(
            validate_region_name(&format!("{name}({count})")).is_err(),
            "{name} accepted invalid count {count}"
        );
    }
}

#[test]
fn top_non_empty_lines_requires_engine_three_when_declared() {
    let manifest = r#"
id = "codex"
version = "1"
min_engine_version = 2

[[rules]]
id = "background"
state = "working"
region = " top_non_empty_lines(1) "
contains = ["active"]
"#;
    assert!(parse_manifest(manifest).is_err());
}
