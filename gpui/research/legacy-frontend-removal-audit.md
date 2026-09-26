# 删除旧前端（`windows/` + `macos/`）前置审计

> 审计对象：仓库 `D:\developmentProjects\rust\Lithe-IDEA`，分支 `feat/gpui-shell-rewrite`。
> 审计时间基线：`cc26aecd`（工作区另有未提交改动：`gpui/assets/README.md`、`gpui/crates/shared/src/icons/idea.rs`、
> `gpui/tools/generate-idea-icons.mjs` 修改，`gpui/assets/ui-icons/idea-assets.generated.ts` 已删除）。
>
> **本次审计为只读**：未修改、未移动、未删除 `windows/` 或 `macos/` 下任何文件，未改任何代码，
> 未跑 `cargo`，未跑 CI 脚本，未启动应用。唯一产物就是本文档。
>
> **结论先行（6 条）**：
> 1. **`gpui/` 的编译与运行完全不依赖旧前端**（无 build script / `include_*!` / 资源路径 / Cargo 依赖指向它们）。
> 2. **1 个硬阻塞项**：`gpui/tools/extract-locale.mjs` 读 `windows/tauri/src/i18n/{locale,ai-commit}.ts`，删了就不能再生成/校验 locale。
> 3. **1 个半阻塞项**：`gpui/tools/generate-idea-icons.mjs` 读 `windows/tauri/scripts/idea-icon-mappings.json`；删了会**静默改变产物**（16 个别名消失 + 至少 2 个常量改名）并让 `--check` 变红。
> 4. **删目录的那个 PR 必定红**：四个主线工作流的 `pull_request` 都没有 paths 过滤；删 `macos/` 时分类器的 `*)` fallback 还会把**全部 lane** 点亮（§2.2）。
> 5. **`gpui/` 还缺的不止图标**：字体（JetBrains Mono 无副本）、**11 个主题族 / 33 条主题 + 36 个 legacy 语义色 key**、`material` 图标主题、macOS 侧 Markdown 预览/工作台背景/IDEAIcons/gutter 标记等（§3）。
> 6. **两个独立于本次删除的风险**（本审计发现）：`.github/**` 里 **grep `gpui` 零命中**（新前端零 CI 覆盖）；`.agents/notes/**` 有 136 条 "适用范围" 引用 + 9 条指向 `macos/` 的相对链接，删目录会让 Agent Note 校验（含 preview 分支）变红。

---

## 0. 一句话摘要

| 问题 | 结论 |
| --- | --- |
| `gpui/` 构建/运行期是否真的需要旧前端？ | **否**。`gpui/` 下没有 build script、`include_*!`、资源路径或 Cargo 依赖指向 `windows/` 或 `macos/`（证据见 §1.1）。 |
| `gpui/` 里是否还有读旧前端的工具？ | **是，2 个**：`gpui/tools/extract-locale.mjs`（**硬依赖**，删了就不能再生成/校验 locale）与 `gpui/tools/generate-idea-icons.mjs`（软依赖，但删了会**静默改变产物并让 `--check` 变红**）。见 §1.2。 |
| 仓库其它地方删了会不会红？ | **会，而且会阻塞每一个 PR**。四个主线工作流（`ci-windows`/`ci-macos`/`ci-plugins`/`ci-database`）的 `pull_request` **都没有 paths 过滤**；`ci-windows` 的 `frontend`+`rust-tests`+`gate`、`ci-macos` 的 `swift-tests`+`rust-core-tests`+`release-build`+`gate` 必红。另有 `deploy-agent-notes-board`（`macos/Resources/AppIcon.png` 被 build script 硬引用）与四个发布工作流。逐条见 §2。 |
| 分类器会不会因此放大失败面？ | **会**：删 `macos/` 时 `macos/EditorFrontend/**` 与 `macos/Experiments/**` 不匹配任何 `macos/` 模式，落到 `classify-ci-changes.sh:394-398` 的 `*)` → `enable_all_validation` → **全部 lane 被点亮**（从"部分红"升级为"全量红"）。见 §2.2。 |
| `gpui/` 是否还缺旧前端的资源？ | **缺，且不止图标**：① 字体（JetBrains Mono 4 TTF 无副本；Windows 的 Geist 系列没有文件对象）② **11 个主题族 / 33 条主题 + 36 个 legacy 语义色 key**（`builtin/themes/` 35 条主题，gpui 只有 2 条）③ `material` 图标主题 ④ macOS `Resources/**` 的 Markdown 预览渲染器 + 20 个 KaTeX 字体、2 张工作台背景图、SyntaxHighlighting 两个映射 JSON、DatabaseIcons、AppIcon-source.png ⑤ `MavenIcon`/`RunIcon` 内联 SVG。**明确不需要**：`public/tree-sitter/**`（55.9 MB，WebView 专属）、`{android,ios,dev,preview,prod}` 图标、MSIX 图标、`minimal` 主题、Lucide（gpui 自带）。见 §3。 |
| 最大风险 | 删掉后**再也无法**用旧前端做真机行为对照 / Windows 侧测试 / Tauri 构建；`.agents/notes/**` 里 136 条"适用范围"引用 + 9 条指向 `macos/` 的相对链接会让 Agent Note 校验（含 preview 分支）变红；且 `gpui/` **当前没有任何 CI lane**。见 §5、§2.6。 |

---

## 1. `gpui/` 内部对 `macos/`、`windows/` 的引用

### 1.1 「构建/运行期真的需要」——**0 处**

逐一检查了构建期能"看见"文件系统的所有入口，**没有任何一处指向旧前端**：

| 检查项 | 命令/范围 | 结果 |
| --- | --- | --- |
| build script | `glob gpui/**/build.rs` | 只有 `gpui/crates/app/build.rs`；它引用的是 `lithe.rc` 与 `gpui/assets/icons/icon.ico`（`gpui/crates/app/build.rs:15-16`），**无旧前端路径**。 |
| 编译期内嵌 | grep `include_str!` / `include_bytes!` / `include!` / `RustEmbed` in `gpui/**/*.rs` | 命中 14 处，指向 `gpui/assets/**`、`gpui/themes/**`、`third_party/jdtls/manifest.json`。**无一指向 `windows/` 或 `macos/`**。 |
| Cargo 清单 | grep `macos/`、`windows/` in `gpui/**/*.toml` | **0 命中**。 |
| 运行期路径字面量 | grep `"macos/`、`"windows/` in `gpui/**` | **4 命中，全部在 2 个工具的源码里**（`extract-locale.mjs:34,35,405`、`generate-idea-icons.mjs:60`）。Rust 侧 **0 命中**。 |
| 主题监视目录 | `gpui/crates/settings/src/theme.rs:33` | `concat!(env!("CARGO_MANIFEST_DIR"), "/../../themes")` → `gpui/themes`，**不是**旧前端。 |
| 资产内嵌目录 | `gpui/crates/app/src/assets.rs:61` | `#[derive(rust_embed::RustEmbed)]` 指向 `gpui/assets/`，**不是**旧前端。 |

**结论：删除 `windows/`、`macos/` 不会让 `gpui/` 编译失败、也不会让它运行时找不到资源。**

### 1.2 「生成了 `gpui/` 内副本、但生成脚本仍读旧前端」——**2 个，且必须处理**

#### 1.2.1 硬阻塞：`gpui/tools/extract-locale.mjs`

- 读取（真源，硬编码绝对解析）：
  - `gpui/tools/extract-locale.mjs:34` → `windows/tauri/src/i18n/locale.ts`
  - `gpui/tools/extract-locale.mjs:35` → `windows/tauri/src/i18n/ai-commit.ts`
- 产物：`gpui/crates/shared/locales/lithe.zh-CN.yml`、`lithe.en.yml`（`gpui/tools/extract-locale.mjs:39,42-45`）。
- 行为：`loadCatalogs()` 直接 `readFileSync` 上面两个文件（`:256-257`）。文件不存在会抛 `ENOENT`，脚本以非 0 退出；
  `--check` 模式（`:329`、`:412-414`、`:434-436`）同样无法运行。
- **判断：这是唯一的"删了就不能再生成"的硬阻塞项。** 删掉 `windows/` 之后：
  1. 两份 YAML **不能再重新生成**，`--check` **不能再运行**（这条今天就在 `gpui/PLAN.md:1241` 被当作核心校验命令使用）；
  2. 手工维护 YAML 时必须同时改掉脚本里的 `LOCALE_TS`/`AI_COMMIT_TS`/`sourceLabel`，否则脚本永远是坏的；
  3. 好消息：产物本身是**全量快照**，文案不会丢——`gpui/crates/shared/locales/README.md:140-143` 已明确
     「本目录已经是 Windows 前端的全量文案快照……删掉 `windows/` 前端不会丢文案。真源被删除后，本目录即成为新的文案真源」。
- 建议（二选一，见 §4 步骤 4）：
  - **A（推荐）**：删除前把两个 TS 真源**原样复制**到 `gpui/crates/shared/locales/source/`（或 `gpui/tools/locale-source/`），
    改 `LOCALE_TS`/`AI_COMMIT_TS` 指向新位置，脚本与 `--check` 继续可用，`sourceLabel` 改成「已迁至 gpui 的历史快照」。
  - **B**：删掉脚本的"生成"语义，只在 `locales/README.md` 里写明「YAML 已是唯一真源，手工维护」，
    并把脚本标记为 `@deprecated`、删除其 `--check` 用法文档（涉及 `gpui/crates/shared/locales/README.md:13,27-28,141`、
    `gpui/PLAN.md:540,1107,1241`、`gpui/crates/shared/src/lib.rs:39`、`gpui/crates/shared/src/i18n.rs:5` 等）。

#### 1.2.2 半阻塞：`gpui/tools/generate-idea-icons.mjs`

- 真源（必需）：`gpui/assets/ui-icons/idea/**` 的 `.svg`（`gpui/tools/generate-idea-icons.mjs:54`）——**在 `gpui/` 内，不受影响**。
- 可选输入：`gpui/tools/generate-idea-icons.mjs:58-61` → `windows/tauri/scripts/idea-icon-mappings.json`。
- 产物：`gpui/crates/shared/src/icons/idea.rs`（`:62`）。
- 现状行为：文件缺失**不报错**，只打印 warning 并退化成"由文件名推导常量名、无别名"（`:108-114`）。**但退化会改变产物**：
  - 当前产物有 `ALL: [IdeaIcon; 79]` 与 `ALIASES: [(&str, IdeaIcon); 16]`
    （`gpui/crates/shared/src/icons/idea.rs:606-608` 区段的 `ALIASES` 声明）。
  - 因为规范名（canonical）取的是**同一 SVG 的所有显示名里字典序最小的那个**（`:101-105`、`:152-154`）：
    - `expui/general/chevronDown.svg` 的显示名是 `CaretDownIcon` 与 `ChevronDownIcon`
      （`windows/tauri/scripts/idea-icon-mappings.json:134`、`:162`）→ 规范名 `CARET_DOWN_ICON`
      （`gpui/crates/shared/src/icons/idea.rs:202`），`ChevronDownIcon` 是别名。
      **丢失映射后会变成 `CHEVRON_DOWN_ICON`。**
    - `expui/general/settings.svg` 的显示名是 `GearIcon` 与 `GearSixIcon`（同文件 `:379`、`:383`）→ 规范名 `GEAR_ICON`
      （`gpui/crates/shared/src/icons/idea.rs:340`），`GearSixIcon` 是别名。**丢失映射后会变成 `SETTINGS_ICON`。**
  - 于是：**16 个别名全部消失 + 至少 2 个规范常量改名**，而 `gpui/crates/shared/src/icons/idea.rs` 里
    `CARET_DOWN_ICON` / `GEAR_ICON` 等常量**已有真实调用点**（如 `gpui/crates/settings/src/dialog.rs`
    头部齿轮用真源图标）→ 重新生成会**直接编译失败**；不重新生成则 `--check`（`:289-302`）**必然以退出码 1 结束**。
