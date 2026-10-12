# 阶段归档 · p-mv4wy3wmjpbn

- 收束时间:2026-10-12T07:12:00.986Z
- 所属目标:g-mv4vy2b25aa9
- 收束之辞:视觉层盘点完成，四步全部交付，产出五份证据文件（reports/ui-audit/ 下）。核心成果：①矩阵 macOS 视觉条目实为 6 条（任务书列的 5 条中 project-sidebar-* 是两条），逐条判定 4 条有对应实现、2 条部分，另 2 条含视觉成分的条目检索命中 0；②两套图标命名空间被类型刻意隔开，lithe-ui 的 306 个 Lucide 图标在产品界面零使用（IconName:: 全仓 7 处命中全是注释/断言）；③一次自我更正——原据目录名 legacy/ 判为「两代图标语言」，被 SOURCES.md:11-15 与逐字节实测（legacy/ui 与 macOS expui/general 同名 36 文件全同）推翻，两边同属 expui；④两条硬发现——图标生成器 --check 失败 129 条 exit 1（声明的生成链路不可复现）、7 张图浅色档携带深色美术（71 处走保色路径可见、52 处不可见）；⑤宿主零硬编码色值但 palette_for 未覆盖 tooltip.*/ansi/chart，保留 Ely 暖色默认值，这是浅色下 tooltip 近黑底的机制来源；⑥排期 V1–V10 共 21 人日、四批依赖顺序、7 条 token/组件约定、7 条风险。另登记 2 条未解释项：u-1qkzgi 已解释（目录名误导），u-lv6oc3 仍开着（7 张极性错误图的根因与引入步骤未能从仓库内证据判定）。

## 步骤
- [advanced] 1. 读 shared/platform-feature-matrix/features/ 下 macOS 视觉条目 JSON，记录 id/status/evidence；再在 rust/lithe-ui 与 rust/lithe-gpui 用 grep 找对应物，逐条判定。已交付：6 条主条目 + 4 条附条目逐条判定；project-sidebar-* 实际为 2 条已列全。 → reports/ui-audit/macos-visual-entries.md
- [advanced] 2. 清点 lithe-ui 设计系统实物（主题文件数、pub fn 数、Palette/Syntax 槽位、图标与字体数、文案条目数、画廊页面数）；再查宿主如何消费组件库主题与是否有第二套颜色。已交付。 → reports/ui-audit/design-system-inventory.md, reports/ui-audit/host-theme-wiring.md
- [advanced] 3. 对照 macOS 共享控件 owner 与 IDEA 真源取值，三方对照判定是否存在结构性冲突。已交付：圆角/树行/滚动条/提示/动画/图标族六项对照。 → reports/ui-audit/macos-owner-vs-idea.md
- [advanced] 4. 汇总前四步，产出四段式结论与排期：缺口表、token/组件对齐建议、图标字体缺口、兼容性说明、工作量（人日）与依赖顺序。已交付：V1-V10 行动清单共 21 人日，四批依赖顺序，7 条 token/组件约定，7 条风险。 → reports/ui-audit/visual-gap-and-schedule.md
