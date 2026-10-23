#!/usr/bin/env python3
"""Build the macOS GUI + launchd installer from an existing release build."""

import argparse
import plistlib
import shutil
import subprocess
import tempfile
from pathlib import Path
from xml.etree import ElementTree as ET

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
HELPERS = Path("Library/PrivilegedHelperTools")
SERVICE = "io.github.batterybridge.service"
CLIENT = "io.github.batterybridge.cli"
LAUNCHER = "io.github.batterybridge.launcher"


def stage_payload(destination, build):
    app = build / "bundle/macos/BatteryBridge.app"
    for source in (app, build / "batterybridge", build / "batterybridge-service"):
        if not source.exists():
            raise FileNotFoundError(f"Build the release binaries first: {source}")
    shutil.copytree(app, destination / "Applications/BatteryBridge.app", symlinks=True)
    helpers = destination / HELPERS
    helpers.mkdir(parents=True)
    for source, name in (("batterybridge", CLIENT), ("batterybridge-service", SERVICE)):
        shutil.copy2(build / source, helpers / name)
        (helpers / name).chmod(0o755)
    (helpers / LAUNCHER).write_text(
        "#!/bin/sh\nset -eu\n"
        "/usr/bin/install -d -o root -g admin -m 0750 /var/run/batterybridge\n"
        f"exec /{HELPERS / SERVICE}\n"
    )
    (helpers / LAUNCHER).chmod(0o755)
    with (REPO / "packaging/launchd/io.github.batterybridge.plist").open("rb") as stream:
        daemon = plistlib.load(stream)
    daemon["ProgramArguments"] = [f"/{HELPERS / LAUNCHER}"]
    target = destination / "Library/LaunchDaemons/io.github.batterybridge.plist"
    target.parent.mkdir(parents=True)
    with target.open("wb") as stream:
        plistlib.dump(daemon, stream)
    target.chmod(0o644)


def distribution(version):
    root = ET.Element("installer-gui-script", {"minSpecVersion": "2"})
    ET.SubElement(root, "title").text = "BatteryBridge Developer Preview"
    ET.SubElement(root, "welcome", {"file": "Welcome.html"})
    ET.SubElement(root, "conclusion", {"file": "Conclusion.html"})
    ET.SubElement(root, "options", {"customize": "never", "hostArchitectures": "arm64",
                                    "require-scripts": "true"})
    ET.SubElement(root, "domains", {"enable_localSystem": "true",
                                   "enable_currentUserHome": "false",
                                   "enable_anywhere": "false"})
    allowed = ET.SubElement(root, "allowed-os-versions")
    ET.SubElement(allowed, "os-version", {"min": "11.0"})
    outline = ET.SubElement(root, "choices-outline")
    ET.SubElement(outline, "line", {"choice": "batterybridge"})
    choice = ET.SubElement(root, "choice", {"id": "batterybridge", "visible": "false",
                                           "title": "BatteryBridge"})
    ET.SubElement(choice, "pkg-ref", {"id": "io.github.batterybridge.installer"})
    ET.SubElement(root, "pkg-ref", {"id": "io.github.batterybridge.installer",
                                    "version": version, "onConclusion": "None"}).text = "component.pkg"
    return ET.tostring(root, encoding="unicode")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-dir", type=Path, default=REPO / "target/release")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--version", default="0.1.0.1", help="Numeric Installer receipt version")
    args = parser.parse_args()
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="batterybridge-pkg-") as temporary:
        work = Path(temporary)
        payload = work / "payload"
        stage_payload(payload, args.build_dir.resolve())
        for binary in (payload / HELPERS / CLIENT, payload / HELPERS / SERVICE,
                       payload / "Applications/BatteryBridge.app/Contents/MacOS/batterybridge-gui"):
            subprocess.run(["lipo", str(binary), "-verify_arch", "arm64"], check=True)
        # Do not let Installer relocate this app to a developer's existing build.
        components = work / "components.plist"
        subprocess.run(["pkgbuild", "--analyze", "--root", str(payload), str(components)], check=True)
        with components.open("rb") as stream:
            bundles = plistlib.load(stream)
        for bundle in bundles:
            bundle["BundleIsRelocatable"] = False
            bundle["BundleOverwriteAction"] = "upgrade"
        with components.open("wb") as stream:
            plistlib.dump(bundles, stream)
        subprocess.run(["pkgbuild", "--root", str(payload), "--identifier",
                        "io.github.batterybridge.installer", "--version", args.version,
                        "--install-location", "/", "--ownership", "recommended",
                        "--component-plist", str(components),
                        "--scripts", str(HERE / "scripts"), str(work / "component.pkg")], check=True)
        (work / "Distribution.xml").write_text(distribution(args.version))
        subprocess.run(["productbuild", "--distribution", str(work / "Distribution.xml"),
                        "--package-path", str(work), "--resources", str(HERE / "resources"),
                        str(output)], check=True)
    print(output)


if __name__ == "__main__":
    main()
