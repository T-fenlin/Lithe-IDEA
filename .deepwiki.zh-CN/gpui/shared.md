# Host Shared Crate

`rust/gpui/crates/shared/` 是所有 gpui crate 共同依赖的层。它依赖 `gpui-kit`、`lithe-core`（进程内）、`serde`、`uuid` 和 `rust-i18n`，并且**不依赖任何其他 gpui crate**。

| 文件 | 职责 |
| --- | --- |
| `core_client.rs` | Core 信封层：请求组装、响应解包、`CoreError` |
| `document.rs` | 版本化 JSON 原语：每 key 容错解析、未知 key 保留、原子写入 |
| `i18n.rs` | `tr` / `tr_args` 包装，以及 wired-key 回归表 |
| `icons/mod.rs` | 图标渲染辅助（`idea_icon_svg`、`FileIcon`）和 4 步“如何引用图标”的契约 |
| `icons/idea.rs` | 自动生成的 IntelliJ `expui` 常量表（79 个图标 + 16 个别名） |
| `icons/file_icon.rs` | 文件类型图标主题查询，以及 Lucide fallback |
| `workspace_config/mod.rs` | `.lithe/` 模块根、项目身份、session、toolchain 层、“默认不共享”的保护 |
| `workspace_config/paths.rs` | 所有 `.lithe/` 成员路径的唯一命名源 |
| `workspace_config/project.rs` | `.lithe/project.json` 读写，稳定 UUID v4 |
| `workspace_config/session.rs` | `.lithe/session.local.json` |
| `workspace_config/toolchain.rs` | 五个 toolchain 覆盖值 + `project-local > global > auto` 解析 |
| `workspace_config/sharing.rs` | 把 `.lithe/` 写入本地 Git exclude，并维护 `.lithe/.gitignore` |
| `workspace_config/test_repo.rs` | `#[cfg(test)]` 的临时 Git repo，保证 guard tests 不碰外部仓库 |
| `locales/` | `lithe.en.yml`、`lithe.zh-CN.yml`、提取 README，以及 vendored `source/*.ts` |

## `core_client.rs` —— 调用 Core 的唯一入口

```rust
pub const DEFAULT_TIMEOUT_MILLIS: u64 = 120_000;              // :37
static NEXT_OPERATION_ID: AtomicU64 = AtomicU64::new(1);       // :40

pub struct CoreRequest { /* 所有字段 PRIVATE */ }            // :121-126
pub struct CoreClient;                                         // :191  (unit, Copy)
pub fn core_json(command, payload) -> Result<Option<Value>, CoreError>;  // :241
```

值得了解的设计：

- **字段都是 private**，这样调用方不能伪造信封 key（`:115-118`）。
- **`id` 和 `operationId` 永远相同。** 若未通过 `with_operation_id` 显式绑定，值来自全局计数器 `format!("lithe-gpui-{command}-{n}")`。原因在 `:166-168`：Core 把 `operationId` 用于取消和陈旧结果，`id` 用于响应关联；在这个 app 中两者设计成 1:1。
- **`CoreClient` 是一个零大小命名类型**，因此“调用 Core”是可搜索的名字，而不是自由函数（`:188-189`）。
- **调用是同步的。** 这个模块文档是一条指令，而非描述：调用方必须用 `cx.background_spawn` 包起来，绝不能在 `render` 或 UI 线程里直接调用（`:27-28`）。

`CoreError` 有三个变体（`:48-68`）：`InvalidResponse`、`Reported { code, message }`、`MissingData`。两个细节：

- `code()` 仅在 `Reported` 时返回 `Some`，并且其注释明确要求**不要据此分支**（`:71-80`）。
- `is_not_a_repository()` 是仅有的分支谓词，匹配 Core 的稳定信号 `code == "invalid_request" && message == "Not a Git repository"`（`:82-99`），并且对应测试固定了两半（`:254-286`）。

这个模块刻意不重解释领域级 “success” 值——例如 `git.status` 返回 `repositoryRoot: null` 并不意味着错误（`:44-46`）。

## `document.rs` —— JSON 文档原语

这个产品中的所有设置类文档都使用它。三个点很重要：

1. **`known_keys::<T>()` 是从 `T::default().serialize()` 推导出来的**（`:63-73`）。没有手写 key 列表，因为之前曾经出现过“写入但读回失败”，且**没有任何诊断**。两条硬约束随之出现：容器需要 `#[serde(default)]`，而且**不能有 `skip_serializing_if`**——否则字段会被当成未知 key，最终丢失。
2. **`parse` 是按 key 容错，不会整体失败**（`:75-166`）。先尝试整份 `serde` 解析，再回退到按 key 重试，所以一个坏 key 不会把整份文档一并带走。
3. **写入是原子方式**：固定的 `.tmp` 同级临时文件 + `rename`（`:225-266`），在 Windows 上等价于 `MoveFileEx(MOVEFILE_REPLACE_EXISTING)`。

