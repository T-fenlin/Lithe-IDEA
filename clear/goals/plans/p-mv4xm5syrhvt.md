# 阶段归档 · p-mv4xm5syrhvt

- 收束时间:2026-10-12T07:32:42.740Z
- 所属目标:g-mv4xlo04jgir
- 收束之辞:第一阶段的取证已完成，产出两份证据文件、共 25 条带定位证据的结论（不取裁决）：

- `reports/gpui-extensions-evidence-catalog-contract.md`：A1-A5（扩展目录与数据模型）+ B1-B6（插件清单与能力 API 契约），11 条。核心发现是**实测**得到——`plugins.listCatalog` 在当前基线不存在，`cargo test -p lithe-gpui-extensions` 4 passed / 2 failed，失败项报 `not_supported: Unsupported core command`；根因经 `git merge-base --is-ancestor` 定位为分支谱系分叉（命令只活在 `feat/lithe-gpui-components` 谱系）。
- `reports/gpui-extensions-evidence-page-bundled.md`：C1-C6（扩展管理页）+ D1-D7（内置扩展打包）+ E（规模对照），14 条。核心发现是扩展页只有 2 个持久化动作、`removable` 判定恒为 false 使卸载入口永不可达、GPUI 无 UI 扩展 SDK。

计划本身有两次返工：前两步未声明 `tests`，导致无法在步内对候选判断作裁决，已作废并保留产物。裁决与汇编改在下一阶段用带 `tests` 的步骤完成。

## 步骤
- [void] 1. 把已取证的「扩展目录与数据模型」「插件清单与能力 API 契约」两块写成带行号证据的清单：Core 命令表实况、两个 fixture 的 schema 分歧、GPUI 侧两个平面（catalog 平面 / BUNDLED_EXTENSIONS 平面）的 id 空间、能力 API 的宿主归属。 → reports/gpui-extensions-evidence-catalog-contract.md (作废:原步骤未声明 tests，无法在本步对判断作裁决；产物文件保留，改为带 tests 的等价步骤重新登记。)
- [void] 2. 写「扩展管理页」与「内置扩展打包」两块：页面现有动作与缺失动作、四个状态覆盖、removable 判定的不可达性、图标槽留空与 PLUGIN_ICON 已登记的矛盾、内置资源与两端对照（主题 5 vs 9、图标包 1 vs 6、字体来源）。 → reports/gpui-extensions-evidence-page-bundled.md (作废:同样未声明 tests，无法在本步作裁决；产物文件已写好并保留，改由带 tests 的步骤统一裁决 5 条判断。)
- [void] 3. 汇编最终交付：结论 / 证据与定位 / 风险与条件 / 建议与验收 / 交接给主理人；给出按优先级的行动项、人日估算与依赖关系（含构建链路未通前哪些验收做不了）。 → reports/gpui-extensions-gap-and-schedule.md (作废:为把「证据汇总」与「判断裁决」分开，先作废原交付步，改为先裁决 5 条判断再汇编最终报告。)
