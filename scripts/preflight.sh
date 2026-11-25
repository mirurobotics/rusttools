#!/bin/sh
set -e
REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

echo "Lint"
echo "===="
"$REPO_ROOT/scripts/lint.sh"

echo "Test"
echo "===="
cargo run --quiet -- covgate