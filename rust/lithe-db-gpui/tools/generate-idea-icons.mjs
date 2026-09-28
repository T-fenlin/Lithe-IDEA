#!/usr/bin/env node
/**
 * generate-idea-icons.mjs —— 把 `lithe-db-gpui/assets/ui-icons/**` 里的 SVG 真源，生成成 gpui 侧可用的
 * Rust 图标清单（资源路径常量表）。
 *
 * 背景：旧前端的 Vite 清单（`ui-icons/idea-assets.generated.ts`，用 `import x from
 * "./idea/.../x.svg?url"` 把每张 SVG 变成 URL，导出 `名称 → { light, dark }`）**已按维护者
 * 要求删除**（出处见 `lithe-db-gpui/assets/README.md` 第 7 节；内容可在 git 历史 `d13b254a` 里追溯）。
 * gpui 侧本来也不认识 `?url`（`lithe-db-gpui/` 下没有 Vite / 没有 `package.json`），所以那份 TS 对 gpui
 * 从来没有用。本脚本产出它的 Rust 等价物：**键名沿用旧前端那份 Vite 清单的历史命名**，同一对
 * `light` / `dark`，但值是 **AssetSource 的资源路径**（`&'static str`），交给
 * `gpui::svg().path(..)` 用。名字与别名**由本脚本按文件名推导**（规则 3），不再读任何 TS。
 *
 * 真源（只读）：
 *   lithe-db-gpui/assets/ui-icons/idea/ 下的 *.svg
 *       —— 实际产出的依据。**每张 SVG 都必须出现在产物里**（脚本会校验无孤儿、无悬空引用）。
 *   lithe-db-gpui/tools/idea-icon-mappings.json
 *       —— 只读参考，**只用来取"旧前端那 95 个显示名"**（`SearchIcon` 这种），让 Rust 侧的
 *           常量名与旧前端对齐。它随旧前端（`windows/`）删除而迁入本目录，所以是可选输入：
 *           文件缺失时脚本仍然成功，
 *           只是退化成"由文件名推导常量名"（见下）。它**不是**那份已删除的 `.ts`。
 *
 * 产物：
 *   lithe-db-gpui/crates/shared/src/icons/idea.rs
 *
 * 用法：
 *   node lithe-db-gpui/tools/generate-idea-icons.mjs          # 重新生成清单
 *   node lithe-db-gpui/tools/generate-idea-icons.mjs --check  # 只校验产物与文件系统一致（不写文件）
 *
 * 命名与 light/dark 配对规则：
 *   1. **配对**：`foo.svg` 的 dark 变体就是同一目录下的 `foo_dark.svg`（IntelliJ expui 的约定，
 *      旧前端 `generate-idea-icons.ts:95` 用的是同一条规则）。
 *   2. **缺 dark 变体**：`dark` 回落到 `light` 的路径**并且** `has_dark: false`。
 *      旧前端也是这么兜的（`generate-idea-icons.ts:131`），但它在类型上看不出"这是兜的"；
 *      Rust 侧多一个 `has_dark` 字段，让调用方与 `--check` 都能区分"真变体"与"兜底"。
 *      ⚠️ `*_dark.svg` **不是独立的图标**，只是同主干的变体：它们会被排除在"基准"扫描之外，
 *      并且只有当同主干的浅色文件存在时才算一个变体；孤立的 `_dark.svg` 会被判为错误
 *      （产物里不允许出现由它生成的常量）。
 *   3. **常量名**：映射文件里有的图标用它的显示名去掉 `Icon` 后缀（`SearchIcon` → `SEARCH_ICON`，
 *      与旧前端 `toIdentifier` 同源）；映射文件里没有的用文件主干 PascalCase
 *      （`chevronDown.svg` → `CHEVRON_DOWN_ICON`）。两种来源都做重名检测，冲突即失败退出。
 *   4. 命名空间前缀 `idea/`：所有路径都带前缀，避免与 gpui-kit `AllAssets` 的 `icons/…`
 *      （Lucide 字形）撞名 —— 两者在同一个 `AssetSource` 里共存，前缀是唯一的区分手段。
 *   5. 产物是**纯 `&'static str` 常量表**，不依赖 gpui（`lithe-db-gpui-shared` 只向下依赖）。
 */

