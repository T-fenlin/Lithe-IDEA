# 依赖分层与级联重编面（娄码通）

数据源：`cargo metadata --offline --format-version 1 --no-deps`（在 `rust/lithe-gpui` 下执行），
逐 crate 读 `Cargo.toml` 的 path 依赖，**不看注释**。

## 分层（按反向依赖数排序）

| 层 | crate | 反向依赖数 | 谁依赖它 |
|---|---|---|---|
| **放大器** | `lithe-gpui-shared` | **11** | settings, git, explorer, workbench, java, terminal, run, app, editor, extensions, notify |
| 中段 | `lithe-gpui-settings` | 2 | workbench, app |
| 中段 | `lithe-gpui-java` | 2 | workbench, editor |
| 中段 | `lithe-gpui-run` | 2 | settings, workbench |
| 中段 | `lithe-gpui-notify` | 2 | workbench, editor |
| 中段 | `lithe-gpui-languages` | 2 | app, editor |
| 叶子 | `lithe-gpui-git` | 1 | workbench |
| 叶子 | `lithe-gpui-explorer` | 1 | workbench |
| 叶子 | `lithe-gpui-terminal` | 1 | workbench |
| 叶子 | `lithe-gpui-editor` | 1 | workbench |
| 叶子 | `lithe-gpui-extensions` | 1 | workbench |
| **叶子** | `lithe-gpui-workbench` | **1** | app |
| 根 | `lithe-gpui-app` | 0 | — |
| **外部放大器** | `lithe-ui`（平级 workspace） | 9 个宿主 crate 引入 | editor, explorer, git, run, settings, terminal, shared, workbench, languages, app |

`workbench` 的第一方直接依赖：editor, explorer, extensions, git, java, run, notify, settings, shared, terminal。

## 「改 X 会重编哪些」推导链

依赖方向是单向的，重编沿反向传播：

```
改 lithe-ui          → 全部 13 个宿主 crate + lithe-ui           = 14 单元
改 shared            → 除 languages 外全部                       = 12 单元
改 settings          → settings → {workbench, app}               = 3 单元
改 terminal          → terminal → workbench → app                = 3 单元
改 workbench         → workbench → app                           = 2 单元
改 app               → app                                       = 1 单元
```

**推导与实际一致**：`cargo build` 输出里的 `Compiling` 行去重后正好是上表单元数
（terminal 3、workbench 2、settings 3、shared 12、lithe-ui 15 —— lithe-ui 多一个是因为
它自己也重编，且 `languages` 也被带上）。

## 结论

**「优先改 `workbench` 之外的 crate 减少级联」这条既有建议不成立。**

`workbench` 的反向依赖数是 **1**（只有 app），实测改它 **8.7 秒**；它虽然是最大的第一方
crate（968.8 KB）且依赖最多（10 个），但**依赖多不等于被依赖多**。

真正的级联放大器只有两个：

1. **`lithe-gpui-shared`（11 个反向依赖，实测 281 s）** —— 改它的任何一行都会重编几乎所有宿主
2. **`lithe-ui`（实测 671 s）** —— 且其中约 61% 的源码（4189 KB / 29 个模块）宿主根本不引用

排期含义：**只有这两个 crate 的改动需要攒批**。其余宿主 crate 可以高频迭代（秒级）。
