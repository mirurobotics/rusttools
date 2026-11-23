#!/bin/sh
set -e
REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

cargo update --verbose
echo ""
