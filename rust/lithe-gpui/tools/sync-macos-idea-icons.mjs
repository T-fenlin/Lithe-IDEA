#!/usr/bin/env node
/**
 * sync-macos-idea-icons.mjs —— 把 **macOS 图标文件夹**（`macos/Resources/IDEAIcons/**`，界面真源）
 * 复制进 gpui 宿主，并让它与宿主旧图标包（来自已删除的 `windows/tauri/src/ui/icons/**`）**取并集**：
 *
 *   rust/lithe-gpui/assets/icons/idea/
 *     ├── <macOS 原样布局>        fileTypes/ expui/ general/ nodes/ vcs/ debugger/ terminal/
 *     │                           toolwindows/ testState/ gutter/ maven/ actions/  ← 逐字节复制
 *     ├── legacy/**               旧包里「macOS 没有、但界面仍在用」的文件（保留原相对路径）
 *     ├── LICENSE-APACHE-2.0.txt  随 macOS 文件夹一并复制（Apache-2.0 归属）
 *     ├── NOTICE.txt
 *     ├── SOURCES.md              来源与保留清单（人读）
 *     └── extension.json          图标主题清单：定义路径改指新布局，id / fileExtensions 不变
 *
 * 同时**删掉** `assets/icons/` 下其余图标主题文件夹（`lithe/ symbols/ material/ pierre/`）——
 * 它们与 `idea/` 一样都是从已删除的 Windows 前端搬来的副本，现在只保留这一套。
 *
 * 「仍在用」= 下列两者之一，别的旧文件一律删除：
 *   1. `extension.json` 的 `fileExtensions` / `folders` / 三个默认值点名到的定义；
 *   2. 宿主代码里 `idea::<CONST>` 引用到的常量（由 `idea.rs` 反查其 light/dark 文件）。
 *
 * 同时重新生成 `crates/shared/src/icons/idea.rs`（常量名保持不变，路径改指新布局；不写注释，
 * 与「清空 gpui 全部代码注释」那条裁决一致）。
 *
 * 用法：
 *   node rust/lithe-gpui/tools/sync-macos-idea-icons.mjs --dry-run   # 只打印计划，不写文件
 *   node rust/lithe-gpui/tools/sync-macos-idea-icons.mjs --apply     # 真的写
 *   node rust/lithe-gpui/tools/sync-macos-idea-icons.mjs --check     # 校验包与两套来源一致
 *
 * 映射规则（legacy 路径 → macOS 文件）：
 *   1. 去掉 legacy 的 `icons/` 前缀与目录，取主干 + 主题后缀（`_dark` / `_light`）；
 *   2. 在 macOS 侧按「归一化主干」找同名文件（大小写、`-`/`_` 不敏感）；
 *   3. 主题极性按 IntelliJ 约定换算：legacy 的 `X.svg`（浅色）优先取 macOS 的 `X_light.svg`
 *      （没有就取 `X.svg`）；legacy 的 `X_dark.svg` 优先取 macOS 的 `X.svg`（没有就取 `X_dark.svg`）；
 *   4. `tools/macos-idea-icon-map.json` 的 `overrides` 可以逐条改写上面的自动结果（语义不同名时用它）。
 */

import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..", "..", "..");
const MACOS_DIR = resolve(REPO_ROOT, "macos/Resources/IDEAIcons");
const PKG_DIR = resolve(REPO_ROOT, "rust/lithe-gpui/assets/icons/idea");
const MANIFEST = join(PKG_DIR, "extension.json");
const MAP_JSON = join(HERE, "macos-idea-icon-map.json");
const IDEA_RS = resolve(REPO_ROOT, "rust/lithe-gpui/crates/shared/src/icons/idea.rs");
const CRATES_DIR = resolve(REPO_ROOT, "rust/lithe-gpui/crates");

const LEGACY_SOURCE = resolve(REPO_ROOT, "windows/tauri/src/ui/icons");
const ASSETS_ICONS = resolve(REPO_ROOT, "rust/lithe-gpui/assets/icons");
const LEGACY_ROOT_NAME = "icons";
const LEGACY_DIR = "legacy";
const REMOVED_THEME_DIRS = ["lithe", "symbols", "material", "pierre"];

