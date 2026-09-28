#!/usr/bin/env node
/**
 * check-ui-px.mjs —— 守住「Application UI 里不许直接写 px(...)」这条编码规范。
 *
 * 规范真源：
 *   gpui/docs/gpui-kit/0.6.6/zh-CN/docs/coding-guides.md:253 —— 应用布局要用 rem-based helper，
 *   **不是**直接 `px(...)`；
 *   同文件 :288 —— 「Application UI 中**每个**直接 `px(...)` 都应视为 review finding。只有
 *   documented physical/platform boundary、measured runtime geometry、raster/data color 或
 *   **theme/token definition 本身**可以例外。**方便或"匹配截图"都不是有效理由**」。
 *
 * 为什么需要它（写这个脚本的直接原因）：
 *   一个代理在 `gpui/crates/git/src/model.rs` 里用「这些值不在 gpui 的固定 rem 档位上」当理由，
 *   保留了 29 处 `px(...)`（那些 `/// ⚠️ **保留 `px(...)`**：… 不在档位上` 的注释现在还在文件里）。
 *   「不在档位上」**不属于 :288 的任何一类例外** —— 档位外本来就有 `rems(P / 16.)` 这条正路。
 *   这种"自认为合理地留 px"读起来像已论证过，review 时最容易被放过；所以判据必须机器化，
 *   由脚本按 :288 的四类例外来判定，而不是由写代码的人自己声明。
 *
 * 本仓库的正确写法（见 `gpui/crates/workbench/src/command_palette.rs:64`、
 * `gpui/crates/settings/src/row.rs:45`）：
 *   - **档位内**的值（4 的倍数且落在 helper 覆盖的档位上）用固定 helper：
 *     `p_3()` / `h_6()` / `size_4()` / `gap_2()` / `text_sm()` …；
 *   - **档位外**的值写 helper 底层的 `rems(P / 16.)`（**rem base = 16px**，即 1rem = 16px，
 *     **不是** `/4.`），并写注释说明这个数字的来源；
 *   - 确实需要一个 `Pixels` 值时，用 `window.rem_size()` 把 rem 换算成 Pixels，而不是写死 `px(N.)`。
 *
 * 只读扫描，不改任何文件。扫描范围：`gpui/crates/` 下每个 crate 的 `src/` 目录里的所有 `.rs`（含 `app`）。
 *
 * 用法：
 *   node gpui/tools/check-ui-px.mjs            # 扫描并判定：有白名单以外的命中 → 退出码 1
 *   node gpui/tools/check-ui-px.mjs --list     # 额外打印白名单命中、排除统计
 *   node gpui/tools/check-ui-px.mjs --selftest # 用内置样例自证判据会红（不读仓库）
 *
 * 误报控制（三条排除规则缺一不可，否则会有上百处噪音）：
 *   1. 注释与字符串里的 `px(` 不是调用（行注释 `//` / `///` / `//!`、斜杠星号块注释
 *      （含 `/**` 与 `/*!` 变体，Rust 里可嵌套）、以及各类字面量）；
 *   2. `px(` 前面紧邻标识符字符或 `.` 的是形近调用（`rem_px(` / `idea_icon_svg_px(` / `h_px(` /
 *      `w_px(` / `.px(gutter)`）；
 *   3. `#[cfg(test)]` 模块体内的代码不算（按大括号配对定范围，**不是**行号白名单）。
 */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(HERE, "..", "..");
const CRATES_DIR = resolve(REPO_ROOT, "gpui", "crates");

/** 每一条命中的统一修复指引（档位内 helper / 档位外 rems / 真要 Pixels 时换算）。 */
const FIX_GUIDANCE =
  "档位内的值用对应 helper（p_3() / h_6() / size_4() / gap_2() / text_sm()…）；档位外的值用 rems(P / 16.)（rem base = 16px，不是 /4.）并写注释；确实需要 Pixels 时用 window.rem_size() 换算。";

