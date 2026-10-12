# GPUI 统一前端 · 构建发布层结论

**角色**：常通关（构建发布工程师）
**基线**：`3085f5d1`，分支 `feat/menu-bar-opaque`
**范围**：只做 CI/CD、交叉编译、三平台打包签名与发布层的分析与方案；不改任何代码
**配套证据**：`reports/gpui-br-s1-ci-lanes.md`、`gpui-br-s2-matrix-gate.md`、
`gpui-br-s3-packaging-surface.md`、`gpui-br-s4-version-and-toolchain.md`、
`gpui-br-s5-macos-and-linux.md`、`classify-gpui-changes.ps1`

---

## 结论（先说要紧的）

1. **GPUI 的构建链缺口不是"缺一条 workflow"，而是"校验信号失真"。**
   CI 不但不构建 GPUI，而且会在 GPUI 改动上亮绿灯。基线提交 `3085f5d1` 本身就是
   一次 GPUI 改动（`rust/lithe-ui/src/buttons/button.rs`、`rust/lithe-ui/src/menus/mod.rs`），
   它命中的五条车道一条都不编 GPUI。

2. **矩阵门禁根本没被触发。** `platformPaths` 8 条不含 GPUI；
   未命中即直接放行，"每批功能必须同步矩阵"这条硬约束在 GPUI 上**没有执行者**。

3. **主理人 §2.1 的每条断言我都复核过，全部成立**；但我补出四条他没写的缺口，
   其中两条会改变排期形状（版本号注入、公证）。

4. **macOS 从零建链路的工作量被显著低估。** A3 记 5 人日，按我的证据应为 **8～11 人日**，
   因为它不只是"新建一条 workflow"，而是要在**两个独立 workspace、无版本注入、
   无工具链钉版、无公证**的地基上建一条可发布链路。

5. **Linux 现在不该建链路。** 依赖已解析，但真机可行性（门 G3）未过；
   在没有通过 G3 前建 CI 是把流水线建在可能起不了窗口的目标上。

---

## 一、独立复核主理人 §2.1「构建链缺失」（逐条）

主理人原文（`docs/development/gpui-platform-gap-and-schedule-2026-10.md:207-221`）分四句。
逐条核对：

| # | 主理人断言 | 复核结论 | 证据 |
|---|---|---|---|
| 1 | 「`.github/workflows/` 18 个工作流逐个检索 `lithe-gpui`，命中 0」 | **成立** | 实测工作流 **18** 个；检索整个 `.github/`（含 `actions/`）`lithe-gpui\|lithe_gpui` **命中 0** |
| 2 | 「`scripts/` 下唯一引用是 `sign-windows.ps1:3` 的默认参数」 | **成立，但需补两点** | 唯一命中确在 `sign-windows.ps1:3` |
| 3 | 「`build-windows.ps1` 构建的是 `windows/tauri`（第 11 行）」 | **成立** | `build-windows.ps1:10` `$windowsApp = Join-Path $root "windows/tauri"`（主理人写第 11 行，实为 **10** 行，`$bundledExtensionsSource` 在 11 行） |
| 4 | 「`crates/app/Cargo.toml` 唯一的平台条件是 `embed-resource`」 | **成立** | `crates/app/Cargo.toml:14-16`；`build.rs:1-13` 只做图标嵌入 |

**第 2 条的补充（主理人未指出）：**

- `sign-windows.ps1:3` 的默认目录是 **`lithe-gpui-target/debug`**，不是 release。
  本机实测：`lithe-gpui-target/release/Lithe.exe` **不存在**，只有 `debug/Lithe.exe`（87.3 MB）。
  按默认参数调用它，签的是**开发用 debug 二进制**。
- **`sign-windows.ps1` 没有被任何工作流引用。** 检索 `sign-windows` 全仓 5 处命中，
  全部在 `docs/development/gpui-*.md` 三份文档里。它是一把上了膛但没接到扳机上的枪。

**主理人 §2.1 未写出的四条缺口：**

