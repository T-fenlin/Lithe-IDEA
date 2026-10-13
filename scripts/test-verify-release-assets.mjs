import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

import {
  collectActual,
  collectExpected,
  compareAssets,
  formatReport,
  parseArguments,
} from "./verify-release-assets.mjs";

test("parseArguments accepts repeated --expected and --expected-file", () => {
  const options = parseArguments([
    "--tag",
    "preview-gpui-0.0.1",
    "--repo",
    "1lck/Lithe-IDEA",
    "--expected",
    "a.zip",
    "--expected",
    "b.zip",
    "--expected-file",
    "list.txt",
  ]);
  assert.equal(options.tag, "preview-gpui-0.0.1");
  assert.equal(options.repo, "1lck/Lithe-IDEA");
  assert.deepEqual(options.expected, ["a.zip", "b.zip"]);
  assert.equal(options.expectedFile, "list.txt");
});

test("parseArguments requires --tag", () => {
  assert.throws(() => parseArguments(["--expected", "a.zip"]), /--tag is required/);
});

test("collectExpected reads names from a file and deduplicates", () => {
  const directory = mkdtempSync(path.join(tmpdir(), "lithe-release-assets-"));
  const listPath = path.join(directory, "list.txt");
  writeFileSync(listPath, "a.zip\n\nb.zip\na.zip\n", "utf8");
  const expected = collectExpected({ expected: ["c.zip"], expectedFile: listPath });
  assert.deepEqual([...expected].sort(), ["a.zip", "b.zip", "c.zip"]);
});

test("collectActual trims names and drops blank lines", () => {
  const actual = collectActual("a.zip\r\n\n b.zip \n");
  assert.deepEqual([...actual].sort(), ["a.zip", "b.zip"]);
});

test("compareAssets reports missing and unexpected names", () => {
  const result = compareAssets(new Set(["a.zip", "b.zip"]), new Set(["b.zip", "c.zip"]));
  assert.deepEqual(result.missing, ["a.zip"]);
  assert.deepEqual(result.unexpected, ["c.zip"]);
});

test("compareAssets is empty for an exact match", () => {
  const result = compareAssets(new Set(["a.zip"]), new Set(["a.zip"]));
  assert.deepEqual(result.missing, []);
  assert.deepEqual(result.unexpected, []);
});

test("formatReport names missing and unexpected assets", () => {
  const report = formatReport({
    tag: "preview-gpui-0.0.1",
    repo: "1lck/Lithe-IDEA",
    expected: new Set(["a.zip", "b.zip"]),
    actual: new Set(["b.zip", "c.zip"]),
  });
  assert.match(report, /Release asset check for 1lck\/Lithe-IDEA@preview-gpui-0\.0\.1/);
  assert.match(report, /Missing assets:\n {2}- a\.zip/);
  assert.match(report, /Unexpected assets:\n {2}\+ c\.zip/);
});

test("formatReport confirms an exact match", () => {
  const report = formatReport({
    tag: "preview-gpui-0.0.1",
    repo: null,
    expected: new Set(["a.zip"]),
    actual: new Set(["a.zip"]),
  });
  assert.match(report, /All expected assets are published and no extra assets remain\./);
});
