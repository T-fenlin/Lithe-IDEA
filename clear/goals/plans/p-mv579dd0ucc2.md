# 阶段归档 · p-mv579dd0ucc2

- 收束时间:2026-10-12T12:03:46.372Z
- 所属目标:g-mv572w41nwid
- 收束之辞:构建发布层六步走完，全部为只读检索与配置原文核对，未改任何代码。

拿到的东西：
1. **假绿灯实证**（s1）：GPUI 不在 rust/ workspace 里（rust/Cargo.toml:2）；classify-ci-changes.sh:308-316 的 rust/* 兜底把 GPUI 改动映射到 rust_core+rust_database+macos_release+windows+windows_rust 五条开关，逐条核对这五条的构建目标（SharedRust 用 rust/Cargo.toml、WindowsRust 用 windows/tauri/src-tauri/Cargo.toml、frontend 跑 windows/tauri 的 bun 测试、macos_release 走 swift build），无一条指向 lithe-gpui。用 reports/classify-gpui-changes.ps1 对基线提交 3085f5d1 的真实改动路径（它就是一次 lithe-ui 改动）复跑，输出五条开关全 true。
2. **矩阵门禁不触发**（s2）：platformPaths 8 条不含 GPUI，未命中即打印放行并退出 0，不检查任何能力 JSON。
3. **打包面清单**（s3）：assets/themes/extensions/fonts 全部编译期内嵌；repository_*_dir() 的 17 处调用点全在 cfg(test) 内，生产路径 0 处；只需 exe + LanguageServers/{jdtls,jdk}。
4. **版本号与工具链**（s4）：HOST_VERSION=env!(CARGO_PKG_VERSION)=0.0.1，唯一 build.rs 只嵌图标；该常量经 extensions/src/core_bridge.rs:7 进插件宿主契约的 hostVersion。两套 workspace 无 rust-toolchain 文件，10 处 CI 用 @stable。
5. **macOS 与 Linux**（s5）：三条 macOS 工作流的产物源是 SwiftPM 的 .build/；全仓无公证（notarytool/stapler 命中 0）；Linux 无任何构建入口但依赖已解析。
6. **汇总**（s6）：三平台清单、Windows 六环节、macOS M1–M8 共 10.5 人日、迁移六步顺序、profile 不足分析、P0–P3 共 17.5 人日、两个待人决策点、八项未覆盖。

对主理人 §2.1 的复核结论：四句断言全部成立（仅 build-windows.ps1 行号应为 10 而非 11），另补出四条他未写出的缺口 N1–N4。

## 步骤
- [advanced] 1. 读 scripts/classify-ci-changes.sh 全文与 .agents/skills/write-stable-tests/scripts/test-stability-windows.ps1，确认 rust/* 兜底把 GPUI 改动映射到哪些开关，以及 SharedRust / WindowsRust 两个 scope 实际编译的 manifest 路径；同时看 ci-windows.yml / ci-macos.yml 的 job if 条件 → reports/gpui-br-s1-ci-lanes.md
- [advanced] 2. 读 scripts/verify-platform-feature-matrix-change.mjs 的 platformPaths 常量与 compare 逻辑，并对照 develop-lithe SKILL「Keep the platform feature matrix current」小节列出的平台路径 → reports/gpui-br-s2-matrix-gate.md
- [advanced] 3. 汇总 GPUI 侧全部运行时资源解析路径并分类：assets（rust-embed 编译期内嵌）、themes/extensions/fonts/icons（include_str!/include_bytes!）、jdtls/jdk（current_exe + LanguageServers）、cache（LOCALAPPDATA / ~/Library/Caches）。逐条判定生产路径是否读源码树，并列出打包期必须随包分发的清单 → reports/gpui-br-s3-packaging-surface.md
- [advanced] 4. 检索 GPUI 全目录的版本号来源与 build.rs，确认有无发布版本注入通路（对照 macOS 端 Info.plist 里 LitheBuildGitRevision / LitheBuildTimestamp 的做法）；并核对两套独立 workspace 有无 rust-toolchain 文件、CI 用的是哪个 toolchain action → reports/gpui-br-s4-version-and-toolchain.md
- [advanced] 5. 核对 release-macos.yml / ci-macos.yml / release-preview-macos.yml 的构建目标是否含 GPUI，全仓是否有公证（notarytool/stapler）步骤，以及 Linux 侧有无构建入口与依赖前提（查 Cargo.lock 的 wayland/x11/ash 与选型笔记的 Linux 结论） → reports/gpui-br-s5-macos-and-linux.md
- [advanced] 6. 整合前 5 步检验结果，写出三平台流水线清单、macOS 从零建链路分步清单与人日、release 工作流迁移顺序，并逐条复核主理人 §2.1 的每句断言与 §3.2 的 A2/A3 人日 → reports/gpui-build-release-findings.md
