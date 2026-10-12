# 优化项清单与排期输入（娄码通 · 核心实现层）

## 清单

| # | 项 | 现状证据 | 目标 | 测量方法 | 人日 | 依赖 |
|---|---|---|---|---|---|---|
| **O1** | `group()` / `quick_open_rows()` 改哈希索引 | `search_model.rs:152-171`（`paths.iter().position`）、`:205-217`（`rows.iter().any`）；**实测** 20k 时 57.1× / 5.2× | n=10k 时 `group` 从 22.4 ms 降到 <1 ms | `criterion` 基准，n ∈ {1k, 5k, 10k, 20k}，断言哈希版 ≤ 线性版 1/10 | **0.5** | 无 |
| **O2** | 命令目录按帧重建 | `command_palette.rs:174-180` 每次 `visible_rows` 都 `action_catalog(cx)` 重建 Vec 并整体消费；`search_everywhere.rs:196-201` 同 | 目录缓存，仅命令集变化时重建 | 基准 + 分配计数 | **1** | 无 |
| **O3** | `action_matches` 每条候选一次 `format!+to_lowercase` | `search_model.rs:390` | 零分配匹配 | 基准 + 分配计数 | **0.5** | 无 |
| **O4** | `debug_transport` 轮询改阻塞唤醒 | `:280-291` 5 ms、`:293-311` 10 ms 轮询 | 去掉轮询延迟 | 测 DAP 一次往返耗时，改后对比 | **2** | 需 jdtls 资产 + 实机会话 |
| **O5** | `java/events.rs` 读多写少锁换 `RwLock` | `:62-67` 5 个 `Mutex` 字段 | 降低诊断更新串行化 | `perf` / 帧耗时采样 | **1.5** | 需实机 Java 会话 |
| **O6** | `lithe-ui` 模块级 feature 门 | `lithe-ui/src/lib.rs:1-49` 45 个无条件 `pub mod`；宿主只用 17 个；未用 29 个 / 4189 KB | 重建时间与使用面成正比 | 改后实测 `touch lithe-ui/src/lib.rs` 的 `cargo build` 耗时（基线 **671.4 s**） | **3～5** | **需架构师确认** |
| **O7** | 统一 `CARGO_TARGET_DIR` 共享依赖产物 | `lithe-ui/Cargo.toml:128` 独立 workspace | 减少 `gpui-pre` 等公共依赖重复编译 | 改后实测全量重建耗时 | **1** | 需评估 feature 解析冲突 |
| **O8** | 生产路径 `let _ =` 补诊断 | 78 处中约 5～10 处在生产路径，如 `settings/src/theme.rs:169` | 失败有 `S1_*` 打点 | 逐处读码 + 触发失败路径看日志 | **1** | 无 |
| **O9** | `paths.rs` 的 unsafe 补 `// SAFETY:` | `paths.rs:156-157` | 安全不变量成文 | 评审 | **0.2** | 无 |
| **O10** | 清理 3 个终端死常量 | `terminal/src/constants.rs:77/79/86`；`:82-85` 注释自述「当前没有读点」 | 删掉或补读点 | 评审 | **0.2** | 与终端面板改动合并 |

**合计 11～14 人日**（不含 O6 的架构决策等待）。

## 建议顺序与依赖

```
第一批（1 天，只动 workbench 私有代码 ⇒ 每次验证 8.7 s）
  O1 → O3 → O2        同文件、同一次重建、纯收益无风险
        ↓
第二批（1～2 天）
  O8 → O9 → O10       清理类，随相关功能顺带
        ↓
第三批（1 天，独立可测）
  O7
        ↓
待架构确认
  O6                   触碰 lithe-ui 移植边界，必须经架构师
        ↓
环境前置就绪后
  O4 → O5              需 jdtls 资产 + 实机调试会话
```

**关键依赖说明**：

- **O1～O3 无阻塞**：只动 `workbench` 私有代码，实测 `cargo build` **8.7 s**，
  可高频迭代
- **O4、O5 的验收依赖环境前置**：`.artifacts/jdtls` + `.artifacts/jdk` 就位与实机调试会话。
  环境没解决前**可以写代码，不能验收** ⇒ 不应排在需要交付验收结论的位置
- **O6、O7 都动构建配置，必须串行**，且各自需要一次全量重建做基线（各约 9～11 分钟）
- **O6 是唯一需要架构裁决的项**：`lithe-ui` 逐文件移植自 Ely GPUI Components
  （`Cargo.toml:7-8`、`UPSTREAM.md`），加 feature 门会抬高上游 rebase 成本

## 对总排期的三条输入

1. **「15 分钟/次、每天 3～4 次」不应作为排期系数。**
   实测三档曲线：宿主 crate 私有代码 **1.4～8.7 s**、`shared` **221～281 s**、
   `lithe-ui`/全量 **547～671 s**。真正的系数是**「每批涉及 `shared`/`lithe-ui` 的重建次数」**。

2. **`shared` 的 11 个反向依赖是最大单点。**
   任何跨 crate 的接口改动（例如 `CommandId` 加变体、`CoreError` 加变体）都会触发 281 s 重建。
   建议把这类改动**集中到独立批次**，与业务改动分离。

3. **构建链路建好之前做不了验收的项**：O4、O5（需实机调试会话）；
   以及所有需要 `S1_*` 启动日志验证的功能项。O1～O3、O7～O10 不受影响。

## 验收标准（逐项）

| 项 | 完成标准 |
|---|---|
| O1 | `criterion` 基准存在且通过；n=10k 时哈希版 ≤ 线性版 1/10；`cargo test -p lithe-gpui-workbench` 全绿 |
| O2 | 基准显示命令目录不再每帧重建；现有 `search_model` 测试全绿 |
| O3 | 基准 + 分配计数对比；匹配语义不变（现有测试覆盖） |
| O4 | DAP 一次往返延迟有实测前后对比 |
| O5 | 诊断更新期间无 UI 卡顿（帧耗时采样） |
| O6 | `touch lithe-ui/src/lib.rs` 后 `cargo build -p lithe-gpui-app` 耗时显著低于基线 671.4 s；9 个 example 仍可构建 |
| O7 | 全量重建耗时实测低于基线 548.8 s |
| O8 | 触发失败路径时 `S1_*` 有对应打点 |
| O9 | 评审通过 |
| O10 | 无 `#[allow(dead_code)]` 残留（该 3 处） |

**全局门禁**：每批 `cargo fmt` → `cargo clippy` → 受影响 crate `cargo test`。
**不跑** `./scripts/verify-rust-core.sh`（本次不触碰 `rust/lithe-core`）。
涉及 `shared/` 时跑 `./scripts/verify-shared-contracts.sh`。
**矩阵无需改动**（全部为内部优化，无用户可观察变化）。
