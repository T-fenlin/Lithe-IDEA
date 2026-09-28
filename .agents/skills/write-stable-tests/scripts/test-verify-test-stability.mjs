#!/usr/bin/env node

// verify-test-stability.mjs 的用例集。
//
// 旧前端删除后，闸门只剩 Rust 一条通道，所以这里的扫描器用例全部改用 Rust 样本；
// Swift / Bun 计时运行器的用例（无界等待、runner 停摆、逐项预算）随对应运行器一并删除。
// 保留的 Rust 侧覆盖：扫描规则与例外注解、新增行筛选、编译失败归因、进程树终止、
// 逐项超时与预算。

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseAddedLines, scanFile } from "./verify-test-stability.mjs";
import { parseJUnitCases } from "./parse-junit-cases.mjs";
import { parseArguments as parseRustTimingArguments, run as runRustTestsWithTiming } from "./run-rust-tests-with-timing.mjs";
import { runProcess } from "./test-timing-lib.mjs";

// ── 扫描规则 ────────────────────────────────────────────────────────────────────

// 只有 `#[cfg(test)]` 区域里的写法才算违规；生产代码里的退避 sleep 是合理的。
const rustViolations = scanFile(
  "rust/lithe-core/src/example.rs",
  `use std::thread;
fn production_backoff() { thread::sleep(Duration::from_millis(1)); }
#[cfg(test)]
mod tests {
    #[test]
    fn blocks() {
        thread::sleep(Duration::from_millis(1));
    }
}
`,
);
assert.deepEqual(rustViolations.map((violation) => violation.rule), ["rust-real-sleep"]);
assert.equal(rustViolations[0].line, 7);

const receiveViolations = scanFile(
  "rust/lithe-db-gpui/crates/git/tests/observation.rs",
  `#[test]
fn waits() {
    let value = receiver.recv();
    assert!(value.is_ok());
}
`,
);
assert.deepEqual(receiveViolations.map((violation) => violation.rule), ["rust-unbounded-receive"]);

// 有限等待是正确做法，必须放行。
const boundedReceive = scanFile(
  "rust/lithe-db-gpui/crates/git/tests/observation.rs",
  `#[test]
fn waits() {
    let value = receiver.recv_timeout(Duration::from_secs(1));
    assert!(value.is_ok());
}
`,
);
assert.deepEqual(boundedReceive, []);

// `select!` 的结构判定：有 default 臂、有超时分支都必须放行，两种都没有才是违规。
const selectWithoutEscape = scanFile(
  "rust/lithe-db-gpui/crates/terminal/tests/session.rs",
  `#[test]
fn forwards() {
    select! {
        line = reader.next() => { handle(line); }
        status = child.wait() => { record(status); }
    }
}
`,
);
assert.deepEqual(
  selectWithoutEscape.map((violation) => violation.rule),
  ["rust-unbounded-select"],
);

const selectWithDefault = scanFile(
  "rust/lithe-db-gpui/crates/terminal/tests/session.rs",
  `#[test]
fn forwards() {
    select! {
        line = reader.next() => { handle(line); }
        default => { /* nothing buffered yet */ }
    }
}
`,
);
assert.deepEqual(selectWithDefault, []);

const selectWithTimeout = scanFile(
  "rust/lithe-db-gpui/crates/terminal/tests/session.rs",
  `#[test]
fn forwards() {
    select! {
        line = reader.next() => { handle(line); }
        _ = tokio::time::sleep(Duration::from_secs(1)) => { panic!("no output"); }
    }
}
`,
);
assert.deepEqual(selectWithTimeout, []);

// 例外注解：理由够长才放行，太短仍然拦。
const annotated = scanFile(
  "rust/lithe-db-gpui/crates/terminal/tests/session.rs",
  `#[test]
fn blocks_on_native_boundary() {
    // test-stability: allow(rust-unbounded-receive) reason: the native API only exposes a blocking call
    let value = receiver.recv();
    drop(value);
}
`,
);
assert.deepEqual(annotated, []);

const shortAnnotation = scanFile(
  "rust/lithe-db-gpui/crates/terminal/tests/session.rs",
  `#[test]
fn blocks_on_native_boundary() {
    // test-stability: allow(rust-unbounded-receive) reason: native
    let value = receiver.recv();
    drop(value);
}
`,
);
assert.match(shortAnnotation[0].message, /Exception reason is too short/);

// 非 Rust 文件不再被扫描（Swift / TypeScript 规则已随旧前端删除）。
assert.deepEqual(scanFile("macos/Tests/LitheTests/Example.swift", "gate.wait()\n"), []);
assert.deepEqual(scanFile("windows/tauri/src/example.test.ts", "setTimeout(r, 1);\n"), []);