| # | 缺口 | 证据 | 影响 |
|---|---|---|---|
| **N1** | **版本号无注入通路，且这个号进插件契约** | `shared/src/lib.rs:24` `HOST_VERSION = env!("CARGO_PKG_VERSION")`（全部 crate version 实测为 `0.0.1`）；唯一 `build.rs` 只嵌图标；`extensions/src/core_bridge.rs:7` 把 `HOST_VERSION` 作为 `hostVersion` 写进插件宿主契约 JSON | 发布版 hostVersion 恒为 `0.0.1`，**插件版本兼容判断失效**。这不是显示问题，是**契约字段错误** |
| **N2** | **工具链未钉版，且两套 workspace 互不预警** | 仓库根/`rust/`/`rust/lithe-gpui/`/`rust/lithe-ui/` 检索 `rust-toolchain*` **全部不存在**；10 处 CI 用 `dtolnay/rust-toolchain@stable`；`lithe-ui` 声明 `rust-version = "1.95"` 而其余 13 个 crate 无该字段；两套 workspace 的 resolver 分别是 3 与 2 | `gpui-pre` 约每周重基，新版 rustc 报新 lint 时**会在没人改 GPUI 的情况下开始失败**，而 CI 因假绿灯发现不了 |
| **N3** | **macOS 公证缺失（旧端就已缺失）** | workflows/scripts/macos 检索 `notarytool\|stapler\|notariz` 命中 **0**；`release-macos.yml:108-121` 有 Developer ID 签名，`:155` 只做 `codesign --verify` | Gatekeeper 会拦未公证的 app。**建 GPUI 链路时必须一并补上，否则新链路一开始就带洞** |
| **N4** | **矩阵门禁不在触发范围内** | `verify-platform-feature-matrix-change.mjs:10` 的 `platformPaths` 8 条不含 GPUI；`:12-13` 未命中即放行 | "每批功能同步矩阵"在 GPUI 上无执行者（与 s1 的假绿灯是两个独立缺口，叠加后 GPUI 改动**既不被编译校验、也不被矩阵约束**） |

---

## 二、三平台流水线清单

| 平台 | 架构 | 制品格式 | 签名 | 校验 | 现状 |
|---|---|---|---|---|---|
| **Windows** | `x86_64-pc-windows-msvc` | `Lithe.exe`（当前）；**建议补 NSIS/MSI 安装器** | Authenticode（`signtool`，`sign-windows.ps1` 已有但未接线） | SHA-256 + `signtool verify /pa` | 本地可出 exe，**流水线无入口** |
| **macOS** | `aarch64-apple-darwin` / `x86_64-apple-darwin` | `.app` + DMG | Developer ID 签名 + **公证（缺）** | `codesign --verify --deep --strict` + `spctl` + `hdiutil imageinfo` + SHA-256 | **完全无入口** |
| **Linux** | —— | —— | —— | —— | **不建议现在建**（真机可行性未过） |

### Windows 侧完整构建发布链路设计

必须补齐的环节（按依赖顺序）：

```
1. 产出 exe
   CARGO_TARGET_DIR=<out>/target cargo build -p lithe-gpui-app --release --locked
   （--locked 是前提：rust/lithe-gpui/Cargo.lock 已入库，必须用它锁版本）

2. 落位语言服务器（打包面见 s3）
   LanguageServers/jdtls/**   <- scripts/prepare-jdtls.ps1
   LanguageServers/jdk/**     <- scripts/prepare-jdk.ps1
   这两个脚本已存在且被旧端复用，可直接调。

3. 版本号注入（N1，需先补）
   在 build.rs 里 println!("cargo:rustc-env=LITHE_BUILD_VERSION={...}")
   并让 shared/src/lib.rs 优先读该环境变量、回退 CARGO_PKG_VERSION。
   同时注入 git revision / branch / dirty / timestamp（对照 macOS 的四个 Info.plist 键）。

4. 签名
   scripts/sign-windows.ps1 -BinaryDirectory <out>/target/release -RequireSignature
   注意：默认参数指向 debug，必须显式传 release 目录。

5. 打包（当前缺失，需新建）
   仓库现有 NSIS 走 Tauri（package-windows.ps1:145-152 调 tauri build --bundles nsis），
   GPUI 没有等价物。两条路：
   (a) 用 NSIS/WiX 独立脚本打包 exe + LanguageServers；
   (b) 先只发 zip（最小可用），安装器作为后续项。
   建议先 (b) 打开发布通路，再补 (a)。

6. 校验与发布
   SHA-256 + signtool verify；上传 GitHub Release。
```

