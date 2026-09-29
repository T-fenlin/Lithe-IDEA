#!/usr/bin/env node
/**
 * test-classify-ci-changes.mjs —— `scripts/classify-ci-changes.sh` 的可执行用例集。
 *
 * 旧前端（`macos/`、`windows/`）删除后，分类器只剩 `rust_core` / `rust_database` / `gpui`
 * 三条 lane，外加 `rust_comments` 与 `metadata` 两个辅助输出。本文件只覆盖这几条。
 *
 * 特别保护一条历史回归：删目录那类改动（`D` 状态）过去会落到 `*)` fallback 上，而旧
 * `macos/` 模式有覆盖空洞（`macos/EditorFrontend/**`、`macos/Experiments/**` 不匹配任何
 * 模式），于是 fallback 触发并点亮**全部** lane。现在 `macos/*` / `windows/*` 同样走
 * `*)`，但只会点亮这三条真实 lane，且不会点亮 `rust_comments` / `metadata`。
 * `wholeDirectoryDeletion` 用例把这条行为钉住。
 *
 * 为什么是 Node 而不是 `.sh`：本仓库的 CI 跑在 Linux，但**开发机常常是 Windows**，
 * 而 Windows 上 `bash` 解析到的是 WSL 的 `bash.exe`——没有装 WSL 发行版时它会直接失败，
 * 于是这条最需要"改分类器时立刻验证"的用例集反而跑不起来。分类器本身是纯 shell，
 * 这里用 `bash` 子进程驱动它，用例编排与断言留在 Node（与 `scripts/*.mjs` 的既有做法一致）。
 *
 * 用法：node scripts/test-classify-ci-changes.mjs
 */

import { execFileSync } from "node:child_process";
import { appendFileSync, copyFileSync, existsSync, mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const CLASSIFIER = join(HERE, "classify-ci-changes.sh");

/**
 * 找 Git 自带的 bash。
 *
 * 不能直接用 `bash`：Windows 上它解析到 `C:\Windows\System32\bash.exe`（WSL 的转发器），
 * 没装 WSL 发行版时它对任何参数都直接失败，于是这套用例在 Windows 开发机上永远跑不起来。
 * Git Bash 随 Git 一起装，位置由 `git --exec-path` 推出来，不写死盘符。
 */
function resolveBash() {
  const execPath = execFileSync("git", ["--exec-path"], { encoding: "utf8" }).trim();
  // `<install>/mingw64/libexec/git-core` → `<install>/bin/bash.exe`
  const installRoot = resolve(execPath, "..", "..", "..");
  for (const candidate of [
    join(installRoot, "bin", "bash.exe"),
    join(installRoot, "usr", "bin", "bash.exe"),
    join(installRoot, "bin", "bash"),
  ]) {
    if (existsSync(candidate)) return candidate;
  }
  return "bash";
}

const BASH = resolveBash();

/** 在临时目录里跑 git / bash，用完即删。 */
const testRoot = mkdtempSync(join(tmpdir(), "lithe-classify-"));
/**
 * 分类器在临时仓库里的副本。
 *
 * **必须调用这个副本**：`classify-ci-changes.sh` 用 `BASH_SOURCE` 推出 `ROOT_DIR` 并 `cd` 过去，
 * 所以它永远针对"自己所在仓库"分类。直接调用仓库里的那份会把它指向真正的 Lithe 仓库，
 * 于是拿临时仓库的 revision 去问真仓库，得到 `Not a valid object name`。
 */
const classifierInTestRoot = join(testRoot, "scripts", "classify-ci-changes.sh");

function git(args, cwd = testRoot) {
  return execFileSync("git", args, { cwd, encoding: "utf8" });
}

function write(relativePath, lines) {
  const target = join(testRoot, relativePath);
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, `${lines.join("\n")}\n`, "utf8");
}

function classify(base, head) {
  return execFileSync(BASH, [classifierInTestRoot, base, head], {
    cwd: testRoot,
    encoding: "utf8",
  });
}

/** 期望输出。字段顺序必须与 `classify-ci-changes.sh` 末尾的 `printf` 顺序一致。 */
function classification({ rustCore, rustDatabase, gpui, rustComments, metadata }) {
  return [
    `rust_core=${rustCore}`,
    `rust_database=${rustDatabase}`,
    `gpui=${gpui}`,
    `rust_comments=${rustComments ?? "false"}`,
    `metadata=${metadata ?? "false"}`,
    "",
  ].join("\n");
}

