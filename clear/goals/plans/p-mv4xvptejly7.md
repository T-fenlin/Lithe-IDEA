# 阶段归档 · p-mv4xvptejly7

- 收束时间:2026-10-12T07:38:25.182Z
- 所属目标:g-mv4vxrqwy2q5
- 收束之辞:核心实现层盘点完成，五步全部交付，产出 1 份主报告 + 4 份证据文件。

**拿到的结论**：
1. **构建成本被高估**：既有「15 分钟/次、每天 3～4 次」只对全量成立。实测三档曲线 —— 改宿主 crate 私有代码 1.4～8.7 s、改 `shared` 221～281 s、改 `lithe-ui`/全量 547～671 s。真正的放大器是 `shared`（11 个反向依赖）与 `lithe-ui`，**不是** `workbench`（反向依赖仅 1，改它 8.7 s）。既有建议「优先改 workbench 之外的 crate」方向是反的。
2. **错误处理前提被推翻**：简报点名的四个文件（keymap_io.rs 12 / recent_projects.rs 10 / run/plan.rs 4 / git/changes.rs 1）生产路径 unwrap/expect **全为 0**，全在 `#[cfg(test)]` 内。全目录生产路径仅 40 处，其中 33 处在 tests/ 与测试设施，真正生产路径 4 处。
3. **unsafe 无风险**：`lithe-gpui` 12 处 unsafe 全在 `settings/src/paths.rs` 的 cfg(test) 段，`ENV_LOCK` 串行化不变量成立；生产代码 0 处。
4. **并发模型自洽**：无 tokio、无 `block_on`、无 `RwLock`；`core_json` 40 个调用点基本都在 `background_spawn` 内。唯一结构性债是 `debug_transport.rs` 的 5/10 ms 轮询。
5. **一处已测量的算法债**：`search_model.rs:152 group()` O(n²)，n=20k 时比哈希实现慢 57.1×；但默认上限下约 0.5 ms，是「随规模恶化的债」而非当前热点。
6. **dead_code 18 处全是接线债**：17 处「实现已在只缺调用点」+ 1 处组件库迁移遗留死常量；`todo!`/`unimplemented!`/`TODO` 全为 0。
7. **`lithe-ui` 解耦可量化**：45 个模块中宿主只用 17 个，未使用的 29 个占 4189 KB（61%），每次都在编译。

**优化清单**：O1～O10 共 11～14 人日，四批顺序含依赖链；O6（`lithe-ui` feature 门）需架构师确认。

**4 条未解释项**：2 条交给人（简报更正、工作区 ACL 约束是否仍适用），1 条排除（explorer 泄漏本次未复现，触发条件不存在），1 条解释（测量方法缺陷：沙盒不能写外部 target，已改用工作区内 target）。

**未跑**：未启动任何 Lithe 进程，未跑 cargo test/clippy/fmt，未做实机验证，未做 flamegraph。全部构建结论来自 cargo check/build 计时。

## 步骤
- [advanced] 1. 统计 dead_code 与 unwrap/expect 的生产/测试分布；同时清点 unsafe、并发原语与阻塞调用 → reports/gpui-core-impl-optimization-2026-10.md
- [advanced] 2. 读 Cargo.toml 依赖图，列出 13 个 crate 的依赖方向与分层，判定改动各 crate 的级联重编面 → reports/gpui-core-impl-evidence/dep-layers.md
- [advanced] 3. 实测：无改动 check/build 耗时；touch terminal、workbench、settings、shared、lithe-ui 各自的耗时与重编 crate 数；从既有 target 的产物 mtime 重建历史构建时间线 → reports/gpui-core-impl-evidence/build-measurements.md
- [advanced] 4. 读 search_model.rs / debug_transport.rs / search_index.rs，找出可定位的热点；对 group()/quick_open_rows() 的 O(n^2) 写基准实测 → reports/gpui-core-impl-evidence/hotspots.md
- [advanced] 5. 汇总优化项清单：每项含现状证据/目标/测量方法/人日估算/依赖关系，并给出本维度排期顺序 → reports/gpui-core-impl-evidence/optimization-list.md
