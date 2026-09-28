# Contracts & Fixtures

`shared/` is **not compiled**. It holds the compatibility surfaces that a change must not
break silently: prose contracts, JSON schemas, and roughly 90 fixtures that pin the wire
format byte-for-byte.

`shared/README.md:1-35` is the index.

## Source of truth per concern

| Concern | Source of truth |
| --- | --- |
| Application behaviour, data rules, ownership, lifecycle, error categories | `shared/contracts/application-boundary.md` (457 lines) |
| The JSON command surface (~180 commands), event schema, protocol version | `shared/contracts/rust-core-api.md` (1 824 lines) |
| JSON envelope shape | `rust-core-api.md:33-71` |
| Error codes (11 stable categories) | `application-boundary.md:276-295` + `rust/lithe-core/src/protocol/error.rs` |
| Determinism rules | split across `application-boundary.md:11-19`, `rust-core-api.md:319-320` ("Add a fixture under `shared/fixtures/` before changing a response shape or search rule") and `.agents/skills/develop-lithe/SKILL.md:112-128` |
| Git rebase / patch exchange / repository setup | `git-rebase-session.md`, `git-patch-exchange.md`, `git-repository-setup.md` |
| GitHub integration | `github.md` |
| AI commit generation | `ai-commit.md` (own `AI_COMMIT_*` code namespace) |
| Terminal shell selection | `terminal-profiles.md` |
| Update state machine | `update-v1.md` + `update-v1.schema.json` (15 stable codes) |
| **i18n** | **no contract exists** — see the gap below |

`application-boundary.md:6-7` self-declares that "the verification scripts are the
executable source of boundary checks", which is why
`./scripts/verify-shared-contracts.sh` is a required gate.

## What the data rules say

`application-boundary.md:11-19` is nine lines that constrain the entire codebase:

1. UTF-8 JSON at every process and language boundary.
2. Workspace paths are relative to the opened workspace and use `/` separators.
3. Absolute paths may appear at native editor/process boundaries and as LSP `file://` URIs,
   but are **not persisted as cross-platform identifiers**.
4. Product-facing line numbers are one-based.
5. Editor/LSP positions explicitly use **zero-based lines and UTF-16 columns**.
6. Missing locations are `null`.
7. **Lists have deterministic ordering so contract fixtures can be compared directly.**
8. Every asynchronous operation exposes `idle`, `loading`, `ready` and `failed` outcomes.
9. Failures contain a stable `code` and a user-facing `message`; platform details belong in
   `details`.

The same file also carries the feature-ownership table, the module lifecycle contract
(`:42-107`), the document lifecycle contract, the UI boundary, and the run-configuration /
Maven / Java-build boundaries.

## JSON schemas

11 schemas in `shared/contracts/` plus one in `docs/reference/`. All draft 2020-12, all
`$id` under `https://lithe.dev/contracts/…` (or `…/schemas/` for the language-provider one).

| Schema | Document |
| --- | --- |
| `run-configuration-v1.schema.json` | Legacy run configuration (v1: `type` enum) |
| `run-configuration-v2.schema.json` | **Current** (v2: `provider`, `execution`, `category`, `extensions`, root-level `toolchain`) |
| `project-manifest-v1.schema.json` | `.lithe/project.json`: `version`, `id`, `defaultRunConfiguration` |
| `maven-local-v1.schema.json` | `.lithe/maven.local.json` |
| `maven-launch-context-v1.schema.json` | Transient launch context (reactorPath, profiles, skipTests) |
| `maven-portable-configuration-v1.schema.json` | Portable Maven defaults |
| `toolchain-local-v1.schema.json` | `.lithe/toolchains/local.json` |
| `toolchain-requirements-v1.schema.json` | Generated toolchain requirements |
| `editor-syntax-theme-v1.schema.json` | 28 syntax roles + 6 fallbacks + 20-colour light/dark palettes |
| `workbench-background-v1.schema.json` | `none` / `bundled 01-10` / `custom` + opacity |
| `update-v1.schema.json` | Normalized update state + 15 stable error codes |

Schema changes are a **compatibility surface**: they need a version bump and a matching
change in both the reader and the writer (`shared/src/document.rs` carries the
newer-than-supported read-only rule).

## Fixtures

Roughly 90 JSON files in 19 category directories, plus one non-JSON acceptance project.