**注意**：更新器（`MISSING_UPDATER`，`menu_bar.rs:928`）在 GPUI 侧完全未实现
（主理人 #43 已列），因此**Windows 侧第一版没有自动更新**，
`release-windows.yml:187-202` 的 `TAURI_SIGNING_PRIVATE_KEY` 强制校验**不适用于 GPUI**。

### macOS 侧：从零建链路分步清单与人日

| 步骤 | 内容 | 人日 | 前置 |
|---|---|---|---|
| M1 | 先验证 `gpui-pre` 0.3.8 在 macOS 上**能否构建**（最小 `cargo build -p lithe-gpui-app --target aarch64-apple-darwin`） | **1** | 一台 macOS 机器或 `macos-26` runner |
| M2 | 补版本号注入（与 Windows 共用同一机制，N1） | 1 | — |
| M3 | 钉工具链：给 `rust/lithe-gpui` 与 `rust/lithe-ui` 各加 `rust-toolchain.toml`（N2） | 0.5 | — |
| M4 | 建 `.app` bundle 骨架：`Info.plist`（含版本键）、`Contents/MacOS/Lithe`、`Contents/Resources/LanguageServers/{jdtls,jdk}`。**落位需实测**（s3 §4 已标注为推断） | **2** | M1 |
| M5 | Developer ID 签名 + **公证**（N3）：复用 `import-macos-signing.sh`，新增 `notarytool submit --wait` + `stapler staple` | **2** | M4；需 Apple 凭据 |
| M6 | DMG 产出：可参考 `create-dmg.sh` 的 hdiutil 重试逻辑，但 `LITHE_ARCH`/`app` 路径需改为 GPUI 的 | 1 | M4 |
| M7 | 新建 `release-*.yml`：编排 M2–M6，矩阵 arm64 + x86_64，缓存键用 `rust/lithe-gpui/Cargo.lock` | **2** | M2–M6 |
| M8 | 首轮实机验收：启动、`S1_*` 打点、`secondary-` 键位（主理人 §2.7 的待验证推断） | 1 | M7 |
| | **合计** | **10.5** | |

**对照**：主理人 A3 记 **5 人日**。差异来自 A3 只算了"建 workflow"，
未计入 M1（可行性验证）、M2（版本注入）、M3（工具链钉版）、M5（公证）。
**若 M1 失败，M4–M8 全部作废**——这是必须先做 M1 的理由。

### Linux 侧评估

**不建议现在建链路。** 理由（s5 §3）：依赖树已解析
（`Cargo.lock` 含 `ash`/`calloop`/`nix`/`wayland-sys`/`x11`），
但选型笔记 `:294` 明确「渲染走 wgpu → Vulkan 1.3，**没有证据支持无独显 / 软件渲染路径**」，
`:707` 的门 G3（Linux 真机）尚未通过。

在没有通过 G3 前建 Linux CI，等于把流水线建在一个可能起不了窗口的目标上。
**建议：G3 通过后再评估，当前只需在排期里保留这个门。**

---

## 三、现有 release 工作流迁移顺序

**关键前提**：删除 `macos/` 与 `windows/tauri` 后，现有 4 条 release 工作流
（`release-windows.yml`、`release-macos.yml`、`release-preview-windows.yml`、
`release-preview-macos.yml`）**全部失去构建对象**。
`package-windows.ps1:18`、`build-windows.ps1:10`、`package-app.sh:6` 都硬编码了旧端路径。

**迁移顺序（按依赖，不按时间）：**

