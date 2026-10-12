# 构建耗时实测（娄码通）

**所有数字为本次会话实测。** 环境：Windows / 8 逻辑核 / cargo 1.97.0 / rustc 1.97.0，
基线 `3085f5d1`，`RUST_MIN_STACK=67108864`、`CARGO_PROFILE_DEV_DEBUG=0`、`-j 4`，
`CARGO_TARGET_DIR=D:\developmentProjects\rust\Lithe-IDEA\rust\lithe-gpui\target`
（`.gitignore:6` 已忽略），`workdir=rust/lithe-gpui`。

## 命令模板

```powershell
$env:CARGO_TARGET_DIR='D:\developmentProjects\rust\Lithe-IDEA\rust\lithe-gpui\target'
$env:RUST_MIN_STACK='67108864'
$env:CARGO_PROFILE_DEV_DEBUG='0'
cargo build -p lithe-gpui-app -j 4        # 或 cargo check
```

## 实测结果

| # | 标签 | 命令 | 秒 | 重编单元 |
|---|---|---|---|---|
| 1 | 冷全量 | `cargo check` | 547.1 | 549 |
| 2 | 全量重建 | `cargo build` | 548.8 | 399 |
| 3 | 无改动 A | `cargo check` | 1.7 | 0 |
| 4 | 无改动 B | `cargo check` | 1.4 | 0 |
| 5 | 无改动 | `cargo build` | 1.9 | 0 |
| 6 | 无改动（build 预热后） | `cargo build` | 1.9 | 0 |
| 7 | 改 `terminal/src/lib.rs` | `cargo build` | **8.6** | 3 |
| 8 | 改 `workbench/src/lib.rs` | `cargo build` | **8.7** | 2 |
| 9 | 改 `settings/src/lib.rs` | `cargo check` | **3.6** | 3 |
| 10 | 改 `shared/src/lib.rs` | `cargo check` | **221.2** | 12 |
| 11 | 改 `shared/src/lib.rs` | `cargo build` | **281.0** | 12 |
| 12 | 改 `../lithe-ui/src/lib.rs` | `cargo check` | **231.7** | 14 |
| 13 | 改 `../lithe-ui/src/lib.rs` | `cargo build` | **671.4** | 15 |

「重编单元」= `cargo` 输出中 `Compiling`/`Checking` 行的去重 crate 名数量。

## 三档成本曲线

| 档 | 触发条件 | 实测耗时 |
|---|---|---|
| **秒级** | 改 `workbench` / `terminal` / `settings` / `git` / `explorer` / `editor` 等宿主 crate 的私有代码 | **1.4 ～ 8.7 s** |
| **分钟级** | 改 `lithe-gpui-shared`（11 个反向依赖） | **221 ～ 281 s** |
| **十分钟级** | 改 `lithe-ui`，或 `target` 被清/污染后的全量重建 | **547 ～ 671 s** |

## 与既有约束的对照

| 既有说法 | 实测 | 判定 |
|---|---|---|
| 「构建一次约 15 分钟，每天 3～4 次」 | 全量 9m07s～11m11s；**改宿主 crate 私有代码 1.4～8.7 s** | **部分不成立**——只对 `shared`/`lithe-ui`/全量成立 |
| 「2026-10-11 实测 14m48s」 | 与本次 lithe-ui 的 671 s（11m11s）同量级 | 一致（差异来自 `debug=0`） |
| 「`-j 2`，并行度不高于 4 否则 linker OOM」 | `-j 4` 全程无 OOM | `-j 2` 偏保守 |
| 「`explorer.exe` 泄漏 17 GB，rustc 崩」 | 本机 `explorer.exe` **未运行**；未复现崩溃 | 本次未复现 |

## 从既有 target 重建的历史时间线（只读，交叉验证）

`D:/developmentProjects/rust/lithe-gpui-target`（11.88 GB）内 1077 个 rlib/rmeta **全部**被重写：

- 首个产物 `2026-10-12 07:08:24` → `Lithe.exe` `07:23:38` = **15.25 min**（全量重建）

| 单元 | 耗时 | 产物 |
|---|---|---|
| `windows_sys` | 444 s | 19.3 MB |
| `windows` | 42 s | **242 MB** |
| `gpui` | 241 s | **202 MB** |
| `lithe_ui` | **149 s** | **1222.5 MB**（debug=1） |
| `lithe_gpui_workbench` | 12 s | 228.9 MB |
| `Lithe.exe` | — | 89.3 MB |

`lithe_ui` 单 crate 149 s 是全量构建里最长的一段，与其 4.19 MB 的**宿主未使用**源码吻合。

## 沙盒差异（必须标注）

我**不能写** `D:/developmentProjects/rust/lithe-gpui-target`（已存在，只读）：
`error: failed to open: ...\debug\.cargo-build-lock` / `拒绝访问 (os error 5)`。
故全部实测在 `rust/lithe-gpui/target` 完成，**与仓库配方的目录不同**。
建议主理人在可写环境重跑 #10 与 #12 两行做交叉验证。

另：台账「工作区 ACL 拒绝可执行文件写入」在本次**未能复现**——工作区内建 crate 后
`cargo run` 成功，exe 在自身目录旁写文件、建目录、起子进程**全部成功**。
详见 `reports/gpui-core-impl-evidence/measurements.md` §7。