```
shared/fixtures/
  ai/           commit-generation-v1
  community/    discourse-auth-v1
  core/         cancellation
  debug/        dap-session, run-in-terminal, stepping-filters,
                breakpoint-relocation, java-test-launch, exception-info,
                variable-paging, disconnect-policy                    (8)
  diagnostics/  redact-text-v1, build-manifest-v1
  documents/    lifecycle-v1
  editor/       line-edit-v1, diff-review-v1, svg-preview-v1
  editor-themes/ lithe-v1
  execution/    maven, gradle, framework-identity,
                maven-java-main-source-sets-v1, standalone-java-compile-run-v1  (5)
  git/          26 files: write, worktrees, worktree-creation, tag-names,
                repository-setup, remote-url, references-response, rebase-session,
                push-preview, patch-exchange, history-rewrite, history-response,
                history-page-response, history-page-date-request, graph,
                fetch-skip-boolean, fetch-plan, execution-policy, execution-events,
                diff, console-presentation, console-lifecycle, console-command,
                command-response, command-error-response
  history/      basic
  java/         basic, run-markers-v1
  lsp/          10: semantic-tokens, project-preparation, maven-profile-events,
                jdt-workspace-key, jdt-workspace-fingerprint, jdt-direct-launch,
                jdt-cache-retention, java-workspace-policy, java-navigation,
                java-build-report
  maven/        7
  modules/      built-in-v1
  mybatis/      basic
  plugins/      official-v1, language-support-v1
  run-configuration/  10: basic, single-module, multi-module, no-entry,
                invalid-version, toolchain-mismatch, project-environment,
                editor-save, hybrid-spring-vue, maven-module-ownership
  search/       basic, advanced
  settings/     workbench-background-v1
  spring/       basic
  terminal/     shell-selection-v1
  updates/      update-v1
  workspace/    repositories-v1
  projects/     lithe-spring-boot-git-graph/   (a real Maven project + .class files)
```

### Shape and purpose

Fixtures are cross-platform **compatibility surfaces** in a `cases[]` / `request` /
`expected` shape:

```jsonc
// shared/fixtures/documents/lifecycle-v1.json
{ "version": 1,
  "cases": [ { "name": "...", "state": {...}, "event": {...},
               "expected": { "state": {...}, "action": "..." } } ] }
```

Adding or changing a fixture is a change to the contract. The rule from
`rust-core-api.md:319-320` is explicit: **add a fixture before changing a response shape or
a search rule.**

### How they are consumed

Two loading patterns, both in `lithe-core`:

1. **Compile-time `include_str!`** — dominant. `src/tests/languages.rs:11-14`,
   `src/tests/run_configuration.rs:11-13`, `src/tests/protocol.rs:22`,
   `git/mod.rs:7011`, `lsp/languages/project_preparation.rs:59-61`, and more.
2. **Runtime read via `CARGO_MANIFEST_DIR`** — `src/tests/support.rs:6-11`.

**Only `lithe-core` reads fixtures.** A grep of `rust/gpui/**/*.rs` for `shared/fixtures`
returns zero hits: the host re-implements the contract shapes as Rust types
(`gpui/crates/shared/src/workspace_config/toolchain.rs:27-30`) and cites the contracts in
doc comments only. That is intentional — the host owns presentation, not the wire format.

### Structural validation

`scripts/verify-shared-contracts.sh` parses every fixture and schema, then runs
hand-written Ruby structural assertions over **7 specific fixtures**:

| Fixture | What is asserted |
| --- | --- |
| `maven/platform-contract-v1.json` | Module and source-root shapes |
| `modules/built-in-v1.json` | Built-in module manifest |
| `plugins/official-v1.json` | Only released downloads are referenced |
| `github/pull-request-v1.json` | Includes a **credential-leak regex** |
| `editor-themes/lithe-v1.json` | **Fallback cycle detection** |
| `settings/workbench-background-v1.json` | Background preference values |
| `updates/update-v1.json` | Update state + stable error codes |

The other ~83 fixtures are checked here only for JSON validity; their semantics are pinned
by individual `lithe-core` tests.

`shared/fixtures/projects/lithe-spring-boot-git-graph/` is the one non-JSON fixture: an
acceptance project materialized into a real Git repository by
`scripts/create-git-graph-fixture.sh` (default `/tmp/…`) with `main`, a `feature/users`
merge commit, a `feature/orders` rebase history, tag `v0.1.0` and an `origin/main`.

## Known gaps and drift

- **`shared/fixtures/core/cancellation.json` appears unused.** No reference anywhere in the
  repository. Its two expected errors (`cancelled`, `timed_out`) are covered by unit tests
  in `protocol/cancellation.rs:191-195` instead. Either wire it up or delete it.
- **There is no i18n contract.** The only normative statements are
  `application-boundary.md:226-228` — "Domain and adapter layers return stable reasons
  rather than user-facing prose; each product's presentation layer owns localized
  notification text." The implementation lives entirely host-side in
  `gpui/crates/shared/src/i18n.rs` + `locales/`.
- **`rust/gpui/README.md:70-86`** and **`rust/gpui/PLAN.md`** predate the workspace merge
  and the frontends' removal; several paths in them are wrong.
- **Locale key counts disagree** across three sources in `locales/` — treat the yml file,
  not its header, as authoritative.

## Rule of thumb

If you change any of these, you have changed a compatibility surface:

- a `CoreCommand` variant or its name (`protocol/command.rs`)
- an `ErrorCode` variant (`protocol/error.rs`)
- a `CoreRequest` / `CoreResponse` / `CoreError` field
- any `Response*` type in `protocol/contracts.rs`
- a document under `.lithe/` or a settings key
- a JSON schema

Update the contract prose **and** every consumer, and add or update a fixture.

## Sources

- `shared/README.md`
- `shared/contracts/` (all files)
- `shared/fixtures/` (all files)
- `scripts/verify-shared-contracts.sh`
- `scripts/create-git-graph-fixture.sh`
- `rust/lithe-core/src/protocol/`, `src/tests/support.rs`
- `rust/gpui/crates/shared/src/workspace_config/`
- `docs/reference/language-providers.schema.json`
- `.agents/skills/develop-lithe/SKILL.md`
