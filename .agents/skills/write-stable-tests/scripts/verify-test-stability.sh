#!/usr/bin/env bash
set -euo pipefail

# 闸门只剩 Rust 一条通道（Swift / TypeScript 规则随旧前端删除），所以这里不再传
# `--platform`；其余参数（`--all` / `--base` / `--head`）原样透传给 .mjs。
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
exec node "$SCRIPT_DIR/verify-test-stability.mjs" "$@"
