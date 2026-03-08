from __future__ import annotations

import subprocess
from pathlib import Path


def get_repo_root() -> Path:
    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=True,
        capture_output=True,
        text=True,
    )
    return Path(result.stdout.strip())


def get_staged_renames(repo_root: Path) -> dict[str, str]:
    result = subprocess.run(
        ["git", "diff", "--cached", "--name-status", "-z", "--diff-filter=R"],
        cwd=repo_root,
        check=True,
        capture_output=True,
    )
    parts = [part.decode("utf-8") for part in result.stdout.split(b"\0") if part]
    rename_map: dict[str, str] = {}
    i = 0
    while i + 2 < len(parts):
        status = parts[i]
        old_path = parts[i + 1]
        new_path = parts[i + 2]
        if status.startswith("R"):
            rename_map[old_path] = new_path
        i += 3
    return rename_map


def get_staged_files(repo_root: Path) -> list[str]:
    result = subprocess.run(
        ["git", "diff", "--cached", "--name-only", "-z", "--diff-filter=ACMR"],
        cwd=repo_root,
        check=True,
        capture_output=True,
    )
    return sorted(
        part.decode("utf-8")
        for part in result.stdout.split(b"\0")
        if part
    )


def apply_rename_map_to_baseline(baseline_data: dict, rename_map: dict[str, str]) -> dict:
    if not rename_map:
        return baseline_data

    updated = {**baseline_data}
    results = updated.get("results", {})
    rewritten_results = {}

    for path, findings in results.items():
        new_path = rename_map.get(path, path)
        rewritten_findings = []
        for finding in findings:
            rewritten_finding = dict(finding)
            rewritten_finding["filename"] = rename_map.get(
                rewritten_finding["filename"],
                rewritten_finding["filename"],
            )
            rewritten_findings.append(rewritten_finding)
        rewritten_results[new_path] = rewritten_findings

    updated["results"] = rewritten_results
    return updated
