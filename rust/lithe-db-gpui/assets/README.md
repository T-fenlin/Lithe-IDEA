# GPUI Kit 图片资源（由 Windows 前端提取）

本目录是 `lithe-db-gpui/`（GPUI Kit 桌面外壳）自己的**位图/图标**资源目录，内容**全部复制自 Windows 前端**，
二进制原样保留（未转码、未重新压缩、未改像素）。与 `lithe-db-gpui/crates/shared/locales/`（i18n 文案提取）
是同一批「把 gpui 前端需要的资源搬到 gpui 名下」的工作。

- 提取时间基线：分支 `feat/gpui-shell-rewrite`，真源为工作区中的 `windows/tauri/**`。
- 真源目录此后**不再被修改**（提取只做只读读取），本目录才是 gpui 前端的所有权所在。
- ⚠️ **本目录当前还没有任何 Rust/TS 代码引用**（`lithe-db-gpui/**` 里不存在 `assets/icons/`、`assets/images/`
  或 `logo.png` 的引用点）。落地只是为了在删除 `windows/` 之前保住资源本体；
  接线（窗口图标、欢迎页 logo、标题栏项目菜单 logo）由后续任务完成。

## 1. 已复制文件

| 来源（Windows 前端） | 目标（gpui） | 字节数 | sha256 |
| --- | --- | --- | --- |
| `windows/tauri/src-tauri/icons/32x32.png` | `lithe-db-gpui/assets/icons/32x32.png` | 2219 | `F960620A37DCAC4E2B05B4305238AC71E835CBB1CB3599B8D2B23DE9C2EF1334` |
| `windows/tauri/src-tauri/icons/64x64.png` | `lithe-db-gpui/assets/icons/64x64.png` | 5917 | `4EADE7B59A5008BDDCBDDEE339431362663526B785267705A9BE90D068B0B940` |
| `windows/tauri/src-tauri/icons/128x128.png` | `lithe-db-gpui/assets/icons/128x128.png` | 17207 | `E7492A0368CC30F994AC524677FAA17571CD9F7F516554CDD775362F789864A6` |
| `windows/tauri/src-tauri/icons/128x128@2x.png` | `lithe-db-gpui/assets/icons/128x128@2x.png` | 62254 | `3DD91692517D29906E57964B4413FD8C111AA8A59515DADFBAB3ACC1C866C87C` |
| `windows/tauri/src-tauri/icons/icon.ico` | `lithe-db-gpui/assets/icons/icon.ico` | 102639 | `D2904104872DB56520A5A36B22B3C8EB6EC09398B8D8018B8CC79CE9C47388FD` |
| `windows/tauri/src-tauri/icons/icon.png` | `lithe-db-gpui/assets/icons/icon.png` | 265429 | `4EED529C05B46CC51BC60896C08E3D0FB25F766C4E2C659EF29E5D53CFCAF403` |
| `windows/tauri/src-tauri/icons/icon.icns` | `lithe-db-gpui/assets/icons/icon.icns` | 1659491 | `5A58925F632EDCA77CB2CEFCE1D6190C5E774CB8F1B9B64F10C45501100F200A` |
| `windows/tauri/public/logo.png` | `lithe-db-gpui/assets/images/logo.png` | 810582 | `1FD4B09A488B276B5A3902D3FFCC92629EE092910FE775A0AC3EFBD9F2F6CBDA` |

合计 **8 个文件 / 2 925 738 字节**。全部 8 个文件复制后都用 `Get-FileHash -Algorithm SHA256` 与真源
逐字节比对过（哈希相同、长度相同）。

区分两类用途：

- `lithe-db-gpui/assets/icons/**`：可执行文件 / 窗口 / 任务栏图标（`32x32` 是 Windows 窗口与任务栏用的那一张，
  见第 3 节；`icon.ico` 是 exe 图标；`icon.icns`/`icon.png` 是 macOS 侧打包图标）。
- `lithe-db-gpui/assets/images/logo.png`：界面里显示的品牌 logo（欢迎页、标题栏项目菜单）。

补充一致性证据：`logo.png` 与 `macos/Resources/AppIcon-source.png` **逐字节相同**（同一 sha256），
`icon.icns` 与 `macos/Resources/AppIcon.icns` **逐字节相同**——两套旧前端共用同一份品牌美术，
所以本目录提取的就是这份共同真源，不存在「Windows 版/macOS 版 logo 不一样」的问题。
（`icon.png` 与 `macos/Resources/AppIcon.png` 不是同一张：前者 265 429 B，后者 1 017 133 B，
两者是同一美术的不同导出尺寸/构图，本次只取了 Windows 前端这一份。）

## 2. 未复制文件（跳过清单与理由）

