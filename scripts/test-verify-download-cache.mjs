#!/usr/bin/env node

import assert from "node:assert/strict";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..");
const verifier = path.join(scriptDirectory, "verify-download-cache.mjs");
const emptySha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const testRoot = await fs.mkdtemp(path.join(os.tmpdir(), "lithe-cache-verifier-"));
const testEnvironment = { ...process.env, GITHUB_ACTIONS: "false" };

function verify(argumentsList, environment = testEnvironment) {
  return spawnSync(process.execPath, [verifier, ...argumentsList], { encoding: "utf8", env: environment });
}

function diagnostics(result) {
  return [result.stdout, result.stderr].filter(Boolean).join("\n");
}

function assertSucceeded(result) {
  assert.equal(result.status, 0, diagnostics(result));
}

function run(command, argumentsList, workingDirectory = testRoot) {
  const result = spawnSync(command, argumentsList, { cwd: workingDirectory, encoding: "utf8" });
  assertSucceeded(result);
  return result.stdout.trim();
}

// 这里原本还有三组断言，分别钉住 Windows 的 Bun 缓存工作流、Windows 的 JDTLS/JDK 构建脚本、
// 以及 macOS 的 JDK 缓存 action 与打包脚本。旧前端删除后它们比对的文件（`ci-windows.yml`、
// `release-*-{macos,windows}.yml`、`build-windows.ps1`、`package-app.sh`、`preview.sh`、
// `verify-macos-package.sh`、`.github/actions/prepare-macos-dependency-cache/`）已不复存在，
// 所以整组移除。剩下的用例直接驱动 `verify-download-cache.mjs` 的行为，覆盖 Cargo、JDTLS、
// JDK 与 Bun 四条下载面 —— 那才是校验器本身的职责；工作流里怎么配缓存属于 CI 配置的职责。