const args = new Set(process.argv.slice(2));
const APPLY = args.has("--apply");
const CHECK = args.has("--check");
const DRY_RUN = args.has("--dry-run");
for (const arg of args) {
  if (!["--apply", "--check", "--dry-run"].includes(arg)) {
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

const toPosix = (path) => path.split(sep).join("/");
const relFrom = (root, path) => toPosix(relative(root, path));

function splitVariant(baseName) {
  const match = /^(.*?)(?:_(dark|light))?$/i.exec(baseName);
  return { stem: match[1], variant: match[2] ? match[2].toLowerCase() : null };
}

const normalizeStem = (stem) => stem.replace(/[^A-Za-z0-9]/g, "").toLowerCase();

// ---------------------------------------------------------------------------
// 读入
// ---------------------------------------------------------------------------

const macosFiles = walk(MACOS_DIR);
const macosSvg = macosFiles.filter((path) => path.endsWith(".svg"));
const macosIndex = new Map();
for (const path of macosSvg) {
  const rel = relFrom(MACOS_DIR, path);
  const base = rel.split("/").pop().replace(/\.svg$/i, "");
  const { stem, variant } = splitVariant(base);
  const key = normalizeStem(stem);
  const entry = macosIndex.get(key) ?? { plain: null, light: null };
  if (variant === "light") entry.light = rel;
  else if (variant === null) entry.plain = rel;
  macosIndex.set(key, entry);
}

const legacySvgPaths = walk(join(PKG_DIR, LEGACY_ROOT_NAME), (path) => path.endsWith(".svg"));
const legacyRel = legacySvgPaths.map((path) => relFrom(PKG_DIR, path));
const legacyContents = new Map(legacyRel.map((rel) => [rel, readFileSync(join(PKG_DIR, rel))]));
const legacyByRel = new Set(legacyRel);

const manifest = JSON.parse(readFileSync(MANIFEST, "utf8"));
const iconBlock = manifest.icons[0];
const overrides = existsSync(MAP_JSON)
  ? JSON.parse(readFileSync(MAP_JSON, "utf8")).overrides ?? {}
  : {};

const ideaRs = readFileSync(IDEA_RS, "utf8");
const constants = [];
{
  const lines = ideaRs.split("\n");
  for (let i = 0; i < lines.length; i += 1) {
    const head = /^pub const ([A-Z0-9_]+): IdeaIcon = IdeaIcon \{$/.exec(lines[i]);
    if (!head) continue;
    const light = /^\s*light: "([^"]+)",$/.exec(lines[i + 1]);
    const dark = /^\s*dark: "([^"]+)",$/.exec(lines[i + 2]);
    if (light && dark) constants.push({ name: head[1], light: light[1], dark: dark[1] });
  }
}

const referencedConstants = new Set();
for (const path of walk(CRATES_DIR, (p) => p.endsWith(".rs"))) {
  for (const match of readFileSync(path, "utf8").matchAll(/\bidea::([A-Z][A-Z0-9_]*)\b/g)) {
    referencedConstants.add(match[1]);
  }
}

const referencedDefinitions = new Set([
  ...Object.values(iconBlock.fileExtensions ?? {}),
  ...Object.values(iconBlock.folders ?? {}),
  ...[iconBlock.defaultFile, iconBlock.defaultFolder, iconBlock.defaultFolderOpen].filter(Boolean),
]);

// ---------------------------------------------------------------------------
// 映射与计划
// ---------------------------------------------------------------------------

function macosTargetFor(legacyPath) {
  if (Object.prototype.hasOwnProperty.call(overrides, legacyPath)) return overrides[legacyPath];
  const base = legacyPath.split("/").pop().replace(/\.svg$/i, "");
  const { stem, variant } = splitVariant(base);
  const entry = macosIndex.get(normalizeStem(stem));
  if (!entry) return null;
  if (variant === "dark") return entry.plain ?? entry.light ?? null;
  return entry.light ?? entry.plain ?? null;
}

function newPathForLegacy(legacyPath) {
  const target = macosTargetFor(legacyPath);
  if (target) return target;
  return `${LEGACY_DIR}/${legacyPath.replace(/^icons\//, "")}`;
}

