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
  const javaDebugArchive = path.join(jdtlsCache, `vscode-java-debug-1.0.0-${emptySha256}.vsix`);
  const windowsJavaDebugArchive = path.join(jdtlsCache, `vscode-java-debug-1.0.0-${emptySha256}.zip`);
  const javaDebugLicense = path.join(jdtlsCache, `java-debug-EPL-1.0-1.0.0-${emptySha256}.txt`);
  const javaTestArchive = path.join(jdtlsCache, `vscode-java-test-1.0.0-${emptySha256}.vsix`);
  const javaTestLicense = path.join(jdtlsCache, `java-test-MIT-1.0.0-${emptySha256}.txt`);
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
      javaDebugExtensionVersion: "1.0.0",
      javaDebugServerVersion: "1.0.0",
      javaDebugArchiveSHA256: emptySha256,
      javaDebugLicenseSHA256: emptySha256,
      javaTestExtensionVersion: "1.0.0",
      javaTestArchiveSHA256: emptySha256,
      javaTestLicenseSHA256: emptySha256,
    }),
  );
  const jdtlsArtifacts = [
    jdtlsArchive,
    javaDebugArchive,
    windowsJavaDebugArchive,
    javaDebugLicense,
    javaTestArchive,
    javaTestLicense,
  ];
  for (const artifact of jdtlsArtifacts) {
    await fs.writeFile(artifact, "");
  }
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
  for (const artifact of jdtlsArtifacts) {
    assert.equal(await fs.readFile(artifact, "utf8"), "");
  }
  await assert.rejects(fs.access(unexpected));
  assert.match(diagnostics(result), /not referenced by the JDTLS manifest/);
  assert.match(result.stdout, /JDTLS download cache verified: 6 artifact/);

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
  await fs.writeFile(fakeBun, "#!/bin/sh\nprintf '1.3.14\\n'\n", { mode: 0o700 });
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

  const swiftSource = path.join(testRoot, "swift-source");
  const swiftpmCache = path.join(testRoot, "swiftpm-cache");
  const swiftRepositoryRoot = path.join(swiftpmCache, "repositories");
  const swiftRepository = path.join(swiftRepositoryRoot, "fixture-dependency-deadbeef");
  const swiftResolved = path.join(testRoot, "Package.resolved");
  const swiftRemote = "https://github.com/example/fixture-dependency.git";
  await fs.mkdir(swiftSource, { recursive: true });
  run("git", ["init", "--quiet"], swiftSource);
  run("git", ["config", "user.name", "Cache Test"], swiftSource);
  run("git", ["config", "user.email", "cache-test@example.invalid"], swiftSource);
  await fs.writeFile(path.join(swiftSource, "Package.swift"), "// swift-tools-version: 6.2\n");
  run("git", ["add", "Package.swift"], swiftSource);
  run("git", ["commit", "--quiet", "-m", "fixture"], swiftSource);
  const swiftRevision = run("git", ["rev-parse", "HEAD"], swiftSource);
  await fs.mkdir(swiftRepositoryRoot, { recursive: true });
  run("git", ["clone", "--quiet", "--mirror", swiftSource, swiftRepository]);
  run("git", ["remote", "set-url", "origin", swiftRemote], swiftRepository);
  await fs.writeFile(
    swiftResolved,
    JSON.stringify({
      originHash: "fixture",
      pins: [{
        identity: "fixture-dependency",
        kind: "remoteSourceControl",
        location: swiftRemote,
        state: { revision: swiftRevision, version: "1.0.0" },
      }],
      version: 3,
    }),
  );
  const swiftpmArguments = [
    "--skip-cargo",
    "--swiftpm-cache",
    swiftpmCache,
    "--swiftpm-resolved",
    swiftResolved,
    "--swift-version",
    "6.2",
  ];
  result = verify([...swiftpmArguments, "--write-swiftpm-manifest"]);
  assertSucceeded(result);
  await fs.access(path.join(swiftpmCache, ".lithe-integrity.json"));
  result = verify(swiftpmArguments);
  assertSucceeded(result);
  assert.match(result.stdout, /SwiftPM dependency cache verified: 1 repository/);

  const changedResolved = JSON.parse(await fs.readFile(swiftResolved, "utf8"));
  changedResolved.originHash = "changed-fixture";
  await fs.writeFile(swiftResolved, JSON.stringify(changedResolved));
  result = verify(swiftpmArguments);
  assertSucceeded(result);
  assert.match(diagnostics(result), /Package\.resolved SHA-256 changed/);
  await fs.access(swiftRepository);

  await fs.writeFile(path.join(swiftRepository, "tampered"), "tampered\n");
  result = verify(swiftpmArguments);
  assertSucceeded(result);
  assert.match(diagnostics(result), /SHA-256 mismatch|file set does not match/);
  await assert.rejects(fs.access(swiftRepository));

  process.stdout.write("Download cache verifier tests passed.\n");
} finally {
  await fs.rm(testRoot, { force: true, recursive: true });
}
