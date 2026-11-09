#!/usr/bin/env python3
"""Build portable archives and publish directly to Codeberg (Python 3.11+)."""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[1]
API = "https://codeberg.org/api/v1/repos/luqenov/xournalpp-discord-rpc"
_next_request_at = 0.0
TARGETS = {
    "x86_64-unknown-linux-musl": "x86_64",
    "aarch64-unknown-linux-musl": "aarch64",
}


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True).strip()


def validate_tag(tag):
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    if tag != f"v{version}":
        raise ValueError(f"tag must match Cargo.toml: v{version}")
    if git("status", "--porcelain", "--untracked-files=all"):
        raise ValueError("commit or stash changes before releasing")
    if git("rev-parse", f"refs/tags/{tag}^{{commit}}") != git("rev-parse", "HEAD"):
        raise ValueError("check out the release tag before releasing")


def checksums(files):
    return "".join(
        f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n"
        for path in sorted(files)
    )


def build(args):
    validate_tag(args.tag)
    targets = args.target or list(TARGETS)
    destination = ROOT / "dist" / args.tag
    if destination.exists():
        raise ValueError(f"output already exists: {destination}; move it before rebuilding")
    destination.parent.mkdir(exist_ok=True)
    # Publish the directory only after every build succeeds.
    with tempfile.TemporaryDirectory(dir=destination.parent) as temporary:
        stage = Path(temporary)
        for target in targets:
            subprocess.run(
                ["cargo", args.builder, "--release", "--locked", "--target", target],
                cwd=ROOT, check=True,
            )
            target_dir = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
            if not target_dir.is_absolute():
                target_dir = ROOT / target_dir
            binary = target_dir / target / "release" / "xournal-rpc"
            archive = stage / f"xournal-rpc-{args.tag}-linux-{TARGETS[target]}.tar.gz"
            with tarfile.open(archive, "w:gz") as output:
                for path in [binary, ROOT / "README.md", ROOT / "LICENSE",
                             ROOT / "packaging/xournal-rpc.service"]:
                    output.add(path, arcname=path.name)
        (stage / "SHA256SUMS.txt").write_text(checksums(stage.glob("*.tar.gz")))
        stage.rename(destination)
    print(f"Archives and checksums: {destination}")


def release_files(directory, tag):
    files = sorted(directory.glob("*.tar.gz"))
    allowed = {f"xournal-rpc-{tag}-linux-{arch}.tar.gz" for arch in TARGETS.values()}
    if not files or any(path.name not in allowed for path in files):
        raise ValueError("missing archives or unexpected archive names")
    manifest = directory / "SHA256SUMS.txt"
    if manifest.read_text() != checksums(files):
        raise ValueError("SHA256SUMS.txt does not match the archives")
    return [*files, manifest]


def api(method, path, token, data=None, content_type="application/json"):
    global _next_request_at
    if isinstance(data, dict):
        data = json.dumps(data).encode()
    request = urllib.request.Request(
        API + path, data=data, method=method,
        headers={"Authorization": f"token {token}", "Accept": "application/json",
                 "Content-Type": content_type},
    )
    # Publication is sequential: at most one request per second, with no retry loop.
    now = time.monotonic()
    if now < _next_request_at:
        time.sleep(_next_request_at - now)
    _next_request_at = max(now, _next_request_at) + 1.0
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            body = response.read()
            return response.status, json.loads(body) if body else None
    except urllib.error.HTTPError as error:
        if error.code == 404 and method == "GET":
            return 404, None
        if error.code == 429:
            retry_after = error.headers.get("Retry-After", "unspecified")
            raise RuntimeError(
                f"Codeberg rate limit reached (Retry-After: {retry_after}); "
                "stopping without automatic retries"
            ) from error
        raise RuntimeError(f"Codeberg API: {method} {path}: HTTP {error.code}") from error


def upload(release_id, path, token):
    # Forgejo expects multipart/form-data with an `attachment` field.
    boundary = uuid.uuid4().hex
    data = io.BytesIO()
    data.write((f'--{boundary}\r\nContent-Disposition: form-data; '
                f'name="attachment"; filename="{path.name}"\r\n'
                'Content-Type: application/octet-stream\r\n\r\n').encode())
    data.write(path.read_bytes())
    data.write(f"\r\n--{boundary}--\r\n".encode())
    query = urllib.parse.urlencode({"name": path.name})
    api("POST", f"/releases/{release_id}/assets?{query}", token, data.getvalue(),
        f"multipart/form-data; boundary={boundary}")


def publish(args):
    validate_tag(args.tag)
    files = release_files(ROOT / "dist" / args.tag, args.tag)
    body = args.notes.read_text() if args.notes else f"Release {args.tag}"
    if args.dry_run:
        print(f"Would publish {args.tag} directly to Codeberg with:")
        print(body)
        for path in files:
            print(path.name)
        return  # No requests, including reads, in dry-run mode.
    token = os.environ.get("CODEBERG_TOKEN", "").strip()
    if not token:
        raise ValueError("set CODEBERG_TOKEN to a Codeberg token with repository write access")
    tag = urllib.parse.quote(args.tag, safe="")
    # Require an existing remote tag, resolving annotated tags to their commit.
    _, resolved = api("GET", f"/tags/{tag}", token)
    if not resolved or resolved["commit"]["sha"] != git("rev-parse", "HEAD"):
        raise ValueError("Codeberg tag does not point to the local release commit")
    status, release = api("GET", f"/releases/tags/{tag}", token)
    if status == 404:
        _, release = api("POST", "/releases", token, {
            "tag_name": args.tag, "name": args.tag, "body": body,
            "draft": True, "prerelease": "-" in args.tag,
        })
    elif not release["draft"]:
        raise ValueError("release is already published; refusing to overwrite it")
    if any(asset["name"] not in {path.name for path in files}
           for asset in release.get("assets", [])):
        raise ValueError("draft contains unexpected assets; review them in Codeberg first")
    release_id = release["id"]
    # Retry interrupted uploads into the same draft. Only replace matching draft assets.
    for path in files:
        for asset in release.get("assets", []):
            if asset["name"] == path.name:
                api("DELETE", f'/releases/{release_id}/assets/{asset["id"]}', token)
        upload(release_id, path, token)
    _, release = api("PATCH", f"/releases/{release_id}", token, {
        "body": body, "draft": False,
    })
    print(f'Published: {release["html_url"]}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build_parser = commands.add_parser("build", help="build musl archives and SHA256SUMS.txt")
    build_parser.add_argument("tag")
    build_parser.add_argument("--target", choices=TARGETS, action="append")
    build_parser.add_argument("--builder", choices=["build", "zigbuild"], default="zigbuild")
    publish_parser = commands.add_parser("publish", help="upload and publish a Codeberg release")
    publish_parser.add_argument("tag")
    publish_parser.add_argument("--notes", type=Path)
    publish_parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    try:
        {"build": build, "publish": publish}[args.command](args)
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