真源 `windows/tauri/src-tauri/icons/` 共 **208 个文件 / 14 946 218 字节**，其中图片 200 个；
`windows/tauri/public/` 共 97 个文件，其中图片只有 `logo.png` 1 个。

| 跳过对象 | 数量 / 体积 | 理由 |
| --- | --- | --- |
| `icons/android/**` | 15 个 PNG + 3 个 XML | Android 启动图标（`mipmap-*` + `ic_launcher*`），gpui 只做桌面，无用。 |
| `icons/ios/**` | 18 个 PNG | iOS AppIcon 变体，gpui 只做桌面，无用。 |
| `icons/dev/**`、`icons/preview/**`、`icons/prod/**` | 各 50 个图片文件、各 3 753 468 B，合计约 11.26 MB | 三套目录**互相逐字节相同**（同 sha256），且是**过期的旧美术**：`windows/tauri/src-tauri/icons/{dev,preview,prod}/32x32.png` 为 2458 B，而当前生效的主目录 `icons/32x32.png` 是 2219 B——`858a502e fix(windows): enlarge taskbar icon` 只更新了主目录（`128x128.png`、`128x128@2x.png`、`32x32.png`、`64x64.png`、`icon.ico`、`icon.png` 六个文件），三个子目录是改动前的重复快照。全仓库检索 `icons/dev`、`icons/preview`、`icons/prod` **没有任何引用点**（`tauri.conf.json` 只引用主目录），属可丢弃的重复数据，故不搬。 |
| `icons/Square*.png`（9 个）、`icons/StoreLogo.png` | 10 个 PNG，约 202 KB | Windows Store / MSIX 打包用的磁贴与商店图标（`Square30x30Logo`…`Square310x310Logo`、`StoreLogo`）。gpui 侧目前没有 MSIX / 商店打包目标，先不搬；若将来做 MSIX，再按需从 `windows/tauri/src-tauri/icons/` 取。 |
| `windows/tauri/public/tree-sitter/**` 与 `public/tree-sitter.wasm` | 96 个文件，约 55.9 MB | 不是图片：是编辑器语法高亮的 WASM 解析器与 `.scm` 查询文件，属于另一类运行时资源（gpui 侧是否需要、如何取用是独立议题），不在本次「图标/logo 图片」范围内。 |

已跳过的图片合计约 **12.83 MB**（icons 目录图片）+ 55.9 MB（tree-sitter 非图片资源）未进入 `lithe-db-gpui/`。

## 3. 应用图标在旧前端的用途与引用点

- `windows/tauri/src-tauri/tauri.conf.json:27`：
  `"icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.ico"]`
  —— Tauri 打包时的应用图标清单：`icon.ico` 进 exe（Windows PE 资源），PNG 供窗口/任务栏与其它平台。
  `64x64.png`、`128x128@2x.png`、`icon.png`、`icon.icns` 未在配置里显式列出，是 `tauri icon` 生成的
  同源美术导出，一起搬过来备用。
- `windows/tauri/src-tauri/src/host.rs:16`：
  `const WINDOW_TASKBAR_ICON: tauri::image::Image<'_> = tauri::include_image!("./icons/32x32.png");`
  —— 编译期内嵌 32px 图标。
- `windows/tauri/src-tauri/src/host.rs:19`：`window.set_icon(WINDOW_TASKBAR_ICON.clone())`
  与 `:300`：`.icon(WINDOW_TASKBAR_ICON)`——**Windows 窗口是无边框的**
  （`tauri.windows.conf.json` 里 `"decorations": false`），所以任务栏/窗口图标要在创建窗口后手工 `set_icon`。
  这条对 gpui 有直接参考价值：gpui 侧同样需要显式给窗口设置图标。
- `windows/tauri/src-tauri/src/host.rs:451`：回归测试 `bundled_windows_icon_includes_taskbar_sizes`
  直接读 `icons/icon.ico` 校验内嵌尺寸；`:471-480` 的 `taskbar_icon_fills_available_canvas`
  校验 32px 位图的可见边界（正是 `858a502e` 加的那组断言）。gpui 侧若也做图标断言，可参考这两处。

## 4. `logo.png` 在原前端的用途与引用点

`logo.png` 是 810 582 B 的品牌 logo 位图（界面里由 `rounded-*` 裁形、`object-contain` 缩放），
在原 Windows 前端有 3 个引用点：