// ── 新增行筛选 ──────────────────────────────────────────────────────────────────

const diff = `diff --git a/rust/lithe-db-gpui/crates/git/tests/observation.rs b/rust/lithe-db-gpui/crates/git/tests/observation.rs
--- a/rust/lithe-db-gpui/crates/git/tests/observation.rs
+++ b/rust/lithe-db-gpui/crates/git/tests/observation.rs
@@ -2,0 +3,2 @@
+let receiver = channel();
+let value = receiver.recv();
`;
const added = parseAddedLines(diff);
assert.deepEqual([...added.get("rust/lithe-db-gpui/crates/git/tests/observation.rs")], [3, 4]);

// 只看新增行：未改动的旧违规不报。
const selectedViolations = scanFile(
  "rust/lithe-db-gpui/crates/git/tests/observation.rs",
  "#[test]\nlet old = receiver.recv();\nlet other = receiver.recv();\n",
  new Set([3]),
);
assert.deepEqual(
  selectedViolations.map((violation) => violation.rule),
  ["rust-unbounded-receive"],
);
assert.equal(selectedViolations[0].line, 3);

// ── 报告解析 ────────────────────────────────────────────────────────────────────

assert.deepEqual(
  parseJUnitCases(
    '<testsuite><testcase classname="scheduler" name="fires &amp; clears" time="0.125"/><testcase name="skips" time="0"><skipped/></testcase></testsuite>',
  ),
  [
    { name: "scheduler / fires & clears", status: "passed", durationMs: 125 },
    { name: "skips", status: "skipped", durationMs: 0 },
  ],
);

// ── Rust 计时运行器 ──────────────────────────────────────────────────────────────

// 编译失败必须归因到「编译」这一步，而不是伪装成某个测试超时。
const rustCompileFailureRoot = mkdtempSync(
  path.join(os.tmpdir(), "lithe-test-stability-rust-compile-failure-"),
);

try {
  const reportPath = path.join(rustCompileFailureRoot, "rust-compile-failure.json");

  await assert.rejects(
    runRustTestsWithTiming(
      {
        manifest: path.join(rustCompileFailureRoot, "Cargo.toml"),
        package: null,
        warnMs: 50,
        maxMs: 200,
        buildTimeoutMs: 1000,
        suiteTimeoutMs: 2000,
        report: reportPath,
        keepGoing: false,
      },
      {
        runProcessImpl: async ({ onStdoutLine = () => {} }) => {
          onStdoutLine(
            JSON.stringify({
              reason: "compiler-message",
              message: {
                rendered: "error[E0425]: cannot find value `missing` in this scope\n",
              },
            }),
          );

          return {
            code: 101,
            signal: null,
            timedOut: false,
            terminationConfirmed: true,
            durationMs: 12,
            stdout: "",
            stderr: "",
          };
        },
      },
    ),
    /Cargo test compilation exited with code 101/,
  );

  const compileFailureReport = JSON.parse(readFileSync(reportPath, "utf8"));

  assert.deepEqual(
    compileFailureReport.tests.map(({ name, status }) => ({ name, status })),
    [{ name: "Cargo test compilation", status: "failed" }],
  );
  assert.match(compileFailureReport.tests[0].details, /E0425/);
  assert.match(readFileSync(reportPath.replace(/\.json$/, ".log"), "utf8"), /E0425/);
  assert.match(readFileSync(reportPath.replace(/\.json$/, ".junit.xml"), "utf8"), /failures="1"/);
  assert.ok(existsSync(reportPath.replace(/\.json$/, ".html")));
} finally {
  rmSync(rustCompileFailureRoot, { recursive: true, force: true });
}

// 超时必须杀掉整个进程树，而不只是直接子进程。
if (process.platform !== "win32") {
  let rootPID = null;
  let descendantPID = null;
  const descendantSource = "process.on('SIGTERM', () => {}); setInterval(() => {}, 1000);";
  const rootSource = `
    const { spawn } = require("node:child_process");
    const descendant = spawn(process.execPath, ["-e", ${JSON.stringify(descendantSource)}], {
      detached: true,
      stdio: "ignore",
    });
    console.log(descendant.pid);
    setInterval(() => {}, 1000);
  `;
  try {
    const result = await runProcess({
      command: process.execPath,
      args: ["-e", rootSource],
      timeoutMs: 100,
      terminationGraceMs: 100,
      forcedTerminationTimeoutMs: 1000,
      terminationPollIntervalMs: 10,
      onSpawn: ({ pid }) => {
        rootPID = pid;
      },
      onStdoutLine: (line) => {
        descendantPID = Number(line);
      },
    });
    assert.equal(result.timedOut, true);
    assert.equal(result.terminationConfirmed, true);
    assert.ok(Number.isInteger(descendantPID) && descendantPID > 0);
    assert.throws(() => process.kill(descendantPID, 0), { code: "ESRCH" });
  } finally {
    for (const pid of [descendantPID, rootPID]) {
      if (!Number.isInteger(pid) || pid <= 0) continue;
      try {
        process.kill(pid === rootPID ? -pid : pid, "SIGKILL");
      } catch {
        // The expected path already removed the process tree.
      }
    }
  }
}

