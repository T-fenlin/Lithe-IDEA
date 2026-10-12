# Goal g-mv59zirjh9u3 (rev2 · open)
- **Headline**: 评估给 Lithe CI 增加鸿蒙(OHOS)通道的可行性与价值,并核实 ohos-rs 的真实定位
- **Claim**: 为 Lithe CI 增加鸿蒙(OHOS)构建或检查通道,是否可行、值不值得现在做;ohos-rs 在其中能起什么作用。
- **Promotion threshold**: L3
- **Criteria (full text)** (not itemized):
  给出明确建议(做 / 不做 / 有条件做),并至少列出 4 条可独立核对的事实证据:ohos-rs 的真实定位与其对 Lithe 的适用性、Rust 官方 OHOS target 在本机与本仓库的可用性、仓库既有平台决策是否已覆盖鸿蒙、以及本仓库现有产物(界面层 rust/lithe-gpui 与 rust/lithe-core)对 OHOS 目标的实际可编译性实测结果。每条证据须给出文件路径或命令输出。
> The system writes this file from the ledger (a projection); the authority is `goal/set` and `criteria/revised` in the ledger.
## Judgments (4)
- `h-rison8` [proposed] ohos-rs 是 napi-rs 的鸿蒙分支,提供的是 Node-API 绑定层,不是 Rust 语言本身对鸿蒙的移植;它只对「用 ArkTS 调 Rust」的场景有价值,不改变 Lithe 界面层的可编译性。
  - Refuted if: 若 ohos-rs 实际提供了 OHOS 平台层/工具链/GPUI 后端,或能直接编译 Lithe 的界面产物,则该判断被推翻。
  - scope: {"conditions":{"仓库":"Lithe-IDEA","主机":"Windows x86_64","Rust":"1.97.0 stable-msvc","考察范围":"CI 与构建通道,不含产品排期承诺"},"ranges":{}}
  - uses (framing declarations): []
  - Assertions: 1
- `h-vhv6nt` [proposed] 仓库已有明确的鸿蒙平台决策:排在后面、不阻塞主线、不承诺,只守「平台层保持可替换」这条纪律;因此单独加鸿蒙 CI 会与该决策冲突。
  - Refuted if: 若仓库中并不存在这条决策,或该决策已被新决策取代/撤销,则判断被推翻。
  - scope: {"conditions":{"仓库":"Lithe-IDEA","主机":"Windows x86_64","Rust":"1.97.0 stable-msvc","考察范围":"CI 与构建通道,不含产品排期承诺"},"ranges":{}}
  - uses (framing declarations): []
  - Assertions: 1
- `h-ifyele` [proposed] 当下加鸿蒙 CI 没有可编译的产物:界面层 rust/lithe-gpui 依赖的 gpui-pre 只有 apple/linux/macos/web/windows 平台 crate,没有 OHOS 后端。
  - Refuted if: 若 gpui-pre 依赖图中存在 OHOS/Android 平台 crate,或 lithe-gpui 具备其他 OHOS 构建路径,则判断被推翻。
  - scope: {"conditions":{"仓库":"Lithe-IDEA","主机":"Windows x86_64","Rust":"1.97.0 stable-msvc","考察范围":"CI 与构建通道,不含产品排期承诺"},"ranges":{}}
  - uses (framing declarations): []
  - Assertions: 1
- `h-buvkjj` [proposed] rust/lithe-core 能交叉编译到 aarch64-unknown-linux-ohos,即「Rust 核心上鸿蒙没问题」这条既有说法在编译层面可复现。
  - Refuted if: 若 cargo check 对该 OHOS target 因原生依赖(如 tree-sitter、libgit2、sqlite)或缺少 OHOS NDK 而在本机失败,则该判断被推翻或至少降级为未验证。
  - scope: {"conditions":{"仓库":"Lithe-IDEA","主机":"Windows x86_64","Rust":"1.97.0 stable-msvc","考察范围":"CI 与构建通道,不含产品排期承诺"},"ranges":{}}
  - uses (framing declarations): []
  - Assertions: 1
