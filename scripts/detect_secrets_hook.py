from __future__ import annotations

import shlex
import sys

from detect_secrets.core import baseline
from detect_secrets.core.secrets_collection import SecretsCollection
from detect_secrets.pre_commit_hook import parse_args, pretty_print_diagnostics
from detect_secrets.pre_commit_hook import should_update_baseline

from detect_secrets_common import apply_rename_map_to_baseline
from detect_secrets_common import get_repo_root
from detect_secrets_common import get_staged_renames


def main(argv: list[str] | None = None) -> int:
    try:
        args = parse_args(argv)
    except ValueError:
        return 1

    if not args.baseline:
        secrets = SecretsCollection()
        for filename in args.filenames:
            secrets.scan_file(filename)
        if secrets:
            pretty_print_diagnostics(secrets)
            return 1
        return 0

    repo_root = get_repo_root()
    rename_map = get_staged_renames(repo_root)
    baseline_data = apply_rename_map_to_baseline(
        baseline.load_from_file(args.baseline_filename),
        rename_map,
    )
    args.baseline = baseline.load(baseline_data, filename=args.baseline_filename)
    args.baseline_version = baseline_data["version"]

    secrets = SecretsCollection()
    for filename in args.filenames:
        secrets.scan_file(filename)

    new_secrets = secrets - args.baseline
    if new_secrets:
        pretty_print_diagnostics(new_secrets)
        return 1

    if should_update_baseline(
        args.baseline,
        scanned_results=secrets,
        filelist=args.filenames,
        baseline_version=args.baseline_version,
    ):
        files = " ".join(shlex.quote(filename) for filename in args.filenames)
        print(
            "detect-secrets baseline is out of date.\n"
            "Refresh it explicitly instead of rewriting it during commit:\n"
            f"  pre-commit run detect-secrets-refresh --files {files}\n"
            "Then `git add .secrets.baseline` and retry the commit.\n",
        )
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
