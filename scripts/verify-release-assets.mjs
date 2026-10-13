#!/usr/bin/env node

// Verifies that a GitHub Release carries exactly the expected asset names.
//
// The expected list is the release contract: the packaging workflow must
// publish every name, and the published release must not carry anything else.
// A rolling preview release additionally prunes stale assets before this runs.
//
// Usage:
//   gh release view "$TAG" --repo "$REPO" --json assets --jq '.assets[].name' \
//     | node scripts/verify-release-assets.mjs --tag "$TAG" --repo "$REPO" \
//         --expected Lithe-GPUI-0.0.1-linux-x64.tar.gz --expected ...
//
//   node scripts/verify-release-assets.mjs --tag "$TAG" --expected-file list.txt < names.txt
//
// Reads the published asset names from stdin, one per line. Exits 1 when an
// expected asset is missing or an unexpected asset is present.

import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export function parseArguments(argv) {
  const options = {
    tag: null,
    repo: null,
    expected: [],
    expectedFile: null,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    const value = argv[index + 1];
    if (argument === "--tag") {
      if (!value) throw new Error("--tag requires a value");
      options.tag = value;
      index += 1;
    } else if (argument === "--repo") {
      if (!value) throw new Error("--repo requires a value");
      options.repo = value;
      index += 1;
    } else if (argument === "--expected") {
      if (!value) throw new Error("--expected requires a value");
      options.expected.push(value);
      index += 1;
    } else if (argument === "--expected-file") {
      if (!value) throw new Error("--expected-file requires a value");
      options.expectedFile = value;
      index += 1;
    } else {
      throw new Error(`Unknown argument: ${argument}`);
    }
  }
  if (!options.tag) throw new Error("--tag is required.");
  return options;
}

export function collectExpected(options) {
  const expected = new Set(options.expected);
  if (options.expectedFile) {
    const contents = readFileSync(options.expectedFile, "utf8");
    for (const line of contents.split("\n")) {
      const name = line.trim();
      if (name) expected.add(name);
    }
  }
  return expected;
}

export function collectActual(stdin) {
  const actual = new Set();
  for (const line of stdin.split("\n")) {
    const name = line.trim();
    if (name) actual.add(name);
  }
  return actual;
}

// Returns { missing, unexpected } as sorted arrays; both empty means the
// published release matches the contract exactly.
export function compareAssets(expected, actual) {
  const missing = [...expected].filter((name) => !actual.has(name)).sort();
  const unexpected = [...actual].filter((name) => !expected.has(name)).sort();
  return { missing, unexpected };
}

export function formatReport({ tag, repo, expected, actual }) {
  const { missing, unexpected } = compareAssets(expected, actual);
  const lines = [`Release asset check for ${repo ? `${repo}@` : ""}${tag}`];
  lines.push(`Expected ${expected.size} asset(s), found ${actual.size}.`);
  if (missing.length > 0) {
    lines.push("Missing assets:");
    for (const name of missing) lines.push(`  - ${name}`);
  }
  if (unexpected.length > 0) {
    lines.push("Unexpected assets:");
    for (const name of unexpected) lines.push(`  + ${name}`);
  }
  if (missing.length === 0 && unexpected.length === 0) {
    lines.push("All expected assets are published and no extra assets remain.");
  }
  return lines.join("\n");
}

async function readStdin() {
  if (process.stdin.isTTY) return "";
  const chunks = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk);
  }
  return Buffer.concat(chunks).toString("utf8");
}

async function main(argv) {
  const options = parseArguments(argv);
  const expected = collectExpected(options);
  if (expected.size === 0) {
    throw new Error("Provide at least one --expected asset name or --expected-file.");
  }
  const actual = collectActual(await readStdin());
  const report = formatReport({ tag: options.tag, repo: options.repo, expected, actual });
  process.stdout.write(`${report}\n`);
  const { missing, unexpected } = compareAssets(expected, actual);
  if (missing.length > 0 || unexpected.length > 0) {
    process.exitCode = 1;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2)).catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
