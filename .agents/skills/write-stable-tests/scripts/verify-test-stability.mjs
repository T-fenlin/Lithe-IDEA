#!/usr/bin/env node

// verify-test-stability.mjs —— 测试稳定性静态闸门。
//
// 仓库现在是纯 Rust：`rust/lithe-core`（确定性业务逻辑）与 `gpui/`（GPUI Kit 宿主）。
// Swift / TypeScript 规则与 `--platform macos|windows` 通道随旧前端删除一并移除 ——
// 留着它们只会让闸门去扫不存在的目录，并把"该跑哪个平台的测试"这个已经不存在的问题
// 继续交给读者。
//
// 保留的通用机制（与平台无关）：新增行筛选、字符串/行注释剥离、测试区域掩码、
// `test-stability: allow(<rule>) reason: …` 例外注解、以及报告格式。

import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPT_DIRECTORY = path.dirname(fileURLToPath(import.meta.url));
const REPOSITORY_ROOT = path.resolve(SCRIPT_DIRECTORY, "../../../..");

/**
 * Rust 测试里最容易造成"CI 挂到超时"的写法。
 *
 * 规则集按 gpui 宿主真正拥有的资源来定：它现在持有线程、channel、文件监听、终端 PTY
 * 与 JDTLS 子进程，所以"无界等待"和"无人认领的阻塞点"是这里最现实的风险。
 */
const RUST_RULES = [
  {
    id: "rust-real-sleep",
    pattern: /\b(?:std::)?thread::sleep\s*\(/,
    message: "Use a channel, barrier, or injected clock instead of sleeping to coordinate a test.",
  },
  {
    id: "rust-unbounded-receive",
    pattern: /\.recv\(\s*\)/,
    message: "Use recv_timeout or another bounded receive in test synchronization.",
  },
  {
    id: "rust-unbounded-select",
    pattern: /select!\s*\{/,
    // `select!` 本身不是问题；只有不带 `default` 分支且所有分支都无超时时才会永久阻塞。
    // 交给下面的 `selectWithoutDefaultBranchViolations` 做结构判定，这里不按行匹配。
    lineOnly: true,
    message: "select! without a default branch can block forever; add a timeout branch or a default arm.",
  },
  {
    id: "rust-unbounded-blocking-lock",
    pattern: /\.lock\(\s*\)\s*(?:\.unwrap\(\)|;)/,
    message:
      "A poisoned or never-released mutex can deadlock the suite; prefer try_lock with a deadline in tests.",
  },
];

function normalizePath(filePath) {
  return filePath.split(path.sep).join("/").replace(/^\.\//, "");
}

function isRustTestFile(filePath) {
  return normalizePath(filePath).endsWith(".rs");
}

function stripStringsAndLineComments(line) {
  let result = "";
  let quote = null;
  let escaped = false;
  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    const next = line[index + 1];
    if (quote) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === quote) quote = null;
      result += " ";
      continue;
    }
    if (character === "/" && next === "/") break;
    if (character === '"' || character === "'") {
      quote = character;
      result += " ";
      continue;
    }
    result += character;
  }
  return result;
}

/**
 * 标出「属于测试代码」的行。
 *
 * Rust 没有路径约定（不像 Swift 的 `/Tests/`），所以只能从 `#[cfg(test)]` 模块与
 * `#[test]` / `#[tokio::test]` 之类的属性函数里推：`#[test]` 属性的**下一行**先记为待定，
 * 见到 `fn … {` 才开一个区域，按花括号深度配平。
 */
