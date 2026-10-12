# s2 · 矩阵更新门禁是否覆盖 GPUI（构建发布层证据）

基线提交 `3085f5d1`。本文件只记录可复现的原文引用与检索，不含推断。
检索口径逐条标注；本轮全部为只读检索。

---

## 1. 门禁的覆盖名单（原文）

`scripts/verify-platform-feature-matrix-change.mjs:10`：

```js
const platformPaths = ["macos/Sources", "windows/tauri/src", "windows/tauri/src-tauri", "Plugins/mac", "Plugins/win", "frontend/editor", "rust/lithe-core", "shared/contracts"];
```

逐条清点 **8 条**：

| # | 路径 | 归属 |
|---|---|---|
| 1 | `macos/Sources` | macOS Swift 端 |
| 2 | `windows/tauri/src` | Windows Tauri 前端 |
| 3 | `windows/tauri/src-tauri` | Windows Tauri Rust 宿主 |
| 4 | `Plugins/mac` | macOS 插件包 |
| 5 | `Plugins/win` | Windows 插件包 |
| 6 | `frontend/editor` | 共享 Monaco 表现层 |
| 7 | `rust/lithe-core` | 共享命令层 |
| 8 | `shared/contracts` | 跨端契约 |

**`rust/lithe-gpui` 与 `rust/lithe-ui` 均不在名单内。**
（`rust/lithe-core` 在名单内，但它是另一个 workspace——见 s1 §1。）

---

## 2. 门禁的判定逻辑（原文）

`scripts/verify-platform-feature-matrix-change.mjs:11-32`：

```js
const changed = (paths) => git("diff", "--name-only", "--no-renames", "-z", base, head, "--", ...paths).split("\0").filter(Boolean);
if (changed(platformPaths).length === 0) {
  console.log("matrix change gate passed: no platform implementation paths changed");
} else if (process.env.MATRIX_UPDATE_EXEMPT === "true") {
  ...
} else {
  ...  // 要求 shared/platform-feature-matrix/features/*.json 有实质变更
}
```

即：改动**不命中** `platformPaths` → 直接打印
`matrix change gate passed: no platform implementation paths changed` 并退出 0，
**不检查任何能力 JSON**。

门禁的调用点在 `.github/workflows/verify-platform-feature-matrix.yml:41-47`：

```yaml
      - name: Require matrix updates for platform changes
        if: github.event_name == 'pull_request'
        ...
        run: ./scripts/verify-platform-feature-matrix-change.sh
```

---

## 3. 与仓库规则文档的对照

`AGENTS.md` 与 `.agents/skills/develop-lithe/SKILL.md` 的「Keep the platform feature matrix current」
小节要求：改动平台实现路径的 PR 必须同步更新能力 JSON。该小节列举的平台实现路径
与上述 `platformPaths` 一致（macOS/Windows/Plugins/frontend/rust-core/shared-contracts），
**同样不含 GPUI**。

也就是说：**规则文本与门禁实现是一致的**——两者都还没把 GPUI 当作一个平台实现面。
这不是实现漂移，是登记模型本身尚未包含第三个前端。

---

## 4. 结论

**预测**：`platformPaths` 共 8 条，全是旧两端路径，不含 `rust/lithe-gpui` / `rust/lithe-ui`。
**实际**：8 条，逐条核对无 GPUI 路径。**与预测一致，无异常。**

改一行 `rust/lithe-gpui/**` 或 `rust/lithe-ui/**`：

- `verify-platform-feature-matrix-change.sh` 走第一个分支，直接放行；
- 因此「每批功能必须同步矩阵 JSON」这条硬约束在 GPUI 上**没有门禁执行**;
- 但它仍然写在 `develop-lithe` SKILL 的强约束里（人得自觉遵守）。

**这条缺口的性质与 s1 不同**：s1 是"假绿灯"（门禁跑了但没覆盖），本条是"门禁根本没被触发"。
两者叠加的后果是：GPUI 改动既不被编译校验、也不被能力矩阵约束。

---

## 5. 本步未覆盖

- 未实际发起 PR 触发该工作流；结论来自脚本源码与工作流配置原文。
- 未核对 `matrix-exempt` 标签路径在本条缺口的适用性（该标签只在命中 `platformPaths` 后才有意义）。
- 「是否新增 `gpui` 平台字段」属于主理人 §2.10 / A1 的决策项，本步只确认门禁覆盖范围，
  不对登记方案表态（该决策需人确认）。
