"""Set up Latch repository hooks for hosted coding sessions."""

import json
from pathlib import Path
import re
import subprocess
import sys
from tempfile import TemporaryDirectory

VERSION = "v0.1.5"
REPOSITORY = "BarakChamo/latch"
SETUPS = {
    "claude": (
        Path(".claude/settings.json"),
        Path(".claude/latch.sh"),
        "plugins/claude-code/hooks/hooks.json",
    ),
    "copilot": (
        Path(".github/hooks/latch.json"),
        Path(".github/latch.sh"),
        "plugins/copilot/hooks.json",
    ),
    "openhands": (
        Path(".openhands/hooks.json"),
        Path(".openhands/latch.sh"),
        "dev.openhands/hooks/hooks.json",
    ),
}


def _download(url: str) -> bytes:
    return subprocess.check_output(["curl", "-fsSL", "--max-time", "30", url])


def _cloud_hooks(host: str, template_path: str) -> dict[str, list[dict[str, object]]]:
    template = json.loads(_download(
        f"https://raw.githubusercontent.com/{REPOSITORY}/{VERSION}/{template_path}"
    ))
    for groups in template["hooks"].values():
        for group in groups:
            if host == "copilot":
                group["bash"] = group["bash"].replace(
                    '"${PLUGIN_ROOT}/bin/latch"', "sh .github/latch.sh", 1
                )
                continue
            command = {
                "claude": 'sh "${CLAUDE_PROJECT_DIR}/.claude/latch.sh" hook claude-code',
                "openhands": 'sh "${OPENHANDS_PROJECT_DIR}/.openhands/latch.sh" hook openhands',
            }[host]
            for handler in group["hooks"]:
                handler["command"] = command
    return template["hooks"]


def _already_registered(host: str, candidate: dict, entries: list) -> bool:
    for entry in entries:
        if not isinstance(entry, dict) or entry.get("matcher") != candidate.get("matcher"):
            continue
        if host == "copilot":
            if entry.get("bash") == candidate["bash"]:
                return True
        elif isinstance(entry.get("hooks"), list):
            commands = {handler["command"] for handler in candidate["hooks"]}
            if any(
                handler["command"] in commands
                for handler in entry["hooks"]
                if isinstance(handler, dict) and isinstance(handler.get("command"), str)
            ):
                return True
    return False


def _configure(project: Path, host: str) -> None:
    settings, launcher, template_path = SETUPS[host]
    settings_path = project / settings
    launcher_path = project / launcher
    paths = {settings_path, launcher_path, settings_path.parent, launcher_path.parent}
    if any(path.is_symlink() for path in paths):
        raise ValueError("Expected ordinary config files and directories inside this checkout.")

    config = json.loads(settings_path.read_bytes()) if settings_path.exists() else {}
    if not isinstance(config, dict) or not isinstance(config.get("hooks", {}), dict):
        raise ValueError(f"{settings} must contain an object with an optional hooks object.")

    cloud_hooks = _cloud_hooks(host, template_path)
    if host == "copilot":
        if config.get("version", 1) != 1:
            raise ValueError(f"{settings} must use Copilot hook format version 1.")
        config["version"] = 1

    hooks = config if host == "openhands" and "hooks" not in config else config.setdefault("hooks", {})
    for event, groups in cloud_hooks.items():
        if host == "openhands" and event not in hooks:
            event = re.sub(r"(?<!^)(?=[A-Z])", "_", event).lower()
        entries = hooks.setdefault(event, [])
        if not isinstance(entries, list):
            raise ValueError(f"Expected a list for {event} in {settings}.")
        entries.extend(group for group in groups if not _already_registered(host, group, entries))

    launcher_bytes = _download(
        f"https://github.com/{REPOSITORY}/releases/download/{VERSION}/install-latch-plugin.sh"
    )
    if not launcher_bytes.startswith(b"#!/bin/sh\n"):
        raise ValueError("GitHub did not return the expected Latch launcher.")
    if launcher_path.exists() and launcher_path.read_bytes() != launcher_bytes:
        raise ValueError(f"{launcher} already exists with different content.")

    settings_path.parent.mkdir(parents=True, exist_ok=True)
    with TemporaryDirectory(dir=settings_path.parent) as temporary:
        staged = Path(temporary)
        if not launcher_path.exists():
            new_launcher = staged / launcher.name
            new_launcher.write_bytes(launcher_bytes)
            new_launcher.replace(launcher_path)
        new_settings = staged / settings.name
        new_settings.write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        if settings_path.exists():
            new_settings.chmod(settings_path.stat().st_mode & 0o777)
        new_settings.replace(settings_path)

    print(f"Latch hooks configured. Commit {launcher} and {settings}.")


if __name__ == "__main__":
    if len(sys.argv) != 2 or sys.argv[1] not in SETUPS:
        sys.exit("Usage: python3 setup-hooks.py claude|copilot|openhands")
    try:
        checkout = subprocess.check_output(
            ["git", "rev-parse", "--show-toplevel"], text=True
        ).strip()
        _configure(Path(checkout), sys.argv[1])
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        sys.exit(f"Latch setup failed: {error}")