| 引用点 | 那行代码 | 用途 |
| --- | --- | --- |
| `windows/tauri/src/features/layout/components/welcome-screen.tsx:75` | `<img src="/logo.png" alt="" className="size-[42px] rounded-xl" />` | 欢迎页左侧栏顶部：42×42、`rounded-xl`，紧挨「Lithe / 版本号 · Windows」标题（`:76-81`）。 |
| `windows/tauri/src/features/window/components/title-bar/title-project-menu.tsx:155` | `<img src="/logo.png" alt="" className="size-5 scale-[1.19] object-contain" />` | 标题栏「项目菜单」按钮内的品牌图：容器 `size-5` + `rounded-md overflow-hidden`，图片 `scale-[1.19] object-contain`（放大 19% 填满圆角框）。 |
| `windows/tauri/index.html:5` | `<link rel="icon" type="image/png" href="/logo.png" />` | WebView 页面的 favicon（Tauri 是本地 webview，这一条只影响 webview 内部文档图标，不影响系统任务栏图标）。 |

除此之外，`windows/tauri/src/**` 里其它 `logo` 命中都与本资源无关：
`features/window/utils/project-icons.ts:31,53`（按文件名给「项目图标候选」评分的正则，`^logo\.(png|svg|ico)$` 加分）、
`features/ai/**` 的 `logout*` 与 `i18n/locale.ts` 的 `projectIcon.emptyDescription` 文案、
`extensions/bundled/icon-themes/**` 里内嵌的第三方 SVG（terraform/flow/bitbucket 等品牌字形）。

## 5. 校验方式

复制后用下面的命令可以随时复核「目标与真源逐字节一致」（只读，不改任何文件）：

```powershell
$pairs = @(
  @('windows\tauri\src-tauri\icons\32x32.png',      'gpui\assets\icons\32x32.png'),
  @('windows\tauri\src-tauri\icons\64x64.png',      'gpui\assets\icons\64x64.png'),
  @('windows\tauri\src-tauri\icons\128x128.png',    'gpui\assets\icons\128x128.png'),
  @('windows\tauri\src-tauri\icons\128x128@2x.png', 'gpui\assets\icons\128x128@2x.png'),
  @('windows\tauri\src-tauri\icons\icon.ico',       'gpui\assets\icons\icon.ico'),
  @('windows\tauri\src-tauri\icons\icon.png',       'gpui\assets\icons\icon.png'),
  @('windows\tauri\src-tauri\icons\icon.icns',      'gpui\assets\icons\icon.icns'),
  @('windows\tauri\public\logo.png',                'gpui\assets\images\logo.png')
)
$pairs | ForEach-Object {
  "{0} -> {1}" -f $_, ((Get-FileHash $_[0] -Algorithm SHA256).Hash -eq (Get-FileHash $_[1] -Algorithm SHA256).Hash)
}
```

## 6. 归属提醒（删除 `windows/` 前必读）

**这些资源现在归 gpui 前端所有。** 将来删掉 `windows/` / `macos/` 两个旧前端时，不得连带丢失本目录：
`lithe-db-gpui/assets/icons/**` 与 `lithe-db-gpui/assets/images/logo.png` 是 gpui 侧唯一一份品牌图标/logo 真源——
尤其是 `32x32.png`（窗口与任务栏图标）、`icon.ico`（exe 图标）、`icon.icns`（macOS 打包图标）
和 `logo.png`（界面上唯一会展示的品牌图）。

删除前建议把第 1 节的 sha256 与真源再比对一次（第 5 节命令），确认本目录就是最新美术，
再处理第 2 节里按需保留的部分（尤其 `Square*.png` / `StoreLogo.png`，如果届时 gpui 决定要 MSIX 打包）。

---

# 7. 图标资源批量提取（`ui-icons/` 与 `icon-themes/`）

第 1–6 节处理的是**应用图标 / logo**（8 个位图）。本节是同一件事的**第二批**：把 Windows 前端
界面里真正用来画图标的 **SVG 资源**搬到 gpui 名下。做法与第 1 节一致——**只读复制，不改名、不转码、
不重压缩、不改 `viewBox`**；真源 `windows/tauri/**` 未做任何修改。

## 7.1 已复制文件

| 来源（Windows 前端） | 目标（gpui） | 文件数 | 其中 SVG | 字节数 |
| --- | --- | --- | --- | --- |
| `windows/tauri/src/ui/icons/**` | `lithe-db-gpui/assets/ui-icons/**` | 157 | 157 | 130 936 |
| `windows/tauri/src/extensions/bundled/icon-themes/lithe/**` | `lithe-db-gpui/assets/icon-themes/lithe/**` | 459 | 456 | 703 513 |
| `windows/tauri/src/extensions/bundled/icon-themes/symbols/**` | `lithe-db-gpui/assets/icon-themes/symbols/**` | 325 | 322 | 4 511 394 |
| `windows/tauri/src/extensions/bundled/icon-themes/pierre/**` | `lithe-db-gpui/assets/icon-themes/pierre/**` | 149 | 146 | 122 415 |
| `windows/tauri/src/extensions/bundled/icon-themes/idea/**` | `lithe-db-gpui/assets/icon-themes/idea/**` | 104 | 103 | 144 097 |
| **合计** | | **1 194** | **1 184** | **5 612 355** |