`merge_document` + `preserve_unknown` 会递归保留未知 key，包括嵌套对象（`:168-223`）。`read_document_text` 对缺失文件返回 `Ok(None)`，并且绝不创建任何内容（`:268-275`）。

## 图标

两条独立解析路径。

**UI 图标** —— `icons/idea.rs` 由 `gpui/tools/generate-idea-icons.mjs` 生成（`--check` 会校验它）。`struct IdeaIcon { light, dark, has_dark }` 保存的是 `AssetSource` key，而不是文件系统路径。key 都带前缀 `ui-icons/idea/`，这样不会和 gpui-kit 自身的 Lucide `icons/...` key 在同一个 `AssetSource` 中冲突（`:13-14`）。

**文件类型图标** —— `icons/file_icon.rs` 解析 `icon-themes/idea/extension.json`，该文件通过 `include_str!` 在编译时嵌入，因此缺文件是编译错误，而不是静默运行时 fallback（`:82-89`）。它在进程内只解析一次并保存到 `OnceLock`；解析失败会做缓存，不会重复尝试。查找顺序：文件夹 -> `expandedFolders` -> `folders` -> `idea-folder`；文件 -> `filenames` -> **最长到最短后缀** -> `idea-text`（`:156-179, 337-363`）。

主题 id 是编译期常量 `ACTIVE_FILE_ICON_THEME = "idea"`（`:67-72`），这也是为什么其他 3 个图标主题包被排除出二进制——运行时根本无法到达。图标主题切换目前不存在。

**渲染规则**（已测量，且反直觉）：

- gpui 把 SVG 渲染成 **alpha mask**（`MonochromeSprite`，只看 `pixel.alpha()`），因此 SVG 自己的 `fill` 会被忽略。必须显式设置颜色；没有 `text.color` 就完全不画，并且 SVG 自身没有固有尺寸，所以 flex child 会被压成 0 宽——`icons/mod.rs:170-219`。
- `idea_icon_svg_px` 要求 `rems(..)`，不能用 `px(..)`，这样图标会随主题字体大小缩放（`:221-240`）。
- `struct FileIcon { themed: Option<&'static str>, fallback: IconName }` 把双源 fallback 变成**类型级保证**：fallback 永远存在（`:75-99`）。
- 文件类型图标用 `svg()` 渲染单色，而不是 `img()`——`Img` 在 tree row 和状态栏 chip 中会间歇性失效（`:262-280`）。

## i18n

`rust_i18n::i18n!("locales", fallback = "en")` 位于**这个 crate 的根**（`lib.rs:47`），并且必须保留在那里。`t!` 会展开成 `crate::_rust_i18n_try_translate(..)`，这是 `i18n!` 在**调用 crate** 中生成的函数，因此在 `app` 中再 `i18n!` 会创建第二套后端，产生两套真相（`lib.rs:32-46`, `i18n.rs:16-29`）。

- `tr(key) -> SharedString` —— 之所以返回 `SharedString`，因为 call sites 是 GPUI 的 `.child(..)` / `.text(..)`，而 `String` 不是 `IntoElement`。
- `tr_args(key, &[(&str,&str)])` —— 前端占位符语法是 `{name}`，而 `rust-i18n` 4.2 只认识 `%{name}`，所以纯 `interpolate` 会按顺序执行 `String::replace`，并且**保留未知占位符原文**（`i18n.rs:9-12, 115-121, 143-147`）。
- 占位符名称来自 locale 值，而不是调用方的 `format!`（`:83-88`）。
- `WIRED` 表（`i18n.rs:161-931`）列出 388 个 key。测试 `every_wired_key_resolves_in_both_locales` 会拿锁（locale 是进程全局），用 `Drop` guard 恢复上一语言，并断言每个 key 在 zh-CN 中存在并与预先写死的中文字符串一致，而在 `en` 中存在且**与 zh-CN 不同**，还有 8 个产品名例外（`:159-1012`）。
- 模块文档明确定义了哪些内容**不通过 i18n**：内部诊断、缺失 key、占位符复制、CLI / 测试 fixture（`:59-82`）。

`locales/*.yml` 由 `gpui/tools/extract-locale.mjs` 从 vendored `locales/source/*.ts` 生成。`locales/README.md` 是生成契约（`_version: 2`，key 先于 locale，`lithe:` namespace 用于 app 字符串，`gpui_component:` 用于组件覆盖）。yml 和 README 的 header counts 已偏离真实 key count——文件本身才是最终权威。