import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..", "..");
/** `lithe-db-gpui/assets` 的绝对路径；`RustEmbed` 里用的是同一目录（见 `crates/app/src/assets.rs`）。 */
const ASSETS_DIR = resolve(REPO_ROOT, "lithe-db-gpui/assets");
const SVG_ROOT = resolve(ASSETS_DIR, "ui-icons/idea");
/** AssetSource 路径的命名空间前缀（规则 4）。 */
const PATH_PREFIX = "ui-icons/idea";
/** 旧前端的显示名映射（只读参考，可选；见文件头「真源」）。 */
const MAPPINGS_JSON = resolve(HERE, "idea-icon-mappings.json");
const OUT_FILE = resolve(REPO_ROOT, "lithe-db-gpui/crates/shared/src/icons/idea.rs");

const CHECK = process.argv.slice(2).includes("--check");

/** 递归列出 `root` 下所有 `.svg`，返回相对 `root` 的 POSIX 路径（排序，保证产物确定）。 */
function listSvgs(root) {
  const found = [];
  const walk = (dir) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) {
        walk(full);
      } else if (entry.isFile() && entry.name.endsWith(".svg")) {
        found.push(relative(root, full).split(sep).join("/"));
      }
    }
  };
  walk(root);
  return found.sort();
}

/** 文件主干 → `SCREAMING_SNAKE_CASE` 常量名主干（下划线保留为分隔）。 */
function toConstStem(name) {
  return name
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/[^A-Za-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .toUpperCase();
}

/** 文件主干 → PascalCase（用于从文件名派生显示名）。 */
function toPascal(name) {
  return toConstStem(name)
    .split("_")
    .filter(Boolean)
    .map((part) => part[0] + part.slice(1).toLowerCase())
    .join("");
}

/**
 * 读旧前端映射：`<相对 ui-icons/idea 的路径> → [显示名…]`（字典序）。
 * 同一张 SVG 经常被多个显示名引用（旧前端确实如此：`CaretDownIcon` 与 `ChevronDownIcon` 都指向
 * `chevronDown.svg`，`settings.svg` 被 `GearIcon` / `GearSixIcon` 共用 —— 13 个路径有别名）。
 * 取字典序**最小**的那个当规范常量名，其余全部进 [`ALIASES`]，这样 95 个显示名一个都不丢。
 */
function loadDisplayNames() {
  if (!existsSync(MAPPINGS_JSON)) {
    console.warn(
      `warning: ${relative(REPO_ROOT, MAPPINGS_JSON)} not found; ` +
        "falling back to file-name-derived constant names (no legacy aliases)",
    );
    return new Map();
  }
  const parsed = JSON.parse(readFileSync(MAPPINGS_JSON, "utf8"));
  const byPath = new Map();
  for (const entry of parsed.icons ?? []) {
    if (!entry.source || !entry.icon) continue;
    const names = byPath.get(entry.source) ?? [];
    if (!names.includes(entry.icon)) names.push(entry.icon);
    byPath.set(entry.source, names);
  }
  for (const names of byPath.values()) names.sort();
  return byPath;
}

const displayNames = loadDisplayNames();
const allSvgs = listSvgs(SVG_ROOT);
/**
 * "基准" SVG：排除 `_dark.svg` 变体文件。变体由主干的条目通过规则 1 关联，不是独立图标。
 */
const baseSvgs = allSvgs.filter((rel) => !rel.endsWith("_dark.svg"));

// ── 组装条目：每张基准 SVG 一条，声明它的 light / dark 路径与常量名 ────────────────────────
const entries = [];
const problems = [];
const usedConsts = new Map();

// 孤立的 `_dark.svg`（没有同名浅色文件）是资源集里的错误，直接报出来。
for (const rel of allSvgs) {
  if (!rel.endsWith("_dark.svg")) continue;
  if (!baseSvgs.includes(rel.replace(/_dark\.svg$/, ".svg"))) {
    problems.push(`orphan dark variant with no light counterpart: ${rel}`);
  }
}

for (const rel of baseSvgs) {
  const darkRel = rel.replace(/\.svg$/, "_dark.svg");
  const hasDark = allSvgs.includes(darkRel);

  const stem = rel.slice(rel.lastIndexOf("/") + 1).replace(/\.svg$/, "");
  const names = displayNames.get(rel) ?? [];
  const display = names[0];
  const constStem = toConstStem(display ? display.replace(/Icon$/, "") : toPascal(stem));
  if (!constStem) {
    problems.push(`cannot derive a constant name from ${rel}`);
    continue;
  }

  const previous = usedConsts.get(constStem);
  if (previous !== undefined) {
    problems.push(
      `constant name collision: ${constStem} from ${previous} and ${rel} ` +
        "(add a mapping entry or rename one of the SVGs)",
    );
    continue;
  }
  usedConsts.set(constStem, rel);

  entries.push({
    constStem,
    light: `${PATH_PREFIX}/${rel}`,
    dark: `${PATH_PREFIX}/${hasDark ? darkRel : rel}`,
    hasDark,
    source: rel,
    names,
  });
}

// **无孤儿**：基准 SVG 之外的浅色文件不允许存在（说明有文件被漏掉）。
const orphans = baseSvgs.filter((rel) => !entries.some((e) => e.source === rel));
if (orphans.length > 0) {
  problems.push(`SVGs with no manifest entry: ${orphans.join(", ")}`);
}
// **无悬空引用**：产物里出现的每个路径都必须真有文件。
for (const entry of entries) {
  for (const path of [entry.light, entry.dark]) {
    if (!existsSync(join(ASSETS_DIR, path))) {
      problems.push(`asset path does not exist on disk: ${path}`);
    }
  }
}
if (problems.length > 0) {
  console.error(problems.join("\n"));
  process.exit(1);
}

entries.sort((a, b) => a.constStem.localeCompare(b.constStem));

// **旧显示名一个都不丢**：95 个显示名里，规范名之外的都进 ALIASES（16 个）。
const aliasRows = [];
for (const entry of entries) {
  for (const name of entry.names.slice(1)) {
    aliasRows.push({ name, constStem: entry.constStem });
  }
}
aliasRows.sort((a, b) => a.name.localeCompare(b.name));

// ── 渲染产物 ────────────────────────────────────────────────────────────────────────────
const darkCount = entries.filter((e) => e.hasDark).length;
const legacyNameCount = aliasRows.length + entries.length;

let out = "";
out += "//! AUTO-GENERATED by `lithe-db-gpui/tools/generate-idea-icons.mjs` —— **不要手改本文件**。\n";
out += "//!\n";
out += "//! 再生成：`node lithe-db-gpui/tools/generate-idea-icons.mjs`（校验：加 `--check`，退出码 0 表示产物与\n";
out += "//! 文件系统一致）。常量名沿用旧前端那份 Vite 清单（`idea-assets.generated.ts`，**已删除**，\n";
out += "//! 内容见 git 历史 `d13b254a`）的历史命名，但名字与别名由生成器按文件名推导；值是\n";
out += "//! [`AssetSource`] 的**资源路径**而不是打包器 URL。gpui 侧的等价物就是本文件。\n";
out += "//!\n";
out += "//! 常量的 `light` / `dark` 是**给 `gpui::svg().path(..)` 用的路径字符串**（AssetSource 键），\n";
out += "//! 不是文件系统路径。渲染入口与主题选择见 `lithe-db-gpui-app` 的 `assets` 模块：\n";
out += "//! `lithe_db_gpui_shared::icons::idea_icon(icon, is_dark)`。\n";
out += "//!\n";
out += `//! 共 ${entries.length} 个图标（其中 ${darkCount} 个有真实的 \`_dark.svg\` 变体，其余 \`dark\` 回落 light，\n`;
out += "//! 由 [`IdeaIcon::has_dark`] 区分）。路径全部以 `ui-icons/idea/` 为前缀，避免与 gpui-kit\n";
out += '//! `AllAssets` 的 `icons/…`（Lucide 字形）在同一 `AssetSource` 里撞名。\n';
out += "//!\n";
out += `//! **与旧前端命名的对应关系**：旧前端那份 Vite 清单（\`idea-assets.generated.ts\`，**已删除**）\n`;
out += `//! 有 ${legacyNameCount} 个显示名，但只对应 ${entries.length} 张 SVG —— 一张 SVG 常被多个显示名共用（如\n`;
out += "//! `CaretDownIcon` 与 `ChevronDownIcon` 都指 `chevronDown.svg`）。本模块为每张 SVG 出一个规范\n";
out += `//! 常量，其余显示名全部保留在 [\`ALIASES\`]，所以那 ${legacyNameCount} 个显示名一个都没丢。\n`;
out += "\n";
out += "/// 一个 IntelliJ `expui` 图标在浅色 / 深色主题下的两个资源路径。\n";
out += "///\n";
out += "/// 两个字段都是 `AssetSource` 的键（相对 `lithe-db-gpui/assets/`），可直接传给\n";
out += "/// `gpui::svg().path(..)`；`gpui-kit` 的 `IconName`（Lucide）走另一条路径 `icons/…`。\n";
out += "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n";
out += "pub struct IdeaIcon {\n";
out += "    /// 浅色主题使用的 SVG 资源路径。\n";
out += "    pub light: &'static str,\n";
out += "    /// 深色主题使用的 SVG 资源路径；没有真实 dark 变体时等于 `light`。\n";
out += "    pub dark: &'static str,\n";
out += "    /// `dark` 是否是磁盘上真实存在的 `_dark.svg`（`false` 表示 `dark` 是 `light` 的回落值）。\n";
out += "    pub has_dark: bool,\n";
out += "}\n";
out += "\n";
out += "impl IdeaIcon {\n";
out += "    /// 按当前主题明暗挑路径。`dark` 为 `true` 时取深色变体，否则取浅色。\n";
out += "    pub const fn path(&self, dark: bool) -> &'static str {\n";
out += "        if dark {\n";
out += "            self.dark\n";
out += "        } else {\n";
out += "            self.light\n";
out += "        }\n";
out += "    }\n";
out += "}\n";
out += "\n";
out += "/// 全部图标，顺序与常量声明一致（按常量名字典序）。\n";
out += `pub const ALL: [IdeaIcon; ${entries.length}] = [\n`;
for (const entry of entries) {
  out += `    ${entry.constStem}_ICON,\n`;
}
out += "];\n";
out += "\n";

for (const entry of entries) {
  out += `/// \`${entry.light}\`${entry.hasDark ? ` / \`${entry.dark}\`` : "（无 dark 变体）"}\n`;
  out += `pub const ${entry.constStem}_ICON: IdeaIcon = IdeaIcon {\n`;
  out += `    light: "${entry.light}",\n`;
  out += `    dark: "${entry.dark}",\n`;
  out += `    has_dark: ${entry.hasDark},\n`;
  out += "};\n";
}

out += "\n";
out += "/// 旧前端那份 Vite 清单（`idea-assets.generated.ts`，**已删除**）里那些**别名**显示名 → 规范常量。\n";
out += "///\n";
out += "/// 键就是那份清单的 `ideaIconAssets` 键（`\"ChevronDownIcon\"` 这种**原样**未截断的显示名）；\n";
out += "/// 别名由生成器按 `lithe-db-gpui/tools/idea-icon-mappings.json` 推导，不再读那份 TS。\n";
out += "/// 只为迁移期与文档对照保留；新代码应该直接用规范常量（[`ALL`] 里的那些）。\n";
out += `pub const ALIASES: [(&str, IdeaIcon); ${aliasRows.length}] = [\n`;
for (const row of aliasRows) {
  out += `    ("${row.name}", ${row.constStem}_ICON),\n`;
}
out += "];\n";

// ── 写入或校验 ──────────────────────────────────────────────────────────────────────────
if (CHECK) {
  const existing = existsSync(OUT_FILE) ? readFileSync(OUT_FILE, "utf8") : null;
  if (existing !== out) {
    console.error(
      existing === null
        ? `${relative(REPO_ROOT, OUT_FILE)} is missing; rerun the generator`
        : `${relative(REPO_ROOT, OUT_FILE)} is out of date; rerun the generator`,
    );
    process.exit(1);
  }
  console.log(
    `idea-icons check passed (${entries.length} icons, ${darkCount} with a dark variant)`,
  );
  process.exit(0);
}

writeFileSync(OUT_FILE, out);
console.log(
  `Wrote ${relative(REPO_ROOT, OUT_FILE)} (${entries.length} icons, ` +
    `${darkCount} with a dark variant, ${entries.length - darkCount} falling back to light)`,
);
