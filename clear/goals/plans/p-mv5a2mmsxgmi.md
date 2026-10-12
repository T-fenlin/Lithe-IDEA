# 阶段归档 · p-mv5a2mmsxgmi

- 收束时间:2026-10-12T13:21:46.418Z
- 所属目标:g-mv59zirjh9u3
- 收束之辞:上一阶段的 5 步计划在发现 clear/evidence/ 归系统所有、不可写之后,为把全部产物路径改到 reports/ohos-ci/ 而把各步逐一作废重建,结果计划被清空。实际进展并未浪费:四条判断的取证工作已完成(ohos-rs 定位已采集到 crate 清单与 CI 事实;既有鸿蒙决策已定位到 proposed/architecture 的 GPUI 选型 Note 第 8 节并完成 OHOS 引用 0 命中的检索,证据已落盘 reports/ohos-ci/02-existing-harmony-decision.md),只是尚未通过交付。下一阶段用正确的产物路径重建全部五步并逐条交付。

## 步骤
- [void] 1. 核实 ohos-rs 的真实定位:读其 README 自述、crates/napi 与 crates/sys 的 Cargo.toml(package name、keywords、description)、贡献者构成与 .github/workflows/simple-test.yml 实际跑什么,判定它是「Node-API 绑定层」还是「Rust 对鸿蒙的移植/平台层」。 → clear/evidence/ohos-rs-positioning.md (作废:产物路径原写 clear/evidence/ohos-rs-positioning.md,该目录归系统所有、不可写;改为 reports/ohos-ci/ 下后重建本步,判据与检验不变。)
- [void] 2. 核实仓库既有的鸿蒙平台决策与其效力:定位该决策所在 Note 与行号、状态目录(proposed/implemented/rejected)、原文结论,并检索仓库是否已有任何 OHOS/harmony 实现代码或 CI 引用。 → clear/evidence/existing-harmony-decision.md (作废:产物路径原指向 clear/evidence/(系统所有、不可写),任务已实际完成并落盘到 reports/ohos-ci/02-existing-harmony-decision.md;按正确的产物路径重建本步。)
- [void] 3. 核实界面层对 OHOS 的可编译性:列出 gpui-pre 依赖图中全部平台相关 crate(从 Cargo.lock 与 Cargo.toml 的 target 条件依赖两处),判定是否存在 OHOS 后端,并说明 lithe-gpui 在 OHOS target 上能编出什么。 → clear/evidence/gpui-platform-crates.md (作废:产物路径原指向 clear/evidence/(系统所有、不可写),改为 reports/ohos-ci/ 下重建本步。)
- [void] 4. 实测 rust/lithe-core 对 aarch64-unknown-linux-ohos 的交叉编译:先用 rustup target list 确认目标存在并记录本机是否已装,再运行 cargo check --target aarch64-unknown-linux-ohos,记录退出码、失败 crate 与错误原文。 → clear/evidence/ohos-cross-compile-probe.md (作废:产物路径原指向 clear/evidence/(系统所有、不可写),改为 reports/ohos-ci/ 下重建本步。)
- [void] 5. 综合四条证据给出建议:明确回答「CI 加鸿蒙」做/不做/有条件做,并说明若要做,最小可行形态是什么(编译型检查 vs 仅接缝验证)、代价与不覆盖范围。 → clear/evidence/ohos-ci-recommendation.md (作废:综合步需重排在全部证据步之后:先作废,待四条证据步齐备后重新追加。)
- [void] 6. 核实 ohos-rs 的真实定位:读其 README 自述、crates/napi 与 crates/sys 的 Cargo.toml(package name、keywords、description)、贡献者构成与 .github/workflows/simple-test.yml 实际跑什么,判定它是「Node-API 绑定层」还是「Rust 对鸿蒙的移植/平台层」。 → reports/ohos-ci/01-ohos-rs-positioning.md (作废:补步被追加到计划末尾,落到了综合步 s5 之后,顺序不对(证据步必须先于综合步)。作废后按 证据→综合 的正确顺序重排。)
- [void] 7. 核实 ohos-rs 的真实定位:读其 README 自述、五个 crate 的 package name/keywords/description、build script 与 CI 实际编译运行什么,判定它是「Node-API 绑定层」还是「Rust 对鸿蒙的移植/平台层」。 → reports/ohos-ci/01-ohos-rs-positioning.md (作废:产物路径原指向 clear/evidence/(系统所有、不可写),改为 reports/ohos-ci/ 下重建本步。)