/**
 * 白名单：`coding-guides.md:288` 的四类例外里，本仓库当前**确实**成立的 7 行。
 *
 * 匹配口径是「仓库相对路径（`/` 分隔）+ 行号」；同一行上的多个 `px(...)` 一起放行
 * （例如 `size(px(1024.), px(680.))` 一行两处）。**除这 7 条例外以外一律算违规**，
 * 白名单不接受"方便"或"匹配截图"，也不接受"不在 gpui 固定 rem 档位上" —— 档位外用
 * `rems(P / 16.)`。给白名单加条目必须同时写清引用的例外类别，否则就是给规范开后门。
 *
 * ⚠️ **匹配按 `match`（命中行里的一段稳定代码），不按 `line`**：`line` 只记录上次见到的行号，
 * 供人阅读。按行号匹配的版本活不过一个批次就腐烂了 —— 一次正常重构把 `main.rs` 的
 * `window_min_size` 从 :751 挪到 :753，那处合法例外就被当成违规报了出来。
 * 内容匹配的代价是：真的改动了例外所在的那行代码时，这里要跟着更新 `match`，
 * 而"没命中"会被明确警告（见下面的 `notes`），不会静默放行。
 */
const WHITELIST = [
  {
    file: "gpui/crates/app/src/main.rs",
    match: "const MIN_WINDOW",
    line: 69,
    category: "documented physical/platform boundary（coding-guides.md:288 第 1 类）",
    reason:
      "const MIN_WINDOW: Size<Pixels> = size(px(1024.), px(680.)) —— OS 窗口的最小尺寸，属于窗口平台边界；此刻窗口还不存在，没有 rem 基准可换算。",
  },
  {
    file: "gpui/crates/app/src/main.rs",
    match: "fallback = size(px(1280.)",
    line: 70,
    category: "documented physical/platform boundary（coding-guides.md:288 第 1 类）",
    reason: "let fallback = size(px(1280.), px(800.)) —— 兜底窗口尺寸，同上（WindowOptions/Size<Pixels> 的平台边界）。",
  },
  {
    file: "gpui/crates/app/src/main.rs",
    match: "WindowBounds::Windowed(Bounds::new(point(px(60.)",
    line: 73,
    category: "documented physical/platform boundary（coding-guides.md:288 第 1 类）",
    reason:
      "return WindowBounds::Windowed(Bounds::new(point(px(60.), px(40.)), fallback)) —— 窗口在屏幕上的位置与尺寸，同属 OS 窗口平台边界。",
  },
  {
    file: "gpui/crates/app/src/main.rs",
    match: "window_min_size: Some(size(px(1024.)",
    line: 753,
    category: "documented physical/platform boundary（coding-guides.md:288 第 1 类）",
    reason: "window_min_size: Some(size(px(1024.), px(680.))) —— WindowOptions 的最小窗口尺寸，同上。",
  },
  {
    file: "gpui/crates/settings/src/theme.rs",
    match: "px(theme_font_size_for(ui_font_size))",
    line: 107,
    category: "theme/token definition 本身（coding-guides.md:288 第 4 类）",
    reason: "let font_size = px(theme_font_size_for(ui_font_size)) —— 这里定义的正是 `Theme.font_size` token 自身，是 rem 基准的来源，不能再用 rem 表达。",
  },
  {
    file: "gpui/crates/settings/src/theme.rs",
    match: "px(editor_font_size as f32)",
    line: 128,
    category: "theme/token definition 本身（coding-guides.md:288 第 4 类）",
    reason: "let font_size = px(editor_font_size as f32) —— 同上，定义 `Theme.mono_font_size` token 自身。",
  },
  {
    file: "gpui/crates/settings/src/store.rs",
    match: "let desired = px(theme_font_size_for(self.settings.ui_font_size))",
    line: 542,
    category: "theme/token definition 本身（coding-guides.md:288 第 4 类）",
    reason: "let desired = px(theme_font_size_for(self.settings.ui_font_size)) —— 同一条 token 的定义/不变量断言（守卫测试里复算 Theme.font_size 的预期值）。",
  },
];