function rustTestLineMask(lines) {
  const mask = lines.map(() => false);
  let depth = 0;
  let pendingTestModule = false;
  let pendingTestFunction = false;
  const activeDepths = [];

  for (let index = 0; index < lines.length; index += 1) {
    const structural = stripStringsAndLineComments(lines[index]);
    if (/^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]/.test(structural)) {
      pendingTestModule = true;
    }
    if (/^\s*#\s*\[\s*(?:[A-Za-z_][\w:]*::)?test(?:\s*\([^\]]*\))?\s*\]/.test(structural)) {
      pendingTestFunction = true;
    }

    const startsModule = pendingTestModule && /\bmod\s+[A-Za-z_]\w*\s*\{/.test(structural);
    const startsFunction = pendingTestFunction && /\bfn\s+[A-Za-z_]\w*[^;]*\{/.test(structural);
    const startsTestRegion = startsModule || startsFunction;
    if (activeDepths.length > 0 || pendingTestFunction || startsTestRegion) mask[index] = true;

    const opens = (structural.match(/\{/g) ?? []).length;
    const closes = (structural.match(/\}/g) ?? []).length;
    if (startsTestRegion) activeDepths.push(depth + 1);
    depth += opens - closes;
    while (activeDepths.length > 0 && depth < activeDepths[activeDepths.length - 1]) {
      activeDepths.pop();
    }

    if (startsModule) pendingTestModule = false;
    if (startsFunction) pendingTestFunction = false;
  }

  return mask;
}

function exceptionReason(lines, lineIndex, ruleID) {
  const candidates = [lines[lineIndex], lines[lineIndex - 1]].filter(Boolean);
  const escapedRule = ruleID.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`test-stability:\\s*allow\\(${escapedRule}\\)\\s*reason:\\s*(.+)$`);
  for (const candidate of candidates) {
    const match = candidate.match(pattern);
    if (match) return match[1].trim();
  }
  return null;
}

/**
 * `select!` 的每一个分支都是"等这一路就绪"，只要**所有**分支都没有超时也没有
 * `default`，这行就是一个能永久阻塞的同步点。逐个分支收集文本再判定，避免把
 * 同一行里带 `default` 的合法写法误报。
 */
