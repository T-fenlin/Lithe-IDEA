#!/usr/bin/env node
/**
 * extract-locale.mjs —— 把 Windows 前端的界面文案真源转换成 GPUI Kit 的 rust-i18n 资源。
 *
 * 真源：
 *   windows/tauri/src/i18n/locale.ts   （平面 key 的 `catalogs` 对象，内含 "en-US" 与 "zh-CN"）
 *   windows/tauri/src/i18n/ai-commit.ts（locale.ts 通过 `...aiCommitEnglish` / `...aiCommitChinese` 展开）
 *
 * 产物：
 *   gpui/shell/locales/lithe.en.yml
 *   gpui/shell/locales/lithe.zh-CN.yml
 *
 * 用法：
 *   node gpui/tools/extract-locale.mjs          # 重新生成两个 YAML
 *   node gpui/tools/extract-locale.mjs --check  # 只校验产物是否与真源一致（不写文件）
 *
 * 设计取舍：
 *   1. 不做正则解析，而是把三个对象字面量整段抽出来求值（`new Function`）。
 *      这样字符串转义、单双引号、跨行 prettier 折行都由 JS 引擎处理，不会解析错。
 *      如果将来有人在 catalog 里写函数调用/模板拼接，求值结果会出现非字符串值，
 *      本脚本会跳过该 key、打印清单并以退出码 1 结束（宁可缺也不写错）。
 *   2. key 顺序保持真源顺序，便于逐行 review diff。
 *   3. 文件使用 UTF-8、LF、无 BOM。
 */

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..", "..");
const LOCALE_TS = resolve(REPO_ROOT, "windows/tauri/src/i18n/locale.ts");
const AI_COMMIT_TS = resolve(REPO_ROOT, "windows/tauri/src/i18n/ai-commit.ts");
const OUT_DIR = resolve(REPO_ROOT, "gpui/shell/locales");

/** rust-i18n 的 locale 名（YAML 里的语言 key）→ 真源 catalog 的键。 */
const CATALOG_LOCALES = [
  { yamlLocale: "zh-CN", catalogKey: "zh-CN", file: "lithe.zh-CN.yml" },
  { yamlLocale: "en", catalogKey: "en-US", file: "lithe.en.yml" },
];

/** 应用自己的顶层 namespace。页面文案一律挂在这里，不要塞进 gpui_component。 */
const APP_NAMESPACE = "lithe";

/** 官方文档与 gpui-component 内置 locales/ui.yml 使用的 locale 文件版本。 */
const FILE_VERSION = 2;

/**
 * 覆盖 gpui-kit（gpui-component）组件自身文案的 key。
 * 只有「Lithe 文案与组件内置文案同义、且中英文都逐字一致」的条目才允许放进来；
 * 值仍然取自真源，避免手写翻译漂移。
 * 见 gpui/shell/locales/README.md「gpui_component 覆盖段」。
 */
const GPUI_COMPONENT_OVERRIDES = [
  { target: "Dialog.ok", source: "ui.ok" },
  { target: "Dialog.cancel", source: "ui.cancel" },
];

const TS_QUOTES = new Set(['"', "'", "`"]);

/** 从源码里定位 `anchor` 之后的第一个 `{`，返回与之匹配的对象字面量文本（含花括号）。 */
function extractObjectLiteral(source, anchor) {
  const anchorAt = source.indexOf(anchor);
  if (anchorAt === -1) {
    throw new Error(`在源码中找不到锚点：${anchor}`);
  }

  const start = source.indexOf("{", anchorAt + anchor.length);
  if (start === -1) {
    throw new Error(`锚点 ${anchor} 之后找不到对象字面量起始的 “{”`);
  }

  let depth = 0;
  for (let i = start; i < source.length; i += 1) {
    const char = source[i];

    if (TS_QUOTES.has(char)) {
      i = skipStringLiteral(source, i);
      continue;
    }
    if (char === "/" && source[i + 1] === "/") {
      const newline = source.indexOf("\n", i + 2);
      if (newline === -1) break;
      i = newline;
      continue;
    }
    if (char === "/" && source[i + 1] === "*") {
      const end = source.indexOf("*/", i + 2);
      if (end === -1) throw new Error("对象字面量里有未闭合的块注释");
      i = end + 1;
      continue;
    }
    if (char === "{") {
      depth += 1;
      continue;
    }
    if (char === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(start, i + 1);
    }
  }

  throw new Error(`锚点 ${anchor} 的对象字面量没有闭合`);
}

