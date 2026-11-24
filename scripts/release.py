#!/usr/bin/env python3
"""Prepare and publish a Scentkeep release."""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = re.compile(r"(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)")


def run(*args, capture=False):
    result = subprocess.run(args, cwd=ROOT, check=True, text=True,
                            stdout=subprocess.PIPE if capture else None)
    return result.stdout.strip() if capture else None


def fail(message):
    raise SystemExit(message)


def versions():
    cargo = (ROOT / "src-tauri/Cargo.toml").read_text()
    return {
        "package.json": json.loads((ROOT / "package.json").read_text())["version"],
        "src-tauri/tauri.conf.json": json.loads((ROOT / "src-tauri/tauri.conf.json").read_text())["version"],
        "src-tauri/Cargo.toml": re.search(r'^version\s*=\s*"([^"]+)"', cargo, re.M).group(1),
    }


def clean_tree():
    if run("git", "status", "--porcelain", capture=True):
        fail("Working tree has changes. Commit or stash them before running this command.")


def unused_tag(tag):
    if run("git", "tag", "--list", tag, capture=True):
        fail(f"Tag {tag} already exists locally.")
    if run("git", "ls-remote", "--tags", "origin", f"refs/tags/{tag}", capture=True):
        fail(f"Tag {tag} already exists on origin.")


def prepare(version):
    clean_tree()
    unused_tag(f"v{version}")
    current = versions()
    if len(set(current.values())) != 1:
        fail(f"Version files disagree: {current}")
    if version == current["package.json"]:
        fail(f"Already at {version}.")
    for name in ("package.json", "src-tauri/tauri.conf.json"):
        path = ROOT / name
        data = json.loads(path.read_text())
        data["version"] = version
        path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n")
    path = ROOT / "src-tauri/Cargo.toml"
    contents = path.read_text()
    path.write_text(re.sub(r'^(version\s*=\s*)"[^"]+"',
                           lambda match: f'{match.group(1)}"{version}"',
                           contents, count=1, flags=re.M))
    # Finalize npm's lockfile after both installers have run, then validate it
    # with the same clean install used by the release workflow. A successful
    # `npm install` alone does not guarantee that `npm ci` will accept the lock.
    for command in (("npm", "install"), ("bun", "install"),
                    ("npm", "install", "--package-lock-only", "--ignore-scripts"),
                    ("npm", "ci"),
                    ("cargo", "check", "--manifest-path", "src-tauri/Cargo.toml"),
                    ("npm", "run", "check")):
        print("+", *command, flush=True)
        run(*command)
    print(f"Prepared v{version}. Review the diff, commit it, then run: python3 scripts/release.py publish")


def publish():
    clean_tree()
    current = versions()
    if len(set(current.values())) != 1:
        fail(f"Version files disagree: {current}")
    version = current["package.json"]
    if not VERSION.fullmatch(version):
        fail(f"Invalid release version: {version}")
    tag = f"v{version}"
    unused_tag(tag)
    # Reject an invalid committed lockfile before pushing a release tag.
    run("npm", "ci")
    branch = run("git", "branch", "--show-current", capture=True)
    if not branch:
        fail("Check out a branch before publishing.")
    if branch != "main":
        fail("Publish from main after merging the release commit.")
    run("git", "fetch", "origin", "main")
    if run("git", "merge-base", "HEAD", "origin/main", capture=True) != run("git", "rev-parse", "origin/main", capture=True):
        fail("Local main is behind or diverged from origin/main. Pull or resolve it first.")
    if run("git", "rev-list", "--count", "origin/main..HEAD", capture=True) == "0":
        fail("No new commit to release relative to origin/main.")
    print(f"Release {tag} from {branch} ({run('git', 'rev-parse', '--short', 'HEAD', capture=True)})")
    if input("Push commit and release tag to origin? [y/N] ").strip().lower() != "y":
        fail("Cancelled.")
    run("git", "push", "origin", "HEAD")
    run("git", "tag", "-a", tag, "-m", f"Scentkeep {tag}")
    run("git", "push", "origin", tag)
    print(f"Published {tag}. Check the Release workflow and GitHub Release assets.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    prepare_parser = commands.add_parser("prepare", help="Update versions, lockfiles, and run checks")
    prepare_parser.add_argument("version", help="New version, for example 0.2.0")
    commands.add_parser("publish", help="Push the committed version and release tag")
    args = parser.parse_args()
    if args.command == "prepare":
        if not VERSION.fullmatch(args.version):
            parser.error("version must be MAJOR.MINOR.PATCH, for example 0.2.0")
        prepare(args.version)
    else:
        publish()


if __name__ == "__main__":
    try:
        main()
    except (subprocess.CalledProcessError, FileNotFoundError) as error:
        fail(str(error))