function selectWithoutDefaultBranchViolations(lines, selected) {
  const violations = [];
  const rule = RUST_RULES.find((candidate) => candidate.id === "rust-unbounded-select");

  for (let index = 0; index < lines.length; index += 1) {
    if (!/select!\s*\{/.test(lines[index])) continue;

    let depth = 0;
    let started = false;
    let body = "";
    let end = index;
    for (let cursor = index; cursor < lines.length; cursor += 1) {
      const structural = stripStringsAndLineComments(lines[cursor]);
      for (const character of structural) {
        if (character === "{") {
          depth += 1;
          started = true;
        } else if (character === "}") {
          depth -= 1;
        }
        body += character;
        // ⚠️ 只有**见过左花括号之后**才谈"闭合"：`select! {` 这一行在 `{` 之前都是空格，
        // 那时 depth 还是 0，若无条件判 `depth === 0` 就会在第一个空格处误判为已闭合。
        if (started && depth === 0) {
          end = cursor;
          break;
        }
      }
      if (started && depth === 0) break;
    }
    // `select!` 的分支用 `=>` 分隔；没有 `=>` 说明正则没匹配到真实宏调用，跳过。
    if (!started || !body.includes("=>")) continue;
    if (/\bdefault\b/.test(body)) continue;
    if (/recv_timeout\s*\(|after\s*\(|Duration::from/.test(body)) continue;

    const selectedLines = selected
      ? [...selected].filter((lineNumber) => lineNumber >= index + 1 && lineNumber <= end + 1)
      : [];
    if (selected && selectedLines.length === 0) continue;
    const reportLine = selected ? Math.min(...selectedLines) - 1 : index;

    const reason = exceptionReason(lines, index, rule.id);
    if (reason && reason.length >= 16) continue;
    violations.push({
      file: null,
      line: reportLine + 1,
      rule: rule.id,
      message: reason ? `Exception reason is too short. ${rule.message}` : rule.message,
      source: lines[reportLine].trim(),
    });
  }

  return violations;
}

export function scanFile(filePath, content, selectedLineNumbers = null) {
  const normalized = normalizePath(filePath);
  if (!isRustTestFile(normalized)) return [];

  const lines = content.split(/\r?\n/);
  const mask = rustTestLineMask(lines);
  const selected = selectedLineNumbers ? new Set(selectedLineNumbers) : null;
  const violations = [];

  for (let index = 0; index < lines.length; index += 1) {
    const lineNumber = index + 1;
    if (selected && !selected.has(lineNumber)) continue;
    if (!mask[index]) continue;

    for (const rule of RUST_RULES) {
      if (rule.lineOnly) continue;
      if (!rule.pattern.test(lines[index])) continue;
      const reason = exceptionReason(lines, index, rule.id);
      if (reason && reason.length >= 16) continue;
      violations.push({
        file: normalized,
        line: lineNumber,
        rule: rule.id,
        message: reason ? `Exception reason is too short. ${rule.message}` : rule.message,
        source: lines[index].trim(),
      });
    }
  }

  for (const violation of selectWithoutDefaultBranchViolations(lines, selected)) {
    violations.push({ ...violation, file: normalized });
  }
  return violations;
}

export function parseAddedLines(diff) {
  const files = new Map();
  let currentPath = null;
  let newLine = 0;
  for (const line of diff.split(/\r?\n/)) {
    if (line.startsWith("+++ ")) {
      const value = line.slice(4);
      currentPath = value === "/dev/null" ? null : value.replace(/^b\//, "");
      continue;
    }
    const hunk = line.match(/^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
    if (hunk) {
      newLine = Number(hunk[1]);
      continue;
    }
    if (!currentPath || line.startsWith("\\ No newline")) continue;
    if (line.startsWith("+") && !line.startsWith("+++")) {
      if (!files.has(currentPath)) files.set(currentPath, new Set());
      files.get(currentPath).add(newLine);
      newLine += 1;
    } else if (!line.startsWith("-")) {
      newLine += 1;
    }
  }
  return files;
}

function walk(directory, result = []) {
  if (!statSync(directory).isDirectory()) return result;
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if ([".git", ".build", ".artifacts", "target", "node_modules"].includes(entry.name)) continue;
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) walk(absolute, result);
    else result.push(normalizePath(path.relative(REPOSITORY_ROOT, absolute)));
  }
  return result;
}

function git(...arguments_) {
  return execFileSync("git", ["-c", `safe.directory=${REPOSITORY_ROOT}`, ...arguments_], {
    cwd: REPOSITORY_ROOT,
    encoding: "utf8",
    maxBuffer: 32 * 1024 * 1024,
  });
}

function parseArguments(arguments_) {
  const options = { all: false, base: null, head: "HEAD" };
  for (let index = 0; index < arguments_.length; index += 1) {
    const argument = arguments_[index];
    if (argument === "--all") options.all = true;
    else if (argument === "--base") options.base = arguments_[++index];
    else if (argument === "--head") options.head = arguments_[++index];
    else if (argument === "--help") options.help = true;
    else throw new Error(`Unknown argument: ${argument}`);
  }
  return options;
}

function changedFiles(options) {
  const diffArguments = ["-c", "core.quotePath=false", "diff", "--unified=0", "--no-color"];
  if (options.base) diffArguments.push(options.base, options.head);
  else diffArguments.push("HEAD");
  diffArguments.push("--");
  const changed = parseAddedLines(git(...diffArguments));

  if (!options.base) {
    const untracked = git("ls-files", "--others", "--exclude-standard").split(/\r?\n/).filter(Boolean);
    for (const file of untracked) changed.set(normalizePath(file), null);
  }
  return changed;
}

export function run(options) {
  const candidates = options.all
    ? new Map(walk(REPOSITORY_ROOT).map((file) => [file, null]))
    : changedFiles(options);
  const violations = [];
  for (const [file, selectedLines] of candidates) {
    if (!isRustTestFile(file)) continue;
    const absolute = path.resolve(REPOSITORY_ROOT, file);
    let content;
    try {
      content = readFileSync(absolute, "utf8");
    } catch {
      continue;
    }
    violations.push(...scanFile(file, content, selectedLines));
  }
  return violations;
}

function main() {
  let options;
  try {
    options = parseArguments(process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 2;
    return;
  }
  if (options.help) {
    console.log("Usage: verify-test-stability.mjs [--all] [--base REV --head REV]");
    return;
  }

  const violations = run(options);
  if (violations.length === 0) {
    console.log(`Test stability check passed (${options.all ? "full tree" : "added lines"}, rust).`);
    return;
  }

  console.error(`Test stability check found ${violations.length} blocking issue(s):`);
  for (const violation of violations) {
    console.error(`${violation.file}:${violation.line}: [${violation.rule}] ${violation.message}`);
    console.error(`  ${violation.source}`);
  }
  console.error("Use deterministic synchronization. Exceptions require a bounded implementation and a reasoned test-stability annotation.");
  process.exitCode = 1;
}

if (path.resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) main();
