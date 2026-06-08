#!/usr/bin/env python3
"""Check for Android SDK version updates and update flake.nix."""

import os
import re
import sys
import urllib.request
import xml.etree.ElementTree as ET

FLAKE_PATH = "flake.nix"
REPO_URL = "https://dl.google.com/android/repository/repository2-3.xml"


def strip_ns(tag: str) -> str:
    return tag.split("}", 1)[-1] if "}" in tag else tag


def fetch_xml(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "curl/7.0"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.read()


def parse_repo(data: bytes):
    root = ET.fromstring(data)

    stable_id = "channel-0"
    for elem in root.iter():
        if strip_ns(elem.tag) == "channel":
            if elem.text and "stable" in elem.text.lower():
                stable_id = elem.get("id", "channel-0")
                break

    build_tools, ndks, platforms = [], [], []

    for pkg in root.iter():
        if strip_ns(pkg.tag) != "remotePackage":
            continue

        path = pkg.get("path", "")
        channel_ref = None
        is_obsolete = False

        for child in pkg:
            tag = strip_ns(child.tag)
            if tag == "channelRef":
                channel_ref = child.get("ref")
            elif tag == "obsolete":
                is_obsolete = True

        if is_obsolete or channel_ref != stable_id:
            continue

        if path.startswith("build-tools;"):
            v = path.split(";", 1)[1]
            if all(part.isdigit() for part in v.split(".")):
                build_tools.append(v)
        elif path.startswith("ndk;"):
            v = path.split(";", 1)[1]
            if all(part.isdigit() for part in v.split(".")):
                ndks.append(v)
        elif re.match(r"^platforms;android-\d+$", path):
            platforms.append(int(path.split("android-")[1]))

    return build_tools, ndks, platforms


def version_key(v: str):
    return tuple(int(x) for x in v.split("."))


def set_github_output(name: str, value: str) -> None:
    github_output = os.environ.get("GITHUB_OUTPUT")
    if github_output:
        delimiter = "GHADELIMITER"
        with open(github_output, "a") as f:
            f.write(f"{name}<<{delimiter}\n{value}\n{delimiter}\n")
    else:
        print(f"[output] {name}={value!r}")


def main() -> int:
    print("Fetching Android SDK repository manifest...")
    data = fetch_xml(REPO_URL)
    build_tools, ndks, platforms = parse_repo(data)

    if not build_tools or not ndks or not platforms:
        print("ERROR: failed to parse versions from repository XML", file=sys.stderr)
        return 1

    latest_bt = max(build_tools, key=version_key)
    latest_ndk = max(ndks, key=version_key)
    latest_plat = str(max(platforms))

    print(f"Latest build-tools : {latest_bt}")
    print(f"Latest NDK         : {latest_ndk}")
    print(f"Latest platform    : android-{latest_plat}")

    with open(FLAKE_PATH) as f:
        flake = f.read()

    m_bt = re.search(r'buildToolsVersion\s*=\s*"([^"]+)"', flake)
    m_ndk = re.search(r'ndkVersion\s*=\s*"([^"]+)"', flake)
    m_plat = re.search(r'platformVersions\s*=\s*\[\s*"([^"]+)"\s*\]', flake)

    if not m_bt or not m_ndk or not m_plat:
        print("ERROR: could not parse current versions from flake.nix", file=sys.stderr)
        return 1

    cur_bt, cur_ndk, cur_plat = m_bt.group(1), m_ndk.group(1), m_plat.group(1)

    print(f"\nCurrent build-tools : {cur_bt}")
    print(f"Current NDK         : {cur_ndk}")
    print(f"Current platform    : android-{cur_plat}")

    changes = []
    if cur_bt != latest_bt:
        changes.append(f"- build-tools: `{cur_bt}` → `{latest_bt}`")
    if cur_ndk != latest_ndk:
        changes.append(f"- NDK: `{cur_ndk}` → `{latest_ndk}`")
    if cur_plat != latest_plat:
        changes.append(f"- platform: `android-{cur_plat}` → `android-{latest_plat}`")

    if not changes:
        print("\nAll Android SDK versions are up to date.")
        set_github_output("updated", "false")
        return 0

    print("\nUpdates found:\n" + "\n".join(changes))

    updated = flake
    updated = re.sub(
        r'(buildToolsVersion\s*=\s*")[^"]+(")',
        rf'\g<1>{latest_bt}\2',
        updated,
    )
    updated = re.sub(
        r'(ndkVersion\s*=\s*")[^"]+(")',
        rf'\g<1>{latest_ndk}\2',
        updated,
    )
    updated = re.sub(
        r'(platformVersions\s*=\s*\[)\s*"[^"]+"\s*(\])',
        rf'\g<1> "{latest_plat}" \2',
        updated,
    )

    with open(FLAKE_PATH, "w") as f:
        f.write(updated)

    print("flake.nix updated.")

    pr_body = (
        "Automated update of Android SDK versions in `flake.nix`.\n\n"
        "## Changes\n\n" + "\n".join(changes)
    )
    set_github_output("updated", "true")
    set_github_output("pr_body", pr_body)
    return 0


if __name__ == "__main__":
    sys.exit(main())