| 序 | 动作 | 为什么这个顺序 | 风险 |
|---|---|---|---|
| 1 | 先建 **GPUI 的 preview 链路**（Windows 先），与旧端**并行存在** | 新链路未验证前不能拆旧链路 | 无 |
| 2 | 用 preview 链路跑通「构建→打包→签名→上传」全链路 | 发布工作流比 preview 多了版本校验、更新清单、AtomGit 同步，先用 preview 验证主干 | 无 |
| 3 | **补 N1（版本注入）与 N2（工具链钉版）** | 这两条是**发布**的前置：没有真实版本号无法生成 release notes 与更新清单 | 中 |
| 4 | 建 macOS GPUI 链路（M1–M8） | 依赖 M1 可行性 | **高**（M1 可能失败） |
| 5 | **删除旧两端界面代码 + 旧 release 工作流**，把 GPUI 链路改名为正式 release | 到这一步新链路已能独立产出三端产物 | 高（不可逆） |
| 6 | 处理遗留在旧工作流里的**共享资产** | 见下 | 中 |

**第 6 条尤其要注意** —— 旧工作流里有若干**不随界面代码删除**的东西：

- `sync-atomgit-release.yml`（`release-windows.yml:284-291`、`release-macos.yml:389-395`）
  —— 这是**发布渠道**，不是旧端功能，必须迁移而非删除。
- `update-homebrew-cask`（`release-macos.yml:305-388`）—— Homebrew 渠道，
  GPUI 的 DMG 产出后**可以继续复用**（它只依赖 `*.dmg.sha256`），
  但 `Casks/lithe.rb` 与 `update-homebrew-cask.rb` 需确认仍指向新产物。
- `create-macos-update-manifest.rb` / Sparkle 相关（`release-macos.yml:178-186`、`:247-258`）
  —— Sparkle 是 macOS 的更新机制。**GPUI 侧没有 Sparkle 等价物**
  （`MISSING_UPDATER`）。所以 GPUI 的 macOS 第一版**要么不提供自动更新，
  要么需要决定是否把 Sparkle 接到 Rust 应用上**。**这是一个需要人决策的点。**
- `scripts/verify-runtime-bundle-immutability.sh` —— 该脚本只扫
  `macos/Sources/**.swift` 与 `windows/tauri/src-tauri/src/**.rs`（`:13`、`:26`）。
  **旧两端删除后它会变成空扫描**，而 GPUI 的生产路径（`jdtls.rs`、`workspace.rs`）
  **完全不在它的检查范围内**。AGENTS.md 要求新增资源路径必须覆盖这个校验，
  因此**该脚本必须扩展覆盖 `rust/lithe-gpui`**。

---

## 四、`[profile.release]` 与平台条件是否够用

**不够。** 逐条核对：

### 4.1 `rust/lithe-gpui/Cargo.toml:14-18` 的 release profile

```toml
[profile.release]
strip = true
lto = true
codegen-units = 1
opt-level = 3
```

| 项 | 评价 |
|---|---|
| `strip = true` | **对 macOS 有问题**：符号被剥掉后，公证与后续崩溃分析都受影响。macOS 端通常保留 dSYM 另存。**建议按平台拆分** |
| `lto = true` + `codegen-units = 1` | 正确，但会让 15 分钟构建雪上加霜（主理人 §2.2 的瓶颈）。发布构建可接受，**开发构建必须走 debug profile** |
| 缺 `panic` 策略 | 未设置。GPUI 是 GUI 应用，`panic = "abort"` 可减体积，但会损失 `catch_unwind` 能力。**需决策** |
| 缺平台条件 | 全文无 `[target.'cfg(...)'.profile]`。macOS/Windows 需要不同的 strip 策略时无处表达 |

### 4.2 `crates/app/Cargo.toml:14-16` 的平台条件

```toml
[target.'cfg(windows)'.build-dependencies]
embed-resource = "3"
```

**只有这一处**，且只解决 Windows 图标嵌入。**缺的是：**

