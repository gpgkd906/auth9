from __future__ import annotations

import argparse
import json
import sys

from detect_secrets.__version__ import VERSION
from detect_secrets.core import baseline
from detect_secrets.core.secrets_collection import SecretsCollection
from detect_secrets.pre_commit_hook import should_update_baseline

from detect_secrets_common import apply_rename_map_to_baseline
from detect_secrets_common import get_repo_root
from detect_secrets_common import get_staged_files
from detect_secrets_common import get_staged_renames


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Refresh .secrets.baseline for staged or explicit files.",
    )
    parser.add_argument("filenames", nargs="*")
    parser.add_argument(
        "--baseline",
        default=".secrets.baseline",
        help="Path to the baseline file.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    repo_root = get_repo_root()
    baseline_path = repo_root / args.baseline
    filenames = args.filenames or get_staged_files(repo_root)

    if not filenames:
        print("No staged or explicit files to refresh.")
        return 0

    baseline_data = apply_rename_map_to_baseline(
        baseline.load_from_file(str(baseline_path)),
        get_staged_renames(repo_root),
    )
    baseline_version = baseline_data["version"]
    secrets = baseline.load(baseline_data, filename=str(baseline_path))

    scanned_results = SecretsCollection()
    for filename in filenames:
        scanned_results.scan_file(filename)

    if not should_update_baseline(
        secrets,
        scanned_results=scanned_results,
        filelist=filenames,
        baseline_version=baseline_version,
    ):
        print(f"{args.baseline} is already up to date.")
        return 0

    if baseline_version != VERSION:
        upgraded = dict(baseline_data)
        upgraded["results"] = secrets.json()
        output = baseline.upgrade(upgraded)
    else:
        output = baseline.format_for_output(secrets)

    baseline_path.write_text(json.dumps(output, indent=2) + "\n")
    print(f"Updated {args.baseline}.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
