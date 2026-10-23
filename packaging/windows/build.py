#!/usr/bin/env python3
"""Build the protected per-machine NSIS installer without a WebView toolchain."""
import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess
import sys
import urllib.request
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from app import BUILD, REPO, VERSION

TOOL_URL = 'https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.13/nsis-3.13.zip'
TOOL_SHA256 = 'ba63dffc4410ee89193e1cb5a41989991bd77c61068da17e3156d136b7b0b3d8'


def toolchain():
    # Use the same pinned compiler in the preparation test and full installer.
    root = REPO / 'target/tools/nsis-3.13'
    compiler = root / 'makensis.exe'
    if compiler.exists():
        return compiler
    root.mkdir(parents=True, exist_ok=True)
    archive = root / 'toolchain.zip'
    urllib.request.urlretrieve(TOOL_URL, archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != TOOL_SHA256:
        archive.unlink()
        raise RuntimeError('NSIS toolchain checksum mismatch')
    with zipfile.ZipFile(archive) as source:
        source.extractall(root.parent)
    archive.unlink()
    if not compiler.exists():
        raise FileNotFoundError(compiler)
    return compiler


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--build-dir', type=Path, default=REPO / 'target/release')
    parser.add_argument('--output', type=Path, default=REPO / 'target/release/bundle/nsis/BatteryBridge-windows-x86_64-setup.exe')
    parser.add_argument('--prepare-tools', action='store_true')
    args = parser.parse_args()
    compiler = toolchain()
    if args.prepare_tools:
        print(compiler)
        return
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    for name in ('batterybridge.exe', 'batterybridge-service.exe', 'batterybridge-gui.exe'):
        if not (args.build_dir / name).is_file():
            raise FileNotFoundError(args.build_dir / name)
    subprocess.run([str(compiler), '/V3', f'/DVERSION={VERSION}', f'/DBUILD_NUMBER={BUILD}',
                    f'/DBUILD_DIR={args.build_dir.resolve()}', f'/DOUTPUT={output}', 'installer.nsi'],
                   cwd=Path(__file__).resolve().parent, check=True)
    print(output)


if __name__ == '__main__':
    main()
