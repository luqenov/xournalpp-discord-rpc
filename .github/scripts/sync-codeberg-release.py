#!/usr/bin/env python3
import argparse
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request

CODEBERG_API = "https://codeberg.org/api/v1"


def api(method, url, token=None, data=None, headers=None):
    req = urllib.request.Request(url, data=data, method=method)
    if token:
        req.add_header("Authorization", f"token {token}")
    req.add_header("Accept", "application/json")
    if headers:
        for k, v in headers.items():
            req.add_header(k, v)
    try:
        with urllib.request.urlopen(req) as resp:
            return resp.status, resp.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()
    except Exception as e:
        print(f"ERROR request {method} {url}: {e}", file=sys.stderr)
        raise


def fetch_github_body(github_repo, tag):
    token = os.environ.get("GITHUB_TOKEN", "")
    headers = {"Accept": "application/vnd.github+json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    url = f"https://api.github.com/repos/{github_repo}/releases/tags/{urllib.parse.quote(tag)}"
    try:
        status, body = api("GET", url, None, headers=headers)
        if status == 200:
            data = json.loads(body)
            return data.get("body") or f"Release {tag}"
        print(f"WARN: GitHub release body fetch returned {status}, using fallback", file=sys.stderr)
    except Exception as e:
        print(f"WARN: could not fetch GitHub body: {e}", file=sys.stderr)
    return f"Release {tag}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", required=True)
    ap.add_argument("--owner", default="luqenov")
    ap.add_argument("--repo", default="xournalpp-discord-rpc")
    ap.add_argument("--github-repo", default="luqenov/xournalpp-discord-rpc")
    ap.add_argument("--files", nargs="+", required=True)
    ap.add_argument("--body-file")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    files = args.files
    missing = [f for f in files if not os.path.isfile(f)]
    if missing:
        print(f"ERROR: missing files: {missing}", file=sys.stderr)
        return 2

    body = ""
    if args.body_file:
        with open(args.body_file) as f:
            body = f.read()
    else:
        body = fetch_github_body(args.github_repo, args.tag)

    print(f"Tag: {args.tag}")
    print(f"Codeberg repo: {args.owner}/{args.repo}")
    print(f"Body ({len(body)} chars):")
    print(body)
    print(f"Files: {files}")

    if args.dry_run:
        print("DRY RUN: would (re)create Codeberg release and upload assets. No API writes performed.")
        return 0

    token = os.environ.get("CODEBERG_TOKEN", "").strip()
    if not token:
        print("ERROR: CODEBERG_TOKEN env is required. "
              "Create one at https://codeberg.org/user/settings/applications "
              "and add it as GitHub secret CODEBERG_TOKEN.", file=sys.stderr)
        return 1

    base = f"{CODEBERG_API}/repos/{args.owner}/{args.repo}"

    status, raw = api("GET", f"{base}/releases/tags/{urllib.parse.quote(args.tag)}", token)
    if status == 200:
        existing = json.loads(raw)
        rid = existing["id"]
        print(f"Found existing Codeberg release id={rid}, deleting (tag will be kept)")
        status, raw = api("DELETE", f"{base}/releases/{rid}", token)
        if status not in (200, 204):
            print(f"ERROR delete failed {status}: {raw[:500]}", file=sys.stderr)
            return 1
        print("Deleted.")
    elif status == 404:
        print("No existing Codeberg release for this tag.")
    else:
        print(f"ERROR get release {status}: {raw[:500]}", file=sys.stderr)
        return 1

    payload = json.dumps({
        "tag_name": args.tag,
        "name": args.tag,
        "body": body,
        "draft": False,
        "prerelease": False,
    }).encode()
    status, raw = api("POST", f"{base}/releases", token, data=payload,
                      headers={"Content-Type": "application/json"})
    if status not in (200, 201):
        print(f"ERROR create release {status}: {raw[:1000]}", file=sys.stderr)
        return 1
    created = json.loads(raw)
    rid = created["id"]
    print(f"Created Codeberg release id={rid} url={created.get('html_url','')}")

    for f in files:
        name = os.path.basename(f)
        qname = urllib.parse.quote(name)
        with open(f, "rb") as fh:
            data = fh.read()
        url = f"{base}/releases/{rid}/assets?name={qname}"
        status, raw = api("POST", url, token, data=data,
                          headers={"Content-Type": "application/octet-stream"})
        if status not in (200, 201):
            print(f"ERROR upload {name} {status}: {raw[:1000]}", file=sys.stderr)
            return 1
        print(f"Uploaded {name} ({len(data)} bytes)")

    print(f"OK: Codeberg release {args.tag} synced with {len(files)} assets.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