四套图标包小计 **1 037 个文件 / 1 027 个 SVG / 5 481 419 字节**。

两个来源目录的相对结构**原样保留**（只加了 `ui-icons/`、`icon-themes/<包名>/` 这一层前缀）：

- `lithe-db-gpui/assets/ui-icons/` 下是 `idea/{expui/{actions,bookmarks,fileTypes,general,ide,image,javaee,nodes,run,toolwindows,vcs},fileTypes,vcs}/`。
  `expui/general/` 一个目录就占 98 个文件；157 个 SVG 全部被
  `lithe-db-gpui/crates/shared/src/icons/idea.rs`（生成物，见 7.3 节）引用，无孤儿文件。
- `lithe-db-gpui/assets/icon-themes/lithe/` 含浅色变体 `icons/light/{files,folders}/`（各 186 / 42 个，与深色目录一一对应），
  另有 `extension.json`、`generate-icons.ts`、`preview.html`。
- `symbols/`、`pierre/` 各带 `LICENSE`；`pierre/` 另有 `UPSTREAM.md`；三套包都带各自的 `extension.json`
  （文件类型/目录 → SVG 的映射表）。**这些非 SVG 文件一并复制**，因为 `extension.json` 就是图标查找的真源。

`ui-icons/` 里**已经没有任何非 SVG 文件**：旧前端那份 `idea-assets.generated.ts`（19 373 B 的
Vite `?url` 导入清单，95 个显示名 → 明/暗两个 SVG 路径，共 188 条 import）**已按维护者要求删除**。
gpui 侧没有任何 TS 工具链会编译 `lithe-db-gpui/assets/`（`lithe-db-gpui/` 下无 `package.json` / `tsconfig.json`），
所以它在这里从来只是**只读的映射参考数据**，不是可执行代码。gpui 侧的等价物是
`lithe-db-gpui/crates/shared/src/icons/idea.rs`（生成物），由 `lithe-db-gpui/tools/generate-idea-icons.mjs` 直接扫
文件系统生成 —— **生成器不读那份 TS**，只把它的历史显示名当命名来源（`windows/tauri/scripts/idea-icon-mappings.json`），
所以删除它不影响再生成。

## 7.2 完整性证据

全量 1 194 个文件都已用 `Get-FileHash -Algorithm SHA256` 与真源**逐文件比对**（源/目标各算一次哈希，
再比字符串）：**1194/1194 全部命中，0 个缺失、0 个哈希不同、0 个多余文件**
（`ui-icons` 这一组的源侧多出一个已被删除的 `idea-assets.generated.ts`，见 7.2 节 `$excludedNames` 的排除规则）。

抽样（每组至少 1 个，`ui-icons` 取 5 个）的 sha256 如下，可单独复核：

| 来源 | 字节 | SHA256 |
| --- | --- | --- |
| `ui/icons/idea/expui/general/search.svg` | 364 | `81E8241E7B38407420C573DFA264CD6DF5DEAC0E8D59B935CE4C02B607BBE342` |
| `ui/icons/idea/expui/general/search_dark.svg` | 364 | `AA3C118BE2522E16522422AAA2FC2453DFA1016156D5E6A0E1B3484FFAC15D14` |
| `ui/icons/idea/expui/nodes/folder.svg` | 549 | `E78B061F701E39613936BC68751ED51E6CB2B1451DBA322CD98FFDFD1A105065` |
| `ui/icons/idea/expui/actions/newFolder.svg` | 1 187 | `34B1350D9580A132B85454AA57D8E42C8F9602402C589DDF0240FB96C4706120` |
| `icon-themes/lithe/icons/files/typescript.svg` | 1 089 | `F41CBA0789D95ED06D0A0CB4F8A1275011EF9BFA67BBB533C0A6C031C9954287` |
| `icon-themes/lithe/icons/light/files/typescript.svg` | 1 089 | `687B9F49F1CBB525E0565025B775ABE9BF90629E21D48A6812CBA2B0B58EFF54` |
| `icon-themes/lithe/icons/folders/folder.svg` | 848 | `D3C5877A3DE5A11585E76AAA672AC5382B0C206C14414F90C806EBF8537A6A89` |
| `icon-themes/symbols/icons/files/rust.svg` | 9 930 | `06F608E10EADB776373C33812C47EBEDFE43D3DB5CEEA91DC3080ECDCE74B867` |
| `icon-themes/pierre/icons/astro-color.svg` | 601 | `CFEEA578858C1336DBF817675D08D1F4AE6B80A8380090501A1E078A35CEFBB7` |
| `icon-themes/idea/icons/expui/fileTypes/c.svg` | 920 | `BA02BFB94E13C0D8F9688A53276A3C0DF09BBF95848596F1CA93AF5C2EFA3672` |

