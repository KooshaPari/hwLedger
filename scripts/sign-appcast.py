#!/usr/bin/env python3
"""
Minimal Sparkle appcast generator — signs a DMG with the Ed25519 private key
and emits appcast.xml. Replaces Homebrew's `generate_appcast` (deprecated +
Gatekeeper-rejected on 2026 macOS).

Usage:
  sign-appcast.py <dmg_path> <version> <key_b64_path> <appcast_out_path> [download_base_url]
"""
import base64
import hashlib
import os
import sys
from datetime import datetime, timezone
from xml.sax.saxutils import escape

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey


def main():
    if len(sys.argv) < 5:
        print(__doc__, file=sys.stderr)
        sys.exit(2)
    dmg_path, version, key_path, appcast_out = sys.argv[1:5]
    download_base = sys.argv[5] if len(sys.argv) > 5 else (
        "https://github.com/KooshaPari/hwLedger/releases/download"
    )

    with open(key_path, "r") as f:
        priv_b64 = f.read().strip()
    priv_raw = base64.b64decode(priv_b64)
    sk = Ed25519PrivateKey.from_private_bytes(priv_raw)

    with open(dmg_path, "rb") as f:
        dmg_bytes = f.read()
    sig = base64.b64encode(sk.sign(dmg_bytes)).decode()
    size = len(dmg_bytes)
    sha256 = hashlib.sha256(dmg_bytes).hexdigest()

    dmg_name = os.path.basename(dmg_path)
    pub_date = datetime.now(timezone.utc).strftime("%a, %d %b %Y %H:%M:%S +0000")
    download_url = f"{download_base}/v{version}/{dmg_name}"

    xml = f"""<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0"
     xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle"
     xmlns:dc="http://purl.org/dc/elements/1.1/">
    <channel>
        <title>hwLedger</title>
        <link>https://kooshapari.github.io/hwLedger/appcast.xml</link>
        <description>hwLedger release feed</description>
        <language>en</language>
        <item>
            <title>Version {escape(version)}</title>
            <sparkle:version>{escape(version)}</sparkle:version>
            <sparkle:shortVersionString>{escape(version)}</sparkle:shortVersionString>
            <sparkle:minimumSystemVersion>14.0</sparkle:minimumSystemVersion>
            <pubDate>{pub_date}</pubDate>
            <description><![CDATA[See the GitHub Release notes for changes in v{escape(version)}.]]></description>
            <enclosure url="{escape(download_url)}"
                       sparkle:version="{escape(version)}"
                       sparkle:edSignature="{sig}"
                       length="{size}"
                       type="application/octet-stream" />
            <!-- sha256: {sha256} -->
        </item>
    </channel>
</rss>
"""
    os.makedirs(os.path.dirname(appcast_out) or ".", exist_ok=True)
    with open(appcast_out, "w") as f:
        f.write(xml)
    print(f"appcast written: {appcast_out}")
    print(f"  version:    {version}")
    print(f"  dmg:        {dmg_name}")
    print(f"  size:       {size} bytes")
    print(f"  sha256:     {sha256}")
    print(f"  signature:  {sig[:32]}...")


if __name__ == "__main__":
    main()
