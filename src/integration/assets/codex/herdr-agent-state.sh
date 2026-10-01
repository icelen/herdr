#!/bin/sh
# installed by herdr
# managed by herdr; reinstalling or updating the integration overwrites this file.
# add custom hooks beside this file instead of editing it.
# HERDR_INTEGRATION_ID=codex
# HERDR_INTEGRATION_VERSION=9

set -eu

action="${1:-}"
hook_input_file="$(mktemp "${TMPDIR:-/tmp}/herdr-codex-hook.XXXXXX")" || exit 0
trap 'rm -f "$hook_input_file"' EXIT HUP INT TERM
cat >"$hook_input_file" 2>/dev/null || true

case "$action" in
  session|working|idle) ;;
  *) exit 0 ;;
esac

# A shared Codex app-server daemon runs hooks for every session with the
# environment of whichever pane started it, so HERDR_PANE_ID would point at the
# wrong pane. Only report to a pane when the pane's own Codex process runs the
# hook; the worktree session record below is written either way.
in_daemon=0
case "$(ps -ww -o command= -p "$PPID" 2>/dev/null || true)" in
  *" app-server"*) in_daemon=1 ;;
esac

[ "$action" = "session" ] || [ "$in_daemon" = "0" ] || exit 0
command -v python3 >/dev/null 2>&1 || exit 0

HERDR_ACTION="$action" HERDR_HOOK_INPUT_FILE="$hook_input_file" HERDR_CODEX_IN_DAEMON="$in_daemon" python3 - <<'PY'
import json
import os
import random
import socket
import subprocess
import time

source = "herdr:codex"
action = os.environ.get("HERDR_ACTION", "")
pane_id = os.environ.get("HERDR_PANE_ID")
socket_path = os.environ.get("HERDR_SOCKET_PATH")
hook_input_file = os.environ.get("HERDR_HOOK_INPUT_FILE")
in_daemon = os.environ.get("HERDR_CODEX_IN_DAEMON") == "1"

hook_input = {}
if hook_input_file:
    try:
        with open(hook_input_file, encoding="utf-8") as handle:
            content = handle.read()
        if content.strip():
            hook_input = json.loads(content)
    except Exception:
        hook_input = {}

hook_event_name = str(hook_input.get("hook_event_name") or "")
expected_events = {"session": ("SessionStart",), "working": ("UserPromptSubmit",), "idle": ("Stop", "Interrupt")}[action]
if hook_event_name and hook_event_name not in expected_events:
    raise SystemExit(0)

request_id = f"{source}:{int(time.time() * 1000)}:{random.randrange(1_000_000):06d}"
report_seq = time.time_ns()
session_id = hook_input.get("session_id")
agent_session_id = session_id if isinstance(session_id, str) and session_id else None
if action == "session":
    transcript_path = hook_input.get("transcript_path")
    if not isinstance(transcript_path, str) or not transcript_path.strip():
        raise SystemExit(0)
inherited_session_id = os.environ.get("CODEX_THREAD_ID")
if inherited_session_id and inherited_session_id != agent_session_id:
    raise SystemExit(0)


def is_subagent_session():
    # Subagent threads record their parent in the transcript's session_meta.
    try:
        with open(hook_input.get("transcript_path") or "", encoding="utf-8") as handle:
            meta = json.loads(handle.readline())
    except Exception:
        return False
    payload = meta.get("payload") if meta.get("type") == "session_meta" else None
    return isinstance(payload, dict) and isinstance(payload.get("source"), dict)


def record_worktree_session(session_id):
    # Herdr reads this per-worktree record to resume the Codex session in the
    # pane working there, which also covers sessions a shared daemon runs.
    cwd = hook_input.get("cwd")
    if not isinstance(cwd, str) or not os.path.isdir(cwd):
        return
    try:
        result = subprocess.run(
            ["git", "-C", cwd, "rev-parse", "--git-path", "herdr-codex-session"],
            capture_output=True,
            text=True,
            timeout=2,
        )
    except Exception:
        return
    relative = result.stdout.strip()
    if result.returncode != 0 or not relative:
        return
    path = os.path.join(cwd, relative)
    temporary = f"{path}.{os.getpid()}.tmp"
    try:
        with open(temporary, "w", encoding="utf-8") as handle:
            handle.write(session_id + "\n")
        os.replace(temporary, path)
    except Exception:
        try:
            os.unlink(temporary)
        except Exception:
            pass


if action == "session" and agent_session_id:
    if is_subagent_session():
        raise SystemExit(0)
    record_worktree_session(agent_session_id)

if in_daemon or os.environ.get("HERDR_ENV") != "1" or not pane_id or not socket_path:
    raise SystemExit(0)
session_start_source = hook_input.get("source") if action == "session" else None
if not isinstance(session_start_source, str) or not session_start_source:
    session_start_source = None
if agent_session_id:
    params = {
        "pane_id": pane_id,
        "source": source,
        "agent": "codex",
        "seq": report_seq,
        "agent_session_id": agent_session_id,
    }
    if action == "session":
        if session_start_source:
            params["session_start_source"] = session_start_source
        method = "pane.report_agent_session"
    else:
        params["state"] = action
        method = "pane.report_agent"
    request = {
        "id": request_id,
        "method": method,
        "params": params,
    }
else:
    raise SystemExit(0)

try:
    client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    client.settimeout(0.5)
    client.connect(socket_path)
    client.sendall((json.dumps(request) + "\n").encode())
    try:
        client.recv(4096)
    except Exception:
        pass
    client.close()
except Exception:
    pass
PY