let baseRevision;

function assertClassification(name, expected, mutate) {
  // 每次从基线重建，保证用例之间互不影响（bash 版用 `git reset --hard` + `git clean`）。
  git(["reset", "--hard", "-q", baseRevision]);
  git(["clean", "-fdq"]);
  mutate();
  git(["add", "-A"]);
  git(["commit", "-q", "-m", name]);

  const actual = classify(baseRevision, "HEAD");
  if (actual !== expected) {
    process.stderr.write(
      `Classifier case ${name} failed:\nExpected:\n${expected}\nActual:\n${actual}\n`,
    );
    process.exit(1);
  }
}

try {
  git(["init", "-q"]);
  git(["config", "user.email", "ci@example.invalid"]);
  git(["config", "user.name", "CI Test"]);

  write("rust/lithe-core/src/lib.rs", ["//! Test module.", "pub fn value() -> u8 { 1 }"]);
  write("rust/lithe-core/src/tests/mod.rs", ["#[test]", "fn source_test() { assert_eq!(1, 1); }"]);
  write("rust/lithe-core/tests/value.rs", ["#[test]", "fn value_is_one() { assert_eq!(1, 1); }"]);
  write("rust/lithe-db-sidecar/src/main.rs", ["fn main() {}"]);
  write("infra/docker/database-validation/compose.yaml", ["services: {}"]);
  write("rust/lithe-gpui/crates/app/src/main.rs", ["fn main() {}"]);
  write("rust/lithe-gpui/crates/editor/src/lib.rs", ["pub fn view() {}"]);
  write("rust/lithe-gpui/tools/extract-locale.mjs", ['console.log("check");']);
  write("rust/lithe-gpui/themes/lithe-dark.json", ["{}"]);
  // 旧前端目录进**基线**提交：`whole-directory-deletion` 用例要测的是"从有到删"这一个 diff。
  // 若在同一个用例里先提交再加再删，`git diff base..HEAD` 会互相抵消成空，测不到任何东西。
  write("windows/tauri/src/value.ts", ["export const value = 1;"]);
  write("macos/EditorFrontend/build.ts", ["export const value = 1;"]);
  write(".github/workflows/ci-rust.yml", ["name: Core"]);
  write("Casks/lithe.rb", ['cask "lithe" do', "end"]);
  write("README.md", ["# Test"]);
  mkdirSync(join(testRoot, "scripts"), { recursive: true });
  copyFileSync(CLASSIFIER, classifierInTestRoot);

  git(["add", "-A"]);
  git(["commit", "-q", "-m", "base"]);
  baseRevision = git(["rev-parse", "HEAD"]).trim();

  const cases = [
    ["readme", classification({ rustCore: "false", rustDatabase: "false", gpui: "false" }),
      () => write("README.md", ["# Updated"])],

    ["rust-comment", classification({ rustCore: "false", rustDatabase: "false", gpui: "false", rustComments: "true" }),
      () => write("rust/lithe-core/src/lib.rs", ["//! Updated test module.", "pub fn value() -> u8 { 1 }"])],

    ["rust-code", classification({ rustCore: "true", rustDatabase: "false", gpui: "false" }),
      () => write("rust/lithe-core/src/lib.rs", ["//! Test module.", "pub fn value() -> u8 { 2 }"])],

    ["rust-test", classification({ rustCore: "true", rustDatabase: "false", gpui: "false" }),
      () => write("rust/lithe-core/tests/value.rs", ["#[test]", "fn value_is_two() { assert_eq!(2, 2); }"])],

    ["rust-source-test", classification({ rustCore: "true", rustDatabase: "false", gpui: "false" }),
      () => write("rust/lithe-core/src/tests/mod.rs", ["#[test]", "fn source_test() { assert_eq!(2, 2); }"])],

    ["database-rust", classification({ rustCore: "false", rustDatabase: "true", gpui: "false" }),
      () => write("rust/lithe-db-sidecar/src/main.rs", ["fn main() { println!(\"updated\"); }"])],

    ["database-docker", classification({ rustCore: "false", rustDatabase: "true", gpui: "false" }),
      () => write("infra/docker/database-validation/compose.yaml", ["services:", "  mariadb: {}"])],

    // 合并成一个 workspace 后，rust/Cargo.toml 与 rust/Cargo.lock 同时覆盖 Core 与
    // gpui 宿主，解析策略一变两侧都可能受影响，因此三条 lane 一起点亮。
    ["rust-workspace", classification({ rustCore: "true", rustDatabase: "true", gpui: "true" }),
      () => write("rust/Cargo.toml", ["[workspace]"])],

    // gpui 宿主通过命令信封驱动 Rust Core，所以外壳源码改动同时验证两条 lane。
    ["gpui-source", classification({ rustCore: "true", rustDatabase: "false", gpui: "true" }),
      () => write("rust/lithe-gpui/crates/app/src/main.rs", ["fn main() { println!(\"updated\"); }"])],

    ["gpui-crate-source", classification({ rustCore: "true", rustDatabase: "false", gpui: "true" }),
      () => write("rust/lithe-gpui/crates/editor/src/lib.rs", ["pub fn view() { }"])],

    ["gpui-tool", classification({ rustCore: "false", rustDatabase: "false", gpui: "true" }),
      () => write("rust/lithe-gpui/tools/extract-locale.mjs", ['console.log("regenerated");'])],

    ["gpui-theme", classification({ rustCore: "false", rustDatabase: "false", gpui: "true" }),
      () => write("rust/lithe-gpui/themes/lithe-dark.json", ['{"background":"#000"}'])],

    // 回归：gpui 的目录整体移进 rust/ 之后，`rust/*` 这个 catch-all 会先于任何
    // `rust/lithe-gpui/**` 模式匹配，把宿主改动判成"未知 Rust 路径"而丢掉 gpui lane。
    // 上面的 gpui-source 用例守住顺序，这条守住兜底：gpui 下没有被专门模式覆盖的
    // 文件（这里是 crate 根的 PE 资源 lithe.rc）也必须仍然算 gpui 改动。
    // 用 .md 做这条会被更早的 `*.md` 空操作分支拦掉，那样测不到 catch-all 的顺序。
    ["gpui-unclassified-fallback", classification({ rustCore: "false", rustDatabase: "false", gpui: "true" }),
      () => write("rust/lithe-gpui/crates/app/lithe.rc", ["1 ICON"])],

    // shared 契约同时是 Rust Core 与 gpui 宿主的兼容面。
    ["shared-fixture", classification({ rustCore: "true", rustDatabase: "false", gpui: "true" }),
      () => write("shared/fixtures/core/test.json", ['{"operation":"updated"}'])],

    ["metadata", classification({ rustCore: "false", rustDatabase: "false", gpui: "false", metadata: "true" }),
      () => write("Casks/lithe.rb", ['cask "lithe" do', '  version "1.0.0"', "end"])],

    ["unknown-script", classification({ rustCore: "true", rustDatabase: "true", gpui: "true" }),
      () => write("scripts/unclassified-fixture.sh", ["#!/bin/zsh", "print -- unknown"])],

    // 只追加注释，分类器本体必须保持可执行：改写它会让被测对象变成一个空脚本。
    ["classifier", classification({ rustCore: "true", rustDatabase: "true", gpui: "true" }),
      () => appendFileSync(classifierInTestRoot, "# classifier test change\n", "utf8")],

    // 跨所有权边界的 rename：分类器必须保守地点亮全部 lane。
    ["rename-rust-to-markdown", classification({ rustCore: "true", rustDatabase: "true", gpui: "true" }),
      () => {
        // `git mv` 不会创建目标目录，跨目录 rename 时必须先建出来。
        mkdirSync(join(testRoot, "docs"), { recursive: true });
        git(["mv", "rust/lithe-core/src/lib.rs", "docs/lib.md"]);
      }],

    // 回归：删旧前端的目录过去会因 `macos/` 模式覆盖空洞落到 `*)` 并点亮全部 lane。
    // 现在 `macos/*` / `windows/*` 同样走 `*)`，但只点亮这三条真实 lane。
    ["whole-directory-deletion", classification({ rustCore: "true", rustDatabase: "true", gpui: "true" }),
      () => git(["rm", "-rq", "macos", "windows"])],
  ];

  for (const [name, expected, mutate] of cases) {
    assertClassification(name, expected, mutate);
  }

  process.stdout.write("CI change classifier tests passed\n");
} finally {
  rmSync(testRoot, { recursive: true, force: true });
}
