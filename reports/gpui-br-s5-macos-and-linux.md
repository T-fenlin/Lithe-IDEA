# s5 · macOS 车道与 Linux 侧（构建发布层证据）

基线提交 `3085f5d1`。本文件回答：**macOS 侧的三条工作流会不会构建 GPUI？全仓有没有公证？
Linux 侧有没有构建入口与依赖前提？**

检索口径逐条标注；本工作区是 Windows 主机，**macOS 与 Linux 相关结论全部为只读配置/代码，
未在任何 macOS 或 Linux 机器上执行过**。

---

## 1. macOS 三条工作流的构建目标

| 工作流 | 构建步骤 | 实际目标 | 是否含 GPUI |
|---|---|---|---|
| `release-macos.yml` | `:134 ./scripts/package-app.sh` | `macos/` 的 Swift 应用 | 否 |
| `release-preview-macos.yml` | `:126 ./scripts/package-app.sh` | 同上 | 否 |
| `ci-macos.yml` | `:378 Build complete macOS test package` + `:277 ./scripts/verify-rust-core.sh` | Swift 应用 + `lithe-core` | 否 |

`scripts/build-macos.sh` 的实际动作（`:21-22`、`:64`、`:66-112`）：

```
:21   "$ROOT_DIR/scripts/verify-macos-app-build-safety.sh"
:22   "$ROOT_DIR/scripts/build-editor.sh"            <- 共享 Monaco 前端
:64   RUST_LIBRARY="$(scripts/build-rust-core.sh "${RUST_BUILD_ARGS[@]}")"   <- 只调 rust-core
:112  swift "${SWIFT_ARGS[@]}"
```

即 **`swift build` + `build-rust-core.sh`**，`RUST_TARGET` 为
`aarch64-apple-darwin` / `x86_64-apple-darwin`（`:27-28`）。

`scripts/package-app.sh` 的证据同样明确（`:101`、`:106`）：

```
:101  arch_binary="$ROOT_DIR/.build/$ARCH-apple-macosx/release/Lithe"
:106  cp "$arch_binary" "$APP_DIR/Contents/MacOS/Lithe"
```

产物来自 **SwiftPM 的 `.build/`**，不是 `cargo`。`Info.plist` 取自
`macos/Resources/Info.plist`（`:6`）。

**结论：三条 macOS 工作流的构建目标都是 Swift 应用与 `lithe-core`，
没有一条引用 `rust/lithe-gpui`。**

另据 s1：`.github/` 全目录检索 `lithe-gpui` 命中 0，与本节互相印证。

---

## 2. 公证：全仓没有

检索 `.github/workflows/**`、`scripts/**`、`macos/Sources/**`，
模式 `notarytool|stapler|notariz`：

**命中 0（构建与发布脚本内）。** 唯一命中在
`macos/Sources/Lithe/Views/Terminal/TerminalView.swift`，是 SwiftTerm 终端视图里的
无关词形（`TerminalView.swift:8/12/26/28/34/38/119/155/271`），不是公证步骤。

**含义（这一条比主理人记的范围更大）**：
主理人 §2.1 只写了"Windows 与 macOS 各有一条能产出产物并签名的 CI 工作流"。
实际情况是：**macOS 端有签名（Developer ID，`import-macos-signing.sh`），但没有公证。**

```
scripts/import-macos-signing.sh:1-19   导入 Developer ID 证书到临时 keychain
release-macos.yml:108-121              调用它，未配置时降级为 ad-hoc 签名
release-macos.yml:155                  codesign --verify --deep --strict
```

macOS 上未经公证的 app 在用户机器上会触发 Gatekeeper 拦截
（"无法验证开发者"），需要用户右键打开或去系统设置放行。
**这是一个已经存在于旧端、并将原样带到 GPUI 的发布质量缺口**，
不是 GPUI 新引入的。要给 GPUI 建链路时**必须一并补上**，否则新链路一开始就带着这个洞。

---

## 3. Linux：无入口，但依赖树已解析

