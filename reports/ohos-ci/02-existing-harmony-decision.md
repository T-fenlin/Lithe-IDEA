# 证据:仓库既有的鸿蒙平台决策

## 1. 决策在哪里,处于什么生命周期

```
$ Get-ChildItem .agents/notes -Recurse -File -Filter *.md | 检索 "鸿蒙|平台层保持可替换|unified-gpui-frontend-selection"
.agents\notes\proposed\architecture\2026-10-08-unified-gpui-frontend-selection-and-layer-boundaries.md  ::  提议中
```

**全仓库仅此一份**提到鸿蒙的平台决策,位于 `proposed/`(尚未落地、等待确认),
文件头状态行写明「状态:提议中」。没有任何 `implemented/`、`rejected/` 或 `archived/`
下的笔记取代、撤销或改写过它。

按 `.agents/notes/AGENTS.md` 的目录规则,`proposed/` 表达的生命周期是
「尚未落地或等待确认的方案」——即这条决策**仍然是当前有效方向**,但它本身还没进入落地状态。
这两件事要分开:决策文本有效 ≠ 决策已实施。

## 2. 决策原文(第 8 节,`:289-313`)

标题:**平台:三端证据分级,鸿蒙只留缝不承诺**

关键三段逐字引用:

- `:300-303` ——
  > **Rust 核心上鸿蒙没问题**:`aarch64` / `armv7` / `x86_64-unknown-linux-ohos` 是 Rust 官方
  > **Tier 2(带宿主工具)**目标;经 NAPI 从 ArkTS 调 Rust 也已经被验证过。
  > 本仓库的 `lithe-core` 本来就产出 `staticlib` / `cdylib`,所以"另一个前端经同一 C ABI 消费 Core"
  > 在接缝层面是既有能力。

- `:304-309` ——
  > **但 GPUI 上鸿蒙不是"再编一个目标"**:GPU 渲染只是框架的一半,另一半是**平台层**
  > (申请窗口、拿到绑定窗口的 GPU surface、收输入与输入法事件),这一层是按操作系统各写一份的。
  > 鸿蒙受认可的 UI 层是 **ArkTS/ArkUI**,原生 GPU 代码只能作为 **XComponent 里的客人**——
  > 也就是说 **ArkTS 薄壳省不掉**,而且还要为 GPUI 写一个鸿蒙后端。
  > 全生态目前只有一个原型在做这件事,**规模极小**(几个提交、一个作者、刚建不久),
  > 且依赖一个个人 fork。

- `:311-313` ——
  > **决定**:鸿蒙排在后面,**不阻塞主线,也不承诺**。今天只守一条纪律:
  > **平台层保持可替换**,不要把它和某一端的写法绑死。这样将来要接鸿蒙时,
  > 问题收窄成"给平台层加一个后端",而不是"推翻整套界面代码"。

同文件另两处相关表述:

- `:8` —— 目标平台是「macOS、Windows、Linux,鸿蒙 PC 排在后面」。
- `:50` —— 「**鸿蒙 PC 不能按"一套 GPUI 打四端"规划**」。
- `:732` —— 风险清单把它记为「**鸿蒙是单原型赌注**:全生态只有一个规模极小的原型。已决定推后,
  但必须守住『平台层可替换』」。

## 3. 仓库里有没有 OHOS 相关实现代码或 CI

```
$ grep -r  ".github"        "ohos|openharmony|harmonyos|鸿蒙|arkts"   -> No matches found
$ grep -r  "*.rs"           'target_os = "ohos"|unknown-linux-ohos|cfg(ohos'  -> No matches found
```

- `.github/` 下 **0 命中**:没有任何 workflow、action 或配置提到鸿蒙。
  (本仓库全部 workflow 的 runner 只有 `ubuntu-latest` / `macos-26` / `macos-14` / `windows-latest`,
  无 Linux 产品构建入口。)
- 全部 `.rs` 源码 **0 命中**:没有任何 `cfg(target_os = "ohos")` 条件编译,也没有引用
  `*-unknown-linux-ohos` 目标的地方。

即:**仓库当前对鸿蒙是「零代码、零 CI」的纯空白状态**,不是「有一些、缺一块」。

## 4. 对本次问题的含义

1. 「CI 加一个鸿蒙车道」在当前仓库状态下的直接后果是:新增一个**没有任何源码为其编译**的车道。
   界面层 `rust/lithe-gpui` 在 OHOS 目标上无后端可编(见 `03-gpui-platform-crates.md`),
   因此这条车道要么永远红,要么只能 `continue-on-error`——后者等于一个不拦截任何东西的装饰。
2. 既有决策并没有禁止将来做鸿蒙,它禁止的是**把平台层与某一端写法绑死**,并把鸿蒙明确排在后面、
   **不承诺**。在决策仍为「提议中」且未修订的情况下,单方面加一条鸿蒙 CI 属于**绕过该决策的排期
   纪律**,而不是执行它。若确实要加,正确做法是先修订这份 Note 的第 8 节(把鸿蒙从「不承诺」
   改为「纳入 CI 编译检查」),再动 workflow。
3. 决策里那句「Rust 核心上鸿蒙没问题」是**尚未被本仓库验证过的主张**,它引的是 Rust 官方
   Tier 2 目标与外部 NAPI 经验,不是本仓库的实测。这正是 `04-ohos-cross-compile-probe.md`
   要实测的对象——决策文本不能当作编译事实使用。

## 5. 与 ohos-rs 的交叉印证

决策 `:308-309` 写「全生态目前只有一个原型在做这件事……且依赖一个个人 fork」,
与 `01-ohos-rs-positioning.md` 采集到的事实一致:ohos-rs 的前两名贡献者是 napi-rs 原作者
与维护者(2127 / 556 提交),其后是依赖机器人;它是 napi-rs 的一次平台分支,不是独立新框架。
两者互相印证,但**都不能证明** GPUI 有鸿蒙后端。
