#!/usr/bin/env bash
set -euo pipefail

if (( $# != 2 )); then
    printf 'Usage: %s <base-revision> <head-revision>\n' "$0" >&2
    exit 2
fi

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
BASE_REVISION="$1"
HEAD_REVISION="$2"
cd "$ROOT_DIR"

git cat-file -e "${BASE_REVISION}^{commit}"
git cat-file -e "${HEAD_REVISION}^{commit}"

rust_diff_is_comment_only() {
    local path="$1"
    local in_hunk=false
    local saw_change=false
    local line content

    while IFS= read -r line; do
        if [[ "$line" == @@* ]]; then
            in_hunk=true
            continue
        fi
        if [[ "$in_hunk" == true && ( "$line" == +* || "$line" == -* ) ]]; then
            saw_change=true
            content="${line:1}"
            if [[ ! "$content" =~ ^[[:space:]]*(//.*)?$ ]]; then
                return 1
            fi
        fi
    done < <(git diff --no-color --unified=0 "$BASE_REVISION" "$HEAD_REVISION" -- "$path")

    [[ "$saw_change" == true ]]
}

# 旧前端（`macos/`、`windows/`）删除后，剩下的验证 lane 只有 Rust Core、数据库 crate
# 与 GPUI 宿主。`swift` / `plugins` / `swift_database` / `macos_release` / `windows` /
# `windows_rust` / `java_jdt` / `git_validation` 及其模式已随对应工作流一并移除：
# 留着它们会让 `*)` fallback 在删目录的改动上点亮不存在的 lane。
rust_core=false
rust_database=false
gpui=false
rust_comments=false
metadata=false

enable_all_validation() {
    rust_core=true
    rust_database=true
    gpui=true
}

while IFS=$'\t' read -r status first_path _; do
    if [[ "$status" == R* || "$status" == C* ]]; then
        # Renames and copies can cross ownership boundaries, so classify them
        # conservatively instead of trusting only the destination extension.
        enable_all_validation
        break
    fi

    path="$first_path"
    lowercase_path="${path,,}"

    case "$lowercase_path" in
        *.md|*.mdx)
            # Text-only documentation does not reserve a runner.
            ;;
        casks/*|scripts/update-homebrew-cask.rb)
            metadata=true
            ;;
        scripts/classify-ci-changes.sh|scripts/test-classify-ci-changes.sh)
            # A classifier change must exercise every lane that it can select.
            enable_all_validation
            ;;
        .github/workflows/ci-database.yml)
            rust_database=true
            ;;
        .github/workflows/ci-rust.yml)
            rust_core=true
            gpui=true
            ;;
        .github/*|docs/*|.agents/*|.idea/*|.gitignore|license)
            ;;
        rust/lithe-core/src/*.rs)
            # Comment-only Rust Core edits use the documentation verifier but
            # skip unit tests.
            if [[ "$status" == M* ]] && rust_diff_is_comment_only "$path"; then
                rust_comments=true
            else
                rust_core=true
            fi
            ;;
        rust/lithe-core/tests/*|rust/lithe-core/cargo.toml|rust/lithe-core/include/*)
            rust_core=true
            ;;
        rust/lithe-gpui/crates/*/src/*|rust/lithe-gpui/crates/*/build.rs|rust/lithe-gpui/crates/*/cargo.toml)
            # The gpui host drives Rust Core through the command envelope, so a
            # shell change is validated together with the core contract.
            #
            # ⚠️ These cases must stay **above** the `rust/*` catch-all below:
            # `rust/lithe-gpui/**` would otherwise match that first and only turn on
            # the Core/database lanes, silently dropping the gpui lane.
            gpui=true
            rust_core=true
            ;;
        rust/lithe-gpui/tools/*)
            # Locale and icon generators are reproducible-artifact checks.
            gpui=true
            ;;
        rust/lithe-gpui/assets/*|rust/lithe-gpui/themes/*|rust/lithe-gpui/crates/*/locales/*|rust/lithe-gpui/crates/*/src/icons/*)
            gpui=true
            ;;
        rust/lithe-gpui/*)
            gpui=true
            ;;
        rust/cargo.toml|rust/cargo.lock)
            # One lock file now covers the Core crates and the gpui host, so a
            # resolution change can affect either side.
            rust_core=true
            rust_database=true
            gpui=true
            ;;
        rust/lithe-db-mcp/*|rust/lithe-db-sidecar/*)
            rust_database=true
            ;;
        rust/*)
            # Unknown Rust workspace paths fail closed across all Rust-backed
            # crates because workspace configuration can affect every crate.
            rust_core=true
            rust_database=true
            ;;
        shared/*)
            # Shared contracts and fixtures are compatibility surfaces consumed by
            # Rust Core and by the gpui host.
            rust_core=true
            gpui=true
            ;;
        scripts/verify-rust-core.sh|scripts/verify-rust-core-comments.sh|scripts/verify-rust-core-layout.sh|scripts/verify-shared-contracts.sh|scripts/verify-java-semantic-ownership.mjs|scripts/test-rust-core-comments.mjs|scripts/create-git-graph-fixture.sh)
            rust_core=true
            gpui=true
            ;;
        scripts/build-database-mcp.sh|scripts/build-database-sidecar.sh|scripts/database-sidecar-smoke.sh|scripts/database-validation-smoke.sh)
            rust_database=true
            ;;
        scripts/verify-test-stability.sh|scripts/verify-agent-notes.sh)
            enable_all_validation
            ;;
        scripts/verify-download-cache.mjs|scripts/test-verify-download-cache.mjs)
            enable_all_validation
            ;;
        scripts/prepare-lithe-pr-review.mjs|scripts/test-prepare-lithe-pr-review.mjs|scripts/update-repo-charts.py)
            ;;
        infra/docker/*)
            rust_database=true
            ;;
        *)
            # New or unclassified repository areas are validated conservatively
            # until their ownership is made explicit above.
            enable_all_validation
            ;;
    esac
done < <(git diff --name-status --find-renames "$BASE_REVISION" "$HEAD_REVISION")

printf 'rust_core=%s\n' "$rust_core"
printf 'rust_database=%s\n' "$rust_database"
printf 'gpui=%s\n' "$gpui"
printf 'rust_comments=%s\n' "$rust_comments"
printf 'metadata=%s\n' "$metadata"