try {
  const cargoCache = path.join(testRoot, "cargo-cache", "registry");
  const cargoLock = path.join(testRoot, "Cargo.lock");
  const crate = path.join(cargoCache, "fixture-1.0.0.crate");
  await fs.mkdir(cargoCache, { recursive: true });
  await fs.writeFile(
    cargoLock,
    `version = 4\n\n[[package]]\nname = "fixture"\nversion = "1.0.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "${emptySha256}"\n`,
  );
  await fs.writeFile(crate, "");

  let result = verify(["--cargo-cache", cargoCache, "--cargo-lock", cargoLock]);
  assertSucceeded(result);
  assert.equal(await fs.readFile(crate, "utf8"), "");

  await fs.writeFile(crate, "corrupted");
  result = verify(["--cargo-cache", cargoCache, "--cargo-lock", cargoLock]);
  assertSucceeded(result);
  await assert.rejects(fs.access(crate));
  assert.match(diagnostics(result), /SHA-256 mismatch/);

  await fs.writeFile(crate, "corrupted in Actions mode");
  result = verify(
    ["--cargo-cache", cargoCache, "--cargo-lock", cargoLock],
    { ...testEnvironment, GITHUB_ACTIONS: "true" },
  );
  assertSucceeded(result);
  await assert.rejects(fs.access(crate));
  assert.equal(result.stderr, "");
  assert.match(result.stdout, /::warning title=Cargo cache entry rejected::.*SHA-256 mismatch/);

  const jdtlsCache = path.join(testRoot, "jdtls-cache");
  const jdtlsManifest = path.join(testRoot, "manifest.json");
  const jdtlsArchive = path.join(jdtlsCache, `jdtls-1.0.0-${emptySha256}.tar.gz`);
  const unexpected = path.join(jdtlsCache, "unexpected.download");
  await fs.mkdir(jdtlsCache, { recursive: true });
  await fs.writeFile(
    jdtlsManifest,
    JSON.stringify({
      version: "1.0.0",
      archiveSHA256: emptySha256,
      licenseSHA256: emptySha256,
      lombokVersion: "1.0.0",
      lombokSHA256: emptySha256,
      lombokLicenseSHA256: emptySha256,
    }),
  );
  await fs.writeFile(jdtlsArchive, "");
  await fs.writeFile(unexpected, "unexpected");

  result = verify([
    "--cargo-cache",
    cargoCache,
    "--cargo-lock",
    cargoLock,
    "--jdtls-cache",
    jdtlsCache,
    "--jdtls-manifest",
    jdtlsManifest,
  ]);
  assertSucceeded(result);
  assert.equal(await fs.readFile(jdtlsArchive, "utf8"), "");
  await assert.rejects(fs.access(unexpected));
  assert.match(diagnostics(result), /not referenced by the JDTLS manifest/);

  const jdkCache = path.join(testRoot, "jdk-cache");
  const jdkManifest = path.join(testRoot, "jdk-manifest.json");
  const jdkArchive = path.join(jdkCache, `jdk-21.0.0-windows-x86_64-${emptySha256}.zip`);
  const macJdkArchive = path.join(jdkCache, `jdk-21.0.0-macos-aarch64-${emptySha256}.tar.gz`);
  const unexpectedJdk = path.join(jdkCache, "unexpected.download");
  await fs.mkdir(jdkCache, { recursive: true });
  await fs.writeFile(
    jdkManifest,
    JSON.stringify({
      version: "21.0.0",
      platforms: {
        "windows-x86_64": { sha256: emptySha256 },
        "macos-aarch64": { sha256: emptySha256 },
      },
    }),
  );
  await fs.writeFile(jdkArchive, "");
  await fs.writeFile(macJdkArchive, "");
  await fs.writeFile(unexpectedJdk, "unexpected");

  result = verify([
    "--cargo-cache",
    cargoCache,
    "--cargo-lock",
    cargoLock,
    "--jdk-cache",
    jdkCache,
    "--jdk-manifest",
    jdkManifest,
  ]);
  assertSucceeded(result);
  assert.equal(await fs.readFile(jdkArchive, "utf8"), "");
  assert.equal(await fs.readFile(macJdkArchive, "utf8"), "");
  await assert.rejects(fs.access(unexpectedJdk));
  assert.match(diagnostics(result), /not referenced by the JDK manifest/);

  await fs.writeFile(jdkArchive, "corrupted");
  result = verify([
    "--cargo-cache",
    cargoCache,
    "--cargo-lock",
    cargoLock,
    "--jdk-cache",
    jdkCache,
    "--jdk-manifest",
    jdkManifest,
  ]);
  assertSucceeded(result);
  await assert.rejects(fs.access(jdkArchive));
  assert.match(diagnostics(result), /JDK cache entry rejected.*SHA-256 mismatch/s);

  const fakeBin = path.join(testRoot, "bin");
  const fakeBun = path.join(fakeBin, "bun");
  const bunCache = path.join(testRoot, "bun-cache");
  const bunLock = path.join(testRoot, "bun.lock");
  const cachedPackage = path.join(bunCache, "fixture@1.0.0", "index.js");
  await fs.mkdir(fakeBin, { recursive: true });
  // 假 `bun` 必须按平台落成不同的形态：POSIX 上 `#!/bin/sh` 脚本可直接执行；Windows 上
  // `spawnSync("bun")` 走 `CreateProcess`，只认可执行扩展名，没有 shebang 的裸名脚本会
  // 直接 `ENOENT`（而 `.sh` 需要 Git Bash 才能跑）。所以这里写 `bun.cmd`。
  if (process.platform === "win32") {
    await fs.writeFile(path.join(fakeBin, "bun.cmd"), "@echo off\r\necho 1.3.14\r\n");
  } else {
    await fs.writeFile(fakeBun, "#!/bin/sh\nprintf '1.3.14\\n'\n", { mode: 0o700 });
  }
  await fs.mkdir(path.dirname(cachedPackage), { recursive: true });
  await fs.writeFile(bunLock, "fixture-lock\n");
  await fs.writeFile(cachedPackage, "export default 1;\n");
  const bunEnvironment = { ...testEnvironment, PATH: `${fakeBin}${path.delimiter}${process.env.PATH}` };
  const bunArguments = [
    "--cargo-cache",
    cargoCache,
    "--cargo-lock",
    cargoLock,
    "--bun-version",
    "1.3.14",
    "--bun-lock",
    bunLock,
    "--bun-cache",
    bunCache,
  ];

  result = verify([...bunArguments, "--write-bun-manifest"], bunEnvironment);
  assertSucceeded(result);
  await fs.access(path.join(bunCache, ".lithe-integrity.json"));
  result = verify(bunArguments, bunEnvironment);
  assertSucceeded(result);
  assert.match(result.stdout, /Bun download cache verified: 1 file/);

  await fs.writeFile(cachedPackage, "tampered\n");
  result = verify(bunArguments, bunEnvironment);
  assertSucceeded(result);
  assert.match(diagnostics(result), /SHA-256 mismatch/);
  await assert.rejects(fs.access(cachedPackage));

  // SwiftPM 缓存校验已随 macOS 旧前端（`Package.swift` / SwiftPM 依赖图）删除而移除，
  // 剩下的下载面是 Cargo、JDTLS、JDK 与 Bun —— 上面的用例已逐一覆盖。

  process.stdout.write("Download cache verifier tests passed.\n");
} finally {
  await fs.rm(testRoot, { force: true, recursive: true });
}
