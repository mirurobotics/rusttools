#!/bin/sh
# LINT_FIX: 1 = auto-fix (default), 0 = check-only (CI)
set -e
REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

LINT_FIX="${LINT_FIX:-1}"

echo "Custom Linter"
echo "-------------"
if [ "$LINT_FIX" = "1" ]; then
    cargo run -- --path "$REPO_ROOT/src" --fix --config "$REPO_ROOT/.lint-imports.toml"
else
    cargo run -- --path "$REPO_ROOT/src" --config "$REPO_ROOT/.lint-imports.toml"
fi
echo ""

echo "Cargo fmt"
echo "---------"
if [ "$LINT_FIX" = "1" ]; then
    cargo fmt
else
    cargo fmt -- --check
fi
echo ""

echo "Unused external dependencies"
echo "----------------------------"
cargo machete
echo ""

echo "Security vulnerabilities"
echo "------------------------"
cargo audit
echo ""

echo "Clippy"
echo "------"
if [ "$LINT_FIX" = "1" ]; then
    cargo clippy --fix --allow-dirty --all-targets -- -D warnings
fi
cargo clippy --no-deps --all-targets -- -D warnings
echo ""

echo "Lint complete"
echo ""