抽样里 `lithe/icons/files/typescript.svg` 与 `lithe/icons/light/files/typescript.svg` **长度相同但哈希不同**
（1 089 B / `F41C…` vs `687B…`），可以确认浅色变体确实是**另一份美术**而不是深色文件的复制品，
第 7.1 节说的"含 `icons/light/`"不是空话。

### 复核对全量的方式

下面的脚本对**每一组**做源/目标全量 SHA256 比对，并同时报告「源里有而目标里没有」「目标里有而源里没有」
「两边都有但哈希不同」三类差异（只读，不改任何文件）。它比第 5 节的逐对写法更适合上千个文件：

⚠️ 必须先从**相对路径**拿到绝对路径再切前缀：`Get-ChildItem` 返回的 `FullName` 是绝对路径，
用相对的 `$g.s.Length` 去 `Substring` 会切错位置，结果会误报成"1194 个全部缺失"。

⚠️ 源侧多出一个**未搬入**的文件：`windows/tauri/src/ui/icons/idea-assets.generated.ts` —— 它这次
**已按维护者要求删除**（见 7.5 节），而真源 `windows/**` 是只读、不动。所以脚本用
`$excludedNames` 在**双方**都排除这个文件名，否则 `ui-icons` 那一组会永远报 `src=158 dst=157 bad=1`。
排除后 `ui-icons` 组两侧都是 157，与 7.1 节的表格口径一致。

```powershell
$groups = @(
  @{n='ui-icons'; s='windows\tauri\src\ui\icons';                               d='gpui\assets\ui-icons'},
  @{n='lithe';    s='windows\tauri\src\extensions\bundled\icon-themes\lithe';   d='gpui\assets\icon-themes\lithe'},
  @{n='symbols';  s='windows\tauri\src\extensions\bundled\icon-themes\symbols'; d='gpui\assets\icon-themes\symbols'},
  @{n='pierre';   s='windows\tauri\src\extensions\bundled\icon-themes\pierre';  d='gpui\assets\icon-themes\pierre'},
  @{n='idea';     s='windows\tauri\src\extensions\bundled\icon-themes\idea';    d='gpui\assets\icon-themes\idea'}
)
# 生成物：已从 gpui 侧删除，真源侧仍在（只读），比对时两边一起排除。
$excludedNames = @('idea-assets.generated.ts')
$gt = 0; $gb = 0; $ge = 0
foreach ($g in $groups) {
  $s = (Resolve-Path $g.s).Path
  $d = (Resolve-Path $g.d).Path
  $sf = Get-ChildItem $s -Recurse -File | Where-Object { $excludedNames -notcontains $_.Name }
  $df = Get-ChildItem $d -Recurse -File | Where-Object { $excludedNames -notcontains $_.Name }
  $bad = 0
  foreach ($f in $sf) {
    $t = Join-Path $d $f.FullName.Substring($s.Length)
    if (-not (Test-Path -LiteralPath $t)) { $bad++; continue }
    if ((Get-FileHash $f.FullName -Algorithm SHA256).Hash -ne
        (Get-FileHash -LiteralPath $t -Algorithm SHA256).Hash) { $bad++ }
  }
  $extra = ($df | Where-Object {
    -not (Test-Path -LiteralPath (Join-Path $s $_.FullName.Substring($d.Length)))
  }).Count
  "{0,-9} src={1,-5} dst={2,-5} bad={3} extra={4}" -f $g.n, $sf.Count, $df.Count, $bad, $extra
  $gt += $sf.Count; $gb += $bad; $ge += $extra
}
"TOTAL src=$gt bad=$gb extra=$ge"
```

预期输出（`idea-assets.generated.ts` 删除后实测即为此值）：

```
ui-icons  src=157   dst=157   bad=0 extra=0
lithe     src=459   dst=459   bad=0 extra=0
symbols   src=325   dst=325   bad=0 extra=0
pierre    src=149   dst=149   bad=0 extra=0
idea      src=104   dst=104   bad=0 extra=0
TOTAL src=1194 bad=0 extra=0
```

## 7.3 跳过了什么、为什么

真源 `icon-themes/` 下有 **6 个**目录，本次只搬了 **4 个**（任务指定的那 4 个）：