### 3.1 无任何构建入口

检索 `.github/workflows/**` 与 `scripts/**`，模式
`linux-gnu|unknown-linux|AppImage|appimage|\.deb|\.rpm`：

命中的 3 条全部是**误报**——它们是 java-debug 插件的 JAR 文件名
（`scripts/prepare-jdtls.ps1:63`、`scripts/prepare-jdtls.sh:47`、
`scripts/verify-macos-package.sh:10` 的 `com.microsoft.java.debug.plugin-*.jar`），
与 Linux 目标无关。

**结论：仓库没有任何 Linux 构建或打包入口。** 无 `ubuntu-latest` 上的构建 job
（`ubuntu-latest` 只用于 `changes` 分类、`gate` 汇总、`publish` 等编排 job）。

### 3.2 依赖侧：Linux 平台的 crate 已在 lock 里

`rust/lithe-gpui/Cargo.lock` 实测含：

| 行 | crate | 作用 |
|---|---|---|
| L329 | `ash` | Vulkan 绑定（wgpu 后端） |
| L954 | `calloop` | Wayland 事件循环 |
| L4546 | `nix` | Unix 系统调用 |
| L8369 | `wayland-sys` | Wayland 协议绑定 |
| L9168 | `x11` | X11 绑定 |

即 `gpui-pre` 的 Linux 平台层**在依赖图上是被解析出来的**（与 macOS 的
`objc2`/`cocoa` 33 条、Windows 的 `windows`/`windows-sys` 8 条并列）。

### 3.3 但依赖解析 ≠ 可构建

选型笔记 `.agents/notes/proposed/architecture/2026-10-08-...md` 对 Linux 的判断
（`:286-294`、`:707`）：

```
:294  | Linux | 可行但有边界：Wayland 与 X11 都默认开，渲染走 wgpu → Vulkan 1.3
      （没有证据支持无独显 / 软件渲染路径），文字走 cosmic-text + fontconfig，
      输入法两条路都在但近一年多都修过；中文输入法与字体回退是这一端最该先验的点 |
:707  5. 门 G3：Linux 真机。构建 + 起窗口 + 中文输入法与字体回退 + 拖动分栏
```

`Cargo.lock` 里 **没有 `swiftshader` / `lavapipe` / llvmpipe 等软件渲染后端**的证据，
与"没有证据支持无独显路径"一致。

**结论：Linux 现在不具备建链路的前提。** 不是"缺一条 workflow"，
而是**上游平台层的真机可行性未验证**（门 G3 尚未通过）。
在没有通过 G3 之前建 Linux CI，等于把 CI 建在一个可能起不了窗口的目标上。

---

## 4. 与"macOS车道无GPUI"判断的对照

**推翻条件**：`release-macos.yml` / `release-preview-macos.yml` / `ci-macos.yml`
的任一构建步骤引用 `rust/lithe-gpui` 或其产物。
**实际**：逐条核对三条工作流的构建步骤，全部指向 SwiftPM 的 `.build/` 与
`build-rust-core.sh`，**未命中推翻条件 → 判断成立。**

---

## 5. 本步未覆盖 / 必须标注的限制

1. **本工作区是 Windows 主机，本节所有 macOS 与 Linux 结论都未在任何 macOS / Linux 机器上执行过。**
   全部来自仓库内配置与脚本原文，以及选型笔记的既有结论。
2. **未验证** `gpui-pre` 0.3.8 在 macOS 上能否真正构建（这是主理人 §4 第 1 条"最大的单点未知"，
   与我的结论一致，但我同样没有新增证据）。
3. **未验证** Linux 的 Wayland/X11 真机行为（门 G3）。
4. 未核对 `ci-macos.yml:378` 的「Build complete macOS test package」完整步骤体
   （只核到它经由 `package-app.sh`，未逐行读该 step）。
5. macOS 公证的具体补法（`notarytool submit` + `stapler staple`，需 Apple ID app-specific
   password 或 API Key）未给实现细节——本步只确认缺口存在。