/** 返回字符串字面量结束引号的下标（跳过转义）。 */
function skipStringLiteral(source, start) {
  const quote = source[start];
  for (let i = start + 1; i < source.length; i += 1) {
    const char = source[i];
    if (char === "\\") {
      i += 1;
      continue;
    }
    if (char === quote) return i;
  }
  throw new Error(`从下标 ${start} 开始的字符串字面量没有闭合`);
}

function loadCatalogs() {
  const aiCommitSource = readFileSync(AI_COMMIT_TS, "utf8");
  const localeSource = readFileSync(LOCALE_TS, "utf8");

  const englishBlock = extractObjectLiteral(aiCommitSource, "export const aiCommitEnglish");
  const chineseBlock = extractObjectLiteral(aiCommitSource, "export const aiCommitChinese");
  const catalogsBlock = extractObjectLiteral(localeSource, "const catalogs =");

  // 只保留对象字面量本体：TS 的类型标注（`Record<...>` / `as const`）自然被排除。
  const evaluate = new Function(
    [
      `const aiCommitEnglish = ${englishBlock};`,
      `const aiCommitChinese = ${chineseBlock};`,
      `const catalogs = ${catalogsBlock};`,
      "return catalogs;",
    ].join("\n"),
  );

  return evaluate();
}

const PLAIN_KEY = /^[A-Za-z_][A-Za-z0-9_.-]*$/;
const YAML_RESERVED = /^(?:true|false|null|yes|no|on|off|~)$/i;

function yamlKey(key) {
  return PLAIN_KEY.test(key) && !YAML_RESERVED.test(key) ? key : JSON.stringify(key);
}

/** JSON 字符串转义是 YAML 双引号标量的子集，直接复用。 */
function yamlValue(value) {
  return JSON.stringify(value);
}

function renderFile({ yamlLocale, entries, overrides, sourceLabel }) {
  const lines = [
    "# 本文件由 gpui/tools/extract-locale.mjs 自动生成，请勿手工编辑。",
    `# 真源：${sourceLabel}`,
    "# 重新生成：node gpui/tools/extract-locale.mjs",
    "#",
    `# rust-i18n 4.2 约定：_version: ${FILE_VERSION}（key 在前、locale 在后）。`,
    `# ${APP_NAMESPACE} namespace = Lithe 页面文案；gpui_component namespace = 覆盖 gpui-kit 组件内置文案。`,
    "# 占位符沿用 Windows 前端的 {name} 语法，见本目录 README.md。",
    `_version: ${FILE_VERSION}`,
    "",
    `${APP_NAMESPACE}:`,
  ];

  for (const entry of entries) {
    lines.push(`  ${yamlKey(entry.key)}:`);
    lines.push(`    ${yamlLocale}: ${yamlValue(entry.value)}`);
  }

  if (overrides.length > 0) {
    lines.push("");
    lines.push("gpui_component:");
    for (const override of overrides) {
      lines.push(`  ${yamlKey(override.target)}:`);
      lines.push(`    ${yamlLocale}: ${yamlValue(override.value)}`);
    }
  }

  return `${lines.join("\n")}\n`;
}

function findPlaceholders(text) {
  const names = new Set();
  for (const match of text.matchAll(/\{([A-Za-z0-9_]+)\}/g)) {
    names.add(match[1]);
  }
  return [...names].sort();
}