- **判断：不是"删了不能生成"，而是"删了会静默产出与仓库里那份不一致的文件"。**
  它与 `extract-locale.mjs` 是同一类风险，必须在删除前一次性结清。
- 与 `windows/tauri/scripts/idea-file-icon-mappings.json` 的关系（**已核实**）：`generate-idea-icons.mjs` **只读 `idea-icon-mappings.json` 这一个 JSON**
  （`:17,58-61,108,115,280`），`idea-file-icon-mappings.json` 是 Windows 侧 `generate-idea-icons.ts` 的输入、gpui 侧**不读**。
- 建议（见 §4 步骤 3）：删除前把 `windows/tauri/scripts/idea-icon-mappings.json`（含 194 条映射）复制进 `gpui/`，
  例如 `gpui/tools/idea-icon-mappings.json`，并改 `MAPPINGS_JSON`（`:58-61`）指向它；
  同时把产物头部那句"由生成器按 `windows/tauri/scripts/idea-icon-mappings.json` 推导"
  （`gpui/tools/generate-idea-icons.mjs:280` → 落到 `gpui/crates/shared/src/icons/idea.rs:607`）改成新路径。

#### 1.2.3 已结算的先例（可作为两条建议的模板）

`gpui/assets/ui-icons/idea-assets.generated.ts` 曾经也被搬进 `gpui/`，**已被删除**，且
`gpui/assets/README.md:273-282` §7.5 明确记录了「生成器不读它，所以删除不影响再生成，`--check` 仍然退出码 0」。
注意：`windows/tauri/src/ui/icons/idea-assets.generated.ts` 那份**旧前端的原件仍在**（只读，未动），
删除 `windows/` 时它会一并消失——它不被任何人引用，**无需保留**。

### 1.3 「只是注释/文档里的『真源出处』引用」——会变成死链接

这些引用**不影响编译**，但删掉 `windows/` / `macos/` 后会全部指向不存在的路径。规模（`gpui/` 全树，排除 `target/`）：

| 类别 | 文件数 | 引用行数 |
| --- | --- | --- |
| Rust 源码注释（`gpui/**/*.rs`） | **32** | **150** |
| 自有 Markdown（`gpui/**/*.md`，排除上游 `gpui/docs/gpui-kit/`） | **19** | **605** |
| 工具与生成物（`extract-locale.mjs`、`generate-idea-icons.mjs`、两份 `locales/*.yml`） | 4 | 9 |
| **合计** | **55** | **约 764** |

Rust 侧最密集的文件（引用行数）：