// 端到端：挂住的测试必须在自己的截止时间被判为 timeout，并写全四种产物。
const fixtureRoot = mkdtempSync(path.join(os.tmpdir(), "lithe-test-stability-rust-"));
try {
  mkdirSync(path.join(fixtureRoot, "src"));
  writeFileSync(
    path.join(fixtureRoot, "Cargo.toml"),
    '[package]\nname = "timing-fixture"\nversion = "0.1.0"\nedition = "2021"\n',
  );
  writeFileSync(
    path.join(fixtureRoot, "src/lib.rs"),
    `#[cfg(test)]
mod tests {
    #[test]
    fn a_quick() { assert_eq!(2 + 2, 4); }

    #[test]
    fn z_hangs() { std::thread::sleep(std::time::Duration::from_secs(5)); }
}
`,
  );
  const reportPath = path.join(fixtureRoot, "timing.json");
  const runnerPath = path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    "run-rust-tests-with-timing.mjs",
  );
  const result = spawnSync(
    process.execPath,
    [
      runnerPath,
      "--manifest", path.join(fixtureRoot, "Cargo.toml"),
      "--warn-ms", "50",
      "--max-ms", "200",
      "--build-timeout-ms", "30000",
      "--suite-timeout-ms", "30000",
      "--report", reportPath,
    ],
    { encoding: "utf8", timeout: 60000 },
  );
  assert.notEqual(result.status, 0, "the Rust timing runner must reject a hanging test");
  assert.ok(
    existsSync(reportPath),
    `the Rust timing runner did not write a report\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`,
  );
  const timingReport = JSON.parse(readFileSync(reportPath, "utf8"));
  assert.deepEqual(
    timingReport.tests.map(({ name, status }) => ({ name, status })),
    [
      { name: "tests::a_quick", status: "passed" },
      { name: "tests::z_hangs", status: "timeout" },
    ],
  );
  const html = readFileSync(path.join(fixtureRoot, "timing.html"), "utf8");
  const junit = readFileSync(path.join(fixtureRoot, "timing.junit.xml"), "utf8");
  assert.match(html, /问题与性能优化队列/);
  assert.match(html, /tests::z_hangs/);
  assert.match(html, /over budget|timeout/);
  assert.match(junit, /errors="1"/);
  assert.match(junit, /tests::z_hangs/);
  assert.ok(existsSync(path.join(fixtureRoot, "index.html")));
} finally {
  rmSync(fixtureRoot, { recursive: true, force: true });
}

