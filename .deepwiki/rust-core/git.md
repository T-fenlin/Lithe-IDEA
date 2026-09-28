# Core: Git

`rust/lithe-core/src/git/` is the largest subsystem in Core: ~6 900 lines in `mod.rs`
alone, plus 20 focused modules. It is a **command facade**, and almost every interesting
design decision in it is about *reviewed* mutations.

## Module map

| File | Responsibility |
| --- | --- |
| `mod.rs` | The command facade. Owns `git.*` handlers, preflight, status, diff, push preview/write, status format strings, and ~1 100 lines of in-file tests from `:6672`. |
| `configuration.rs` | Allowlisted `git config` inspection, provenance, narrowly scoped edits |
| `execution_policy.rs` | Request-scoped Git preferences + deterministic temporary config |
| `execution_events.rs` | The Git event stream (`requestStarted`…`finished`) and observer scoping |
| `fetch.rs` / `fetch_execution.rs` | One shared Fetch command used by both preview and execution; per-remote results preserve partial success and exact changed refs |
| `graph.rs` | Coordinate-free graph projection: integer lanes + typed routes, **no renderer paths** |
| `history.rs` | Bounded reference snapshots + incremental history pages |
| `mutations.rs` | Shared mutation helpers kept out of the facade |
| `patch_exchange.rs` | Lossless UTF-8 patch export/preview/apply with forward checks |
| `progress.rs` | Deterministic phase progress, independent of console rendering and pipe framing |
| `rebase_session.rs` | Reviewed interactive rebase on Git's native, restartable sequencer |
| `rewrite.rs` | Reviewed history mutation with immutable snapshots and durable recovery refs |
| `setup.rs` | `git init` and narrowly scoped commit-identity configuration |
| `console/{mod,command,output,types}.rs` | IDEA-style console folding and search over retained command text |

## Reviewed mutations: preview → write

The dominant pattern. A mutating operation is a two-command pair where the first
produces a *reviewed expectation* and the second refuses to run if reality moved.

```
git.pushPreview   -> GitPushPreviewResponse { localHead, remote, remoteBranch,
                     remoteTrackingOid, tags, commits, hasMore }
git.write { operation: "push", expected: { localHead, remote, ... } }
        ^ if the local head or branch.<n>.pushRemote changed since the preview:
          error code stale_preview / message
          "Git push preview is stale; refresh and try again."
          and NO push is invoked at all
```

Same shape for rebase (`git/rebase_session.rs:753`, `"The rebase preview is stale; refresh
and review it again"`) and history rewrite (`git/rewrite.rs:608,724`, code `stale_preview`
at `:402`). The invariant "preview and write must agree on the destination and the safe
options" is asserted against a real `git` binary in
`rust/lithe-core/tests/git_push.rs:156-260`.

Other guarded mutations:

- **History rewrite** refuses remote-reachable commits and a dirty tree
  (`src/tests/git.rs:1171,1204`); undo preserves staged/unstaged/untracked content and
  keeps a recovery ref (`src/tests/git_history_rewrite.rs:100`).
- **Patch exchange** rejects a stale preview and forward conflicts *without writing*
  (`src/tests/git_patch_exchange.rs:172`), rejects unsafe paths and lossy text (`:204`).
- **Selected commit** with a large path set passes paths on **stdin** rather than the
  command line, and rolls back without overflow (`src/tests/git.rs:483,2500`); a failing
  hook restores the index while preserving real index changes (`:558,623`).

## Command surface

Roughly 40 `git.*` commands. The ones the gpui host actually issues
(`gpui/crates/git/src/{model,changes,branch_info,identity}.rs`, `gpui/crates/java`):

`git.status`, `git.write`, `git.references`, `git.historyPage`, `git.historyCursorClose`,
`git.commitFiles`, `git.operationState`, `git.repositorySetup`, `git.configureIdentity`,
`git.watchContext`, `git.authRespond`.

Three **hard constraints** the host must respect, each documented in
`gpui/crates/git/src/lib.rs`:

1. `git.status.repositoryRoot` may come back **workspace-relative** and must be re-joined
   against the host's root (`gpui/crates/git/src/model.rs:517-528`).
2. "Not a repository" is `ok: true` with `repositoryRoot: null` — **not an error**
   (`model.rs:805-812`).
3. `git.write` returns `ok: true` even when Git exits non-zero, so `operationError` /
   `exitCode` must be inspected (`changes.rs:488-561`).

Contract types live in `protocol/contracts.rs:518-853`; the prose is
`shared/contracts/rust-core-api.md` with dedicated notes for rebase sessions
(`git-rebase-session.md`), patch exchange (`git-patch-exchange.md`) and repository
setup (`git-repository-setup.md`).

## Determinism specifics

| Concern | Enforcement |
| --- | --- |
| Changed paths sorted | `git/mod.rs:6655`; contract doc "Deterministically ordered changed paths" (`contracts.rs:711`) |
| History paging is **cursor-based**, not offset-based | `contracts.rs:640-649`; `next_offset` is explicitly "Deprecated… only for compatibility". Test asserts disjoint pages: `src/tests/git.rs:3600` |
| Recent-checkout list bounded and ordered | Up to five local branches, most-recently-checked-out first (`contracts.rs:618-619`) |
| Graph projection is renderer-neutral and reproducible | Integer lane coordinates, `GraphRoute.id` derived from child hash + parent order + parent hash, rows in supplied history order; test `projection_is_deterministic_for_identical_input` (`git/graph.rs:439`) |
| Windows verbatim paths stripped | `simplified_canonical_path` (`git/mod.rs:3053-3071`) — `\\?\C:\…` becomes unresolvable `//?/…` once separators normalize. Test `:6687` |
| `git.status` must not refresh the index | `src/tests/git.rs:221` |

## Console folding

`git/console/` is transport-independent presentation logic: conservative command
compression that always keeps behaviour-changing flags visible
(`console/command.rs`), IDEA-style progress folding while ordinary output stays
continuous text (`console/output.rs`), and lossless disclosure ranges
(`console/types.rs`). `console/tests.rs` is described as "Cross-platform presentation
fixtures protect semantics, lossless ranges and grouping" — but the gpui bottom pane
currently renders the console **shell only**, with all six toolbar buttons disabled
(`gpui/crates/git/src/log_view.rs:1875-1930`, registered as deviation 6 in
`gpui/crates/git/src/lib.rs:152-154`). The real output would need the Git event channel,
which is out of the six commands in scope for that crate.

## Where to change things

| Task | Start at |
| --- | --- |
| Add a `git.*` command | `protocol/command.rs` (variant + `parse` arm) → `runtime/dispatcher.rs` → `git/mod.rs`; add a fixture first |
| Change a reviewed-mutation guard | `git/{mod,rebase_session,rewrite,patch_exchange}.rs` and its `src/tests/git*.rs` |
| Change the graph layout | `git/graph.rs` only — it holds no renderer paths |
| Change console folding | `git/console/` + `git/console/tests.rs`; the host UI is a separate concern |
| Add an event kind | `git/execution_events.rs:34-65`; check the nesting test at `:537-599` |

## Sources

- `rust/lithe-core/src/git/` (all files)
- `rust/lithe-core/src/tests/git.rs`, `git_history_rewrite.rs`, `git_patch_exchange.rs`, `git_fetch.rs`, `git_repository_setup.rs`
- `rust/lithe-core/tests/git_push.rs`, `git_watch_context.rs`
- `rust/lithe-core/src/protocol/contracts.rs`
- `rust/lithe-core/src/protocol/command.rs`
- `rust/lithe-core/src/runtime/dispatcher.rs`
- `rust/gpui/crates/git/src/lib.rs`, `model.rs`, `changes.rs`
- `shared/contracts/{rust-core-api,git-rebase-session,git-patch-exchange,git-repository-setup}.md`