| 跳过对象 | 数量 / 体积 | 理由 |
| --- | --- | --- |
| `icon-themes/material/**` | 2 个文件 / 527 307 B（`extension.json` 526 237 B + `LICENSE` 1 070 B） | **它一个 `.svg` 文件都没有**：全部图标美术是内联在 `extension.json` 的 `iconDefinitions` 里的 SVG 字符串（`"folder": "<svg …><path fill=\"#fbc02d\" …/></svg>"`）。任务指定的是 lithe / symbols / pierre / idea 四套，material 不在其中；且它不是"SVG 文件树"，照搬进来的话要和 JSON 解析一起设计，属独立议题。⚠️ 注意 material **是** Windows 已注册的内置图标主题（`bundled-icon-theme-assets.ts` 的 glob 里就有 `material`），将来 gpui 若要支持它，得从这份 `extension.json` 里抽字符串，而不是复制文件。 |
| `icon-themes/minimal/**` | 3 个 SVG / 947 B（`file.svg`、`folder.svg`、`folder-open.svg`） | **全仓无任何引用**：`bundled-icon-theme-assets.ts` 的 glob 是 `{idea,material,pierre,symbols}`，`bundled-extension-manifests.ts` 也只 import 这 4 个 `extension.json`；对 `minimal` 的检索只命中无关上下文（如 `features/git/types/ai-commit.ts` 的文案枚举）。属未接线的死资源。 |
| `windows/tauri/src/extensions/icon-themes/**` | 15 个 TS/TSX（约 30 KB） | 不是资源，是**实现代码**（`icon-theme-registry.ts`、`themed-file-icon.tsx`、`use-java-file-icon-kind.ts` 等）。本次只搬资源，且硬约束禁止改/搬 TS 源码逻辑。 |
| `lucide-react` 字形 | 不可枚举（npm 包） | Windows 前端的**大部分**界面图标其实不是仓库里的文件，而是 `windows/tauri/src/ui/icons.tsx:108-118` 的 `Nucleo` 代理在运行时从 `lucide-react`（`package.json` 里 `^0.468.0`）取的字形。`windows/node_modules/` 在工作区里**不存在**（`Test-Path` 为 false），所以这批字形**没有可复制的文件对象**；gpui 侧已经自带同一套 Lucide（`gpui-kit-assets-0.6.6/assets/icons/`，1830 个字形），不需要也不应该从 Windows 侧取。详见 `lithe-db-gpui/research/icon-asset-inventory.md` 第 2 节。 |
| `windows/tauri/src-tauri/icons/**`、`public/**` | 见第 2 节 | 应用图标 / logo，已在第 1–6 节处理过，本轮**没有重做**，`lithe-db-gpui/assets/icons/**` 与 `lithe-db-gpui/assets/images/logo.png` 未被本次改动触碰。 |

已搬入的 4 套里的非 SVG 文件（各包 `extension.json`、`lithe/generate-icons.ts`、`lithe/preview.html`、
`symbols/LICENSE`、`pierre/LICENSE`、`pierre/UPSTREAM.md`）**没有**被当成"非资源"跳过——它们是图标查找与
许可归属的一部分，随目录一起复制。

## 7.4 归属提醒（删除 `windows/` 前必读）

**`lithe-db-gpui/assets/ui-icons/**` 与 `lithe-db-gpui/assets/icon-themes/**` 现在归 gpui 前端所有。**
将来删掉 `windows/` / `macos/` 两个旧前端时，**不能连带丢失**本目录：

- `lithe-db-gpui/assets/ui-icons/**` 是 IntelliJ `expui` 那套 UI 图标的**唯一一份**副本（157 个 SVG）；
  它的映射清单是 `lithe-db-gpui/crates/shared/src/icons/idea.rs`（生成物）。
- `lithe-db-gpui/assets/icon-themes/{lithe,symbols,pierre,idea}/**` 是文件类型/目录图标的**唯一一份**副本
  （1 027 个 SVG + 各包的 `extension.json`）。其中 `symbols/icons/files/cursor.svg`（1 576 205 B）与
  `symbols/icons/folders/folder-cursor.svg`（1 576 419 B）单文件就 1.5 MB，是内嵌位图的 SVG，
  丢了没有第二处可取。
- 真源里**没有**对应文件的那些字形（Lucide 字形与内联 React 组件）不在此列，
  见 `lithe-db-gpui/research/icon-asset-inventory.md` 第 4 节。

删除前建议跑一次第 7.2 节的全量比对脚本，确认本目录就是最新一份，再处理第 7.3 节按需保留的部分
（尤其 `material/extension.json`，如果届时 gpui 决定支持 Material 图标主题）。

### 7.5 `idea-assets.generated.ts` 已删除

