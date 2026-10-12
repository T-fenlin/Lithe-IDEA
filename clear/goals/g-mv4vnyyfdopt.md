# 目标 g-mv4vnyyfdopt(rev2 · open)
- **一句话**:盘点 GPUI 统一前端相对 Windows 与 macOS 两端的功能缺口与优化项，产出带工作量、依赖与验收方式的功能排期
- **主张**:GPUI 统一前端（rust/lithe-gpui）相对 Windows（windows/tauri）与 macOS（macos/ Swift）两端各缺少哪些功能、哪些需要优化，以及按什么顺序与工作量排期补齐。
- **升格门槛**:L3
- **判据(全文)**(未逐条拆分):
  产出 docs/development/gpui-platform-gap-and-schedule-2026-10.md，且同时满足四项可清点条件：①缺口清单表 ≥40 行，每行含 GPUI 现状、对照端、证据路径（文件:行或 JSON id）；②优化项单独成节且每项写明现状证据与目标；③排期表含阶段、周次、人日、依赖、验收方式五列，且阶段间依赖可追溯；④未覆盖范围与未知项单独成节，至少 3 条。
> 这份文件由系统按账本落盘(投影产物);权威是账本里的 `goal/set` 与 `criteria/revised`。
## 假设(3)
- `h-lcaem1` [alive] GPUI 统一前端目前只有 Windows 的构建与发布链路，macOS 上不存在可产出 lithe-gpui 产物的 CI 或脚本。
  - 推翻条件:在 .github/workflows 或 scripts/ 中找到构建/发布 macOS 版 lithe-gpui 产物的入口。
  - scope: {"conditions":{"前端":"GPUI 统一前端 rust/lithe-gpui","对照端":"windows/tauri 与 macos/ Swift EditorFrontend","基线提交":"3085f5d1","分支":"feat/menu-bar-opaque","盘点日期":"2026-10-12"},"ranges":{}}
  - uses (framing declarations): []
  - 断言:1 条
- `h-tjah4t` [refuted] GPUI 相对旧 Windows 端的功能缺口里，多数是「已有实现但零调用点或入口未接」，而非完全没有实现。
  - 推翻条件:抽查缺口项发现多数在 GPUI 侧找不到任何实现入口或数据结构。
  - scope: {"conditions":{"前端":"GPUI 统一前端 rust/lithe-gpui","对照端":"windows/tauri 与 macos/ Swift EditorFrontend","基线提交":"3085f5d1","分支":"feat/menu-bar-opaque","盘点日期":"2026-10-12"},"ranges":{}}
  - uses (framing declarations): []
  - 断言:1 条
- `h-1r6cbm` [alive] 功能矩阵的 windows 列高估了 GPUI 平替进度，因为矩阵没有 gpui 平台字段，且旧端把 7 类能力硬编码为不可用。
  - 推翻条件:矩阵已有 gpui 字段，或 windows 列已按 backend-capabilities.ts 折算。
  - scope: {"conditions":{"前端":"GPUI 统一前端 rust/lithe-gpui","对照端":"windows/tauri 与 macos/ Swift EditorFrontend","基线提交":"3085f5d1","分支":"feat/menu-bar-opaque","盘点日期":"2026-10-12"},"ranges":{}}
  - uses (framing declarations): []
  - 断言:2 条
