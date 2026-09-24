# GPUI Kit 图片资源（由 Windows 前端提取）

本目录是 `gpui/`（GPUI Kit 桌面外壳）自己的**位图/图标**资源目录，内容**全部复制自 Windows 前端**，
二进制原样保留（未转码、未重新压缩、未改像素）。与 `gpui/crates/shared/locales/`（i18n 文案提取）
是同一批「把 gpui 前端需要的资源搬到 gpui 名下」的工作。

- 提取时间基线：分支 `feat/gpui-shell-rewrite`，真源为工作区中的 `windows/tauri/**`。
- 真源目录此后**不再被修改**（提取只做只读读取），本目录才是 gpui 前端的所有权所在。
- ⚠️ **本目录当前还没有任何 Rust/TS 代码引用**（`gpui/**` 里不存在 `assets/icons/`、`assets/images/`
  或 `logo.png` 的引用点）。落地只是为了在删除 `windows/` 之前保住资源本体；
  接线（窗口图标、欢迎页 logo、标题栏项目菜单 logo）由后续任务完成。

## 1. 已复制文件

| 来源（Windows 前端） | 目标（gpui） | 字节数 | sha256 |
| --- | --- | --- | --- |
| `windows/tauri/src-tauri/icons/32x32.png` | `gpui/assets/icons/32x32.png` | 2219 | `F960620A37DCAC4E2B05B4305238AC71E835CBB1CB3599B8D2B23DE9C2EF1334` |
| `windows/tauri/src-tauri/icons/64x64.png` | `gpui/assets/icons/64x64.png` | 5917 | `4EADE7B59A5008BDDCBDDEE339431362663526B785267705A9BE90D068B0B940` |
| `windows/tauri/src-tauri/icons/128x128.png` | `gpui/assets/icons/128x128.png` | 17207 | `E7492A0368CC30F994AC524677FAA17571CD9F7F516554CDD775362F789864A6` |
| `windows/tauri/src-tauri/icons/128x128@2x.png` | `gpui/assets/icons/128x128@2x.png` | 62254 | `3DD91692517D29906E57964B4413FD8C111AA8A59515DADFBAB3ACC1C866C87C` |
| `windows/tauri/src-tauri/icons/icon.ico` | `gpui/assets/icons/icon.ico` | 102639 | `D2904104872DB56520A5A36B22B3C8EB6EC09398B8D8018B8CC79CE9C47388FD` |
| `windows/tauri/src-tauri/icons/icon.png` | `gpui/assets/icons/icon.png` | 265429 | `4EED529C05B46CC51BC60896C08E3D0FB25F766C4E2C659EF29E5D53CFCAF403` |
| `windows/tauri/src-tauri/icons/icon.icns` | `gpui/assets/icons/icon.icns` | 1659491 | `5A58925F632EDCA77CB2CEFCE1D6190C5E774CB8F1B9B64F10C45501100F200A` |
| `windows/tauri/public/logo.png` | `gpui/assets/images/logo.png` | 810582 | `1FD4B09A488B276B5A3902D3FFCC92629EE092910FE775A0AC3EFBD9F2F6CBDA` |

合计 **8 个文件 / 2 925 738 字节**。全部 8 个文件复制后都用 `Get-FileHash -Algorithm SHA256` 与真源
逐字节比对过（哈希相同、长度相同）。

区分两类用途：

- `gpui/assets/icons/**`：可执行文件 / 窗口 / 任务栏图标（`32x32` 是 Windows 窗口与任务栏用的那一张，
  见第 3 节；`icon.ico` 是 exe 图标；`icon.icns`/`icon.png` 是 macOS 侧打包图标）。
- `gpui/assets/images/logo.png`：界面里显示的品牌 logo（欢迎页、标题栏项目菜单）。

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

已跳过的图片合计约 **12.83 MB**（icons 目录图片）+ 55.9 MB（tree-sitter 非图片资源）未进入 `gpui/`。

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
`gpui/assets/icons/**` 与 `gpui/assets/images/logo.png` 是 gpui 侧唯一一份品牌图标/logo 真源——
尤其是 `32x32.png`（窗口与任务栏图标）、`icon.ico`（exe 图标）、`icon.icns`（macOS 打包图标）
和 `logo.png`（界面上唯一会展示的品牌图）。

删除前建议把第 1 节的 sha256 与真源再比对一次（第 5 节命令），确认本目录就是最新美术，
再处理第 2 节里按需保留的部分（尤其 `Square*.png` / `StoreLogo.png`，如果届时 gpui 决定要 MSIX 打包）。