| 缺什么 | 后果 |
|---|---|
| macOS 的 `Info.plist` 生成/嵌入 | `.app` bundle 没有 `Info.plist` 无法成为合法 app；版本键（M2）也无处写入 |
| macOS 的 `.icns` 图标 | 当前 `assets/app-icon/` 下有 `128x128.png` 与 `icon.ico`；**`.icns` 是否存在未核对**（需 mac 端确认）。Dock 与 Finder 图标依赖它 |
| macOS 的 bundle 结构声明 | `cargo build` 产出的是裸可执行文件，不是 `.app`；需外部脚本组装（这正是 M4） |
| 代码签名 entitlement | 若后续需要沙盒或特定权限，需 `.entitlements` 文件 |
| Windows 的版本资源（VERSIONINFO） | `lithe.rc` 当前只嵌图标；可执行文件的属性页「版本」为空。对照 macOS 端有完整版本键，**Windows 侧缺对应物** |

### 4.3 结论

`[profile.release]` **对「能跑」够用，对「能发布」不够**。
最少需要补：平台化 strip 策略、macOS 的 Info.plist 与 icns 生成、Windows 的 VERSIONINFO。

---

## 五、风险与条件

| 风险 | 影响 | 触发条件 | 我的依据 |
|---|---|---|---|
| **`gpui-pre` 在 macOS 无法构建** | M4–M8 全部作废，macOS 端无 GPUI 产品 | M1 首次执行 | 与主理人 §4 第 1 条一致；我未新增证据，**仍是最大单点未知** |
| **假绿灯掩盖 GPUI 编译失败** | 已合入的 GPUI 代码可能从未成功编译过 | 已在发生 | s1：基线提交即 GPUI 改动，五条车道都不编它 |
| **hostVersion 恒为 0.0.1** | 插件版本兼容判断失效 | 发布第一个 GPUI 版本时 | s4：`extensions/src/core_bridge.rs:7` |
| **工具链漂移静默破坏构建** | `gpui-pre` 重基后在新 rustc 上失败 | CI runner 的 stable 追上 | s4：无 rust-toolchain，10 处 `@stable` |
| **未公证的 macOS 产物被 Gatekeeper 拦** | 用户装不上 | 用户首次打开 | s5：全仓无 notarytool/stapler |
| **`verify-runtime-bundle-immutability.sh` 变空扫描** | 违反 AGENTS.md 的运行时只读边界要求 | 删除旧两端后 | 脚本 `:13`/`:26` 只扫旧端路径 |
| **Windows 无更新器** | 无自动更新（对照旧端有 Tauri updater） | 第一版发布 | `menu_bar.rs:928` `MISSING_UPDATER`（主理人 #43） |

---

## 六、建议与验收

| 优先级 | 行动 | 完成标准 | 人日 |
|---|---|---|---|
| **P0** | **扩展 `classify-ci-changes.sh`**：给 `rust/lithe-gpui/*`、`rust/lithe-ui/*` 加显式分支，置一个新的 `gpui` 开关；在 `ci-windows.yml` 新增构建 job 跑 `cargo build -p lithe-gpui-app` + `cargo test` | 改一行 GPUI 代码会让该 job 运行；**故意引入一个编译错误时 CI 变红** | 1 |
| **P0** | **扩展 `verify-platform-feature-matrix-change.mjs` 的 `platformPaths`** 纳入 `rust/lithe-gpui`、`rust/lithe-ui` | 改 GPUI 代码且不动能力 JSON 时门禁报错 | 0.5 |
| **P1** | Windows release 链路（含版本注入、落位 LanguageServers、签名、打包、校验） | CI 产出带签名的 `Lithe.exe` + SHA-256；`signtool verify /pa` 通过 | 4 |
| **P1** | 补版本注入（N1），含 `hostVersion` | 产物内 `hostVersion` 等于 release tag；`LitheBuildGitRevision` 等键存在 | 1.5 |
| **P2** | 钉工具链（N2） | 两套 workspace 各有 `rust-toolchain.toml`；CI 用 `@1.x.y` 而非 `@stable` | 0.5 |
| **P2** | macOS M1 可行性验证 | macOS 上 `cargo build -p lithe-gpui-app` 成功或给出明确失败原因 | 1 |
| **P3** | macOS 全链路（M2–M8） | CI 产出签名+公证的 `.app` 与 DMG；`spctl --assess` 通过 | 8 |
| **P3** | 扩展 `verify-runtime-bundle-immutability.sh` 覆盖 GPUI | 脚本扫描 `rust/lithe-gpui` 的生产路径；在 GPUI 里故意写 bundle 会失败 | 1 |

