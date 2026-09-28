#!/usr/bin/env bash
set -euo pipefail

# Rust Core 的确定性校验：注释规范、模块布局、格式、单测，以及 gpui 宿主侧"能真正驱动 Core"的证据。
#
# 旧前端（`macos/` 的 C 桥 + `swift build` 链接、`windows/` 的 Tauri host）删除后，这里不再验证
# "Core 的 C ABI 符号出现在 macOS 二进制里"——那条链路已随 `macos/Sources/LitheRustCore/bridge.c`
# 一起消失。gpui 宿主**直接链接 crate**（`gpui/crates/*/Cargo.toml` 的 `lithe-core = { path = ... }`），
# 所以对应的证据是"gpui 侧对 Core 的调用能编译并通过其单测"，见末尾的 `cargo test -p lithe-gpui-*`。

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

mkdir -p .artifacts/test-stability
node --test --test-reporter=spec --test-reporter-destination=stdout \
    --test-reporter=junit --test-reporter-destination=.artifacts/test-stability/rust-comment-checker.xml \
    scripts/test-rust-core-comments.mjs
scripts/verify-rust-core-comments.sh
scripts/verify-rust-core-layout.sh
cargo fmt --manifest-path rust/Cargo.toml -p lithe-core -- --check
node .agents/skills/write-stable-tests/scripts/run-rust-tests-with-timing.mjs \
    --manifest rust/Cargo.toml --package lithe-git-host \
    --suite-timeout-ms 120000 \
    --report .artifacts/test-stability/git-host-rust.json
cargo test --manifest-path rust/Cargo.toml -p lithe-core

# gpui 宿主对 Core 的调用面：Core 契约一变，这些 crate 就编不过或行为漂移。
# 宿主在自己的 `gpui/` workspace（见 gpui/Cargo.toml），所以这里换 manifest 而不是加 -p。
cargo test --manifest-path gpui/Cargo.toml \
    -p lithe-gpui-shared -p lithe-gpui-app -p lithe-gpui-workbench

printf '%s\n' "Rust Core verification passed: comments, layout, formatting, core tests, and gpui host integration"
