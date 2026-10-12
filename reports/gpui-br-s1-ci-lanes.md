# s1 · GPUI 改动会被哪些 CI 车道覆盖（构建发布层证据）

基线提交 `3085f5d1`，分支 `feat/menu-bar-opaque`。本文件只记录**可直接复现的检索与原文引用**，不含推断。
检索口径逐条标注；本轮全部为只读检索，未触发任何 CI 运行。

---

## 1. 前提：GPUI 不在 `rust/` workspace 里

`rust/Cargo.toml:2`：

```toml
members = ["lithe-core", "lithe-git-host", "lithe-db-sidecar", "lithe-db-mcp", "lithe-agent-host", "lithe-ide-host"]
```

`rust/lithe-gpui/Cargo.toml:9-10` 自己声明 `[workspace]` / `members = ["crates/*"]`；
`rust/lithe-gpui/Cargo.lock`（232,813 字节）与 `rust/lithe-ui/Cargo.lock` 均已被 git 跟踪
（`git ls-files --error-unmatch` 两者都成功）。

**含义**：`cargo test --manifest-path rust/Cargo.toml` 与 `cargo build --workspace` 都不会触及
`lithe-gpui` 的任何 crate。它是第二个、独立的 workspace。

---

## 2. 分类器把 GPUI 改动映射到哪几条车道

`scripts/classify-ci-changes.sh` 输出 13 个布尔开关，供 `ci-windows.yml` / `ci-macos.yml`
的 job `if` 条件使用。GPUI 路径（`rust/lithe-gpui/**`、`rust/lithe-ui/**`）在 case 表里
**没有专门分支**，只能落到兜底：

```
scripts/classify-ci-changes.sh:308-316
        rust/*)
            # Unknown Rust workspace paths fail closed across all Rust-backed
            # products because workspace configuration can affect every crate.
            rust_core=true
            rust_database=true
            macos_release=true
            windows=true
            windows_rust=true
            ;;
```

即改一行 GPUI 代码 → `rust_core + rust_database + macos_release + windows + windows_rust`
全部置 true。（`rust/lithe-ui/**` 同样命中 `rust/*` 兜底。）

---

## 3. 这五条车道实际编译什么（逐条核对）

| 车道 | 触发处 | 实际构建目标 | 是否编 GPUI |
|---|---|---|---|
| `windows` | `ci-windows.yml:106-109` `frontend` job | `windows/tauri` 前端（Bun/TS）：`bun run typecheck`、`bun test src/...` | 否 |
| `windows_rust` | `ci-windows.yml:451-454` `rust-tests` job → `test-stability-windows.ps1 -Scope WindowsRust` | `windows/tauri/src-tauri/Cargo.toml`（`lithe-project`、`lithe-terminal`） | 否 |
| `rust_core` | 同上 → `-Scope SharedRust` | `rust/Cargo.toml` 的 `lithe-git-host` / `lithe-agent-host` / `lithe-ide-host` / …（见下） | 否 |
| `macos_release` | `release-macos.yml` / `ci-macos.yml` 的构建 job | `scripts/build-macos.sh` → `swift build` + `build-rust-core.sh` | 否 |
| `rust_database` | `ci-database.yml` | `rust/lithe-db-mcp` / `rust/lithe-db-sidecar` | 否 |

`SharedRust` 的实际 manifest 与包名（`.agents/skills/write-stable-tests/scripts/test-stability-windows.ps1:162-184`）：

```
:165-169  -Manifest "rust/Cargo.toml"  -Package "lithe-git-host"
:171-175  -Manifest "rust/Cargo.toml"  -Package "lithe-agent-host"
:177-181  -Manifest "rust/Cargo.toml"  -Package "lithe-ide-host"
:183-184  -Manifest "rust/Cargo.toml"  （后续同理）
```

`WindowsRust`（`:142-159`）：

```
:144-148  -Manifest "windows/tauri/src-tauri/Cargo.toml"  -Package "lithe-project"
:150-154  -Manifest "windows/tauri/src-tauri/Cargo.toml"  -Package "lithe-terminal"
:156-159  -Manifest "windows/tauri/src-tauri/Cargo.toml"
```

**两条 Rust 车道用的 manifest 分别是 `rust/Cargo.toml` 与
`windows/tauri/src-tauri/Cargo.toml`，没有一条指向 `rust/lithe-gpui/Cargo.toml`。**

---

## 4. 工作流与脚本的全局检索（独立复核主理人 §2.1）

检索口径 A：整个 `.github/` 目录（含 `workflows/` 与 `actions/`），模式 `lithe-gpui|lithe_gpui`
→ **命中 0**。工作流总数实测 **18** 个。

检索口径 B：`scripts/` 全目录，模式 `lithe-gpui-app|lithe-gpui` → 唯一命中：

```
scripts/sign-windows.ps1:3
    [string]$BinaryDirectory = (... | Join-Path -ChildPath "lithe-gpui-target/debug"),
```

注意两点（主理人未指出）：

1. 该默认值是 **`debug`**，不是 `release`。`release` 产物在本机不存在：
   `lithe-gpui-target/release/Lithe.exe` **不存在**，只有 `debug/Lithe.exe`（87.3 MB，2026/10/12）。
   也就是说签名脚本按默认参数跑，签的是开发用 debug 二进制。
2. `sign-windows.ps1` 本身**没有被任何工作流调用**（检索 `sign-windows` 只命中 5 处，全在
   `docs/development/gpui-*.md` 三份文档里）。它是一把上了膛但没接到扳机上的枪。

检索口径 C：`rust/lithe-gpui` 目录全文件（含 `.yml`/`.ts`），模式 `lithe-gpui-app|lith-gpui`
→ 无构建脚本引用（`scripts/` 里没有任何脚本构建 GPUI）。

---

## 5. 结论（对应判断「假绿灯」）

**推翻条件**：找到任一条 CI 车道实际会构建 `rust/lithe-gpui` 或其成员 crate。
**本轮检索与逐条核对均未命中该条件 → 判断成立。**

改一行 `rust/lithe-gpui/**` 或 `rust/lithe-ui/**` 的代码，CI 会：

- 跑 `windows/tauri` 的前端类型检查与测试（与 GPUI 无关）；
- 跑 `windows/tauri/src-tauri` 的 Rust 测试（与 GPUI 无关）；
- 跑 `rust/Cargo.toml` 的 host crate 测试（与 GPUI 无关）；
- 触发 `windows` 与 `macos_release` 分支的增量构建（构建的是旧 Tauri / Swift 产物）。

**GPUI 一行代码都不会被编译、一个单测都不会被跑。** CI 全绿与 GPUI 能否编译
**没有任何因果关系**——这是"假绿灯"，不是"覆盖不足"。

严重性说明：这不是"少一条 workflow"的普通缺口，而是**校验信号失真**。在 GPUI 成为目标前端、
旧两端界面代码即将删除的前提下，仓库当前唯一的跨平台功能门禁（`verify-platform-feature-matrix`）
与唯一的编译校验都不覆盖 GPUI。任何声称"GPUI 改动已过 CI"的说法在当前配置下都不成立。

---

## 6. 本步未覆盖

- 未实际触发任何 CI 运行，结论全部来自配置原文与脚本源码。
- 未核对 `ci-plugins.yml` / `ci-database.yml` 之外其余工作流的完整 job 图（已检索 `lithe-gpui`
  命中 0，故不影响结论）。
- 「假绿灯」的**修复方案**在 s6 与最终结论中给，本步只记录现状。
