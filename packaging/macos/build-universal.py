#!/usr/bin/env python3
"""Build a universal GUI, CLI and daemon before assembling the macOS package."""
from pathlib import Path
import subprocess

REPO = Path(__file__).resolve().parents[2]
TARGETS = ('aarch64-apple-darwin', 'x86_64-apple-darwin')


def main():
    for target in TARGETS:
        subprocess.run(['cargo', 'build', '--release', '--locked', '--target', target,
                        '-p', 'batterybridge-cli', '-p', 'batterybridge-service'], cwd=REPO, check=True)
    subprocess.run(['npm', '--prefix', 'apps/gui', 'run', 'tauri', '--', 'build',
                    '--target', 'universal-apple-darwin', '--features', 'desktop', '--bundles', 'app'],
                   cwd=REPO, check=True)
    output = REPO / 'target/universal-apple-darwin/release'
    for binary in ('batterybridge', 'batterybridge-service'):
        inputs = [str(REPO / 'target' / target / 'release' / binary) for target in TARGETS]
        subprocess.run(['lipo', '-create', *inputs, '-output', str(output / binary)], check=True)
        subprocess.run(['lipo', str(output / binary), '-verify_arch', 'arm64', 'x86_64'], check=True)


if __name__ == '__main__':
    main()