/**
 * 白名单自身的体检：`match` 为空/过短/重复都会让"例外"变成后门（过短的片段会顺手放行别的调用），
 * 所以这些情况直接判失败，而不是只警告。
 */
function validateWhitelist() {
  const problems = [];
  const seen = new Set();
  for (const entry of WHITELIST) {
    const text = typeof entry.match === "string" ? entry.match.trim() : "";
    if (text.length < 12) {
      problems.push(
        `白名单条目 ${entry.file} 的 match 为空或过短（${JSON.stringify(entry.match)}）：至少 12 个字符，否则会误放行别的 px() 调用。`,
      );
    }
    const key = `${entry.file}::${text}`;
    if (seen.has(key)) {
      problems.push(`白名单里有重复条目：${key}`);
    }
    seen.add(key);
  }
  return problems;
}

const IDENT_OR_DOT = /[A-Za-z0-9_.]/;
const RAW_STRING_PREFIX = /^(?:b|c)?r(#*)"/;
const CFG_ATTR = /#\s*\[\s*cfg\s*\(([^)]*)\)\s*\]/g;
const TEST_WORD = /(?:^|[^A-Za-z0-9_])test(?:$|[^A-Za-z0-9_])/;

// ---------------------------------------------------------------------------
// Rust 词法：只需要分清「这是代码」还是「这是注释/字面量」，不需要完整 parser。
// ---------------------------------------------------------------------------

function lineStarts(source) {
  const starts = [0];
  for (let i = 0; i < source.length; i += 1) {
    if (source[i] === "\n") starts.push(i + 1);
  }
  return starts;
}

function lineOf(starts, offset) {
  let low = 0;
  let high = starts.length - 1;
  while (low < high) {
    const mid = (low + high + 1) >> 1;
    if (starts[mid] <= offset) low = mid;
    else high = mid - 1;
  }
  return low + 1;
}

function lineText(source, starts, offset) {
  const line = lineOf(starts, offset);
  const start = starts[line - 1];
  let end = source.indexOf("\n", start);
  if (end === -1) end = source.length;
  return source.slice(start, end).replace(/\r$/, "").trim();
}

/** `//`（含 `///` / `//!`）到行尾。返回行尾下标（不含换行符）。 */
function skipLineComment(source, i) {
  const newline = source.indexOf("\n", i + 2);
  return newline === -1 ? source.length : newline;
}

/** 斜杠星号块注释（含 `/**` 与 `/*!` 变体），Rust 的块注释可嵌套。返回闭合后的下标。 */
function skipBlockComment(source, i) {
  let depth = 1;
  let j = i + 2;
  while (j < source.length) {
    if (source.startsWith("/*", j)) {
      depth += 1;
      j += 2;
      continue;
    }
    if (source.startsWith("*/", j)) {
      depth -= 1;
      j += 2;
      if (depth === 0) return j;
      continue;
    }
    j += 1;
  }
  return source.length;
}

/** 普通字符串（`"…"` / `b"…"` / `c"…"`）。`i` 指向开引号，返回闭合引号之后的下标。 */
function skipString(source, i) {
  let j = i + 1;
  while (j < source.length) {
    const char = source[j];
    if (char === "\\") {
      j += 2;
      continue;
    }
    if (char === '"') return j + 1;
    j += 1;
  }
  return source.length;
}

/** 原始字符串 `r"…"` / `r#"…"#` / `br#"…"#`；不是原始字符串则返回 null。 */
function skipRawString(source, i) {
  const match = RAW_STRING_PREFIX.exec(source.slice(i, i + 16));
  if (match === null) return null;
  const terminator = `"${match[1]}`;
  const end = source.indexOf(terminator, i + match[0].length);
  return end === -1 ? source.length : end + terminator.length;
}

/** 字符字面量 `'a'` / `'\n'` / `'\u{1F600}'`；生命周期标注（`'a`）返回 null。 */
function skipCharLiteral(source, i) {
  const next = source[i + 1];
  if (next === undefined) return null;
  if (next === "\\") {
    let j = i + 2;
    while (j < source.length && source[j] !== "'") {
      if (source[j] === "\\") {
        j += 2;
        continue;
      }
      j += 1;
    }
    return j < source.length ? j + 1 : null;
  }
  if (source[i + 2] === "'") return i + 3;
  return null;
}

/** 从 `[` 跳到配对的 `]`（跳过字符串/注释/字符字面量）。返回 `]` 之后的下标。 */
function skipBracketed(source, i) {
  let depth = 0;
  let j = i;
  while (j < source.length) {
    const char = source[j];
    if (char === "[") {
      depth += 1;
      j += 1;
      continue;
    }
    if (char === "]") {
      depth -= 1;
      j += 1;
      if (depth === 0) return j;
      continue;
    }
    const skipped = skipNonCode(source, j);
    if (skipped !== null) {
      j = skipped;
      continue;
    }
    j += 1;
  }
  return source.length;
}

/** 从 `{` 跳到配对的 `}`（跳过字符串/注释/字符字面量）。返回 `}` 的下标。 */
function skipBraced(source, i) {
  let depth = 0;
  let j = i;
  while (j < source.length) {
    const char = source[j];
    if (char === "{") {
      depth += 1;
      j += 1;
      continue;
    }
    if (char === "}") {
      depth -= 1;
      j += 1;
      if (depth === 0) return j - 1;
      continue;
    }
    const skipped = skipNonCode(source, j);
    if (skipped !== null) {
      j = skipped;
      continue;
    }
    j += 1;
  }
  return source.length - 1;
}

/**
 * 若 `i` 处是注释或字面量的开头，返回跳过它之后的下标；否则返回 null。
 * 扫描主循环与花括号/方括号配对共用这一份判据，避免"注释里的括号被当成结构"。
 */
function skipNonCode(source, i) {
  const char = source[i];
  if (char === "/" && source[i + 1] === "/") return skipLineComment(source, i);
  if (char === "/" && source[i + 1] === "*") return skipBlockComment(source, i);
  if (char === '"') return skipString(source, i);
  const raw = skipRawString(source, i);
  if (raw !== null) return raw;
  return skipCharLiteral(source, i);
}

// ---------------------------------------------------------------------------
// 扫描
// ---------------------------------------------------------------------------

/**
 * 找出源码里**真正的** `px(...)` 调用。
 *
 * 返回：
 *   hits        —— 代码位置上的 `px(`（已在词法层排除注释/字面量）；
 *   lookalikes  —— 形近调用（`rem_px(` / `.px(` …）的个数，仅用于统计输出；
 *   codeSpans   —— 代码区段（用于判断 `#[cfg(test)]` 是否真的写在代码里）；
 *   testHits    —— 落在 `#[cfg(test)]` 模块体内的命中（按大括号配对，非行号白名单）。
 */
function scanSource(source) {
  const starts = lineStarts(source);
  const hits = [];
  const codeSpans = [];
  let lookalikes = 0;
  let spanStart = 0;
  let i = 0;

  while (i < source.length) {
    const skipped = skipNonCode(source, i);
    if (skipped !== null) {
      if (i > spanStart) codeSpans.push({ start: spanStart, end: i });
      i = skipped;
      spanStart = i;
      continue;
    }

    if (source[i] === "p" && source[i + 1] === "x" && source[i + 2] === "(") {
      if (IDENT_OR_DOT.test(source[i - 1] ?? "")) {
        lookalikes += 1;
      } else {
        hits.push({
          offset: i,
          line: lineOf(starts, i),
          text: lineText(source, starts, i),
        });
      }
      i += 3;
      continue;
    }

    i += 1;
  }

  if (source.length > spanStart) codeSpans.push({ start: spanStart, end: source.length });

  const testRanges = [];
  for (const match of source.matchAll(CFG_ATTR)) {
    if (!isInCode(codeSpans, match.index)) continue;
    if (!TEST_WORD.test(match[1])) continue;
    const end = testModuleEnd(source, match.index + match[0].length);
    if (end !== null) testRanges.push({ start: match.index, end });
  }

  const testHits = [];
  const production = [];
  for (const hit of hits) {
    const owner = testRanges.find((range) => hit.offset >= range.start && hit.offset <= range.end);
    if (owner === undefined) production.push(hit);
    else testHits.push(hit);
  }

  return { hits, production, testHits, lookalikes, codeSpans };
}

function isInCode(codeSpans, offset) {
  let low = 0;
  let high = codeSpans.length - 1;
  while (low <= high) {
    const mid = (low + high) >> 1;
    const span = codeSpans[mid];
    if (offset < span.start) high = mid - 1;
    else if (offset >= span.end) low = mid + 1;
    else return true;
  }
  return false;
}

/**
 * 从 `#[cfg(test)]` 属性之后找到它所修饰的 `mod` 体，返回模块体结束的 `}` 下标；
 * `mod tests;`（正文在别的文件里）或紧邻的不是 `mod` 时返回 null。
 */
function testModuleEnd(source, from) {
  let j = from;

  for (;;) {
    while (j < source.length && /\s/.test(source[j])) j += 1;
    if (!source.startsWith("#[", j)) break;
    j = skipBracketed(source, j + 1);
  }

  // 可见性：`pub mod` / `pub(crate) mod`
  if (source.startsWith("pub", j) && !/[A-Za-z0-9_]/.test(source[j + 3] ?? "")) {
    j += 3;
    while (j < source.length && /\s/.test(source[j])) j += 1;
    if (source[j] === "(") {
      let depth = 0;
      while (j < source.length) {
        if (source[j] === "(") depth += 1;
        else if (source[j] === ")") {
          depth -= 1;
          if (depth === 0) {
            j += 1;
            break;
          }
        }
        j += 1;
      }
    }
  }

  while (j < source.length && /\s/.test(source[j])) j += 1;
  if (!source.startsWith("mod", j)) return null;
  if (/[A-Za-z0-9_]/.test(source[j + 3] ?? "")) return null;
  j += 3;

  while (j < source.length && /\s/.test(source[j])) j += 1;
  if (!/[A-Za-z_]/.test(source[j] ?? "")) return null;
  while (j < source.length && /[A-Za-z0-9_]/.test(source[j])) j += 1;

  while (j < source.length && /\s/.test(source[j])) j += 1;
  if (source[j] !== "{") return null; // `mod tests;` —— 模块体不在本文件里
  return skipBraced(source, j);
}

/** 收集 `gpui/crates/` 下每个 crate 的 `src/` 里的 `.rs`（不扫 `target/`，也不扫 `crates/` 下的 `tests/`）。 */
function collectSources() {
  const files = [];

  for (const crate of readdirSync(CRATES_DIR, { withFileTypes: true })) {
    if (!crate.isDirectory()) continue;
    const src = join(CRATES_DIR, crate.name, "src");
    if (!isDirectory(src)) continue;
    walk(src, files);
  }

  return files.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
}

function walk(dir, files) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "target") continue;
      walk(path, files);
      continue;
    }
    if (entry.isFile() && entry.name.endsWith(".rs")) files.push(path);
  }
}