`lithe-db-gpui/assets/ui-icons/idea-assets.generated.ts`（旧前端的 Vite `?url` 生成物）**已按维护者要求
删除**：它不在 gpui 的构建路径上（`lithe-db-gpui/` 下没有 TS 工具链），`lithe-db-gpui/tools/generate-idea-icons.mjs`
也**不读它**（生成器扫文件系统 + 自己按文件名推导名字与别名），所以删除不影响再生成，
`--check` 仍然退出码 0。**若要追溯它的内容，看 git 历史：`d13b254a` 是最后一次带上它的提交**
（`git show d13b254a:lithe-db-gpui/assets/ui-icons/idea-assets.generated.ts`）。它带来的 3 个口径变化：
文件数 158 → **157**、总量 1 195 → **1 194**、字节数 150 309 / 5 631 728 → **130 936 / 5 612 355**
（差额 19 373 B 正好是那份 TS）。真源 `windows/tauri/src/ui/icons/` 下那份仍在（只读，未动），
第 7.2 节脚本因此按文件名在两侧一起排除，见那里的 `$excludedNames`。

---

# 8. 谁在用 / 谁没接线（内嵌范围收窄记录）

本节是**第 1–7 节之后的状态盘点**，不改动前面任何历史记录与 sha256 表。回答两个问题：
本目录里每一组资源**有没有代码路径能取到**，以及没有的那几组现在**是不是还进二进制**。

## 8.1 逐组实测与接线状态

数字用 `Get-ChildItem -Recurse -File` + `Measure-Object Length -Sum` **实测**（不是估算），与
`lithe-db-gpui/crates/app/src/assets.rs` 的 `LitheAssets::iter().count()`（收窄后 270）互相印证。
"是否内嵌"指**是否进 `Lithe.exe` 的 rust-embed 静态表**——磁盘上文件一个都没少。

| 子目录 / 文件 | 文件数 | 字节数 | 约 | 代码引用情况 | 本次是否内嵌 |
| --- | ---: | ---: | ---: | --- | --- |
| `icon-themes/idea/**` | 104 | 144 097 | 0.14 MiB | **在用**：`lithe-db-gpui/crates/shared/src/icons/file_icon.rs:72` 的 `ACTIVE_FILE_ICON_THEME = "idea"` + `:89` 的 `include_str!` 读 `extension.json` | 是 |
| `icon-themes/lithe/**` | 459 | 703 513 | 0.67 MiB | **零引用** | **否（`#[exclude]`）** |
| `icon-themes/pierre/**` | 149 | 122 415 | 0.12 MiB | **零引用** | **否（`#[exclude]`）** |
| `icon-themes/symbols/**` | 325 | 4 511 394 | 4.30 MiB | **零引用** | **否（`#[exclude]`）** |
| `ui-icons/idea/**` | 157 | 130 936 | 0.13 MiB | **在用**：`lithe-db-gpui/crates/shared/src/icons/idea.rs` 的 79 个常量路径指向这里 | 是 |
| `icons/**` | 7 | 2 115 156 | 2.02 MiB | **在用**：`lithe-db-gpui/crates/app/build.rs:1,16` + `lithe.rc:22` 把 `icons/icon.ico` 嵌成 PE 资源 ID 1（其余 6 个 `32x32/64x64/128x128/128x128@2x.png`、`icon.png`、`icon.icns` 目前**没有**引用点，是窗口/任务栏/macOS 打包的备用导出） | 是 |
| `images/logo.png` | 1 | 810 582 | 0.77 MiB | **保留内嵌**：`lithe-db-gpui/crates/workbench/src/project_menu.rs:37,40` 明确写"真源那个触发器画 `/logo.png`，本侧改画徽标"——即当前**没有** `img("images/logo.png")` 调用点，品牌 logo 的接线（欢迎页/标题栏）属后续任务 | 是 |
| `README.md`（本文件，根目录） | 1 | 随文档编辑变化 | — | 自身文档，也在 `#[folder]` 下面——**已被 `#[exclude = "README.md"]` 挡在二进制外**（它一变，所有"总文件数 / 总字节数"就跟着漂） | **否（`#[exclude]`）** |
| **合计** | **1 203** | **8 567 891** | **8.17 MiB** | | 收窄后内嵌 **269 个**（`README.md` 已由 `#[exclude]` 挡在二进制外），资源合计 **3 200 771 字节（3.05 MiB）** |

三套零引用包合计 **933 个文件 / 5 337 322 字节（5.09 MiB）**，占本目录 62% 的体积、被无条件内嵌
——直到本节这次收窄。

⚠️ **口径历史（别再照抄旧读数）**：本节最初的读数是 `iter().count() = 270`、全量 `8 561 207` 字节。
两者都会漂，因为**本文件自己也在 `#[folder]` 范围内**——`assets.rs` 加上
`#[exclude = "README.md"]` 之后是 **269 个 / 3 200 771 字节**，`lithe-db-gpui/assets/**` 全量是
**1 203 个 / 8 567 891 字节**（差的 6 684 B 正是本文件这段被追加的内容，属自我指涉的读数）。
`assets.rs` 的测试现在把内嵌字节**钉到精确值 `3_200_771`**，再漂会直接红。

