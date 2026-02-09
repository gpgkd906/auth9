#!/usr/bin/env python3
import argparse
import os
from pathlib import Path

TEXT_EXTENSIONS = {
    ".md", ".toml", ".rs", ".ts", ".tsx", ".js", ".json", ".yml", ".yaml",
    ".env", ".sh", ".txt", ".html", ".css", ".Dockerfile"
}

PLACEHOLDERS = [
    "{{project_name}}",
    "{{core_port}}",
    "{{portal_port}}",
    "{{namespace}}",
]


def is_text_file(path: Path) -> bool:
    if path.suffix in TEXT_EXTENSIONS:
        return True
    if path.name.startswith("Dockerfile"):
        return True
    return False


def replace_placeholders(path: Path, values: dict) -> None:
    if not is_text_file(path):
        return
    try:
        content = path.read_text(encoding="utf-8")
    except Exception:
        return

    for key, val in values.items():
        content = content.replace(key, str(val))

    path.write_text(content, encoding="utf-8")


def copy_tree(src: Path, dst: Path) -> None:
    if dst.exists() and any(dst.iterdir()):
        raise RuntimeError(f"Target directory not empty: {dst}")
    for root, dirs, files in os.walk(src):
        rel = Path(root).relative_to(src)
        target_root = dst / rel
        target_root.mkdir(parents=True, exist_ok=True)
        for name in files:
            (target_root / name).write_bytes((Path(root) / name).read_bytes())


def main() -> None:
    parser = argparse.ArgumentParser(description="Initialize a new project from templates")
    parser.add_argument("--name", required=True, help="Project name (e.g., acme)")
    parser.add_argument("--root", required=True, help="Target directory")
    parser.add_argument("--core-port", type=int, default=8080)
    parser.add_argument("--portal-port", type=int, default=3000)
    parser.add_argument("--namespace", default=None)
    args = parser.parse_args()

    namespace = args.namespace or args.name

    skill_dir = Path(__file__).resolve().parents[1]
    template_dir = skill_dir / "assets" / "template"

    if not template_dir.exists():
        raise RuntimeError(f"Template directory not found: {template_dir}")

    target_dir = Path(args.root).resolve()
    target_dir.mkdir(parents=True, exist_ok=True)

    copy_tree(template_dir, target_dir)

    values = {
        "{{project_name}}": args.name,
        "{{core_port}}": args.core_port,
        "{{portal_port}}": args.portal_port,
        "{{namespace}}": namespace,
    }

    for root, _, files in os.walk(target_dir):
        for name in files:
            replace_placeholders(Path(root) / name, values)

    print("Project initialized:")
    print(f"  name: {args.name}")
    print(f"  root: {target_dir}")
    print(f"  core port: {args.core_port}")
    print(f"  portal port: {args.portal_port}")
    print(f"  namespace: {namespace}")


if __name__ == "__main__":
    main()