## `workspace_config/` —— `.lithe/`

四层结构，项目级覆盖使用**文件名后缀** `.local.json`，使“是否应该提交”在名称中直接可见，并且 `.gitignore` 需要一条规则（`paths.rs:31-32`）：

```
built-in defaults  <  global settings.json  <  .lithe/settings.json  <  .lithe/settings.local.json
```

`WorkspaceConfigPaths`（`paths.rs:60-160`）命名所有成员：`project.json`、`settings.json`、`settings.local.json`、`git.json`、`run/configurations.json`、`run/generated.json`、`run.local.json`（顺带 legacy `run/local.json` 作为只读 fallback）、`toolchains/requirements.json`、`toolchains.local.json`、`maven/config.json`、`maven.local.json`、`lsp/language-providers.json`、`session.local.json`、`.gitignore`。

### 项目身份

`ProjectManifest { id: Option<String> }`，`PROJECT_MANIFEST_VERSION = 1`。`id` 是**UUID v4**，第一次使用时生成并写入，之后 `resolve_project_id` 永远不重写已存在值（`project.rs:249-284`）。UUID 形状被断言（36 字符，version nibble 为 `4`，variant `8|9|a|b`）。回退身份是 `path_identity(root)` —— Core 的 `lsp.jdtWorkspaceKey` 去掉可选 fingerprint 后的版本，即 `normalized_workspace_identity + SHA-256`（`:286-302`）。

三个微妙规则：

- `id` 不能使用 `skip_serializing_if`，否则它会从 `known_keys` 派生集合中消失，导致另一个 key 出错时它会变成不可读（`:21-25`）。
- **新于支持版本**的文档是只读：读取它并回退到 path identity，永远不覆盖（`:261-270`）。
- 没有 `id` 时在 guard 之前就被拒绝写入，避免 `id: null` 覆盖现有身份（`:124-125, 213-221`）。

这个模块不拥有的字段（例如 Core 的 `defaultRunConfiguration`）会通过 `document::preserve_unknown` 原样透传。

### Toolchain 层

五个值分布在两个文件：`run.local.json` -> `toolchain.java.homePath`、`toolchain.maven.executablePath`、`toolchain.maven.javaHomePath`；`maven.local.json` -> `settingsPath`、`localRepositoryPath`。

优先级是 `project-local > global default > auto-discovery`，且在 `ToolchainPaths::resolve(local, global) -> (Self, OverrideOrigins)` 中用单一 `pick` 闭包实现所有五个字段（`toolchain.rs:158-215`）。正是这个单实现保证 settings 页的“effective value”和语言服务注册永远一致。

- **空字符串表示“这里没设置”**，同时也是“清空值”的方式，因为省略字段会让 unknown-key preservation 把旧值重新带回来（`:32-37`）。
- **旧版双重读取是按文件级，而不是字段级**：先读 `run.local.json`；不存在时再读 `.lithe/run/local.json`。新文件永远优先，即使值为空也如此（`:38-44`）。写出只走新路径。
- `ensure_writable` 在磁盘文档声明的版本更新时会拒绝整份保存（`:406-434`）。

### Session

`SessionState { open_files, active_file, left_sidebar_visible, top_activity_view, bottom_kind, bottom_visible, right_view, right_visible }`，version 1，`MAX_OPEN_FILES = 40`，并带 documented truncation rule：追加 active file，以保证它永远不会丢失（`session.rs:61-76`）。`resolve_session_files` 在**恢复时**跳过 `outside_root` / `missing`，而不是读时处理（`:256-307`）。`is_plain_relative` 拒绝空、绝对路径（包括 `C:foo`）和 `..` 路径（`:309-327`）。

### “默认不共享” 保护

`sharing.rs` 负责实现“打开项目不能把机器本地配置提交到仓库”的规则。两道门槛：

1. `<repo>/.git/info/exclude` 通过 Core `git.write { operation: excludePatterns }` 实现。
2. `.lithe/.gitignore` 中写入 `['*.local.json', 'run/classes/', '**/*.tmp']`，是 append-only 且幂等的（所有都存在时不重复写）。

exclude 文件的位置通过 Core 的 `git.watchContext` -> `gitCommonDirectory` 询问，因为 worktree、submodule 和 bare repo 会把它放在别处（`:151-171`）。`excludePatterns` 选用而不是 `exclude`，是因为它能精确表达“相同的本地配置禁用共享是同一种行为”。

## 参考资料

- `rust/gpui/crates/shared/` 下全部文件
- `gpui/tools/extract-locale.mjs`
- `shared/contracts/application-boundary.md`
- `shared/contracts/*` 相关文档
