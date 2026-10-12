# s4 · 版本号注入通路与工具链钉版（构建发布层证据）

基线提交 `3085f5d1`。本文件回答两个对发布链路有决定作用的问题：
**GPUI 产物的版本号从哪来？工具链钉没钉版？**

检索口径：`rust/lithe-gpui/**` 与 `rust/lithe-ui/**` 全文件；`.github/workflows/*.yml`。
本机实测项单独标注。

---

## 1. 版本号：没有注入通路，且这个号会流进插件契约

### 1.1 唯一的版本常量

```
crates/shared/src/lib.rs:24
pub const HOST_VERSION: &str = env!("CARGO_PKG_VERSION");
```

`env!("CARGO_PKG_VERSION")` 是**编译期**宏，取的是**本 crate 的 `Cargo.toml` 的 `version`**。
`lithe-gpui-shared` 的 `version = "0.0.1"`。

### 1.2 全部 crate 的 version（实测）

| crate | version |  | crate | version |
|---|---|---|---|---|
| app | 0.0.1 | | notify | 0.0.1 |
| editor | 0.0.1 | | run | 0.0.1 |
| explorer | 0.0.1 | | settings | 0.0.1 |
| extensions | 0.0.1 | | shared | 0.0.1 |
| git | 0.0.1 | | terminal | 0.0.1 |
| java | 0.0.1 | | workbench | 0.0.1 |
| languages | **0.1.0** | | lithe-ui | 0.0.1 |

全部是 `0.0.1` 占位（`languages` 是 `0.1.0`，唯一例外，但它不产出 `HOST_VERSION`）。

### 1.3 唯一的 build.rs 不碰版本号

`rust/lithe-gpui` 下只有**一个** `build.rs`：

```
crates/app/build.rs:1-13
#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=lithe.rc");
    println!("cargo:rerun-if-changed=../../assets/app-icon/icon.ico");
    embed_resource::compile("lithe.rc", embed_resource::NONE) ...
}
#[cfg(not(windows))]
fn main() {}
```

它只做一件事：在 Windows 上把 `lithe.rc` 的图标资源嵌进 exe（ID 必须为 1，理由见
`crates/app/Cargo.toml:11-16` 的注释：`tauri-winres` 默认 32000 级 ID，gpui 找不到）。
**没有任何 `cargo:rustc-env` 或 `println!("cargo:rustc-cfg=...")` 之类的版本注入。**

### 1.4 `HOST_VERSION` 的两个消费点（这条是主理人没指出的）

```
crates/extensions/src/core_bridge.rs:2   use lithe_gpui_shared::{HOST_VERSION, core_json};
crates/extensions/src/core_bridge.rs:7   serde_json::json!({ "hostVersion": HOST_VERSION }),
crates/workbench/src/welcome.rs:319      .child(div().text_xs().text_color(muted).child(HOST_VERSION)),
```

- `welcome.rs:319` 是**欢迎页显示**——次要。
- **`core_bridge.rs:7` 把 `HOST_VERSION` 作为 `hostVersion` 塞进插件宿主契约的 JSON 请求**
  ——这是**兼容性判据面**。插件按 hostVersion 判断能否运行在宿主上时，读到的将是 `0.0.1`。

**后果**：如果发布时不注入真实版本号，
所有 GPUI 发行版的 hostVersion 都是 `0.0.1`，插件的版本兼容判断会失效
（所有版本看起来都一样）。这不是"界面上版本号难看"的问题，是**契约字段错了**。

对照 macOS 端的做法（`release-macos.yml:156-170`）：产物带
`CFBundleShortVersionString`、`CFBundleVersion`，以及
`LitheBuildGitRevision` / `LitheBuildGitBranch` / `LitheBuildGitDirty` / `LitheBuildTimestamp`
四个构建身份键，都由 `scripts/stamp-macos-app-build-info.sh` 写入。
**GPUI 侧没有任何等价机制。**

- 无 `--version` 之外的版本来源；
- 无 git revision / branch / dirty 标记；
- 无构建时间戳。

---

## 2. 工具链：未钉版

### 2.1 两套 workspace 都没有 rust-toolchain 文件（实测）

检索 `rust-toolchain.toml` 与 `rust-toolchain`，覆盖仓库根、`rust/`、`rust/lithe-gpui/`、
`rust/lithe-ui/` → **全部不存在**。

注：旧 Windows 端**有** `windows/tauri/rust-toolchain.toml`
（见 `release-windows.yml:121` 的缓存键 `hashFiles(..., 'windows/tauri/rust-toolchain.toml')`），
即旧端钉了版，GPUI 没钉。

### 2.2 CI 用的是 `stable`

`dtolnay/rust-toolchain@stable` 出现在 10 处（检索 `.github/workflows/*.yml`）：

```
ci-windows.yml:463, ci-windows.yml:559
ci-macos.yml:129, ci-macos.yml:267, ci-macos.yml:335
ci-database.yml:134
release-macos.yml:94
release-preview-macos.yml:82
release-preview-windows.yml:50
release-windows.yml:62
```

用 `@stable` 意味着**跟随当天的 stable**。本机实测 `rustc 1.97.0`
（commit-hash `2d8144b7880597b6e6d3dfd63a9a9efae3f533d3`，host `x86_64-pc-windows-msvc`）。

### 2.3 为什么这在 GPUI 上比在旧端更危险

`rust/lithe-ui/Cargo.toml:5` 声明 `rust-version = "1.95"`，
其余 13 个 GPUI crate **都没有 `rust-version`**（实测）。

结合 `rust/lithe-gpui/Cargo.toml` 用 `resolver = "3"` 与 `edition = "2024"`
（`app/Cargo.toml:4`），而 `rust/Cargo.toml:3` 用 `resolver = "2"`：
两套 workspace 的 edition/resolver 都不同，工具链漂移时**不会互相预警**。

更实际的触发场景：`gpui-pre` 是「一个人重新打包的 Zed 快照、约每周重基」
（选型笔记 `.agents/notes/proposed/architecture/2026-10-08-...md:14`）。
新版 rustc 报新的 lint 或 behavior change 时，`cargo build -p lithe-gpui-app` 会
**在没人改过 GPUI 代码的情况下开始失败**，而 CI 因为 s1 的假绿灯根本不会发现。

---

## 3. 与两项判断的对照

**预测 A**：`HOST_VERSION = env!(CARGO_PKG_VERSION)` 取值 0.0.1，无发布注入。
**实际**：一致。唯一 build.rs 只做图标嵌入，无版本注入。**无异常。**

**预测 B**：无 rust-toolchain 文件，CI 用 `dtolnay/rust-toolchain@stable`。
**实际**：一致，两套 workspace 都没有钉版文件，10 处 CI 用 `@stable`。**无异常。**

---

## 4. 本步未覆盖

1. **未验证**在 CI 的 `windows-latest` / `macos-26` runner 上当前 stable 的确切版本
   （本机是 1.97.0，runner 上的值未取）。
2. 未核对 `lithe-ui` 的 `rust-version = "1.95"` 与当前 stable 1.97 之间是否有
   已知的 breaking lint。
3. 插件宿主契约的 `hostVersion` 消费方（谁读它、怎么比较）在 `windows/tauri/src/features/host-api`
   ——该目录不属本步范围，未查。**建议交给插件扩展工程师核对。**
4. 未核对 GitHub Release 的 tag 与 GPUI 产物版本号之间是否需要一致性约束
   （旧端由 `release-windows.yml:47` 强制 `MAJOR.MINOR.PATCH` 格式）。
