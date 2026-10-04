#!/usr/bin/env node
/**
 * sync-gpui-kit-icons.mjs —— 宿主**不再内嵌 gpui-kit 的整份图标目录**（`AllAssets`，1830 个 Lucide 字形），
 * 只登记「实际会被请求」的那批名字（`gpui_kit::assets::icon_assets!` 生成的子集）。
 *
 * 子集 = 下面两部分的并集：
 *   1. 宿主代码里的 `IconName::<名字>`（扫 `rust/lithe-gpui/crates/**` 的 `.rs`）；
 *   2. gpui-kit **组件内部**自己渲染的图标名（`COMPONENT_ICONS`，见下方说明）。
 *
 * 产物写进 `rust/lithe-gpui/crates/app/src/assets.rs` 的 `gpui_kit::assets::icon_assets!` 宏调用里。
 *
 * 用法：
 *   node rust/lithe-gpui/tools/sync-gpui-kit-icons.mjs            # 打印计划（不写文件）
 *   node rust/lithe-gpui/tools/sync-gpui-kit-icons.mjs --apply    # 写回 assets.rs
 *   node rust/lithe-gpui/tools/sync-gpui-kit-icons.mjs --check    # 宿主用到的名字是否都已登记（CI 用）
 */

import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..", "..", "..");
const CRATES_DIR = resolve(REPO_ROOT, "rust/lithe-gpui/crates");
const ASSETS_RS = join(CRATES_DIR, "app/src/assets.rs");

/**
 * gpui-kit **组件内部**会渲染的图标名（不是宿主代码写的那些）。
 *
 * 来源：`gpui-component-0.7.0` 与 `gpui-base-0.7.0` 的 `src/**` 里出现的 `IconName::<名字>`
 * （2026-10-04 实测 51 个）。**升级 gpui-kit 后要重新实测并更新本表**：
 *   rg -o 'IconName::[A-Za-z0-9_]+' <registry>/gpui-component-0.7.0/src <registry>/gpui-base-0.7.0/src | sort -u
 * 漏登记的表现是"对话框/下拉/输入框的图标静默变空白"，不会有编译错误。
 */
const COMPONENT_ICONS = [
  "ArrowDown", "Asterisk", "Ban", "BookOpen", "Calendar", "CaseSensitive", "Check",
  "ChevronDown", "ChevronLeft", "ChevronRight", "ChevronsUpDown", "ChevronUp", "CircleCheck",
  "CircleX", "Close", "Copy", "Ellipsis", "ExternalLink", "Eye", "EyeOff", "Github", "Inbox",
  "Info", "Inspector", "Loader", "Maximize", "Minimize", "Minus", "PanelBottom",
  "PanelBottomOpen", "PanelLeft", "PanelLeftClose", "PanelLeftOpen", "PanelRight",
  "PanelRightClose", "PanelRightOpen", "Plus", "RefreshCw", "Replace", "Search",
  "SortAscending", "SortDescending", "Star", "StarFill", "TriangleAlert", "Undo2", "User",
  "WindowClose", "WindowMaximize", "WindowMinimize", "WindowRestore",
];

const args = new Set(process.argv.slice(2));
const APPLY = args.has("--apply");
const CHECK = args.has("--check");
for (const arg of args) {
  if (!["--apply", "--check"].includes(arg)) {
    console.error(`不认识的参数：${arg}`);
    process.exit(2);
  }
}

function walk(dir, predicate = () => true) {
  if (!existsSync(dir)) return [];
  const out = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...walk(path, predicate));
    else if (entry.isFile() && predicate(path)) out.push(path);
  }
  return out.sort();
}

const hostIcons = new Set();
for (const path of walk(CRATES_DIR, (p) => p.endsWith(".rs"))) {
  for (const match of readFileSync(path, "utf8").matchAll(/\bIconName::([A-Z][A-Za-z0-9_]*)\b/g)) {
    hostIcons.add(match[1]);
  }
}

const wanted = [...new Set([...hostIcons, ...COMPONENT_ICONS])].sort();

const source = readFileSync(ASSETS_RS, "utf8");
const macroPattern =
  /gpui_kit::assets::icon_assets!\(\s*GpuiKitFallbackIcons,\s*\[([\s\S]*?)\]\s*\);/;
const macroMatch = macroPattern.exec(source);
const registered = macroMatch
  ? macroMatch[1]
      .split(/[,\s]+/)
      .map((part) => part.trim())
      .filter((part) => part.length > 0)
  : [];

const missing = wanted.filter((name) => !registered.includes(name));
const stale = registered.filter((name) => !wanted.includes(name));

console.log(`宿主代码用到的图标名：${hostIcons.size}`);
console.log(`gpui-kit 组件内部用到的：${COMPONENT_ICONS.length}`);
console.log(`并集（要登记的子集）：${wanted.length}`);
console.log(`当前 assets.rs 已登记：${registered.length}`);

if (CHECK) {
  if (!macroMatch) {
    console.error("assets.rs 里找不到 `gpui_kit::assets::icon_assets!(GpuiKitFallbackIcons, [..]);`");
    process.exit(1);
  }
  if (missing.length > 0) {
    console.error(
      `有 ${missing.length} 个被引用的图标名没登记（会静默画空白）：${missing.join(", ")}\n` +
        "跑 `node rust/lithe-gpui/tools/sync-gpui-kit-icons.mjs --apply` 修",
    );
    process.exit(1);
  }
  if (stale.length > 0) {
    console.error(
      `有 ${stale.length} 个已登记的图标名不再被引用（可以删掉）：${stale.join(", ")}\n` +
        "跑 `node rust/lithe-gpui/tools/sync-gpui-kit-icons.mjs --apply` 收窄",
    );
    process.exit(1);
  }
  console.log("--check：登记的子集与「宿主 + 组件」用到的名字一致");
  process.exit(0);
}

if (!macroMatch) {
  console.error("assets.rs 里找不到 icon_assets! 宏调用，无法写回");
  process.exit(1);
}

const lines = [];
for (let i = 0; i < wanted.length; i += 4) {
  lines.push(`            ${wanted.slice(i, i + 4).join(", ")},`);
}
const block = ["gpui_kit::assets::icon_assets!(", "    GpuiKitFallbackIcons,", "    [", ...lines, "    ]", ");"].join("\n");

if (!APPLY) {
  console.log("\n将要写入的宏调用：");
  console.log(block);
  console.log("\n（加 --apply 真的写回；--check 只校验）");
  process.exit(0);
}

writeFileSync(ASSETS_RS, source.replace(macroPattern, block));
console.log(`\n已写回 assets.rs：登记 ${wanted.length} 个图标名`);
console.log("接着跑 `cargo fmt --manifest-path rust/Cargo.toml -p lithe-gpui-app`（宏调用的换行交给 rustfmt）");
