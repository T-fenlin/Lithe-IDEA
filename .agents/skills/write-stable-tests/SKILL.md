---
name: write-stable-tests
description: Write and review deterministic, bounded Lithe tests for Rust. Use whenever creating, modifying, or reviewing test code or test infrastructure, especially concurrency, timers, polling, subprocess, watcher, lifecycle, or cancellation tests that could hang CI.
---

# Write Stable Tests

Apply this Skill after `develop-lithe`. Its purpose is to make a broken test
fail locally with a useful diagnostic instead of waiting for a CI job timeout.

## Scope

Lithe is a pure Rust repository: `rust/lithe-core` (deterministic commands and
contracts) plus the GPUI Kit host in `gpui/`. The Swift macOS product, the
React/Tauri Windows product, and the shared Monaco editor package were removed;
there are no platform test lanes left to keep green.

Two test surfaces remain, and they have different constraints:

- **Rust Core** (`rust/lithe-core`): deterministic, no UI, no long-lived
  resources. Tests are plain `cargo test`.
- **GPUI host** (`gpui/crates/*`): owns windows, threads, channels, file
  watchers, terminal PTYs, and JDTLS child processes. This is where hanging is
  actually possible, so most of the rules below exist for it.

## Read the platform guidance

- For Rust Core and gpui host tests, read
  [references/rust-and-gpui.md](references/rust-and-gpui.md).
- For HTML/JUnit output, performance budgets, or CI artifacts, read
  [references/test-reporting.md](references/test-reporting.md).

## Preserve these invariants

- Every wait has an explicit local deadline. The CI step timeout is never the
  first mechanism capable of terminating a stuck test.
- Do not use real-time sleeps to synchronize state. Inject a clock, scheduler,
  event, continuation, channel, or controllable test double.
- Do not move a blocking wait into a spawned task merely to make an async test
  compile. Blocking a cooperative executor or the UI thread is forbidden.
- Every spawned task, timer, process, thread, continuation, stream, and gate has
  one owner and a cleanup path that runs after assertion failures as well as
  success. Prefer `Drop` or an explicit teardown guard.
- A concurrency test describes and controls its event order: operation starts,
  reaches the synchronization point, is released or cancelled, and terminates.
- Polling is a last resort. It must use a monotonic deadline, produce a useful
  timeout diagnostic, and poll an observable boundary rather than private state.
- Unit tests do not depend on real network services, installed developer tools,
  machine speed, personal paths, or wall-clock time. Put unavoidable external
  dependencies in an explicitly identified integration test. In particular, do
  not start a real JDTLS, a real terminal, or a real file watcher in a unit test.
- Assert observable behavior. Do not weaken production behavior, expose private
  state solely for a test, or delete a test to satisfy the stability gate.

## Required workflow

1. Read the changed behavior, implementation, and nearby tests. Identify every
   asynchronous boundary and resource whose completion the test will await.
2. Choose deterministic synchronization before writing assertions. For a race
   regression, write the intended event sequence explicitly in the test or its
   test-double names.
3. Run the fast static gate before the test suite:

   ```bash
   ./.agents/skills/write-stable-tests/scripts/verify-test-stability.sh
   ```

   On Windows use:

   ```powershell
   ./.agents/skills/write-stable-tests/scripts/verify-test-stability.ps1
   ```

4. Run the affected suites and record per-test durations. A changed test is not
   verified until its individual duration appears in the generated report:

   ```bash
   cargo test --manifest-path rust/Cargo.toml -p lithe-core
   node .agents/skills/write-stable-tests/scripts/run-rust-tests-with-timing.mjs \
       --manifest rust/Cargo.toml --package lithe-gpui-workbench \
       --suite-timeout-ms 120000 \
       --report .artifacts/test-stability/gpui-workbench.json
   ```

5. Run the broader affected validation required by `develop-lithe`. Report the
   report path, slowest changed tests, exact commands, and any suite that could
   not run on the current platform. Open
   `.artifacts/test-stability/index.html` to review failures, module health, and
   performance warnings before handoff.

## Exceptions

Do not add a scanner exception merely to make the gate pass. If a real-time or
blocking primitive is unavoidable at a native synchronous boundary, keep it
off the cooperative executor, add a short local timeout, guarantee cleanup, and
place this annotation immediately above the relevant line:

```text
test-stability: allow(<rule-id>) reason: <why deterministic synchronization is impossible>
```

The reason must describe the architectural constraint, not restate the code.
New exceptions require explicit mention in the handoff.