function neededLegacyPaths() {
  const needed = new Set();
  for (const group of ["iconDefinitions", "lightIconDefinitions"]) {
    for (const defs of Object.values(iconBlock[group] ?? {})) {
      for (const [name, path] of Object.entries(defs)) {
        if (!referencedDefinitions.has(name)) continue;
        const rel = path.replace(/^\.\//, "");
        if (legacyByRel.has(rel)) needed.add(rel);
      }
    }
  }
  for (const constant of constants) {
    if (!referencedConstants.has(constant.name)) continue;
    for (const path of [constant.light, constant.dark]) {
      const rel = path.replace(/^icons\/idea\//, "");
      if (legacyByRel.has(rel)) needed.add(rel);
    }
  }
  return needed;
}

const neededLegacy = neededLegacyPaths();
const keptConstants = constants.filter(
  (constant) => referencedConstants.has(constant.name) || constant.name === "ALL",
);
const droppedConstants = constants.filter(
  (constant) => !referencedConstants.has(constant.name) && constant.name !== "ALL",
);
const droppedDefinitions = [];
for (const group of ["iconDefinitions", "lightIconDefinitions"]) {
  for (const [defName, defs] of Object.entries(iconBlock[group] ?? {})) {
    for (const name of Object.keys(defs)) {
      if (!referencedDefinitions.has(name)) droppedDefinitions.push(`${group}.${defName}.${name}`);
    }
  }
}
const legacyKeptMacos = [...neededLegacy].filter((rel) => macosTargetFor(rel));
const legacyKeptAsIs = [...neededLegacy].filter((rel) => !macosTargetFor(rel));
const legacyDrop = legacyRel.filter((rel) => !neededLegacy.has(rel));

function report() {
  console.log(`macOS 图标文件夹：${macosSvg.length} 个 SVG + ${macosFiles.length - macosSvg.length} 个附件 → 逐字节复制进 gpui`);
  console.log(`旧包（Windows 派生）：${legacyRel.length} 个 SVG`);
  console.log(`  · 保留：${neededLegacy.size}（其中 ${legacyKeptMacos.length} 个换成 macOS 美术、${legacyKeptAsIs.length} 个继续用旧美术）`);
  console.log(`  · 删除（清单与代码都没引用）：${legacyDrop.length}`);
  console.log(`idea.rs 常量：保留 ${keptConstants.length}、删除 ${droppedConstants.length}`);
  console.log(`清单定义：删除未被 fileExtensions/folders/默认值引用的 ${droppedDefinitions.length} 条`);
}

if (DRY_RUN) {
  report();
  console.log("\n--- 保留但 macOS 没有的旧文件（前 40）---");
  for (const rel of legacyKeptAsIs.slice(0, 40)) console.log(`  ${rel}`);
  console.log("\n--- 会被删除的旧文件（前 40）---");
  for (const rel of legacyDrop.slice(0, 40)) console.log(`  ${rel}`);
  console.log("\n--- 常量新旧路径（前 25）---");
  for (const constant of keptConstants.slice(0, 25)) {
    console.log(`  ${constant.name}: ${constant.light} -> ${newPathForLegacy(constant.light)}`);
  }
  process.exit(0);
}

if (CHECK) {
  report();
  const problems = [];
  for (const constant of keptConstants) {
    for (const path of [constant.light, constant.dark]) {
      const rel = path.replace(/^icons\/idea\//, "");
      if (!existsSync(join(PKG_DIR, rel))) problems.push(`${constant.name} -> ${rel} 不存在`);
    }
  }
  for (const path of macosFiles.map((p) => relFrom(MACOS_DIR, p))) {
    if (!existsSync(join(PKG_DIR, path))) problems.push(`macOS 文件 ${path} 没被复制进来`);
  }
  if (problems.length > 0) {
    console.error(`--check 失败（${problems.length} 条），例如 ${problems.slice(0, 5).join("；")}`);
    process.exit(1);
  }
  console.log("--check：macOS 副本与 idea.rs 引用都一致");
  process.exit(0);
}

if (!APPLY) {
  console.error("请显式指定 --dry-run / --apply / --check");
  process.exit(2);
}

// ---------------------------------------------------------------------------
// 应用
// ---------------------------------------------------------------------------

rmSync(PKG_DIR, { recursive: true, force: true });
mkdirSync(PKG_DIR, { recursive: true });
for (const dir of REMOVED_THEME_DIRS) {
  rmSync(join(ASSETS_ICONS, dir), { recursive: true, force: true });
}

for (const rel of macosFiles.map((path) => relFrom(MACOS_DIR, path))) {
  const target = join(PKG_DIR, rel);
  mkdirSync(dirname(target), { recursive: true });
  copyFileSync(join(MACOS_DIR, rel), target);
}

for (const rel of [...neededLegacy].sort()) {
  const target = join(PKG_DIR, newPathForLegacy(rel));
  mkdirSync(dirname(target), { recursive: true });
  writeFileSync(target, legacyContents.get(rel));
}

// 清单：只留被引用的定义，路径改指新布局
const nextManifest = JSON.parse(JSON.stringify(manifest));
const nextBlock = nextManifest.icons[0];
for (const group of ["iconDefinitions", "lightIconDefinitions"]) {
  const nextGroup = {};
  for (const [defName, defs] of Object.entries(iconBlock[group] ?? {})) {
    const kept = {};
    for (const [name, path] of Object.entries(defs)) {
      if (!referencedDefinitions.has(name)) continue;
      const rel = path.replace(/^\.\//, "");
      kept[name] = `./${newPathForLegacy(rel)}`;
    }
    nextGroup[defName] = kept;
  }
  nextBlock[group] = nextGroup;
}
for (const name of referencedDefinitions) {
  const found = ["iconDefinitions", "lightIconDefinitions"].some((group) =>
    Object.values(nextBlock[group] ?? {}).some((defs) => name in defs),
  );
  if (!found) {
    console.error(`清单里的 ${name} 在任何分组里都不存在，fileExtensions/folders 会指向空定义`);
    process.exit(1);
  }
}
writeFileSync(MANIFEST, `${JSON.stringify(nextManifest, null, 2)}\n`);

// idea.rs：常量名不变，路径改指新布局
const emitted = [];
for (const constant of keptConstants) {
  const light = `icons/idea/${newPathForLegacy(constant.light.replace(/^icons\/idea\//, ""))}`;
  const dark = `icons/idea/${newPathForLegacy(constant.dark.replace(/^icons\/idea\//, ""))}`;
  emitted.push(
    [
      `pub const ${constant.name}: IdeaIcon = IdeaIcon {`,
      `    light: "${light}",`,
      `    dark: "${dark}",`,
      `    has_dark: ${light === dark ? "false" : "true"},`,
      "};",
      "",
    ].join("\n"),
  );
}
const ideaRsOut = [
  "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
  "pub struct IdeaIcon {",
  "    pub light: &'static str,",
  "    pub dark: &'static str,",
  "    pub has_dark: bool,",
  "}",
  "",
  "impl IdeaIcon {",
  "    pub const fn path(&self, dark: bool) -> &'static str {",
  "        if dark { self.dark } else { self.light }",
  "    }",
  "}",
  "",
  `pub const ALL: [IdeaIcon; ${emitted.length}] = [`,
  ...keptConstants.map((constant) => `    ${constant.name},`),
  "];",
  "",
  ...emitted,
].join("\n");
writeFileSync(IDEA_RS, ideaRsOut);

// 来源说明（人读）
const sources = [
  "# idea 图标包来源",
  "",
  "本目录由 `node rust/lithe-gpui/tools/sync-macos-idea-icons.mjs --apply` 生成，**不要手改**。",
  "",
  "## 一、macOS 图标文件夹的副本（主）",
  "",
  "`fileTypes/ expui/ general/ nodes/ vcs/ debugger/ terminal/ toolwindows/ testState/ gutter/ maven/ actions/`",
  `逐字节复制自 \`macos/Resources/IDEAIcons/**\`（${macosSvg.length} 个 SVG + ${macosFiles.length - macosSvg.length} 个附件，含 Apache-2.0 归属文件）。`,
  "macOS 侧仍是那份真源，本目录是 gpui 用的副本 —— 两边各自独立，改图标请改 macOS 那份后重跑本工具。",
  "",
  "## 二、并集补充：macOS 文件夹没有、但界面仍在用的旧图标（次）",
  "",
  "放在 `legacy/` 下，保留旧包的相对路径（去掉 `icons/` 前缀）。它们来自 Windows 前端",
  `\`windows/tauri/src/ui/icons/**\`（本分支仍在），与 macOS 那份同属 intellij-community expui，`,
  "只是这一版更全（macOS 侧缺 `ui/` 那批通用字形与部分文件类型）。",
  `共 ${legacyKeptAsIs.length} 个：`,
  "",
  ...[...legacyKeptAsIs].sort().map((rel) => `- \`${newPathForLegacy(rel)}\``),
  "",
  "## 三、这次删掉的旧图标",
  "",
  `共 ${legacyDrop.length} 个：清单（\`fileExtensions\` / \`folders\` / 默认值）与宿主代码（\`idea::\` 常量）都没有引用。`,
  "",
  "## 四、同时删除的其它图标主题文件夹",
  "",
  `\`assets/icons/\` 下的 ${REMOVED_THEME_DIRS.map((dir) => `\`${dir}/\``).join("、")} 一并删除：`,
  "它们都是从 Windows 前端搬来的副本，现在只保留本目录这一套（来源见上）。",
  "",
].join("\n");
writeFileSync(join(PKG_DIR, "SOURCES.md"), sources);

report();
console.log("\n已写入：macOS 副本 + legacy 补充 + extension.json + idea.rs + SOURCES.md");
