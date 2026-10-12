# 证据:ohos-rs 的真实定位

采集方式:`gh api` 直连 GitHub API。本机 `curl` 因 schannel `SEC_E_NO_CREDENTIALS` 不可用,
`web_fetch` 因 github.com 解析到非公网 IP 被拒。

## 1. 仓库身份

```
$ gh api repos/ohos-rs/ohos-rs --jq '{name,desc,stars,license,pushed,topics}'
{"archived":false,
 "desc":"A framework for building compiled OpenHarmony SDK in Rust via Node-API(Forked from napi-rs)",
 "lang":"Rust","license":"NOASSERTION","name":"ohos-rs/ohos-rs",
 "pushed":"2026-08-14T02:43:38Z","stars":247,
 "topics":["arkts","harmonyos","harmonyos-next","napi","openharmony","rust"]}
```

自述标题即定位:**via Node-API(Forked from napi-rs)**。README 第一段:

> This project is forked from [napi-rs](https://github.com/napi-rs/napi-rs)

默认分支为 `ohos`,MSRV 1.88.0。README 要求先按 `ohos.rs` 文档装好环境,再用它的 CLI
`ohrs build` 构建,产物是 **`dist` 文件夹,拷进 OpenHarmony/HarmonyNext 工程使用**。

## 2. crate 清单:五个全是 N-API 相关

```
$ gh api repos/ohos-rs/ohos-rs/contents/crates --jq '.[] | .name'
backend  build  macro  napi  sys
```

| 目录 | package name | description / keywords |
|---|---|---|
| `crates/napi` | `napi-ohos` | "N-API bindings";keywords `["NodeJS","Node","HarmonyOS","NAPI","n-api"]` |
| `crates/sys` | `napi-sys-ohos` | "Harmony N-API raw binding";keywords `["NodeJS","FFI","NAPI","n-api","HarmonyOS"]` |
| `crates/macro` | `napi-derive-ohos` | "N-API procedural macros";keywords `["NodeJS","FFI","NAPI","n-api"]` |
| `crates/backend` | `napi-derive-backend-ohos` | "Codegen backend for napi-harmony procedural macro" |
| `crates/build` | `napi-build-ohos` | "N-API build support";keywords `["NAPI","n-api","Harmony","open-harmony","harmony-next"]` |

版本统一 `1.2.0`,license MIT。**没有**窗口、GPU surface、渲染、输入法相关的 crate。

## 3. 它提供的是绑定层:构建脚本即证明

`crates/build/src/ohos.rs`(`napi-build-ohos` 的 build script):

```rust
pub fn setup() {
  let ndk = env::var("OHOS_NDK_HOME").expect("OHOS_NDK_HOME not set.");
  let target = env::var("TARGET").expect("Try to get build target failed.");
  let lib_dir = match target.as_ref() {
    "aarch64-unknown-linux-ohos" => "aarch64-linux-ohos",
    "armv7-unknown-linux-ohos"   => "arm-linux-ohos",
    "x86_64-unknown-linux-ohos"  => "x86_64-linux-ohos",
    _ => "",
  };
  println!("cargo:rustc-link-search={}/native/sysroot/usr/lib/{}", &ndk, &lib_dir);
  println!("cargo:rustc-link-search={}/native/llvm/lib/{}",       &ndk, &lib_dir);
  // link libace_napi.z.so
  println!("cargo:rustc-link-lib=dylib=ace_napi.z");
}
```

`crates/build/src/lib.rs` 的分派:

```rust
if target_env == "ohos" { ohos::setup(); }
else if env::var_os("CARGO_FEATURE_ARKVM_TEST").is_some() { ohos::setup_arkvm_test(); }
```

读出的四件事:

1. 它 **要求外部提供 `OHOS_NDK_HOME`**(OpenHarmony NDK/SDK),自己不提供工具链。
2. 它链接 **`libace_napi.z.so`** —— ArkUI/ArkTS 运行时暴露给原生的 NAPI 符号表。
   这正是「原生代码作为客人被 ArkTS 宿主加载」的链接形态,不是「原生应用自己起窗口」。
3. `crates/build/src/` 下的平台分支是 `android.rs` / `ohos.rs` / `wasi.rs` / `windows.rs`——
   全是**宿主运行时的链接适配**,没有窗口层实现。
4. 它支持的平台三元组就是 Rust 官方那三个 OHOS 目标,**没有提供新目标**。

## 4. 其 CI 实际编译与运行什么

全仓唯一 workflow 是 `.github/workflows/simple-test.yml`,名为 **ArkVM Tests**
(触发分支仅 `ohos`)。job 为 Linux/macOS/Windows 上的 `Build Examples`,以及 Linux 上 `Unit Tests`。
构建示例用的复合 action `.github/actions/build-ohos-examples/action.yml`:

```yaml
- name: Setup OpenHarmony SDK
  uses: openharmony-rs/setup-ohos-sdk@v0.2
  with:
    version: '5.0.0'
- name: Setup Rust
  uses: dtolnay/rust-toolchain@stable
  with:
    target: aarch64-unknown-linux-ohos,armv7-unknown-linux-ohos,x86_64-unknown-linux-ohos
- run: cargo build -p ohrs
- run: ${{ inputs.ohrs-bin }} build -p hello
- run: ${{ inputs.ohrs-bin }} build -p example
- run: ${{ inputs.ohrs-bin }} build -p compact
```

单元测试 job 装 **`harmony-contrib/arkts-vm@v2.0.0`(ArkVM)**,断言宿主产物存在:

```
test -x "${ARK_HOST_TOOLS_DIR}/ark_js_napi_cli"
test -x "${ARK_HOST_TOOLS_DIR}/es2abc"
test -f "${ARK_HOST_TOOLS_DIR}/libace_napi.so"
test -f "${ARK_HOST_TOOLS_DIR}/libets_interop_js_napi.so"
```

然后在**桌面 Linux** 上用 `ark_js_napi_cli`(ArkTS 的 N-API 宿主)跑 `test/suites/*.ts`。
实测套件全为 `compat_mode_*` / `napi_*` 命名,即 **N-API 语义兼容性测试**
(Array、Buffer、Class、Promise、ThreadsafeFunction、serde 等),共约 50 个用例文件。

**它从不构建任何 UI,也从不产出可运行的应用。**

全树 517 个路径中检索 `window` / `XComponent` / `gpu` 相关路径,命中的只有
`docker/windows.Dockerfile`、`crates/build/src/windows.rs` 这类构建适配文件;
GitHub 代码搜索对 `window`、`XComponent`、`gpu` 三个词在该仓库各返回 `total_count: 0`。

## 5. 规模与维护形态

```
$ gh api repos/ohos-rs/ohos-rs/contributors --jq '.[] | "\(.login)\t\(.contributions)"'
Brooooooklyn  2127     <- napi-rs 原作者
richerfu       556
dependabot[bot] 377
renovate[bot]   336
dependabot-preview[bot] 198
forehalo        46
...
```

前两名是 napi-rs 的原作者与主要维护者,其后两位是依赖机器人。
**这是 napi-rs 的一次平台分支(fork),不是独立新写的鸿蒙框架。**
这与仓库既有笔记的判断(「全生态只有一个规模极小的原型……依赖一个个人 fork」)互相印证。

## 6. 判定

| 问题 | 结论 | 依据 |
|---|---|---|
| 它是「Rust 的鸿蒙版本」吗? | **不是。** Rust 对 OHOS 的支持由 Rust 官方提供(`*-unknown-linux-ohos` 是官方 Tier 2 目标,本机 `rustup target list` 可见三个),不需要第三方项目。 | §1 自述、§2 crate 名、`rustup target list` |
| 它是「鸿蒙平台层 / GPUI 后端」吗? | **不是。** 全树无窗口、GPU surface、渲染、输入法实现;代码搜索三词各命中 0。 | §2、§3、§4 |
| 它自带工具链吗? | **不带。** 反向要求 `OHOS_NDK_HOME` 指向外部 OpenHarmony SDK,并链接 `libace_napi.z.so`。 | §3 |
| 它对 Lithe 有用吗? | **只对一类场景有用**:若将来真要把 Rust 逻辑以 **ArkTS 插件(NAPI 模块)** 形式装进鸿蒙应用,它是那块胶水。对「让 `rust/lithe-gpui` 在鸿蒙上跑起来」**没有任何帮助**——GPUI 缺的是平台窗口后端,不是 NAPI 绑定。 | §3、§4、§6 |

## 7. 与本次问题(CI 加鸿蒙)的直接关系

ohos-rs 的存在**不能**作为「Lithe CI 可以加一个鸿蒙车道」的理由:

- 它是第三方 crate 的 fork,不是 Rust 工具链,也不是 GPUI 的平台后端;
- 要用它,前提是先有 OHOS SDK/NDK,而那是**另一个前端形态**(ArkTS 应用 + NAPI 模块),
  与 Lithe 现在的 GPUI 界面层不是同一条路;
- 它自己的 CI 也说明这条路需要 `openharmony-rs/setup-ohos-sdk` + ArkVM 宿主,
  且验证的是 N-API 语义,不是 UI。

因此它对本次评估的贡献是**排除性**的:它把「加鸿蒙 CI」的问题收窄成「Core 能不能编到 OHOS 目标」
(见 `04-ohos-cross-compile-probe.md` 的实测),而不是「有没有 Rust 鸿蒙版本」。