function main() {
  const checkOnly = process.argv.includes("--check");
  const catalogs = loadCatalogs();

  const problems = [];
  const notices = [];
  const skipped = [];
  const outputs = [];

  for (const locale of CATALOG_LOCALES) {
    const catalog = catalogs[locale.catalogKey];
    if (catalog === undefined || typeof catalog !== "object" || catalog === null) {
      throw new Error(`真源里找不到 locale：${locale.catalogKey}`);
    }

    const entries = [];
    for (const [key, value] of Object.entries(catalog)) {
      if (typeof value !== "string") {
        skipped.push(`${locale.catalogKey} :: ${key} (${typeof value})`);
        continue;
      }
      entries.push({ key, value });
    }

    const overrides = GPUI_COMPONENT_OVERRIDES.map(({ target, source }) => {
      const value = catalog[source];
      if (typeof value !== "string") {
        throw new Error(`gpui_component 覆盖 ${target} 引用的真源 key 不存在：${source}`);
      }
      return { target, value };
    });

    outputs.push({ locale, entries, overrides });
  }

  const [chinese, english] = outputs;
  const chineseKeys = new Set(chinese.entries.map((entry) => entry.key));
  const englishKeys = new Set(english.entries.map((entry) => entry.key));

  for (const key of chineseKeys) {
    if (!englishKeys.has(key)) problems.push(`zh-CN 有而 en 缺失：${key}`);
  }
  for (const key of englishKeys) {
    if (!chineseKeys.has(key)) problems.push(`en 有而 zh-CN 缺失：${key}`);
  }

  // 占位符差异只提示、不判失败：真源里英文有 {plural} 这类由调用点提供的复数占位符，
  // 中文用不着，属于预期内差异。见 README「占位符」一节。
  const englishByKey = new Map(english.entries.map((entry) => [entry.key, entry.value]));
  for (const entry of chinese.entries) {
    const englishValue = englishByKey.get(entry.key);
    if (englishValue === undefined) continue;
    const chinesePlaceholders = findPlaceholders(entry.value).join(",");
    const englishPlaceholders = findPlaceholders(englishValue).join(",");
    if (chinesePlaceholders !== englishPlaceholders) {
      notices.push(
        `占位符不一致：${entry.key} (zh-CN: [${chinesePlaceholders}] / en: [${englishPlaceholders}])`,
      );
    }
  }

  mkdirSync(OUT_DIR, { recursive: true });

  let changed = false;
  for (const { locale, entries, overrides } of outputs) {
    const content = renderFile({
      yamlLocale: locale.yamlLocale,
      entries,
      overrides,
      sourceLabel: "windows/tauri/src/i18n/locale.ts（含 ai-commit.ts 展开）",
    });
    const target = resolve(OUT_DIR, locale.file);
    const previous = tryRead(target);

    if (previous === content) continue;
    changed = true;
    if (checkOnly) {
      problems.push(`产物已过期：${relative(REPO_ROOT, target)}`);
      continue;
    }
    writeFileSync(target, content, "utf8");
    console.log(`${previous === null ? "已写入" : "已更新"} ${relative(REPO_ROOT, target)}`);
  }

  for (const { locale, entries, overrides } of outputs) {
    console.log(
      `${locale.file}: ${entries.length} 条 ${APP_NAMESPACE} key + ${overrides.length} 条 gpui_component 覆盖`,
    );
  }

  if (skipped.length > 0) {
    console.warn(`跳过 ${skipped.length} 条无法机械提取的 key：`);
    for (const item of skipped) console.warn(`  - ${item}`);
  }

  for (const notice of notices) console.log(`提示：${notice}`);
  for (const problem of problems) console.warn(`警告：${problem}`);

  if (checkOnly && !changed) {
    console.log("--check：产物与真源一致。");
  }

  if (problems.length > 0 || skipped.length > 0) {
    process.exitCode = 1;
  }
}

function tryRead(path) {
  try {
    return readFileSync(path, "utf8");
  } catch (error) {
    if (error.code === "ENOENT") return null;
    throw error;
  }
}

main();
