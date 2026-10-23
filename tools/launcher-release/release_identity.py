#!/usr/bin/env python3
"""Choose stable launcher day slots and refuse rolling-feed regressions (D-LX15)."""
import datetime as dt
import json
import os
from pathlib import Path
import re
import sys


def stable(value):
    match = re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", value)
    if not match:
        raise ValueError(f"expected stable numeric SemVer: {value!r}")
    version = tuple(map(int, match.groups()))
    if max(version) > 65535:
        raise ValueError("version exceeds Windows VERSIONINFO 16-bit components")
    return version


def choose(pages, epoch, sha, run_url, rolling_version=None):
    releases = [release for page in pages for release in page]
    day = dt.datetime.fromtimestamp(epoch, dt.timezone.utc)
    prefix = f"launcher-{day:%Y%m%d}-"
    tag = prefix + sha[:7]
    dated = []
    for release in releases:
        name = release["tag_name"]
        if not name.startswith("launcher-") or name == "launcher-current":
            continue
        if not re.fullmatch(r"launcher-[0-9]{8}-[0-9a-f]{7}", name):
            raise ValueError(f"malformed launcher tag: {name}")
        if not re.search(r"Stargate Worlds launcher [0-9]+\.[0-9]+\.[0-9]+(?:-[0-9]+)?,", release.get("body") or ""):
            print(f"warning: {name} has no compiled version evidence in release notes", file=sys.stderr)
        if name.startswith(prefix):
            dated.append(release)
    dated.sort(key=lambda release: (release["created_at"], release["tag_name"]))
    existing = next((release for release in dated if release["tag_name"] == tag), None)
    if existing:
        body = existing.get("body") or ""
        if not re.search(re.escape(run_url) + r"(?:[.\s]|$)", body):
            raise ValueError("this commit was already released by another run")
        match = re.search(r"Stargate Worlds launcher ([0-9]+\.[0-9]+\.[0-9]+),", body)
        if not match:
            raise ValueError("published release has no reusable stable identity")
        version = match[1]
        stable(version)
    else:
        # Count legacy releases too; slots must never be reused during migration.
        slots = [len(dated)]
        for release in dated:
            match = re.search(r"Stargate Worlds launcher ([0-9]+\.[0-9]+\.[0-9]+),", release.get("body") or "")
            if match:
                year, month, patch = stable(match[1])
                if (year, month) == (day.year, day.month) and patch // 100 == day.day:
                    slots.append(patch % 100)
        if rolling_version:
            year, month, patch = stable(rolling_version)
            if (year, month) == (day.year, day.month) and patch // 100 == day.day:
                slots.append(patch % 100)
        ordinal = max(slots) + 1
        if ordinal > 99:
            raise ValueError("D-LX15 supports at most 99 releases per UTC day")
        version = f"{day.year}.{day.month}.{day.day * 100 + ordinal}"
        candidate = stable(version)
        if rolling_version and candidate <= stable(rolling_version):
            raise ValueError("candidate is not above launcher-current")
        for release in releases:
            match = re.search(r"Stargate Worlds launcher ([0-9]+\.[0-9]+\.[0-9]+)(?:-[0-9]+)?,", release.get("body") or "")
            if match and candidate <= stable(match[1]):
                raise ValueError("candidate is not above a published launcher version")
    return dict(tag=tag, epoch=str(epoch), version=version, run_url=run_url)


def main():
    if sys.argv[1] == "current":
        candidate = json.loads(Path(sys.argv[2]).read_text())["version"]
        previous = json.loads(Path(sys.argv[3]).read_text())["version"]
        if stable(candidate) < stable(previous):
            raise ValueError("refusing to regress launcher-current")
        return
    pages = json.load(sys.stdin)
    epoch = int(os.environ["STAMP_EPOCH"])
    run_url = f"{os.environ['GITHUB_SERVER_URL']}/{os.environ['GITHUB_REPOSITORY']}/actions/runs/{os.environ['GITHUB_RUN_ID']}"
    reservation = Path("stamp-reservation/identity.json")
    if reservation.exists():
        saved = json.loads(reservation.read_text())
        if saved["run_url"] != run_url or saved["tag"][-7:] != os.environ["GITHUB_SHA"][:7]:
            raise ValueError("reserved release identity belongs to another run or commit")
        stable(saved["version"])
        identity = saved
    else:
        rolling = Path("stamp-current/latest.json")
        rolling_version = json.loads(rolling.read_text())["version"] if rolling.exists() else None
        identity = choose(pages, epoch, os.environ["GITHUB_SHA"], run_url, rolling_version)
    reservation.parent.mkdir(exist_ok=True)
    reservation.write_text(json.dumps(identity) + "\n")
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        for key in ("tag", "epoch", "version"):
            output.write(f"{key}={identity[key]}\n")
    print(f"{identity['tag']}: {identity['version']}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError) as error:
        sys.exit(f"release identity refused: {error}")