## 8.2 为什么那三套"没有任何代码路径"

不是"暂时没找到引用点"，而是**结构上没有运行期入口**：

- 文件图标主题 id 是**编译期常量**：`file_icon.rs:72` 的 `ACTIVE_FILE_ICON_THEME = "idea"`。
- 主题映射表是**编译期内嵌**：`file_icon.rs:89` 的 `include_str!("../../../../assets/icon-themes/idea/extension.json")`，
  `:216` 的 `FileIconTheme::from_json(ACTIVE_FILE_ICON_THEME, IDEA_EXTENSION_JSON)` 只吃这一个常量。
- `lithe-db-gpui/**` 下**没有任何运行期枚举 `icon-themes/` 的代码**：唯一的 `.list("")`（`assets.rs` 启动诊断）
  只是打印数量，不按主题取文件。

所以 `lithe` / `pierre` / `symbols` 即使内嵌进去，也没有一行代码能把它们取出来。
处理方式是 `lithe-db-gpui/crates/app/src/assets.rs` 里给 `#[derive(rust_embed::RustEmbed)]` 加三条

```rust
#[exclude = "icon-themes/lithe/**"]
#[exclude = "icon-themes/pierre/**"]
#[exclude = "icon-themes/symbols/**"]
```

**只挡编译期内嵌，不动磁盘**：`lithe-db-gpui/assets/icon-themes/{lithe,pierre,symbols}/**` 的 933 个文件
**全部原样保留在仓库里**（这同时是刻意的——将来做图标主题切换还要用它们，见第 8.4 节）。
同文件里的 `mod tests` 加了回归测试：内嵌表里出现这三组任一前缀即失败，同时钉住
`icon-themes/idea/extension.json`、`ui-icons/idea/**` 的 svg、`icons/icon.ico`、`images/logo.png`
必须仍在表里——避免"收窄范围"把在用资源一起收走导致文件树图标静默消失。

## 8.3 `material/` 与 `minimal/` 从未被拷进 gpui

真源 `windows/tauri/src/extensions/bundled/icon-themes/` 下有 **6 套**主题，gpui 只搬了 **4 套**
（见 7.1 / 7.3 节），另外两套**从来没进过本目录**，因此这次也没有任何东西可排除：

| 真源目录 | 状态 | 说明 |
| --- | --- | --- |
| `icon-themes/material/**` | **未拷入** | 2 个文件（`extension.json` 526 237 B + `LICENSE` 1 070 B）：**它的美术全部内联在 `extension.json`** 的 `iconDefinitions` 里（`"folder": "<svg …>"` 这样的字符串），**没有任何 `.svg` 文件**。所以"拷文件树"这条路对它不成立，要用得改成解析 JSON 里的 SVG 字符串。 |
| `icon-themes/minimal/**` | **未拷入** | 3 个 SVG（`file.svg` / `folder.svg` / `folder-open.svg`，合计 947 B）：真源侧 `bundled-icon-theme-assets.ts` 的 glob 是 `{idea,material,pierre,symbols}`，不含 `minimal`，属未接线的死资源。 |

即：本目录里 `icon-themes/` 只有 `idea` / `lithe` / `pierre` / `symbols` 四套，
"6 套里少 2 套"是**第 7 节那次提取的既成事实**，不是本次收窄造成的。

## 8.4 恢复 / 启用主题切换的前置条件

**在子目录里加回文件（或去掉 `#[exclude]`）本身不会带来任何行为变化**——现在没有任何代码能按主题取图标。
真要启用图标主题切换，顺序必须是：

1. 把 `lithe-db-gpui/crates/shared/src/icons/file_icon.rs` 从"编译期常量 + `include_str!`"改成
   **运行期主题注册表**：能按主题 id 找到对应包并解析它的 `extension.json`
   （`material` 还要额外支持"美术内联在 JSON 里"这条路径，见 8.3）。
2. 让取图标那一侧走 `LitheAssets::load(..)` / `list(..)` 取 SVG（含 `icons/light/**` 这类变体），
   而不是只认一个编译期内嵌的 JSON。
3. 最后才去掉 `assets.rs` 里对应的 `#[exclude]`，并同步更新第 8.1 节的数字与 `assets.rs` 模块文档里
   `LitheAssets::iter().count()` 的实测值（`assets.rs` 的回归测试会在数字对不上时失败，这是有意的）。

在这三步做完之前去掉 `#[exclude]`，唯一的效果是二进制白多背 **5.09 MiB**。
