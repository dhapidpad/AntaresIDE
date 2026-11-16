#!/usr/bin/env python3
"""Apply the Antares scaffold to a Lapce fork without replacing upstream CI/devcontainer files."""
from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
from pathlib import Path


def _normalize_path(p: Path) -> Path:
    s = str(p)
    if sys.platform == "win32" and re.match(r"^/[a-zA-Z]/", s):
        drive = s[1].upper()
        return Path(f"{drive}:{s[2:]}")
    return p


def copy_file(src: Path, dst: Path, *, overwrite: bool = False) -> str:
    dst.parent.mkdir(parents=True, exist_ok=True)
    if dst.exists() and not overwrite:
        return f"kept existing: {dst}"
    shutil.copy2(src, dst)
    return f"copied: {dst}"


def copy_tree(src: Path, dst: Path, *, overwrite: bool = False) -> list[str]:
    messages: list[str] = []
    if not src.exists():
        return messages
    for item in sorted(src.rglob("*")):
        if not item.is_file():
            continue
        rel = item.relative_to(src)
        messages.append(copy_file(item, dst / rel, overwrite=overwrite))
    return messages


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--target", required=True, type=Path)
    args = parser.parse_args()
    source = _normalize_path(args.source).resolve()
    target = _normalize_path(args.target).resolve()

    if not (target / ".git").exists() or not (target / "Cargo.toml").is_file():
        raise SystemExit("Target must be a Git repository root containing Lapce's Cargo.toml")
    if not (target / "lapce-app/Cargo.toml").is_file():
        raise SystemExit("Target does not look like a Lapce source checkout (lapce-app/Cargo.toml missing)")

    messages: list[str] = []
    # Keep Antares-owned material isolated from upstream's existing docs and workflows.
    tree_map = [
        (".agent", ".agent"),
        (".agents", ".agents"),
        ("antares-agent", "antares-agent"),
        ("config-templates", "config-templates/antares"),
        ("docs", "docs"),
        ("scripts", "scripts"),
        (".github/workflows-antares", ".github/workflows"),
        (".github/ISSUE_TEMPLATE-antares", ".github/ISSUE_TEMPLATE"),
        (".devcontainer/antares", ".devcontainer/antares"),
    ]
    for src_rel, dst_rel in tree_map:
        messages.extend(copy_tree(source / src_rel, target / dst_rel))

    # Antares-owned root files. The upstream LICENSE and Makefile stay intact.
    root_map = {
        "AGENTS.antares.md": "AGENTS.antares.md",
        "GEMINI.antares.md": "GEMINI.antares.md",
        "Makefile.antares": "Makefile.antares",
        "rust-toolchain.toml": "rust-toolchain.toml",
        "THIRD_PARTY_LICENSES.md": "THIRD_PARTY_LICENSES.md",
        "NOTICE": "NOTICE-ANTARES.md",
        "CHANGELOG.md": "CHANGELOG-ANTARES.md",
        "README.md": "README.md",
        ".gitignore.antares": ".gitignore.antares",
    }

    # Preserve the upstream README exactly before replacing it with the Antares-facing README.
    readme_src = source / "README.md"
    readme_dst = target / "README.md"
    if readme_dst.exists() and readme_src.exists():
        current = readme_dst.read_text(encoding="utf-8", errors="replace")
        backup = target / "docs/antares/upstream/lapce-README.md"
        if current.startswith("# Antares IDE"):
            messages.append("kept existing Antares README.md")
        elif backup.exists():
            # Recover from an interrupted first apply only when the current README
            # is byte-identical to the saved upstream copy. Otherwise it may be user-edited.
            if readme_dst.read_bytes() == backup.read_bytes():
                shutil.copy2(readme_src, readme_dst)
                messages.append("recovered interrupted README replacement from upstream backup")
            else:
                messages.append("kept existing custom README.md; upstream README backup already exists")
        else:
            backup.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(readme_dst, backup)
            shutil.copy2(readme_src, readme_dst)
            messages.append(f"preserved upstream README: {backup}")
            messages.append("installed Antares README.md; original README is preserved under docs/antares/upstream/")

    # Keep GitHub automation settings in an Antares-owned path unless upstream already owns them.
    if (source / ".github/dependabot.yml").is_file():
        messages.append(copy_file(source / ".github/dependabot.yml", target / ".github/dependabot.yml"))

    for src_rel, dst_rel in root_map.items():
        if src_rel == "README.md":
            continue
        src = source / src_rel
        if not src.is_file():
            continue
        dst = target / dst_rel
        # When an upstream root agent file already exists, keep it and install Antares guidance beside it.
        messages.append(copy_file(src, dst))

    # Keep all upstream root build/workflow/devcontainer files; integrate using dedicated includes only.
    (target / ".github/workflows").mkdir(parents=True, exist_ok=True)
    (target / ".devcontainer/antares").mkdir(parents=True, exist_ok=True)

    # Provide conventional entry points while preserving any upstream-authored instructions.
    pointers = {
        "AGENTS.md": ("AGENTS.antares.md", "For Antares IDE tasks, also read `AGENTS.antares.md` and `.agent/rules/` before editing."),
        "GEMINI.md": ("GEMINI.antares.md", "For Antares IDE tasks, read `GEMINI.antares.md`, `AGENTS.antares.md`, and `.agent/rules/` first."),
    }
    for entry_name, (rules_file, instruction) in pointers.items():
        entry = target / entry_name
        marker = f"<!-- ANTARES-{entry_name.upper().replace('.', '-')}-POINTER -->"
        if entry.exists():
            existing = entry.read_text(encoding="utf-8")
            if marker not in existing:
                with entry.open("a", encoding="utf-8") as fh:
                    fh.write(f"\n\n{marker}\n{instruction}\n")
                messages.append(f"added Antares pointer to existing {entry_name}")
        else:
            entry.write_text(f"# {entry_name}\n\n{marker}\n{instruction}\n", encoding="utf-8")
            messages.append(f"created {entry_name} entry point")

    # Merge only the Antares ignore entries; never replace upstream .gitignore.
    ignore_src = target / ".gitignore.antares"
    ignore_dst = target / ".gitignore"
    if ignore_src.exists() and ignore_dst.exists():
        existing = ignore_dst.read_text(encoding="utf-8")
        extra = [
            line for line in ignore_src.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        ]
        missing = [line for line in extra if line not in existing.splitlines()]
        if missing:
            with ignore_dst.open("a", encoding="utf-8") as fh:
                fh.write("\n\n# Antares local files and secrets\n" + "\n".join(missing) + "\n")
            messages.append("merged Antares entries into .gitignore")

    # Add the Antares Makefile include to Lapce's root Makefile, idempotently.
    makefile = target / "Makefile"
    include_line = "-include Makefile.antares"
    if makefile.exists():
        lines = makefile.read_text(encoding="utf-8").splitlines()
        if include_line not in lines:
            with makefile.open("a", encoding="utf-8") as fh:
                fh.write("\n# Antares IDE targets (kept separate from upstream targets)\n" + include_line + "\n")
            messages.append("added -include Makefile.antares to root Makefile")

    for message in messages:
        print(message)
    print(f"Antares overlay applied to: {target}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