**总计 P0–P3 约 17.5 人日**（其中 macOS 占 9）。

**必须先做人决策的两点**（我不自行决定）：

1. **macOS 的自动更新怎么办？** 旧端用 Sparkle（`release-macos.yml:178-186`）。
   GPUI 侧没有等价物。选项：(a) 第一版不提供自动更新；
   (b) 把 Sparkle 接到 Rust 应用（工作量未估，需评估 `gpui-pre` 与 Sparkle 的集成）。
2. **Windows 的分发格式？** 当前只有裸 exe。选项：(a) 先发 zip（1 人日）；
   (b) 建 NSIS/MSI 安装器（3～4 人日）。

---

## 七、未覆盖范围（必须标注的限制）

1. **本工作区是 Windows 主机。所有 macOS 与 Linux 结论均为只读配置/代码，
   未在任何 macOS / Linux 机器上执行过。**
2. **未验证 `gpui-pre` 在 macOS 上能否构建**——与主理人 §4 第 1 条同为最大单点未知，
   我同样未新增证据。这就是 M1 必须排第一的原因。
3. **未实际触发任何 CI 运行。** 全部结论来自配置文件与脚本原文；
   s1 的「假绿灯」另用 `reports/classify-gpui-changes.ps1` 对基线提交的真实改动路径做了
   逻辑复现（因本工作区 **bash 不可用**——`bash -c "echo OK"` 返回乱码且 exit=1——
   故未能直接运行原分类器脚本）。
4. **未验证 release profile 下的 DLL 情况。** 本机只有 debug 产物
   （`debug/` 顶层 0 个 DLL，38 个在 `deps/`）。
5. **未核对 `assets/app-icon/` 下有无 `.icns`**，以及 macOS bundle 图标如何处理。
6. **未核对插件宿主契约的 `hostVersion` 消费方**（在 `windows/tauri/src/features/host-api`）。
   **建议交给插件扩展工程师核对。**
7. 未核对 `.github/workflows` 中 `ci-plugins.yml`、`ci-database.yml`、`ci-macos.yml`
   等其余工作流的完整 job 图（已确认无 `lithe-gpui` 引用，不影响结论）。
8. 未验证 CI runner 上当前 stable 的确切 rustc 版本（本机为 1.97.0）。

---

## 八、交接给主理人

| 需核对的对象 | 具体问题 |
|---|---|
| **插件扩展工程师（柯展之）** | `extensions/src/core_bridge.rs:7` 的 `hostVersion` 消费方在哪、怎么比较？版本号注入后这个契约是否需要同步改？ |
| **界面设计师 / 键位（陆井然）** | 主理人 §2.7 的 `secondary-` 归一化推断，需要在 M8 的 macOS 实机验收里验；我的 M8 只负责「能不能启动并打点」，键位语义需他确认 |
| **核心开发工程师（娄码通）** | 发布 profile 的 strip/lto 对构建时长的影响（与他的 §2.2 优化项耦合）；`panic` 策略决策 |
| **主理人** | 上面第六节的两个决策点（macOS 自动更新、Windows 分发格式）；以及 A1「是否新增 `gpui` 平台字段」——**我的 P0 行动依赖它**，因为矩阵门禁扩展后要求回填，而回填必须先有 `gpui` 字段 |

**未覆盖、需他人补的：** macOS 实机的一切验证、Linux 门 G3、`.icns` 资产核查、
`gpui-pre` 在 macOS 上的真实行为。