function isDirectory(path) {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}

function repoPath(path) {
  return relative(REPO_ROOT, path).split(sep).join("/");
}

// ---------------------------------------------------------------------------
// 判定
// ---------------------------------------------------------------------------

function main() {
  const list = process.argv.includes("--list");
  const selftest = process.argv.includes("--selftest");
  const unknown = process.argv
    .slice(2)
    .filter((arg) => arg !== "--list" && arg !== "--selftest");

  if (unknown.length > 0) {
    console.warn(`警告：不认识的参数：${unknown.join(" ")}（只支持 --list / --selftest）`);
    process.exitCode = 1;
    return;
  }

  if (selftest) {
    runSelftest(list);
    return;
  }

  const whitelistProblems = validateWhitelist();
  if (whitelistProblems.length > 0) {
    for (const problem of whitelistProblems) console.warn(`警告：${problem}`);
    console.warn("白名单自身有问题（match 为空/过短/重复）—— 先修 WHITELIST 再跑。");
    process.exitCode = 1;
    return;
  }

  const sources = collectSources();
  const violations = [];
  const whitelisted = [];
  const whitelistSeen = new Set();
  const notes = [];
  let codeCalls = 0;
  let lookalikes = 0;
  let testCalls = 0;
  let substringCount = 0;

  for (const path of sources) {
    const source = readFileSync(path, "utf8");
    const rel = repoPath(path);
    substringCount += source.match(/px\(/g)?.length ?? 0;

    const scan = scanSource(source);
    codeCalls += scan.hits.length;
    lookalikes += scan.lookalikes;
    testCalls += scan.testHits.length;

    for (const hit of scan.production) {
      const entry = WHITELIST.find((item) => item.file === rel && hit.text.includes(item.match));
      if (entry === undefined) {
        violations.push({ file: rel, line: hit.line, text: hit.text });
        continue;
      }
      whitelistSeen.add(entry);
      whitelisted.push({ file: rel, line: hit.line, text: hit.text, entry });
    }
  }

  for (const entry of WHITELIST) {
    if (!whitelistSeen.has(entry)) {
      notes.push(
        `白名单条目 ${entry.file}（match=${JSON.stringify(entry.match)}，上次在 :${entry.line}）这次没有命中：` +
          `要么那处例外已被改掉（那就从白名单里删掉这条），要么代码变了（那就更新 match）。` +
          `本脚本按 match 内容匹配，行号漂移不会误报。`,
      );
    }
  }

  console.log(
    `扫描 ${sources.length} 个 .rs 文件：\`px(\` 子串 ${substringCount} 处 —— 注释/字符串里 ${
      substringCount - codeCalls - lookalikes
    } 处、形近调用 ${lookalikes} 处、\`#[cfg(test)]\` 模块内 ${testCalls} 处；真正的 UI 代码调用 ${
      whitelisted.length + violations.length
    } 处。`,
  );
  console.log(
    `白名单 ${whitelisted.length} 处（表里 ${WHITELIST.length} 条）、违规 ${violations.length} 处。`,
  );

  if (list) {
    for (const item of whitelisted) {
      console.log(`通过：${item.file}:${item.line}  ${item.text}`);
      console.log(`  理由：${item.entry.reason}`);
      console.log(`  类别：${item.entry.category}`);
    }
    for (const note of notes) console.log(`提示：${note}`);
  } else if (notes.length > 0) {
    for (const note of notes) console.warn(`警告：${note}`);
  }

  for (const item of violations) {
    console.warn(`违规：${item.file}:${item.line}  ${item.text}`);
    console.warn(`  修复：${FIX_GUIDANCE}`);
  }

  if (violations.length > 0) {
    console.warn(
      `违规 ${violations.length} 处：Application UI 里每个直接 px(...) 都是 review finding（coding-guides.md:288）；白名单只认 ${WHITELIST.length} 条，且每条都有引用的例外类别。`,
    );
    process.exitCode = 1;
    return;
  }

  console.log("检查通过：没有白名单以外的 px(...) 调用。");
}

// ---------------------------------------------------------------------------
// --selftest：用内置样例证明判据真的会红（一个"永远为绿"的检查等于没有检查）
// ---------------------------------------------------------------------------

const SELFTEST_VIOLATION_LINE = 3;

function selftestSources() {
  const violationLine = `.w(px(${SELFTEST_VIOLATION_LINE * 41}.))`;

  const lines = [
    "use gpui::*;",
    "fn render() -> impl IntoElement {",
    `    div()${violationLine}`, // 第 3 行：唯一的真违规
    "        // px(1.) —— 行注释，排除",
    "        /// px(2.) —— doc 注释，排除",
    "        //! px(3.) —— 内层 doc 注释，排除",
    "        /* px(4.) —— 块注释，排除 */",
    "        /* 外层 /* px(5.) 嵌套 */ 还是注释 */",
    `        .child(format!("px(6.)"))`, // 字符串字面量，排除
    `        .child(r#"px(7.)"#)`, // 原始字符串，排除
    "        .child(rem_px(8.))", // 形近调用，排除
    "        .child(idea_icon_svg_px(9.))", // 形近调用，排除
    "        .child(h_px(10.))", // 形近调用，排除
    "        .child(w_px(11.))", // 形近调用，排除
    "        .child(widget.px(gutter))", // 形近调用（`.px(`），排除
    "}",
    "#[cfg(test)]",
    "mod tests {",
    "    #[test]",
    "    fn it_renders() {",
    "        let _ = div().w(px(12.));", // cfg(test) 内，按大括号配对排除
    "    }",
    "}",
    "",
  ];

  // 白名单样例：把命中对齐到 WHITELIST 里真实存在的「路径 + 行号」上。
  // 这一行同时是「字符串里的 `//` 不是注释」的判据：`"a//b"` 之后的两个 `px()` 必须照样被找到
  // （找晚了整行都会被当成注释吞掉，白名单命中数就会从 2 掉到 0）。
  const whitelistEntry = WHITELIST[0];
  const whitelistLines = [
    // 白名单按 `match` 内容匹配，所以样例只要让那一行**包含** `match` 即可，不需要填到某个行号上。
    `fn window_options() { let _ = format!("a//b"); ${whitelistEntry.match} = size(px(1024.), px(680.)); }`,
  ];

  return [
    { file: "gpui/crates/selftest/src/sample.rs", content: `${lines.join("\n")}\n` },
    { file: whitelistEntry.file, content: `${whitelistLines.join("\n")}\n` },
  ];
}

function runSelftest(list) {
  const problems = [];
  const findings = [];

  for (const sample of selftestSources()) {
    const scan = scanSource(sample.content);
    for (const hit of scan.production) {
      const entry = WHITELIST.find(
        (item) => item.file === sample.file && hit.text.includes(item.match),
      );
      findings.push({ file: sample.file, line: hit.line, text: hit.text, whitelisted: entry !== undefined });
    }
    if (sample.file === "gpui/crates/selftest/src/sample.rs") {
      if (scan.testHits.length !== 1) {
        problems.push(`样例应含 1 处 cfg(test) 内命中，实得 ${scan.testHits.length} 处`);
      }
      if (scan.lookalikes !== 5) {
        problems.push(`样例应含 5 处形近调用，实得 ${scan.lookalikes} 处`);
      }
    }
  }

  const violations = findings.filter((item) => !item.whitelisted);
  if (violations.length !== 1) {
    problems.push(`样例应报 1 条违规，实得 ${violations.length} 条：${JSON.stringify(violations)}`);
  } else if (
    violations[0].file !== "gpui/crates/selftest/src/sample.rs" ||
    violations[0].line !== SELFTEST_VIOLATION_LINE
  ) {
    problems.push(`样例报出的违规位置不对：${violations[0].file}:${violations[0].line}`);
  }

  const whitelisted = findings.filter((item) => item.whitelisted);
  if (whitelisted.length !== 2) {
    problems.push(`样例应放行 2 处白名单命中（同一行两个 px()），实得 ${whitelisted.length} 处`);
  }

  for (const item of findings) {
    console.log(`${item.whitelisted ? "通过" : "违规"}：${item.file}:${item.line}  ${item.text}`);
  }
  if (list) {
    for (const problem of problems) console.warn(`警告：${problem}`);
  }

  if (problems.length > 0) {
    for (const problem of problems) console.warn(`警告：${problem}`);
    console.warn("自检失败：判据没有按预期报红/放行。");
    process.exitCode = 1;
    return;
  }

  console.log("自检通过：样例里只报出那 1 条真违规，注释 / 字符串 / 形近调用 / cfg(test) / 白名单都没报。");
}

main();