| 文件 | 行数 | 示例（文件:行号） |
| --- | --- | --- |
| `gpui/crates/editor/src/editor_view.rs` | 36 | `:89-90` 标签宽度上限 200px 取自 `windows/tauri/src/styles/theme.css:123` |
| `gpui/crates/shared/src/icons/idea.rs` | 19 | `:607` 「别名由生成器按 `windows/tauri/scripts/idea-icon-mappings.json` 推导」 |
| `gpui/crates/editor/src/buffer.rs` | 10 | `:38` 「真机按 `ThemedFileIcon` + 图标主题选（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:246-251`）」 |
| `gpui/crates/settings/src/schema.rs` | 9 | `:6` 「真源 ……/config/default-settings.ts」 |
| `gpui/crates/workbench/src/title_bar.rs` | 8 | `:5` 容器结构取自 `title-bar.tsx:332-360` |
| `gpui/crates/workbench/src/activity_bar.rs` | 8 | `:9`、`:81`、`:137` |
| `gpui/crates/shared/src/i18n.rs` | 5 | `:5` 「项目的文案真源是 `windows/tauri/src/i18n/locale.ts`」 |

文档侧最密集的文件：

| 文件 | 行数 |
| --- | --- |
| `gpui/research/windows/04-theme-and-components.md` | 140 |
| `gpui/research/windows/05-terminal-run-debug.md` | 76 |
| `gpui/assets/README.md` | 56 |
| `gpui/research/windows/03-git-and-bottom.md` | 56 |
| `gpui/research/windows/01-shell.md` | 44 |
| `gpui/UI-MAP-WINDOWS.md` | 30 |
| `gpui/PLAN.md` | 23 |
| `gpui/docs/grill.md` | 20 |
| `gpui/research/icon-asset-inventory.md` | 16 |
| `gpui/crates/shared/locales/README.md` | 14 |
| `gpui/docs/archive/ui-map-macos.md` | 14（**整篇以 macOS 为出处，删 `macos/` 后基本全失效**） |
| `gpui/research/app-icon-and-assets.md` | 12 |

**建议的处理策略（分两类，不要一刀切）：**

1. **有独立价值的"换算结论"引用 → 保留文字 + 加"已删除"标注。**
   这一类的正文（数值、px、换算、被否方案、坑）本身就是 gpui 侧的知识，只有"出处"那一小段会死。
   做法：不改正文，在文档头部或首次出现处加一句统一声明，例如
   `> ⚠️ 本文件引用的 windows/… 、macos/… 路径已在 <删除日> 随旧前端一并删除；行号对应 git 历史中的最后一次提交（见 §4 的 tag/commit 记录）。`
   建议逐个补齐的清单见 §4.6「死链接清单」。
2. **`gpui/docs/archive/ui-map-macos.md` → 明确归档标注或整体退休。**
   `gpui/docs/README.md:98` 已经写明它的命运：「**移到 `docs/archive/`** …… 等 `macos/` 删掉时一并处理」。
   删 `macos/` 时应当**同一次改动**把它标成「历史归档，出处已删除」，而不是留着 14 处悬空行号。
3. **不要改指向 `gpui/` 内的"副本"**——因为**没有副本**。旧前端的 TS/Swift 源码并未被复制进 `gpui/`，
   `gpui/` 里的对应物是**重新实现**（Rust），不是逐行副本。所以「改指向 `gpui/` 内的副本」这个选项**不成立**；
   能落在 `gpui/` 里的只有三类**数据**：locale YAML（已迁）、图标 SVG（已迁）、主题 JSON（已迁）。见 §3。

---

## 2. 仓库其它地方对两个旧前端的依赖

### 2.0 结论速览：删除 PR 必定变红的 CI

| 工作流 | 触发 | 变红的 job | 是否阻塞每个 PR |
| --- | --- | --- | --- |
| `ci-windows.yml` | `pull_request`（**无 paths 过滤**，`:3-8`） | `frontend`、`rust-tests`、`gate` | **是** |
| `ci-macos.yml` | `pull_request`（**无 paths 过滤**，`:3-8`） | `swift-tests`、`rust-core-tests`、`release-build`、`gate` | **是** |
| `ci-plugins.yml` | `pull_request`（无 paths） | `plugin-tests` + `gate`（条件性） | 是（条件性） |
| `ci-database.yml` | `pull_request`（无 paths） | `swift-database-tests` + `gate`（条件性） | 是（条件性） |
| `deploy-agent-notes-board.yml` | `push: [preview]`（`:3-6`） | `verify-agent-notes.mjs`、`build-agent-notes-board.mjs` | 否（合并后 preview 变红） |
| `release-{macos,windows}.yml`、`release-preview-{macos,windows}.yml` | tag / cron | 整条发布流水线 | 否 |

### 2.1 CI 工作流逐条（`.github/workflows/*.yml`）

- **四个主线工作流的 `pull_request` 触发都不带 `paths:` 过滤**（`ci-windows.yml:3-8`、`ci-macos.yml:3-8`、
  `ci-plugins.yml:3-8`、`ci-database.yml:3-8`）→ **删除旧前端的 PR 会跑满全部 lane**。
- `ci-windows.yml`（job `frontend` 的 `if: needs.changes.outputs.windows == 'true'`，`:106`）：
  | 行号 | 内容 | 删掉后 |
  | --- | --- | --- |
  | `:118` | `bun-version-file: windows/tauri/package.json` | **红**（必读输入缺失） |
  | `:151` | `Get-Content -Raw -LiteralPath windows/tauri/package.json`（pwsh 默认 `$ErrorActionPreference='Stop'`） | **红** |
  | `:167` | → `scripts/install-windows-frontend-dependencies.ps1`（脚本 `:4` `Stop`、`:7` 读同一 `package.json`） | **红** |
  | `:170` | `working-directory: windows/tauri`（下一步 `bun run typecheck`） | **红** |
  | `:176` | → `scripts/verify-windows-boundaries.ps1`（见 §2.3） | **红** |
  | `:182-246` | 全部 `bun test` 步骤用相对源路径，依赖 `windows/tauri` 工作目录 | **红** |
  | `:118,151` 的根因 | bun 版本号 `packageManager: bun@1.3.12` **只写在 `windows/tauri/package.json`**，根 `package.json` **没有** `packageManager` 字段 | 删除时必须把 `packageManager` 搬到根 `package.json` |
  | `:297,315` | `hashFiles('windows/tauri/**')` | 不报错（推断），仅缓存键退化 |
  | `:322` | → `scripts/validate-windows-build-caches.ps1`（**未传 `-IncludeWindowsAssets`** → 脚本 `:94-103` 只 warning + `:113 exit 0`） | **不红** |
  | `:337` | `test-stability-windows.ps1 -Scope WindowsRust`（脚本 `:141-157` 用 `-Manifest windows/tauri/src-tauri/Cargo.toml`） | **红** |
  | `:435-473` | job `gate`（`if: always()`），要求被选中 lane 全 success | **红**（传导） |
  | `:35` | `node scripts/test-verify-download-cache.mjs` | 只读 workflow 与 `scripts/*` 文本 → 只删旧前端目录时 **不红**；若同批删 `ci-windows.yml` 或被读脚本则 **红** |
  | `:83` | `verify-test-stability.sh --platform windows` | 对删除路径 `readFileSync` 失败被 `catch { continue }` 吞掉 → **不红** |
- `ci-macos.yml`（job `swift-tests` 的 `if: swift=='true'`，`:137`）：
  | 行号 | 内容 | 删掉后 |
  | --- | --- | --- |
  | `:156-164`、`:177`、`:186`、`:195` | → `test-stability-macos.sh` → `scripts/test-macos.sh:34` `swift test`；根 `Package.swift` 的 target `path:` 全指向 `macos/Sources\|Tests/**` | **红**（SwiftPM 报目标路径缺失） |
  | `:171` | `test-git-performance-baseline.sh`（`git_validation=='true'` 时，`:167`） | **红** |
  | `:242` | → `./scripts/verify-rust-core.sh`（其 `:33-49` `swift build`、`:54-55` `swiftc … macos/Sources/LitheRustCore/bridge.c`） | **红** |
  | `:288` | → `./scripts/probe-macos-monaco.sh`（其 `:37,39,54` 用 `macos/Experiments/Monaco/*`） | **红** |
  | `:326` | `plutil -lint macos/Resources/Info.plist` | **红** |
  | `:335` | → `./scripts/verify-macos-package.sh`（内部走 `package-app.sh`） | **红** |
  | `:351`、`:352`、`:353` | `package-app.sh`、`create-dmg.sh`、`PlistBuddy … macos/Resources/Info.plist` | **红** |
  | `:404-438` | job `gate`（`require_selected_job`） | **红**（传导） |
  | `:37`、`:80`、`:87`、`:94` | `test-classify-ci-changes.sh`、classifier、`git diff --check`、`verify-test-stability.sh --platform all` | **不红**（前者在 `mktemp` 临时仓库自建 fixture，脚本 `:6-11`） |
- `ci-plugins.yml` / `ci-database.yml`：只有当删除路径命中 `plugins/mac/*`（classifier `:149`）、
  `macos/tests/lithetests/*plugintests.swift`（`:158`）、插件宿主源码（`:161`）或 `macos/sources/lithedatabasemodule/*`（`:172`）时才运行；
  运行即 `swift test` → **红**。若 `plugins=false` / `swift_database=false`，job skipped，gate 接受 skipped → **不红**。
- 发布工作流（tag / cron 触发，不在 PR 上，但合并后必然坏）：
  | 行号 | 内容 |
  | --- | --- |
  | `release-windows.yml:70` | `bun-version-file: windows/tauri/package.json` → **红** |
  | `release-windows.yml:155-162` | → `validate-windows-build-caches.ps1 … -IncludeWindowsAssets`（**带这个开关** → 脚本 `:77-79` 读 `package.json`、`:81` `throw`） → **红** |
  | `release-windows.yml:225` | → `./scripts/package-windows.ps1`（`:18,136,140,147-148`） → **红** |
  | `release-preview-windows.yml:58,143-150,191` | 同上 → **红** |
  | `release-macos.yml:129,130`、`release-preview-macos.yml:109,110` | `package-app.sh`、`create-dmg.sh` → **红** |
- `deploy-agent-notes-board.yml`（`push: [preview]`）：
  | 行号 | 内容 |
  | --- | --- |
  | `:34` | `node scripts/verify-agent-notes.mjs` → **红**（`scripts/verify-agent-notes.mjs:242` 对每篇 active note 调 `validateScopePaths` `:148-158`；`:231-234` 还校验 Markdown 相对链接存在） |
  | `:40` | `node scripts/build-agent-notes-board.mjs` → **红**（脚本 `:18` `logoPath = resolve(rootDir, "macos/Resources/AppIcon.png")`、`:136-138` 不存在即 `throw new Error("Lithe Logo 不存在：…")`） |
- `verify-agent-notes.yml` 是唯一带 `paths:` 的工作流（`:4-12`）→ 单纯删目录**不触发它**；
  但删除落地后任何触碰 `.agents/notes/**` 的 PR 都会 **红**。
- `lithe-pr-review.yml:43` 的 `scripts/test-prepare-lithe-pr-review.mjs:17-19,29,40,50` 里虽有 `windows/`、`macos/` 字符串，
  但只是内存 fixture、不读盘 → **不红**。
- 与本次无关、**零风险**：`lithe-issue-claim.yml`、`lithe-issue-priority.yml`、`update-repo-charts.yml`、`sync-atomgit-release.yml`。
- **重要缺口（独立风险）**：`.github/**` 里 **grep `gpui` 零命中**——`gpui/`（新前端）**目前没有任何 CI lane**。
  旧 lane 删掉后，仓库对新前端零覆盖（`rust/lithe-core` 仍被覆盖，但 gpui 外壳不被覆盖）。

### 2.2 `scripts/classify-ci-changes.sh`（CI 的路径分类器）——**比"变红"更棘手**

删除项的 `git diff --name-status` 状态是 `D*`，**不是** `R*`/`C*`，所以 `:68-73` 的 rename/copy 全量升级分支**不触发**。实际行为：

- **只删 `windows/`**：`windows/tauri/src-tauri/*.rs`/`cargo.toml`/`cargo.lock`、`windows/tauri/crates/*` 命中 `:325-327`
  → `windows=true, windows_rust=true`；其余 `windows/**` 命中 `:329-330` → `windows=true`。**不会落到 `*)` fallback。**
- **删 `macos/`：会触发 `*)` fallback（`:394-398` → `enable_all_validation`，`:54-65`）**。
  原因**不是**"pattern 文件消失"，而是分类器自身的 `macos/` 模式有**覆盖空洞**——
  它只有 `macos/sources/*`、`macos/tests/*`、`macos/resources/*` 三类（对比 `windows/*` 在 `:329` 有兜底），
  于是 **`macos/EditorFrontend/**`（3 文件）与 `macos/Experiments/Monaco/**`（11 文件）共 14 个路径不匹配任何模式**。
  - 后果：**过度触发**——`swift`、`plugins`、`swift_database`、`rust_core`、`rust_database`、`macos_release`、
    `windows`、`windows_rust`、`java_jdt`、`git_validation` **全部为真**（不置 `rust_comments`、`metadata`）。
    于是从"部分红"升级为"全量红"：连 `ci-windows.yml` 的 `rust-tests`（`:260` 判 `rust_core=='true' || windows_rust=='true'`）
    与 `ci-database.yml` 的 `rust-database-tests` 都会被拉起。
  - `scripts/test-classify-ci-changes.sh:177-310` **没有任何"整目录删除"用例** → 这个 fallback 过度触发**当前无测试覆盖**。
- 结论（与 §4 的纪律一致）：**不能用"一个 PR 一次性删掉两个目录"落地**；必须先把 lane 与脚本切干净。
- 删完之后这些 `case` 模式变成**永不匹配的死模式**（本身不报错），可顺手清理，**不是阻塞项**。
- `scripts/test-classify-ci-changes.sh` 在 `mktemp` 临时仓库里自建 fixture（`:6-11,16-70`），
  **不依赖真实 `macos/`、`windows/` 目录** → 与本文档早先的怀疑相反，`ci-macos.yml:37` 这一步**不会红**。
- 分类器引用的脚本**全部位于 `scripts/`**，没有一个在 `windows/`、`macos/` 下 → 删目录不会让被引用的脚本文件消失。

### 2.3 `scripts/**` 里会红的构建/校验脚本

`(a)` = 会在 CI/验证里失败；`(b)` = 死链；`(c)` = 良性。

| 脚本:行号 | 引用 | 触发点 | 判定 |
| --- | --- | --- | --- |
| `scripts/verify-windows-boundaries.ps1:5,8,16,31,36,41` | `windows/tauri/**` | **`ci-windows.yml:176`** | (a) **红** |
| `scripts/verify-java-semantic-ownership.mjs:8-12` | `windows/tauri/src`（`statSync`） | `verify-windows-boundaries.ps1:5`、`scripts/verify-core.sh:10` | (a) ENOENT → **红** |
| `scripts/install-windows-frontend-dependencies.ps1:6-7,11,87` | `windows/tauri/**` | `ci-windows.yml:167` | (a) **红** |
| `scripts/build-windows.ps1:10-11,26,37,45,51,58,67` | `windows/tauri/**` | `debug-windows-on-parallels/SKILL.md:121` | (a) |
| `scripts/package-windows.ps1:18-19,136,140,147-148,159` | `windows/tauri/**` | `release-windows.yml:225`、`release-preview-windows.yml:191` | (a) **发布红** |
| `scripts/invoke-windows-tauri-build.ps1:10-11,23` | `windows/tauri/**` | `build-windows.ps1:53`、`package-windows.ps1:154` | (a) 间接 |
| `scripts/validate-windows-build-caches.ps1:55,75,78,81` | `windows/tauri/src-tauri/target`、`Cargo.lock`、`package.json`（`:81` `throw`） | `ci-windows.yml:322`（无开关 → 只 warning，**不红**）；`release-*.yml`（有开关 → **红**） | (a) 有条件 |
| `scripts/verify-windows-boundaries.sh:6-8,13,22,27-31` | `windows/**` | `README.md:223`、`develop-lithe/SKILL.md:241` | (a)（bash 版未接入 CI） |
| `scripts/verify-editor-boundaries.mjs:14,22,24-26,32` | `windows/tauri/package.json`，且 `:24-26` 显式断言 `workspaces.includes("windows/tauri")` | 未接入 workflow（`docs/ARCHITECTURE.md:229`） | (a) **必失败** |
| `scripts/package-app.sh:6-8,170-177` | `macos/Resources/{Info.plist,AppIcon.icns,IDEAIcons,GitGraph,DatabaseIcons,Fonts}` | `ci-macos.yml:335,351`、`release-macos.yml:129`、`release-preview-macos.yml:109` | (a) **红** |
| `scripts/create-dmg.sh:6-7` | `macos/Resources/Info.plist` | `ci-macos.yml:352` 等 | (a) **红** |
| `scripts/preview.sh:64,67-74` | `macos/Resources/*` | 手工/性能基线 | (a) |
| `scripts/verify-macos-app-build-safety.sh:7,45` | `macos/Sources/Lithe/Views/Workbench/WorkbenchView.swift`、`macos/Resources/Info.plist` | `scripts/build-macos.sh:21` | (a) |
| `scripts/build-editor.sh:5` | `macos/EditorFrontend/build.ts` | `scripts/build-macos.sh:22` | (a) |
| `scripts/probe-macos-monaco.sh:37,39,54` | `macos/Experiments/Monaco/{build.ts,main.swift,report.mjs}` | `ci-macos.yml:288` | (a) **红** |
| `scripts/measure-macos-performance-baseline.sh:101,107-114` | `macos/Resources/{Info.plist,AppIcon.icns,Fonts,IDEAIcons,GitGraph,DatabaseIcons}` | 手工基线 | (a) |
| `scripts/verify-rust-core.sh:55` | `macos/Sources/LitheRustCore/bridge.c` | `ci-macos.yml:242` | (a) **红** |
| `scripts/verify-shared-contracts.sh:26,250` | `macos/Sources/Lithe/Resources/SyntaxHighlighting/color-mappings.json` | `scripts/verify-core.sh:9`（**未接入 workflow**） | (a) Ruby ENOENT |
| `scripts/verify-module-boundaries.sh`（`:12,21,29,36,42,46,60,65,71-74,82,89-91,99,106,112,117,123-125,133,139,145-147,155,161,167-172,180,186,191,196,201,207-213,221,227,233,239,244,250,256,261,267,273,278,284,290,295,300,305,310,316,322`） | 大量 `macos/Sources/**`、`macos/Tests/**` | 未接入 workflow | (a) 手工运行必失败 |
| `scripts/verify-service-boundaries.sh:6,24-44` | `macos/Sources/Lithe/**` | `scripts/verify-core.sh:8` | (a) |
| `scripts/build-agent-notes-board.mjs:18,136-138` | `macos/Resources/AppIcon.png`（`:136-138` `throw`） | **`deploy-agent-notes-board.yml:40`** | (a) **preview 红** |
| `scripts/verify-download-cache.mjs:76-81,113` | 默认追加 `windows/tauri/src-tauri/Cargo.lock`；`:113` 无 exists 保护 | `validate-windows-build-caches.ps1:75`、`install-windows-frontend-dependencies.ps1:87` | (a) **被降级为 warning → 不红** |
| `scripts/test-verify-download-cache.mjs`（多处） | 只读 workflow 与 `scripts/*` 文本 | `ci-windows.yml:35` | (c) 文件在则 PASS |
| `scripts/classify-ci-changes.sh` 自身的 `windows/**`、`macos/**` 模式 | — | — | (b) 死模式，不报错 |
| `.agents/skills/write-stable-tests/scripts/run-bun-tests-with-timing.mjs:16,20` | 默认 `workingDirectory = windows/tauri` | `ci-windows.yml:182-246` 未传 `--working-directory` 的步骤 | (a) **红** |
| `.agents/skills/write-stable-tests/scripts/test-stability-windows.ps1:128,132,141-156` | `windows/tauri`、`windows/tauri/src-tauri/Cargo.toml`、`target` | `ci-windows.yml:203,246,331,337` | (a) **红** |

### 2.4 根级清单文件（最容易漏、且会硬失败）

| 文件:行号 | 内容 | 删掉后 |
| --- | --- | --- |
| `Package.swift`（整份；target 的 `path:` 全部是 `macos/Sources/**` / `macos/Tests/**`，约 30 处） | SwiftPM 包定义 | **`swift build` / `swift test` 立即失败**（"Source files for target … should be located under …"）。macOS 侧最硬的一条。 |
| `package.json`（根）`:4-7` `workspaces: ["frontend/editor", "windows/tauri"]` | bun workspace | 悬空 workspace；`scripts/verify-editor-boundaries.mjs:24-26` **显式断言**它存在 → 该脚本必失败。`bun install` 是否直接报错 **未确认**。 |
| `bun.lock:17`（`"windows/tauri": {`）、`bun.lock:1402`（`"lithe": ["lithe@workspace:windows/tauri"]`） | lock 悬空条目 | 需要同步；`--frozen-lockfile` 是否判过期 **未确认** |
| `bunfig.toml:1-3` | `linker = "isolated"` + 注释「Both platform packages…」 | 注释失效（(c)）；`scripts/verify-editor-boundaries.mjs:28-31` 会断言它 |
| `.gitignore:7` | `/windows/build*/` | (b) 死规则，不致失败 |
| `shared/contracts/rust-core-api.md:17` | 「The macOS package uses the small C bridge in `macos/Sources/LitheRustCore/`」 | (b) 死链接；属**跨平台契约**，应改写 |
| `shared/contracts/github.md:38` | 「The macOS product reads `LitheGitHubOAuthClientID` from `macos/Resources/Info.plist`」 | (b) 死链接 + **运行时配置事实**，删除前要确认 gpui 侧从哪读 client id |
| `shared/` 其余 | 只出现平台**名字**（`application-boundary.md:3-4,44-48,134-142,171,176,203,226,248,314,403,423` 等）或 fixture 里的平台枚举值（`shared/fixtures/maven/platform-contract-v1.json:16-17,23-24`、`diagnostics/redact-text-v1.json:39,47` 等） | (c) 良性；但 `scripts/verify-shared-contracts.sh:62` 要求 `storageIdentityCases` 同时覆盖 macos 与 windows → **需一并复核** |
| `infra/**` | `infra/docker/database-validation/*` 对 `windows\|macos\|swift\|tauri` **零命中** | (c) 完全无影响 |

### 2.5 根 `AGENTS.md` / `CLAUDE.md` / `.agents/skills/**`

- 根 `AGENTS.md` **没有任何 `windows/`、`macos/` 路径字面量**（grep 零命中），需要改的是**指针与语义**：
  - `AGENTS.md:7`（"macOS 和 Windows 测试"须遵守 `write-stable-tests`）、`:9`（指向 `release-lithe`）、
    `:11`（指向 `debug-windows-on-parallels`）。
- `CLAUDE.md:1-5` 只有"先读 `AGENTS.md`" + `@AGENTS.md` → **本身无需改写**。
- `.agents/skills/develop-lithe/SKILL.md`（本次任务加载的 Skill）需要改写的具体位置：
  - 所有权表 `:71-80`（`:71-76` 六个 `macos/Sources/**` 条目、`:78` `windows/`、`:79-80` `Plugins/{mac,win}/`）
  - `:86-87`（"macOS is the current reference product…"）、`:99-102`（`MacServiceContainer` 组合根）、
    `:110-113`（`@/platform/tauri-core`）、`:140-147`（`### Swift and macOS` 整节）、
    `:164`（Windows/Tauri Rust crates）、`:189-200`（`### Windows React and Tauri` 整节）
  - 验证表 `:235,241,242`（`test-macos.sh`、`verify-windows-boundaries.sh`、`build-windows.ps1`）；`:236,237,239` 需复核链
- `.agents/skills/debug-windows-on-parallels/SKILL.md`（`:3,13,121,130,140,152,171,187,204`）→ **建议整体退役**，并从 `AGENTS.md:11` 摘除。
- `.agents/skills/lithe-code-review/references/review-policy.md:66-67`、`.github/lithe-review/review-prompt.md:64-65`
  → 两处内容重复，需同步改写。
- `.agents/skills/release-lithe/SKILL.md:31,60-63,80,84`、`.agents/skills/write-stable-tests/**`（`SKILL.md:3,13,16,83-85` 等）
  → 按新平台范围改写。

### 2.6 一个容易被漏掉的连带面：`.agents/notes/**`（Agent Note 校验会红）

- `scripts/verify-agent-notes.mjs:242` 对每篇 active note 调 `validateScopePaths`（`:148-158`：路径不存在即 `fail`），
  `:231-234` 还校验 Markdown 相对链接的目标是否存在。
- **"适用范围"里直接列出 `macos/`/`windows/` 的 bullet：136 处**（grep `^- \`(macos|windows)`）。
  已抽验两篇确在 `## 适用范围` 段内：`proposed/architecture/2026-09-23-gpui-kit-three-platform-ui-rewrite-roadmap.md:398-401`、
  `implemented/process/2026-09-13-ci-build-cache-and-artifact-strategy.md:141,153-154`；
  其余分布见子代理清单（`2026-09-13-repository-ownership-and-sharing-boundaries.md:249-258`、
  `2026-09-13-macos-service-composition-boundaries.md:238-244`、`2026-09-22-windows-ai-commit-design.md:90-92` 等 20+ 篇）。
- **指向 `macos/` 的 Markdown 相对链接：9 处**（`](../../../../macos/`）：
  `proposed/bug-fix/2026-09-13-single-repository-git-status-observation.md:16,122,152,188`、
  `implemented/architecture/2026-09-13-language-tooling-and-lsp-runtime-ownership.md:239`、
  `implemented/feature/2026-09-13-macos-git-graph-intellij-layout.md:31,113,117,123`。
- 触发点：`deploy-agent-notes-board.yml:34`（push preview → 必然红）；`verify-agent-notes.yml:36`（PR 动到 notes 时）。
- **这是 §4 步骤 5 必须新增的一行**：删除旧前端必须**同批**清理 `.agents/notes/**` 的适用范围与链接，
  否则 `deploy-agent-notes-board` 与 `verify-agent-notes` 都会红。

---

## 3. 只在旧前端里、`gpui/` 没有副本的资源（差集）

> 本节把"最近几次迁移已经做过什么"与"还剩什么"做差集。
> 已迁（有 `gpui/` 副本，删除旧前端不会丢）的分组见 §3.1；**无副本**的分组见 §3.2。

### 3.1 已迁完（差集为 0，删除安全）

| 资源 | 旧前端位置 | `gpui/` 副本 | 证据 |
| --- | --- | --- | --- |
| locale 文案（zh-CN + en 全量） | `windows/tauri/src/i18n/locale.ts`、`ai-commit.ts` | `gpui/crates/shared/locales/lithe.{zh-CN,en}.yml` | `gpui/crates/shared/locales/README.md:140-143`：「已经是 Windows 前端的**全量**文案快照……删掉 `windows/` 前端不会丢文案」 |
| 应用图标 8 个位图 | `windows/tauri/src-tauri/icons/{32x32,64x64,128x128,128x128@2x,icon}.{png,ico,icns}`、`windows/tauri/public/logo.png` | `gpui/assets/icons/**`（7）+ `gpui/assets/images/logo.png`（1） | `gpui/assets/README.md:13-27`（8 文件 / 2 925 738 B，逐字节 SHA256 一致） |
| UI 图标（IntelliJ `expui`） | `windows/tauri/src/ui/icons/**`（157 SVG） | `gpui/assets/ui-icons/**`（157） | `gpui/assets/README.md:130-137,159-163` |
| 图标主题 4 套（lithe / symbols / pierre / idea） | `windows/tauri/src/extensions/bundled/icon-themes/{lithe,symbols,pierre,idea}/**`（1037 文件） | `gpui/assets/icon-themes/**`（1037） | `gpui/assets/README.md:130-137,159-163`（1194/1194 全量 SHA256 命中） |
| 主题色数据 | `windows/tauri/src/extensions/themes/builtin/lithe.json`、`styles/theme.css` | `gpui/themes/lithe-{light,dark}.json` | `gpui/themes/README.md:224-289`（逐键对照表；含 17 项降级/合并记录） |

### 3.2 `gpui/` 无副本——需要决策

| # | 资源 | 旧前端位置 / 规模 | `gpui/` 现状 | 建议 |
| --- | --- | --- | --- | --- |
| 1 | `material` 图标主题 | `windows/tauri/src/extensions/bundled/icon-themes/material/`（2 文件 / 527 307 B，美术**内联**在 `extension.json` 的 `iconDefinitions` 里，0 个 `.svg`） | **无副本** | **建议补迁**（成本极低：只有 `extension.json` + `LICENSE`）。它是 Windows **已注册**的内置主题（`bundled-icon-theme-assets.ts` 的 glob 含 `material`），且 `gpui/assets/README.md:246` 自己标了 ⚠️「将来 gpui 若要支持它，得从这份 `extension.json` 里抽字符串」——不迁就**永久丢失**。 |
| 2 | `minimal` 图标主题 | `windows/tauri/src/extensions/bundled/icon-themes/minimal/`（3 SVG / 947 B） | **无副本** | **可明确不需要**：`gpui/assets/README.md:247` 已确证全仓无引用（`bundled-icon-theme-assets.ts` 的 glob 是 `{idea,material,pierre,symbols}`）。 |
| 3 | tree-sitter 语法高亮资源 | `windows/tauri/public/tree-sitter/**` = **96 文件 / 55 913 780 B**（48 `.wasm` 55 813 057 B + 48 `.scm` 100 723 B）；`parsers/` 下 **48 个语言目录**（47 个有 `parser.wasm`，`dotenv/` 只有 `highlights.scm`）+ 顶层 `tree-sitter.wasm` 205 488 B | **无副本**（`gpui/assets/tree-sitter` 不存在；`gpui/crates/**` 里 `tree-sitter\|\.wasm\|scm` 命中 **0**） | **明确不需要（WebView 专属产物）**：消费方是 WebView 的 `web-tree-sitter`（`windows/tauri/src/features/editor/lib/wasm-parser/loader.ts:7,18`、`extension-assets.ts:77,79`；`language-packager.ts:20,360`、`full-extensions.ts:20,112` 的 `BUNDLED_PARSER_BASE_URL = "/tree-sitter/parsers"`）。gpui 走**原生 Rust tree-sitter + gpui-kit cargo feature**（`gpui/crates/editor/Cargo.toml:8` → `gpui-kit = { features = ["tree-sitter-java"] }`，查询来自 grammar crate 自带 `tree_sitter_java::HIGHLIGHTS_QUERY`，见 `gpui/research/editor-syntax-highlighting.md` §1.4/§2.3）。⚠️ **唯一缺口**（约 18.9 KB，值得单独记一笔）：gpui-kit 的 `tree-sitter-languages` 只覆盖 35 种，**以下 17 种语言只有旧前端有 `.scm`**：`dart, dockerfile, dotenv, elisp, elm, nix, objc, ocaml, ql, r, rescript, solidity, systemrdl, terraform, tlaplus, vue, xml`（其中 objc/ql/rescript/solidity/systemrdl/tlaplus 是 TODO 空壳；dockerfile/terraform/xml/dart/ocaml/r 有实际内容且可从上游再取）→ 若 gpui 将来要支持这批语言，**只需补迁 `.scm`（不必迁 wasm）**。 |
| 3b | 完整主题族（**11 个族 / 33 条主题**） | `windows/tauri/src/extensions/themes/builtin/`：**12 个 JSON / 38 585 B / 35 条主题**（ayu、catppuccin、christmas、contrast-themes、dracula、github、lithe、nord、one、solarized、tokyo-night、vitesse）+ `extensions/bundled/themes/` 9 个 `extension.json` / 35 637 B（+ `vercel/manifest.ts`） | **无副本**：`gpui/themes/` 只有 `lithe-dark.json` + `lithe-light.json`，**只有 2 条主题**，且是 gpui-kit schema | **建议补迁（但不能直搬，需转 schema）**：逐 key 比对 `lithe.json` 的 lithe-dark 与 `gpui/themes/lithe-dark.json`——**同名 key 只有 3 个**（`background`/`foreground`/`border`），legacy 独有 **36 个**（`surface`、`subtle-foreground`、`accent`、`selected`、`cursor-vim-*`、`git-*`×6、`terminal-*`×16 …），其中 **18 个 `syntax` key 与 16 个 terminal key 在 gpui 侧零对应**。`gpui/themes/README.md:293-315` 已自认 17 类降级，`:321-333` 明确 `highlight` 段「本次故意不写」并把 `windows/tauri/src/extensions/themes/syntax-token-colors.ts:3-44,99-128` 的 18 色回落表列为 Rust 侧待实现项。**结论：删掉 `builtin/themes/**` 就等于永久失去另外 33 条主题的真源值。** |
| 4 | MSIX / 商店图标 | `windows/tauri/src-tauri/icons/Square*.png`（9）+ `StoreLogo.png`（10 个 PNG / 约 202 KB） | **无副本** | **明确不需要（当前）**：`gpui/assets/README.md:51` 记录了理由（gpui 侧无 MSIX 目标）。若将来做 MSIX 需重新导出。 |
| 5 | `{android,ios,dev,preview,prod}` 图标目录 | `windows/tauri/src-tauri/icons/{android(18),ios(18),dev(50),preview(50),prod(50)}`，合计约 11.26 MB+ | **无副本** | **明确不需要**：`gpui/assets/README.md:48-50` 已证：android/ios 非桌面目标；dev/preview/prod 三套**互相逐字节相同**且是**过期旧美术**（`32x32.png` 2458 B vs 主目录 2219 B），全仓无引用。 |
| 6 | **JetBrains Mono 字体（4 个 TTF / 约 1.11 MB）** | `macos/Resources/Fonts/`：`JetBrainsMono-{Regular,Bold,Italic,BoldItalic}.ttf`（273 900 + 277 828 + 276 840 + 279 832 B）+ `OFL.txt`（4 398 B）；由 `macos/Sources/Lithe/Platform/MacOS/UI/MacBundledFontRegistry.swift:6-11,21` 用 CoreText 注册 | **无副本**（`gpui/**` 下 `*.ttf/*.otf/*.woff/*.woff2` **0 命中**；gpui 侧没有任何 `register_fonts` 调用，只读 `cx.theme().mono_font_family`，见 `gpui/crates/git/src/log_view.rs:957,1731,1744,1845`、`gpui/crates/terminal/src/terminal_view.rs:333,492`） | **建议补迁，理由充分**：真机（macOS）的等宽字体就是这份 JetBrains Mono（随 `.app` bundle 分发）；字族名 `JetBrains Mono` 取自 `lithe.json`；gpui 只能用系统等宽字体，**观感会与真机不同**。⚠️ `OFL.txt` 必须一起迁（许可归属）。迁入后需要一处 `register_fonts` 接线 + 在主题里写 `mono_font.family`。 |
| 6b | Geist Sans / Geist Mono（Windows 侧界面字体） | `windows/tauri/src/styles/theme.css:109-111` 的 `--editor-font-family: "Geist Mono"`，来自 npm `@fontsource/geist-mono` / `geist-sans`（`windows/tauri/package.json:35-36`，`src/styles.css:1-8` `@import`） | **仓库内没有文件对象**（`node_modules` 当前不存在，字体是 Vite 构建期产物） | **不可搬，需要决策**：删除 `windows/` 后若 gpui 要复刻同一字体，只能从 npm 重新取 `@fontsource/geist-mono`/`geist-sans`（版本号记录在 `windows/tauri/package.json:35-36`，**删除前请把这两个版本号抄进 `gpui/research/`**），或改用系统字体。 |
| 7 | macOS IDEAIcons（**144 文件 / 150 908 B**） | `macos/Resources/IDEAIcons/**`：`debugger/` 50、`fileTypes/` 36、`nodes/` 19、`testState/` 8、`toolwindows/` 8、`actions/` 5、`gutter/` 4、`vcs/` 3、`general/` 2、`maven/` 2 + `LICENSE-APACHE-2.0.txt`、`NOTICE.txt` | 与 `gpui/assets/ui-icons/idea/expui/**`（157）**不是同一套**：gpui 那套是 **expui UI 图标**（按 `expui/{actions,bookmarks,fileTypes,general,ide,image,javaee,nodes,run,toolwindows,vcs}` 组织），这里是**调试器 / 文件类型树 / testState / gutter 标记**专用图标 | **建议补迁（部分，需逐目录核对）**：`debugger/`(50)、`testState/`(8)、`gutter/`(4) 在 `gpui/assets/` 下**没有对应物**，是 JDT 调试与运行标记要用的（gpui 的 `gpui/crates/java/**`、`gpui/crates/editor/src/editor_view.rs` 都需要画这类标记）；`fileTypes/`(36) / `nodes/`(19) / `toolwindows/`(8) / `vcs/`(3) 可能与 expui 子集**部分重叠**，需逐张比对。⚠️ **注意**：早先凭文件名抽样以为这里是小驼峰命名（`execute.svg`/`moreVertical.svg`），实测目录结构如上——**不要按文件名想当然，要按子目录核对**。 |
| 7b | **编辑器 gutter 标记 SVG（8 个 / 4 819 B）** | `windows/tauri/src/features/editor/engines/monaco/gutter-icons/{green2,red2,run,run_run,implementedMethod,implementingMethod,overridenMethod,overridingMethod}.svg` | **无副本**（`gpui/assets` 里 `gutter`/`green2`/`red2`/`run_run` **零命中**） | **建议补迁（取决于是否画 gutter 标记）**：这是 Windows 侧 Monaco 的行号旁标记（运行标记、实现/覆写方法标记）。macOS 侧的对应物就是上面 `IDEAIcons/{gutter,testState}/`。若 gpui 要复刻「行号旁 IDE 风格运行/测试标记」（`windows/tauri` 有对应 feature，且 `gpui/crates/java` 已在做 Java 语义跳转），这 8 张（或 macOS 那 12 张）**必须留一份**。 |
| 8 | macOS 应用图标源文件 | `macos/Resources/{AppIcon-source.png 810 582 B, AppIcon.icns 1 659 491 B, AppIcon.png 1 017 133 B, Info.plist 3 184 B}` | `AppIcon-source.png` 与 `gpui/assets/images/logo.png` **逐字节相同**（同 `1FD4B09A…F6CBDA`）→ **已迁**；`AppIcon.icns` 与 `gpui/assets/icons/icon.icns` **逐字节相同**（同 `5A58925F…F200A`）→ **已迁**；`AppIcon.png`（1 017 133 B）与 gpui 的 `icon.png`（265 429 B）**是同一美术的不同导出构图** → **gpui 无副本** | `Info.plist` **明确不需要**（macOS `.app` 打包元数据，`scripts/package-app.sh:6,164,170` 使用）。`AppIcon.png` **需要一次人工裁决**：gpui 只取了 Windows 那份 265 429 B；若认为 1 MB 那份构图更好，删除前补迁（`gpui/assets/README.md:38-39` 已记明这个取舍）。 |
| 9b | **macOS Markdown 预览渲染器（KaTeX / Mermaid / markmap / highlight.js）** | `macos/Sources/Lithe/Resources/MarkdownPreview/`：`index.html`、`preview.css`、`preview.js` + `vendor/`（`katex.min.js` 273 168 B、`katex.min.css` 23 112 B、`mermaid.min.js` 3 164 970 B、`d3.min.js` 270 687 B、`markmap-lib.min.js` 127 127 B、`markmap-view.min.js` 22 843 B、`highlight.min.js` 117 604 B + 6 份 LICENSE）+ `vendor/fonts/`（**20 个 KaTeX `.woff2` / 259 792 B**） | **无副本**；`gpui/crates/**` 里 `markdown` 命中只出现在注释与图标映射里，**Markdown 预览能力整体未实现** | **建议补迁（但需先决策 gpui 的 Markdown 预览路线）**：这是一整套 WebView 时代的 HTML/JS 渲染器，gpui 无法直接复用；但 **20 个 KaTeX 字体是唯一的文件资产**（其余是 npm 包产物）。若不迁，必须把「真机有 Markdown 预览（含公式/图/流程图）、gpui 目前没有」这条**能力缺口**写进 `gpui/research/`。 |
| 9c | **工作台背景图（2 张 JPG / 3 479 660 B）** | `macos/Sources/Lithe/Resources/WorkbenchBackgrounds/{01,02}/background.jpg`（1 506 680 + 1 972 980 B）+ 10 个 `.gitkeep`（03–10 空位） | **无副本** | **建议补迁或明确不需要（二选一，但必须留记录）**：这是 macOS 侧的「工作台背景」美术，Windows 前端**没有**对应物。gpui 侧目前也没有这个功能。若确认 gpui 不做工作台背景，把结论与这 2 张图的名字/哈希写进文档即可；否则这 2 张图是**唯一一份**。 |
| 9d | macOS `SyntaxHighlighting/*.json` | `macos/Sources/Lithe/Resources/SyntaxHighlighting/{format-mappings.json 1 197 B,color-mappings.json 1 536 B}` | 无副本 | **建议补迁**：`color-mappings.json` 已被 `scripts/verify-shared-contracts.sh:26,250` 当作**契约校验的输入**引用（见 §2.3）——删掉它会让那个校验脚本失败，且这两个映射表是 gpui 侧未来做语法高亮的**语义真源**。 |
| 9 | macOS DatabaseIcons / GitGraph | `macos/Resources/DatabaseIcons/**`（10 文件 / 32 621 B：1 PNG + 7 SVG + 2 txt）、`macos/Resources/GitGraph/**`（2 txt / 10 921 B） | 无对应目录 | **DatabaseIcons：建议补迁**（数据库品牌图标，gpui 侧将来做数据库面板会用到；体积极小）。**GitGraph：明确不需要**（只有 2 个 `.txt`，是 macOS 侧绘图数据/说明，不是界面资源）。 |
| 10 | macOS 本地化 | `macos/Resources/{en,zh-Hans}.lproj/**` | 无（gpui 用 `windows/` 的 locale） | **明确不需要**：`gpui/UI-MAP-WINDOWS.md:1696` 已定「文案取本地化原文……来源改成 `windows/tauri/src/i18n/locale.ts`，不是 `macos/Resources/zh-Hans.lproj`」。 |
| 11 | Monaco 编辑器静态资源 | `frontend/editor/**`、`macos/EditorFrontend/**`、`windows/tauri/src/features/editor/**` 的静态部分 | 无 | `frontend/editor/**` **确认保留**（两条 CI lane 都在跑它，见 §6 第 3 条）。`macos/EditorFrontend/**` 与 gpui 无关：gpui 侧编辑器是自研（`gpui/crates/editor/`），**不用 Monaco**。 |
| 12 | Lucide 字形 | `lucide-react` npm 包（`windows/node_modules` 在工作区不存在） | gpui 自带同一套（`gpui-kit-assets-0.6.6/assets/icons/`，1830 字形） | **明确不需要**：`gpui/assets/README.md:249`。 |
| 13 | `MavenIcon` / `RunIcon` 内联 SVG | `windows/tauri/src/features/{maven,run}/components/*-icon.tsx`（**只有内联 path，无 SVG 文件**） | 无 | **建议补迁为 SVG 文件**：`gpui/research/icon-asset-inventory.md:24-26,554` 明确「**不可能**靠搬资源 1:1 还原——真源只有内联 SVG 的 React 组件」。删除前若不落成 SVG，这两个图标将**永久无法 1:1 还原**。 |

### 3.3 逐字节/逐张差集：已测规模 + 待确认部分

**已实测规模**（本次审计用 `Get-ChildItem -Recurse -File | Measure-Object Length -Sum` 直接量的，非引用）：

| 目录 | 文件数 | 字节数 | 构成 |
| --- | --- | --- | --- |
| `windows/tauri/public/tree-sitter/**` | **96** | **55 913 780**（≈53.3 MiB） | 48 `.wasm` + 48 `.scm` |
| `windows/tauri/public/`（整目录） | 97 | — | 上述 96 + `logo.png`（已迁） |
| `windows/tauri/src/extensions/bundled/icon-themes/material/` | 2 | 527 307 | 1 `extension.json` + 1 `LICENSE` |
| `macos/Resources/Fonts/` | 5 | 1 112 798 | 4 `.ttf`（JetBrains Mono）+ 1 `OFL.txt` |
| `macos/Resources/IDEAIcons/` | 139 | 142 242 | 137 `.svg` + 2 `.txt` |
| `macos/Resources/DatabaseIcons/` | 10 | 32 621 | 7 `.svg` + 1 `.png` + 2 `.txt` |
| `macos/Resources/GitGraph/` | 2 | 10 921 | 2 `.txt` |
| `gpui/assets/ui-icons/`（对照） | 157 | 130 936 | 157 `.svg` |
| `gpui/assets/icon-themes/`（对照） | 1 037 | 5 481 419 | 1 027 `.svg` + 各包 `extension.json`/许可 |

**待确认**：`macos/Resources/IDEAIcons/**`（144 文件）与 `gpui/assets/ui-icons/**`（157 文件）的**逐子目录重叠/缺口**
（`debugger/` 50、`fileTypes/` 36、`nodes/` 19、`testState/` 8、`toolwindows/` 8、`actions/` 5、`gutter/` 4、`vcs/` 3、`general/` 2、`maven/` 2；
gpui 那套是 `expui/**` 结构，**不是同一套**）；结果见 §7.3。

**整体总量对照**（供判断"还剩多少没迁"）：

| 位置 | 文件数 | 字节 |
| --- | --- | --- |
| `windows/` 全量（排除构建产物） | 3 183 | 87 757 403 |
| `macos/` 全量 | 1 010 | 21 713 600 |
| `gpui/assets/**` | 1 203 | 8 561 207 |
| `gpui/themes/**` | 3 | 41 179 |

> ⚠️ 上表是**当次实测的快照**，不随仓库演进而更新。`gpui/assets/**` 现在读作 **1 203 个文件 /
> 8 567 891 字节**（差 6 684 B 是 `README.md` 自身被追加的内容；该文件现已由 `#[exclude = "README.md"]`
> 挡在二进制外）；另外 `icon-themes/{lithe,pierre,symbols}`（**933 文件 / 5 337 322 字节**）已用
> `#[exclude]` 排除出内嵌范围，**但磁盘文件一个都没删**（将来做图标主题切换还要用）。
> 内嵌现状以 `gpui/assets/README.md` 第 8 节与 `gpui/crates/app/src/assets.rs` 的测试为准。

`windows/` 的资源类大头就是 `.wasm` 48 个（55 813 057 B，tree-sitter）+ `.png` 194 个（8 203 394 B，图标）+ `.icns`/`.ico`。
**结论：`windows/` 里 87.7 MB 有 63.6% 是明确不需要的 tree-sitter wasm + 过期图标快照；真正的"必须留"只有 38 KB 的主题 JSON、字体（在 macOS 侧）和几十 KB 的图标。**

---

## 4. 删除的可执行步骤（分步、可回滚）

> 前置纪律：**不要用一个 PR 直接删掉目录**——§2.2 证明那会让 CI 分类器把 `swift`/`windows` lane 全部点亮并必然失败。
> 正确做法是"先切断依赖 → 再删目录 → 最后清理文档与 CI"。

### 步骤 0：冻结基线（只读，先做）

```powershell
git -C D:\developmentProjects\rust\Lithe-IDEA status --short
git -C D:\developmentProjects\rust\Lithe-IDEA rev-parse HEAD          # 记下这个 SHA，写进本文档 §4.5
```

### 步骤 1：删除前的"证据留档"（保命步骤）

1. 在**有旧前端的分支上**跑完所有能跑的真机/对照验证，把结论写进 `gpui/research/**`（见 §5 缓解办法）。
2. 打一个永久 tag，作为死链接的追溯锚点：

```powershell
git tag -a legacy-frontends-final -m "最后一份含 windows/ 与 macos/ 旧前端的提交（供死链接追溯）"
```

3. 把"当前 `gpui/` 产物与旧前端真源一致"这件事**记录成证据**（不跑 cargo）：

```powershell
node gpui/tools/extract-locale.mjs --check          # 期望：产物与真源一致，退出码 0
node gpui/tools/generate-idea-icons.mjs --check     # 期望：79 icons（含 dark 变体数），退出码 0
```

### 步骤 2：切断 `gpui/tools/*.mjs` 对旧前端的依赖（**在删除之前做**）

1. **locale**（硬阻塞，二选一）：
   - A（推荐）：`Copy-Item windows/tauri/src/i18n/locale.ts gpui/crates/shared/locales/source/locale.ts`、
     `Copy-Item windows/tauri/src/i18n/ai-commit.ts gpui/crates/shared/locales/source/ai-commit.ts`；
     改 `gpui/tools/extract-locale.mjs:34-35` 指向新位置，改 `:405` 的 `sourceLabel` 为
     「`gpui/crates/shared/locales/source/`（从 `windows/` 迁入的历史快照）」。
   - B：把 `extract-locale.mjs` 标成废弃、`--check` 从文档与 PLAN 的校验命令里移除，
     并在 `gpui/crates/shared/locales/README.md:138-143` 改写「本目录即唯一真源，手工维护」。
2. **icons**（半阻塞）：
   `Copy-Item windows/tauri/scripts/idea-icon-mappings.json gpui/tools/idea-icon-mappings.json`；
   改 `gpui/tools/generate-idea-icons.mjs:58-61` 指向新位置；
   重新生成 `node gpui/tools/generate-idea-icons.mjs` 并确认 diff **为空**（`gpui/crates/shared/src/icons/idea.rs` 不变）。
3. **验证不再读旧前端**：grep `"macos/`、`"windows/` 在 `gpui/**` 应当**零命中**（`git grep -n -e '"macos/' -e '"windows/' -- gpui`）。

### 步骤 3：补迁 §3.2 里决定要留下的资源

优先级从高到低：`material/extension.json`（+`LICENSE`）→ `macos/Resources/AppIcon-source.png` →
`MavenIcon`/`RunIcon` 落成 SVG → （如决定要）`public/tree-sitter/**`。
每迁一组，都在 `gpui/assets/README.md` 追加一节，写明来源、文件数、字节数、SHA256 与"删除后不可再取"。

### 步骤 4：删目录（分两次提交，先 Windows 后 macOS，便于定位回归）

```powershell
git rm -r --quiet windows
git commit -m "chore: 删除 Windows 旧前端（gpui 已接管；资源与文案已迁至 gpui/）"

git rm -r --quiet macos
git commit -m "chore: 删除 macOS 旧前端（gpui 已接管；资源已迁至 gpui/）"
```

### 步骤 5：同批修复所有会红的引用（**与步骤 4 同一个 PR**）

| # | 目标 | 动作 |
| --- | --- | --- |
| 1 | `Package.swift` | 整份重写或删除（约 30 个 target 的 `path:` 全部指向 `macos/Sources/**` / `macos/Tests/**`）；若 macOS 产品真的下线，还应处理 `Casks/`、`scripts/*sparkle*`、`.swift-version`、`Package.resolved`、`.github/actions/setup-macos-toolchain/`、`.github/actions/prepare-macos-dependency-cache/` |
| 2 | 根 `package.json` | 从 `workspaces` 移除 `windows/tauri`（并同步 `bun.lock:17,1402`、`bunfig.toml:1-3`）；**把 `packageManager: bun@1.3.12` 从 `windows/tauri/package.json` 搬到根 `package.json`**（否则 `ci-windows.yml:118,151`、`release-windows.yml:70`、`release-preview-windows.yml:58` 失去版本锚点） |
| 3 | `.github/workflows/ci-windows.yml`、`release-windows.yml`、`release-preview-windows.yml` | 删除或重写（`bun-version-file`、`working-directory`、`hashFiles`、`install-windows-frontend-dependencies.ps1`、`verify-windows-boundaries.ps1`、`test-stability-windows.ps1 -Scope WindowsRust`） |
| 4 | `.github/workflows/ci-macos.yml`、`release-macos.yml`、`release-preview-macos.yml`、`ci-plugins.yml`、`ci-database.yml` 的 macOS 部分 | 删除或重写（`plutil`/`PlistBuddy`/`package-app.sh`/`create-dmg.sh`/`verify-rust-core.sh`/`probe-macos-monaco.sh`） |
| 5 | `scripts/build-windows.ps1`、`install-windows-frontend-dependencies.ps1`、`invoke-windows-tauri-build.ps1`、`package-windows.ps1`、`verify-windows-boundaries.*`、`verify-java-semantic-ownership.mjs:8-12`、`verify-editor-boundaries.mjs:14,22,24-32` | 删除或改写为 gpui 口径（后两个是**未接入 CI 但必失败**的校验脚本） |
| 6 | macOS 侧脚本族（`build-macos.sh`、`package-app.sh`、`create-dmg.sh`、`test-macos.sh`、`verify-macos-*.sh`、`preview.sh`、`package-macos-*.sh`、`measure-macos-performance-baseline.sh`、`build-editor.sh`、`probe-macos-monaco.sh`、`verify-module-boundaries.sh`、`verify-service-boundaries.sh`、`verify-shared-contracts.sh:26,250`、`build-agent-notes-board.mjs:18,136-138`、`scripts/*sparkle*`） | 删除或改写。⚠️ `build-agent-notes-board.mjs:18` 用 `macos/Resources/AppIcon.png` 当文档站 logo → **必须先把 logo 改成 `gpui/assets/images/logo.png`** |
| 7 | `scripts/classify-ci-changes.sh` + `scripts/test-classify-ci-changes.sh` | 删除 `macos/**`、`windows/**` 的 `case` 分支；**并补上 `macos/EditorFrontend|Experiments` 这类空洞的显式模式**（或直接删掉对应目录）——否则下一个删 `macos/` 的 PR 会走 `*)` 全量触发 |
| 8 | `.agents/notes/**` | **清理 136 处"适用范围"里的 `macos/`/`windows/` bullet 与 9 条 `](../../../../macos/` 相对链接**（`scripts/verify-agent-notes.mjs:242,148-158,231-234` 会 `fail`）——见 §2.6 |
| 9 | `shared/contracts/rust-core-api.md:17`、`shared/contracts/github.md:38` | 改写为 gpui 口径（后者是**运行时配置事实**：`LitheGitHubOAuthClientID` 必须先确认 gpui 从哪读）；另复核 `scripts/verify-shared-contracts.sh:62` 要求 `storageIdentityCases` 覆盖 macos+windows |
| 10 | 根 `AGENTS.md:7,9,11`、`.agents/skills/develop-lithe/SKILL.md:71-80,86-87,99-102,110-113,140-147,164,189-200,235,241-242`、`.agents/skills/debug-windows-on-parallels/**`（建议整体退役）、`.agents/skills/lithe-code-review/references/review-policy.md:66-67`、`.github/lithe-review/review-prompt.md:64-65`、`.agents/skills/release-lithe/SKILL.md:31,60-63,80,84`、`.agents/skills/write-stable-tests/**` | 更新所有权表、验证矩阵与 Skill 适用范围 |
| 11 | 新增 `gpui/` 的 CI lane | **建议同批做**：否则删除后仓库对 gpui 外壳零 CI 覆盖（§2.1） |
| 12 | `README.md:219,221-223`、`README.zh-CN.md:218,220-222`、`docs/ARCHITECTURE.md:120,136,151,159,229-233,239,242-245,251`、`docs/DESIGN.md:29,156,253` | 改写成 gpui 口径（死引用） |

### 步骤 6：死链接清理（可独立成 PR，不阻塞构建）

按 §1.3 的规模（约 764 行）分两批：

- **批 1（低成本、高收益）**：给 19 份自有 Markdown 加统一表头声明（一句「出处已随旧前端删除，行号对应 tag `legacy-frontends-final`」）；
  重点处理 `gpui/docs/archive/ui-map-macos.md`（`gpui/docs/README.md:98` 已预告）。
- **批 2（源码注释）**：32 个 Rust 文件的 150 行注释。**不要在删目录的同一次提交里做**——
  那会把"删目录"和"改 55 个文件"混在一起，无法 review。建议单独一个 `docs(gpui): 标注已删除的真源出处` 提交。

### 4.5 回滚方式

- 目录级回滚：`git revert <删除提交>` 或 `git checkout legacy-frontends-final -- windows macos`。
- 由于删除是**纯删除**（步骤 4 只有 `git rm`），回滚不涉及冲突；但**步骤 2 的脚本改写**与**步骤 5 的清单修复**
  需要各自的 revert。这也是把步骤 2/4/5 分成三个提交的原因。

### 4.6 死链接清单（供后续清理；按目录列）

| 目录/文件 | 需处理的行数 | 备注 |
| --- | --- | --- |
| `gpui/crates/editor/src/editor_view.rs` | 36 | 最密集的 Rust 文件 |
| `gpui/crates/shared/src/icons/idea.rs`（生成物） | 19 | `:607` 那句由生成器写入（`generate-idea-icons.mjs:280`），改生成器即可 |
| `gpui/crates/editor/src/buffer.rs` | 10 | |
| `gpui/crates/settings/src/schema.rs` | 9 | |
| `gpui/crates/workbench/src/{title_bar,activity_bar}.rs` | 8 + 8 | |
| `gpui/crates/shared/src/i18n.rs` | 5 | 含 `:5` 的"文案真源"声明 |
| 其余 26 个 Rust 文件 | 1–4 | 见 §1.3 明细 |
| `gpui/research/windows/0{1..7}-*.md` | 44+36+56+140+76+40+7 = **399** | 这 7 份是"Windows 调研原文"，**建议整体保留 + 表头声明**（它们本身是 gpui 侧的规格知识） |
| `gpui/UI-MAP-WINDOWS.md` | 30 | |
| `gpui/PLAN.md` | 23 | 含 `:540`（生成器说明）、`:1241`（`--check` 校验命令） |
| `gpui/assets/README.md` | 56 | 含 §5 的 sha256 复核脚本（**删源后不可再跑**，要标注） |
| `gpui/themes/README.md` | 7 | 含真源对照表表头 |
| `gpui/crates/shared/locales/README.md` | 14 | 含 §7/§8 的"未纳入 catalog 的零散文案"清单（**这是删除前必须处理的实际缺口**，见 §4.7） |
| `gpui/docs/grill.md` | 20 | |
| `gpui/docs/archive/ui-map-macos.md` | 14 | **整篇失效**，优先处理 |
| `gpui/research/{icon-asset-inventory,app-icon-and-assets}.md` | 16 + 12 | |
| `gpui/UI-MAP.md`、`gpui/docs/README.md`、`gpui/README.md` | 8 + 3 + 3 | |

### 4.7 与死链接不同的一类"真实缺口"（别混为一谈）

`gpui/crates/shared/locales/README.md:117-136` 列出的 **7 条"不在 catalog 里、因此没有进入 YAML"的本地化内容**，
是**真的会丢**的东西，不是死链接。删除前必须逐条落定：

1. `DISPLAY_LANGUAGES` / `DisplayLanguage` / `getLocaleCatalog`（配置而非文案，`:123-124`）→ gpui 侧已在
   `gpui/crates/settings/src/schema.rs:42` 有对应记录，**已结算**。
2. `createTranslator` 的缺 key 回退与 `{name}` 插值（`:125-126`）→ gpui 侧在 `gpui/crates/shared/src/i18n.rs` 重新实现，**已结算**。
3. `windows/tauri/src/config/backend-capabilities.ts:1` 的 `BACKEND_UNAVAILABLE_TOOLTIP = "待开发"`（硬编码中文、无英文）→ **未确认 gpui 侧是否有等价文案**。
4. `settings-search.ts:19-20`、`search-index.ts:11,19` 的设置搜索中文关键词串 → **未确认**。
5. `ai-commit-settings-panel.tsx:462-464` 的示例提交信息、`macos-settings-panels.tsx:188` 的硬编码「简体中文」→ **未确认**。
6. 其余含中文的 `.ts/.tsx` 命中均在测试或注释里 → 无需处理。
7. **建议**：删除前把 3–5 项里的每一串中文**摘录进 `gpui/crates/shared/locales/README.md`**（哪怕 gpui 暂时用不上），
   并在 `GPUI_ONLY_KEYS`（`gpui/tools/extract-locale.mjs:78-192`）里按需补键。

---

## 5. 风险清单：删掉后**无法再验证**的东西

| # | 失去的能力 | 影响 | 缓解办法 |
| --- | --- | --- | --- |
| 1 | **真机行为对照**（`windows/` 与 `macos/` 是 gpui 重写的规格真源） | `gpui/PLAN.md:28-31` 明确：权威界面规格 = `macos/Sources/Lithe` 源码；`windows/tauri/src/features/*` 是交叉验证旁证。删掉后**任何"gpui 与真机不一致"的疑问都无法再回源** | ① 保留 §4 步骤 1 的 tag；② 在删除前把**仍被引用的**关键源码片段（不是整仓）摘录进 `gpui/research/`；③ 保留已有的 7 份 `gpui/research/windows/*` 调研原文（它们已经是规格的二手真源，**不要删**） |
| 1b | **macOS 的第三份文案** | macOS 的 `en.lproj`/`zh-Hans.lproj/Localizable.strings`（65 139 + 130 337 B）是**独立维护**的文案，既不是 Windows 的 `locale.ts`、也不是 gpui 的两份 YAML。删掉后**再也无法做 macOS↔Windows 文案一致性 diff** | 删除前跑一次 diff（本次审计**未做**）：把两份 `.strings` 转成可比格式（`plutil -convert json`）与 `lithe.{zh-CN,en}.yml` 对键；差异清单落进 `gpui/crates/shared/locales/README.md` 的「未纳入 catalog 的文案」一节 |
| 1c | **gpui 尚未实现的能力在真机上是"有据可查"的**：Markdown 预览（mermaid/KaTeX/markmap）、工作台背景图、非 tree-sitter 的轻量高亮器（ini/env/toml/xml/yaml/json/properties/config）、gutter 运行/测试标记 | 这些在 `macos/Sources/Lithe/Resources/**` 与 `windows/tauri/src/features/editor/**` 里有**可运行的参考实现与美术**。删掉后 gpui 若要做这些功能，只能从零设计（尤其 2 张工作台背景 jpg 与 8 个 gutter SVG 是**唯一美术**） | ① 删除前把 `macos/Sources/Lithe/Resources/**`（52 文件 / 7 772 218 B）整体归档进 `gpui/research/legacy-macos-resources/` 或 `gpui/assets/`；② 把 gutter 标记的 8 个 SVG 复制进 `gpui/assets/`；③ 在这份审计里保留"真机有、gpui 没有"的能力清单（§3.2 第 9b/9c/7b 行） |
| 2 | **Tauri 构建与 Windows 侧测试**（`cargo test --manifest-path windows/tauri/src-tauri/Cargo.toml`、ConPTY、WebView2、installer、signing、updater） | 这些验证**只能**在 `windows/` 存在时做。gpui 侧终端的 ANSI/VT 能力边界（`gpui/docs/grill.md:421-446`）正是靠 Windows 侧样本标定的 | ① 删除前把 `windows/tauri/crates/terminal/**` 的**协议与行为规格**（事件名、字节编码、尺寸同步、高水位 500 000 B）摘录进 `gpui/research/`；② `gpui/crates/terminal/src/lib.rs:9-43` 已有一份对照表，过期风险低 |
| 3 | **JDTLS 解析的移植对照** | `gpui/crates/java/src/{jdtls,workspace}.rs` 是逐条移植 `windows/tauri/src-tauri/src/lsp.rs` 与 `lsp/jdt_workspace.rs`（`gpui/crates/java/src/lib.rs:29-30`）。删掉后无法再逐行核对 | 删除前确认这两份 Rust 已覆盖：可执行文件发现、启动资源、内嵌 JDK、workspace 指纹、缓存目录与过期回收 |
| 4 | **主题的完整真源**（`lithe.json` 的 17 项降级/合并） | `gpui/themes/README.md:293-315` 记录了 17 项"gpui-kit 里没有对应 token"的降级。这些是**已做的取舍**，但如果将来要还原（例如实心选中底、`--subtle-foreground`、终端 16 色、git 状态色），需要真源值 | 真源值**已经在** `gpui/themes/README.md` 的 §3/§4 表里逐键抄录（含 L/D 两列与行号）→ **保留这两节即足够**，但要确保它们不被后续编辑删掉 |
| 5 | **资源 SHA256 复核脚本** | `gpui/assets/README.md:94-108`、`:197-227` 的两段 PowerShell 以 `windows/tauri/**` 为源。删源后**不可再跑**，只能看文档里已记录的哈希 | 把期望输出（`:229-238` 的 `TOTAL src=1194 bad=0 extra=0` 等）留在文档里；**并在删除前实际跑一次**留档 |
| 6 | **gpui 侧无 CI** | `.github/**` 对 `gpui` 零命中；旧 lane 删除后，仓库对新前端零覆盖 | **删除同批新增 gpui 的 CI lane**（cargo check/test/clippy + `node gpui/tools/*.mjs --check`）。这一条本身是本次审计发现的**独立风险**，与删不删旧前端无关 |
| 7 | **交互验证的替代路径** | `gpui/BLOCKERS.md:24`（B5）已记录：本环境**无法合成鼠标事件**，所有点击/悬停/拖拽只能由维护者在真机验证。删掉旧前端不会改变这一点，但会减少"用旧前端做对照"这条退路 | 沿用现状：需要交互验证的验收项一律标注"待维护者实机确认" |
| 8 | **`.agents/notes/**` 的架构决策真源** | Note 里大量"适用范围"直接写 `macos/**`、`windows/**`，并指向 `macos/` 的源码行号。它是**架构决策与工程取舍的中文真源**（根 `AGENTS.md` 第 2 段）。删目录会让 136 条 bullet + 9 条相对链接失效，并让 `deploy-agent-notes-board` / `verify-agent-notes` 变红 | ① 同批把适用范围改成"已删除的旧前端（见 tag `legacy-frontends-final`）"或改写为 gpui 路径；② **不要删除这些 Note**——它们是取舍的唯一下落 |
| 9 | **主题族与语法高亮真源** | 删掉 `windows/tauri/src/extensions/themes/builtin/**` 就永久失去 ayu/catppuccin/dracula/github/nord/one/solarized/tokyo-night/vitesse 等 33 条主题的**真源值**；`syntax-token-colors.ts:3-44,99-128` 的 18 色回落表也只在 `windows/` 里 | ① 删除前把这 12 个 JSON **原样归档**进 `gpui/themes/legacy-builtin/`（或 `gpui/research/`），标注"未转换的旧 schema 真源"；② 把 18 色回落表抄进 `gpui/themes/README.md` 或 `gpui/research/editor-syntax-highlighting.md` |
| 10 | **字体渲染一致性** | 删掉 `macos/Resources/Fonts/**` 后，真机上等宽字体的**确切文件**（JetBrains Mono 4 个字重）无处可取；gpui 只能用系统字体，且无法再做"与真机字形对齐"的对照 | **删除前补迁**（§3.2 第 6 行）；并把 `windows/tauri/package.json:35-36` 的 `@fontsource/geist-*` 版本号抄进 `gpui/research/`（Geist 系列没有文件对象，只能靠版本号重建） |

---

## 6. 未确认的点

1. **`macos/Resources/IDEAIcons/**`（137 SVG）与 `gpui/assets/ui-icons/**`（157 SVG）的逐张差集**：
   两者**命名体系不同**（前者 `execute.svg`/`more.svg`/`moreVertical.svg` 这类小驼峰，后者 IntelliJ `expui/` 目录结构），
   所以不能按路径比对，需要按内容/形状比对。**未确认**这 137 张里有几张是 `gpui/` 确实没有的。
   → 建议：删除前直接**整目录复制**进 `gpui/assets/`（142 KB，成本极低），把差集问题留给以后。
2. **gpui 侧编辑器是否还需要 `windows/tauri/public/tree-sitter/**` 里的 `.scm` 查询文件**：
   已确认 gpui 走**原生 Rust tree-sitter + gpui-kit cargo feature**（`gpui/research/editor-syntax-highlighting.md`，
   并已在 `gpui-kit = { features = ["tree-sitter-java"] }` 落地），**不消费 WebView 的 `.wasm`**。
   **未确认**的是：gpui-kit 支持的 35 种语言之外（elisp/elm/nix/objc/ocaml/ql/r/rescript/solidity/systemrdl/tlaplus/dotenv/dockerfile）
   的查询文件将来要从哪里取。删除前建议把这份"gpui-kit 没有的语言"清单抄进 `gpui/research/editor-syntax-highlighting.md`。
3. **`frontend/editor/` 的归属**：它是根 `package.json` 的 workspace 成员之一，注释说是"shared Monaco presentation"。
   **已确认它不随 `macos/` 一起消失**：`.github/workflows/ci-macos.yml:280` 与 `.github/workflows/ci-windows.yml:122`
   都跑 `run-bun-tests-with-timing.mjs --working-directory frontend/editor`——**两条 lane 都覆盖它**，
   说明它是**仍在维护的共享包**。**未确认**的是它与 `macos/EditorFrontend/`（3 文件）的引用关系：
   `macos/EditorFrontend/` 存在（已 `Test-Path` 为真），若它引用 `frontend/editor/` 的产物，
   删除 `macos/` 时要保留 `frontend/editor/` 本身（没有任何证据表明要删它，**本次审计不建议动**）。
4. **`shared/contracts/github.md:38` 的 `LitheGitHubOAuthClientID`**：这是**运行时配置事实**（macOS 产品从
   `macos/Resources/Info.plist` 读）。**未确认** gpui 侧从哪里读同一份 client id；
   若 gpui 也要这个 OAuth 流程，删除前必须先把该配置项搬到 gpui 侧。
5. **CI lane 是否需要在删除前就先建好**：本审计只确认了「`.github/**` 对 gpui 零命中」这一事实，
   **未确认**维护者是否已计划把 gpui 的 CI 放在同一批改动里。
6. **`bun.lock` 的同步方式**：根 `package.json` 去掉 `windows/tauri` 后 `bun.lock:17,1402` 会悬空，
   `bun install --frozen-lockfile` 是否报错**未确认**；仓库的预期做法（删 `bun.lock` 重建 vs 手工编辑）也未确认。
7. **删除是否应当一次性删两个目录**：本审计建议分开（§4 步骤 4），但 `macos/` 与 `windows/` 的
   `Package.swift` / `package.json` 依赖相互独立，**未确认**维护者是否有"两个一起删"的发布节奏要求。
8. **`gpui/research/windows/*.md` 与 `gpui/docs/archive/ui-map-macos.md` 的保留策略**：
   建议保留前者、标注后者；**未确认**是否会因为"引用已死"而被整体删除（个人意见：**不应删**，它们是规格知识而非出处索引）。
9. **分支保护是否把这些 gate 列为必过 check**：仓库内无 branch protection 配置可读，
   **未确认** §2.0 的 `gate` 失败是否会被 GitHub 直接拦下合并。
10. **`oven-sh/setup-bun@v2` / `hashFiles()` 对不存在路径的确切行为**：前者推断为失败
    （`ci-windows.yml:118`、`release-windows.yml:70`、`release-preview-windows.yml:58`），
    后者推断为返回空值不报错（`ci-windows.yml:297,315` 等）——两者均**未实测**（本审计不跑 CI）。
11. **`.agents/notes/**` 的 136 处 bullet 是否全部落在 `## 适用范围` 段**：已抽验 2 篇确在段内，
    其余按 grep 统计；**未逐篇确认**（影响的是"哪些必须改"的精确清单，不影响"必须改"这个结论）。
12. **Windows 侧 4 级相对 Markdown 链接**（`](../../../../windows/`）是否存在：只在 `macos/` 模式上做过零命中确认，
    **未枚举所有深度**。

---

## 7. 附：盘点结果回填

> 本节由子代理盘点（`macos/Resources/**`、`windows/tauri/public/**`、CI/scripts 逐条）回填。

### 7.1 CI / scripts / 配置逐条（已回填，正文见 §2）

- **§2.0–§2.6 已按逐文件盘点结果写成**，覆盖：13 个工作流的触发与红/不红判定、
  classifier 的 `*)` fallback 过度触发机制、`scripts/**` 的 (a)/(b)/(c) 三分类、
  根配置（`Package.swift` / `package.json` / `bun.lock` / `bunfig.toml` / `.gitignore`）、
  根 `AGENTS.md` / `CLAUDE.md` / `.agents/skills/**` 的逐行改写清单、
  以及 `.agents/notes/**` 的 136 条 bullet + 9 条相对链接。
- **实测规模已写入 §3.3**。
- 明确判定**零影响**：`infra/**`（对 `windows|macos|swift|tauri` 零命中）、
  `lithe-issue-claim.yml`、`lithe-issue-priority.yml`、`update-repo-charts.yml`、`sync-atomgit-release.yml`、
  `lithe-pr-review.yml`（只有内存 fixture 字符串）、`.github/CODEOWNERS`、`.github/ISSUE_TEMPLATE/**`。

### 7.2 执行时需要顺带纠正/补充的两点

**（a）两处过期的文档陈述（本审计发现）**

| 位置 | 过期陈述 | 事实（本次核实） |
| --- | --- | --- |
| `gpui/assets/README.md:9-11` | 「⚠️ 本目录当前**还没有任何** Rust/TS 代码引用……接线由后续任务完成」 | **已过期**：`gpui/crates/app/src/assets.rs:63-102` 已有 `LitheAssets`（`rust_embed` 包装层 + 回落）并在 `main.rs` 注册，`gpui/crates/shared/src/icons/idea.rs` 的常量也已被 `gpui/crates/settings/src/dialog.rs` 真实调用。执行删除时顺手改正这句，避免误导。 |
| `gpui/assets/README.md:112-118` §6 / `:256-271` §7.4 | 两处「删除 `windows/` / `macos/` 前必读」只说"不要丢本目录" | 需要**补充**本审计的结论：本目录确实不能丢，但**「没丢本目录」不等于「没丢资源」**——字体、`material` 主题、11 个主题族、macOS 侧 Markdown 预览/工作台背景/IDEAIcons/gutter 标记都不在本目录里（§3.2）。 |

**（b）`.artifacts/` 不受影响（一条正向结论）**

根 `.gitignore:12` 忽略 `.artifacts/`，而工作区里 `.artifacts/` **实际存在**（含 `jdtls`、`jdtls-downloads`、`idea-icons`、`p0`–`p5` 等）。
所有第三方载荷（JDTLS、JDK、db sidecar、官方插件产物）都是 `scripts/prepare-{jdtls,jdk}.sh` / `package-app.sh` 的**构建产物**，
**不在 git 内**（`windows/**`、`macos/**` 下 `.exe/.dll/.dylib/.jar/.zip/.dmg` 命中 **0**；`third_party/` 只有 3 个 `manifest.json`）。
→ **删 `windows/` + `macos/` 不会丢任何第三方载荷**；gpui 侧也**已有能力层面副本**（`gpui/crates/java/src/jdtls.rs` 移植了
`find_jdtls_executable`/`jdtls_search_roots`/`resolve_jdtls_launch_resources`/`select_bundled_jdtls_root`/`resolve_java_home`，
并 `include_str!("../../../../third_party/jdtls/manifest.json")`）。

### 7.3 资源差集（已回填，正文见 §3）

- **已迁完（差集 0）**：locale（`lithe.{zh-CN,en}.yml`，比 macOS 的 `Localizable.strings` 更全）、应用图标 7 个位图、
  `logo.png`（与 `macos/Resources/AppIcon-source.png` 同哈希）、`icon.icns`（与 `macos/Resources/AppIcon.icns` 同哈希）、
  `ui-icons` 157、`icon-themes` 1037（idea/lithe/pierre/symbols 四套逐文件哈希一致）、主题色（仅 lithe 明暗两条）。
- **`gpui/` 无副本（建议补迁）**：
  ① 字体：`macos/Resources/Fonts/JetBrainsMono-{Regular,Italic,Bold,BoldItalic}.ttf` + `OFL.txt`（1 112 798 B，**唯一有实体文件的等宽字体真源**）；
  ② **主题：`windows/tauri/src/extensions/themes/builtin/*.json` 12 个 / 38 585 B / 35 条主题**（gpui 只有 2 条）+
     legacy 独有的 36 个语义色 key（18 `syntax` + 16 `terminal` + 6 `git` + `subtle-foreground`/`selected`/`cursor-vim-*`）；
  ③ `macos/Resources/{DatabaseIcons(10),IDEAIcons(144)}`、`macos/Resources/AppIcon.png`（1 017 133 B，待裁决）；
  ④ `windows/tauri/src/features/editor/engines/monaco/gutter-icons/*.svg`（8 个 / 4 819 B）或 macOS 的 `IDEAIcons/{gutter,testState}/`；
  ⑤ `material` 图标主题（2 文件 / 527 307 B，美术内联在 `extension.json`）。
- **`gpui/` 无副本、但取决于产品决策**：`macos/Sources/Lithe/Resources/{MarkdownPreview(+20 KaTeX 字体),WorkbenchBackgrounds(2 jpg / 3.48 MB),SyntaxHighlighting(2 json)}`（合计 52 文件 / 7 772 218 B）；
  gpui-kit 缺失的 17 种语言 `.scm`（约 18.9 KB）；Geist Sans/Mono（**仓库内没有文件对象**，只有 `windows/tauri/package.json:35-36` 的版本号 `@fontsource/geist-sans@^5.2.5` / `geist-mono@^5.2.8`）。
- **明确不需要**：`public/tree-sitter/**`（55.9 MB，WebView 专属）、`src-tauri/icons/{dev,preview,prod,android,ios}/**`
  （191 文件 / 16.4 MB，过期快照 / 移动端）、`Square*/StoreLogo`（10 个）、`icon-themes/minimal`（3 个）、
  Monaco 全家（`frontend/editor` 的 monaco 产物、`macos/{EditorFrontend,Experiments/Monaco}`、2 个 CSS）、
  `bundled/themes/*/extension.json`（9 个，是扩展清单不是颜色真源）、Tauri/工具链配置与 `.d.ts`、
  `macos/Resources/Info.plist`、`macos/Resources/GitGraph/**`（只有 2 个许可文件）、`macos/Resources/{en,zh-Hans}.lproj`（gpui 用 Windows 口径）、
  **全部二进制载荷**（`.exe/.dll/.dylib/.jar/.zip/.dmg` 命中 **0**；JDTLS/JDK/sidecar 都是 `scripts/prepare-*` + `.artifacts/` 的构建产物，**不在 git 内**）。
- **未确认的第三份文案**：macOS 的 `Localizable.strings`（`en` 65 139 B + `zh-Hans` 130 337 B）是**独立维护的第三份文案**
  （与 `windows` 的 `locale.ts`、gpui 的两份 YAML 并列）。删除前**建议做一次 macOS↔Windows 文案 diff**
  （`scripts/verify-agent-notes` 不做这件事，需要人工或自写脚本）——**本次未做**。

---

## 8. 审计方法（可复现）

本次审计使用的只读手段（**未跑 cargo、未跑 CI 脚本、未启动应用、未改动任何文件**）：

```powershell
# 1. gpui/ 内的旧前端路径引用（分文件计数）
Get-ChildItem gpui -Recurse -Include *.rs -File | Where-Object { $_.FullName -notmatch '\\target\\' } |
  ForEach-Object { $c = (Select-String -Path $_.FullName -Pattern '(macos|windows)[/\\]' -AllMatches | Measure-Object).Count
                   if ($c -gt 0) { "{0}: {1}" -f $_.FullName, $c } }

# 2. gpui 的构建期入口（build script / 编译期内嵌 / Cargo 清单）
Get-ChildItem gpui -Recurse -Include build.rs -File
Select-String -Path (Get-ChildItem gpui -Recurse -Include *.rs -File).FullName -Pattern 'include_str!|include_bytes!|RustEmbed'
Select-String -Path (Get-ChildItem gpui -Recurse -Include *.toml -File).FullName -Pattern '(macos|windows)/'

# 3. 会红的脚本与 CI 条目
Select-String -Path scripts/*,.github/workflows/*.yml -Pattern 'windows/tauri|macos/Resources'

# 4. 旧前端资源规模
Get-ChildItem windows/tauri/public -Recurse -File | Measure-Object -Property Length -Sum
Get-ChildItem windows/tauri/src-tauri/icons -Directory
```

**未做的事**（因此未覆盖）：没有执行任何 `cargo`/`swift`/`bun` 命令，因此**未实测**删除后的编译结果；
没有修改或移动 `windows/`、`macos/` 下任何文件；没有创建 tag（步骤 1 的 tag 命令是给执行者用的建议）。
