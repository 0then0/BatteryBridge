#!/bin/sh
set -eu
systemctl daemon-reload
# Saved policy is deliberately retained by the package manager for reinstallation.