// 共享 suite 截止时间必须落到「当前正在跑的那个测试」上。
const deadlineFixtureRoot = mkdtempSync(path.join(os.tmpdir(), "lithe-test-stability-deadline-"));
try {
  const reportPath = path.join(deadlineFixtureRoot, "deadline.json");
  const manifestPath = path.join(deadlineFixtureRoot, "Cargo.toml");
  const executablePath = path.join(deadlineFixtureRoot, "fake-tests");
  let currentTime = 0;
  let invocation = 0;
  const runProcessImpl = async ({ args, onStdoutLine = () => {}, timeoutMs }) => {
    invocation += 1;
    if (invocation === 1) {
      assert.equal(timeoutMs, 1000);
      currentTime = 400;
      onStdoutLine(JSON.stringify({
        reason: "compiler-artifact",
        profile: { test: true },
        executable: executablePath,
        manifest_path: manifestPath,
        target: { name: "deadline-fixture" },
      }));
      return { code: 0, signal: null, timedOut: false, durationMs: 400, stdout: "", stderr: "" };
    }
    if (args[0] === "--list") {
      assert.equal(timeoutMs, 600);
      currentTime = 500;
      return {
        code: 0,
        signal: null,
        timedOut: false,
        durationMs: 100,
        stdout: "tests::first: test\ntests::second: test\n",
        stderr: "",
      };
    }
    if (args[1] === "tests::first") {
      assert.equal(timeoutMs, 500);
      currentTime = 800;
      return {
        code: 0,
        signal: null,
        timedOut: false,
        durationMs: 300,
        stdout: "running 1 test\ntest tests::first ... ok\n",
        stderr: "",
      };
    }
    assert.deepEqual(args.slice(0, 2), ["--exact", "tests::second"]);
    assert.equal(timeoutMs, 200);
    currentTime = 1000;
    return {
      code: null,
      signal: "SIGTERM",
      timedOut: true,
      durationMs: 200,
      stdout: "running 1 test\n",
      stderr: "",
    };
  };

  await assert.rejects(
    runRustTestsWithTiming(
      {
        manifest: manifestPath,
        package: null,
        warnMs: 50,
        maxMs: 500,
        testBudgets: [{ prefix: "tests::second", maxMs: 2000 }],
        buildTimeoutMs: 5000,
        suiteTimeoutMs: 1000,
        report: reportPath,
        keepGoing: false,
      },
      { runProcessImpl, now: () => currentTime },
    ),
    /Rust test suite exceeded 1000ms during test deadline-fixture::tests::second/,
  );
  const deadlineReport = JSON.parse(readFileSync(reportPath, "utf8"));
  assert.deepEqual(deadlineReport.suite, {
    timedOut: true,
    stage: "test deadline-fixture::tests::second",
    durationMs: 1000,
  });
  assert.deepEqual(
    deadlineReport.tests.map(({ name, status }) => ({ name, status })),
    [
      { name: "tests::first", status: "passed" },
      { name: "tests::second", status: "timeout" },
    ],
  );
  assert.match(deadlineReport.tests[1].details, /shared suite deadline expired/);
  assert.ok(existsSync(path.join(deadlineFixtureRoot, "deadline.html")));
  assert.ok(existsSync(path.join(deadlineFixtureRoot, "deadline.junit.xml")));
} finally {
  rmSync(deadlineFixtureRoot, { recursive: true, force: true });
}

// 逐项预算：超预算但通过的测试算 warning，挂住的测试在自己的预算上判 timeout。
const budgetFixtureRoot = mkdtempSync(path.join(os.tmpdir(), "lithe-test-stability-budget-"));
try {
  const report = path.join(budgetFixtureRoot, "budget.json");
  const options = parseRustTimingArguments([
    "--manifest", path.join(budgetFixtureRoot, "Cargo.toml"), "--report", report,
    "--warn-ms", "10", "--max-ms", "100", "--test-budget", "tests::native_=300",
    "--test-budget", "tests::native=250",
  ]);
  for (const budget of ["=300", "tests::native_=0", "tests::native_=5", "invalid"]) {
    assert.throws(() => parseRustTimingArguments(["--manifest", "Cargo.toml", "--test-budget", budget]));
  }
  let currentTime = 0;
  const runProcessImpl = async ({ args, onStdoutLine, timeoutMs }) => {
    let stdout = "";
    let durationMs = 0;
    let timedOut = false;
    if (onStdoutLine) {
      onStdoutLine(JSON.stringify({ reason: "compiler-artifact", profile: { test: true },
        executable: path.join(budgetFixtureRoot, "fake-tests"), manifest_path: options.manifest,
        target: { name: "budget-fixture" } }));
    } else if (args[0] === "--list") {
      stdout = "tests::unit: test\ntests::native_pass: test\ntests::native_hang: test\n";
    } else {
      assert.equal(timeoutMs, args[1] === "tests::unit" ? 100 : 300);
      timedOut = args[1] === "tests::native_hang";
      durationMs = args[1] === "tests::unit" ? 30 : timedOut ? 300 : 150;
      stdout = "running 1 test\n";
    }
    currentTime += durationMs;
    return { code: timedOut ? null : 0, timedOut, durationMs, stdout, stderr: "" };
  };
  await assert.rejects(runRustTestsWithTiming(options, { runProcessImpl, now: () => currentTime }), /1 Rust test/);
  const result = JSON.parse(readFileSync(report, "utf8"));
  assert.deepEqual(result.tests.map(({ status, maxMs }) => ({ status, maxMs })), [
    { status: "passed", maxMs: 100 }, { status: "passed", maxMs: 300 }, { status: "timeout", maxMs: 300 },
  ]);
  const junit = readFileSync(report.replace(".json", ".junit.xml"), "utf8");
  assert.match(junit, /failures="0"/);
  assert.match(junit, /errors="1"/);
  assert.doesNotMatch(junit, /Performance budget exceeded: 150ms/);
  assert.match(readFileSync(report.replace(".json", ".html"), "utf8"),
    /<tr data-status="warning" data-search="[^"\n]*tests::native_pass"/);
} finally {
  rmSync(budgetFixtureRoot, { recursive: true, force: true });
}

console.log("Test stability verifier tests passed.");
