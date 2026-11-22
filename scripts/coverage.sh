#!/bin/sh
set -e
REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
fi

echo "Generating HTML coverage report..."
cargo llvm-cov --html --output-dir target/coverage

echo ""
echo "Report: target/coverage/html/index.html"
