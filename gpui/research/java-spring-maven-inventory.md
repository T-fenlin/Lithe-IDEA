# Java / Spring / Maven 能力盘点与落地排序（gpui 前端）

> 调研对象：仓库 `D:\developmentProjects\rust\Lithe-IDEA`，分支 `feat/gpui-shell-rewrite`。
> 口径：维护者 2026-09-25 明确 **Java 生态第一优先级**（补全 + 跳转 → Maven → Spring / Spring Boot），
> **用户体验高于一切**；仓库硬规则是「先复用成熟上游，不要自研已有子系统」
> （`.agents/skills/develop-lithe/SKILL.md` 的 "Reuse mature developer tooling before building replacements"
> 与 "Respect repository ownership"；`gpui/HANDOFF.md:13-20`、`gpui/HANDOFF.md:118-124`）。
>
> 本文是**只读调研**：除本文件外没有修改任何产品代码。事实与建议分开写，每条事实都带文件路径或 URL；
> 查不清的一律写「未确认」。

---

## 1. 结论速览

### 1.1 能复用什么（一句话：Java/Maven/Spring 的**语义**都已有上游或 Core，缺的全在「接线」和「呈现」）

| 能力 | 谁提供 | 现状 |
| --- | --- | --- |
| Java 语法/类型/补全/诊断/重构 | **JDT LS 1.61.0**（已捆绑，含 **m2e 2.7.800 + 内嵌 Maven 3.9.1600**） | 载荷在 `.artifacts/jdtls`，gpui 只用了 `definition` 一个操作 |
| Maven 项目模型（reactor / 模块 / profiles / sourceRoots） | **Core `maven.scan`**（`rust/lithe-core/src/project/maven.rs:792`） | 已有，gpui 未调用 |
| Maven → JDT 的项目导入（源根、profiles、settings） | **Core `lsp.startServer` 的 `mavenContext`**（`rust/lithe-core/src/lsp/interface/engine.rs:965-1003`） | 已有且**已声明在契约里**（`shared/contracts/rust-core-api.md:1170-1178`），gpui **没传** |
| Spring 语义（配置项/配置值/bean/注入/端点/property 引用） | **Core `spring.index`**（`rust/lithe-core/src/languages/spring.rs:54`） | 已有，Windows/macOS 已接，gpui 未接 |
| Spring Boot Run（`spring-boot:run`） | **Core Run 探测器 + `runConfig.*`**（`rust/lithe-core/src/execution/configuration.rs:2004`、`detectors/maven.rs:17`） | 已有，gpui 未接 |
| 补全菜单（弹出、键盘上下选、Enter 落字、snippet 转换） | **上游 `gpui-base` 0.6.6 的 LSP 补全子系统** | 已有完整实现，**只差把 provider 挂上去** |
| 运行/调试（DAP、Java Test、Java Debug Server） | Core `debug.*` + 载荷内的 java-debug / java-test | 已有，gpui 未接 |

### 1.2 必须自建什么（只有编排与呈现，没有一行 Java 语法）

1. **gpui 侧的补全适配器**：把 Core `lsp.request{operation:"completion"}` 的结果转成
   `CompletionProvider`（一个新文件，约 200~300 行，无 Java 语法）。
2. **Maven 项目探测 -> 视图**：调 `maven.scan`，把模块树/生命周期/profile/依赖渲染出来（`maven.*` 已有数据）。
3. **传给 JDT 的 `mavenContext`**：把 `maven.scan` 的 `relativePath` 与（可选）本地 Maven 设置组装成
   version 1 context，塞进 `lsp.startServer`（**这是让 `src/main/java` 之外的生成源根、profile、settings 生效的唯一开关**）。
4. **JDT 事件泵**：现在只在 `definition` 时顺带等事件；补全/诊断要在**没有请求**的时候也持续收
   `lsp.waitEvents`（诊断、`mavenProfileTask`、`projectImporting` 就绪阶段都要它）。
5. **运行面板**：gpui 目前**只有终端**（`BottomPaneKind::Terminal`，`gpui/crates/workbench/src/workspace.rs:961`），
   没有 Run 工具窗、没有运行配置 UI。

### 1.3 建议的里程碑顺序（每个都「用户当天能感知价值」）

| # | 里程碑 | 用户当天能感知到什么 | 复用的上游 | 大致规模 |
| --- | --- | --- | --- | --- |
| **M1** | **Java 补全**（字节偏移→UTF-16 换算 + provider + 事件泵） | 敲 `Str` 弹出 `String`，Enter 落字 | gpui-base 补全子系统 + Core `lsp.request{completion}` | gpui 侧 1 个新文件 + 少量接线 |
| **M2** | **Maven 项目模型接进 JDT**（`maven.scan` → `mavenContext`） | 打开真实 Spring Boot 多模块工程，`src/main/java` / `target/generated-sources` 都能跳、能补全，profile 生效，不再报「包不存在」 | Core `maven.scan` + `lsp.startServer{mavenContext}` | gpui 侧 1 个新模块 + session 加字段 |
| **M3** | **Maven 工具窗**（模块树 / 生命周期 / Profiles / 依赖树） | 右栏点开 Maven 看到真实模块树，双击跑 `compile` / `package`，输出进底部 | Core `maven.scan` / `launchPlan` / `dependencyPlan` / `dependencies` / `diagnostics` | 一个数据层 + 三个视图，工作量最大的一步 |
| **M4** | **Run：Maven / Spring Boot 一键启动** | Run 工具窗列出 `spring-boot.maven` 配置，点绿三角把应用跑起来，输出进底部、端口可点开 | Core `runConfig.generate` / `resolve` / `createLaunchPlan` + 宿主进程启动 | 中等；gpui 侧要新增 Run 面板 + 进程宿主 |
| **M5** | **Spring 补全/跳转**（`application.properties` / `yml` 配置项、bean 注入跳转、端点视图） | 在 `application.properties` 里补全 Spring 配置项并显示默认值/说明；`@Value("${...}")` 可跳 | Core `spring.index` + 上游补全菜单（同一个 provider 再加一路数据源） | 中等 |

> 排序理由：M1 是维护者列的第一优先级且**当天可见**；M2 决定「打开任何真实 Spring Boot 工程能不能用」，
> 不做 M2 时 M1 在很多工程里会补出一堆假的「包不存在」；M3/M4 是「能开发 Maven 项目」的完整闭环；
> M5 是 Spring 专项，放在 Maven 项目模型与运行链路稳定之后（见 §4 的建议）。

---

## 2. 上游载荷清单

### 2.1 仓库里 `third_party/` 只有清单，没有二进制

`third_party/README.md:3-16` 的第三方策略：**这里只放最小可复现输入**（manifest + 校验和 + 许可证指针），
二进制放忽略的构建缓存。所以 `third_party/` 下**只有三个 manifest**：

```
third_party/README.md
third_party/dbx/manifest.json
third_party/jdk/manifest.json      1334 B
third_party/jdtls/manifest.json    2122 B
```

（`Get-ChildItem third_party -Recurse -File` 的完整输出就是这 4 个文件。）

### 2.2 `third_party/jdtls/manifest.json`（25 行，全部字段已读）

| 字段 | 值 | 出处 |
| --- | --- | --- |
| `version` | `1.61.0` | `third_party/jdtls/manifest.json:2` |
| 归档 | `https://download.eclipse.org/jdtls/milestones/1.61.0/jdt-language-server-1.61.0-202609031315.tar.gz` | 同文件 `:3` |
| 许可证 | **EPL-2.0**（`https://www.eclipse.org/org/documents/epl-2.0/EPL-2.0.txt`） | 同文件 `:5` |
| Lombok | `1.18.46`，**MIT** | 同文件 `:7-11` |
| Java Debug | 扩展 `vscjava.vscode-java-debug` `0.59.0`，服务器插件 `0.53.2`，**EPL-1.0** | 同文件 `:12-18` |
| Java Test | 扩展 `vscjava.vscode-java-test` `0.46.0`，插件 `0.43.1`，**MIT** | 同文件 `:19-26` |
| **最低 Java 主版本** | **21** | 同文件 `:27`（`"minimumJavaVersion": 21`） |

### 2.3 `third_party/jdk/manifest.json`（Temurin 21）

| 平台 | 版本 | 出处 |
| --- | --- | --- |
| `macos-aarch64` / `macos-x86_64` / `windows-x86_64` | `21.0.12.1+1` | `third_party/jdk/manifest.json:6,11,21` |
| `windows-aarch64` | `21.0.12+8` | 同文件 `:16` |
| 许可证 | Temurin（GPLv2 + Classpath Exception） | 同文件 `:2` |

### 2.4 本地已下载的载荷：`.artifacts/jdtls`

`scripts/prepare-jdtls.ps1` 下载 + 校验 + 展开；缓存原件在 `.artifacts/jdtls-downloads`：

| 缓存文件 | 体积 | sha256 前 12 位 |
| --- | --- | --- |
| `jdtls-1.61.0-338e7e73….tar.gz` | 51 037 522 B（≈48.7 MiB） | `338e7e73d618` |
| `lombok-1.18.46-01f7b1a0….jar` | 2 045 364 B | `01f7b1a015e3` |
| `vscode-java-debug-0.59.0-87627e24….zip` | 3 479 669 B | `87627e24dbb5` |
| `vscode-java-test-0.46.0-56c1e14d….zip` | 4 987 746 B | `56c1e14dc73a` |
| `EPL-2.0-*.txt` / `java-debug-EPL-1.0-*.txt` / `java-test-MIT-*.txt` / `lombok-MIT-*.txt` | 14 198 / 11 571 / 1 326 / 5 378 B | 与 manifest 一致 |

展开后的 `.artifacts/jdtls` **实测 76.1 MB**（`Get-ChildItem .artifacts\jdtls -Recurse -File | Measure-Object Length -Sum`）。
`gpui/HANDOFF.md:90` 与 `gpui/PLAN.md:923` 写的是「61.7 MB / 114 个 plugin」——
**当前实测是 76.1 MB**（plugin jar 100 个 + `config_*` 目录 + java-debug/java-test/lombok + `features/`）。
61.7 MB 与 76.1 MB 的差异原因**未确认**（可能是"只算 plugins 目录"与"整个载荷"的口径差；本次没有找到 61.7 的来源脚本）。
打包体积以实测为准。

### 2.5 `.artifacts/jdtls` 到底包含哪些扩展

**方法**：`.artifacts/jdtls/config_win/config.ini:7` 的 `osgi.bundles=` 一行，
按 `,` 切分得到 **106 个 bundle**（`Select-String` + `-split ','` 实测 `count=106`）。
完整枚举见该行；下面按类别列**有意义上游扩展**：

| 扩展 | 版本 | 作用 | 证据 |
| --- | --- | --- | --- |
| `org.eclipse.jdt.ls.core` | 1.61.0.202609031315 | JDT LS 本体 | `config.ini:7`；`plugins/org.eclipse.jdt.ls.core_1.61.0.202609031315.jar` |
| `org.eclipse.jdt.core` | 3.47.0 | Java 模型/编译器前端 | 同上 |
| `org.eclipse.jdt.core.javac` | 1.0.0 | 用 javac 编译 | 同上 |
| `org.eclipse.jdt.core.manipulation` | 1.25.0 | 重构/代码操作 | 同上 |
| `org.eclipse.jdt.debug` / `org.eclipse.debug.core` / `org.eclipse.jdt.launching` | 3.26.100 / 3.24.0 / 3.24.300 | 调试与启动 | 同上 |
| `org.eclipse.jdt.junit.core` / `.runtime` | 3.15.0 / 3.8.100 | JUnit 集成 | 同上 |
| **`org.eclipse.m2e.core`** | **2.7.800.20260522-0510** | **Maven 项目导入（m2e）** | `config.ini:7`；`plugins/org.eclipse.m2e.core_2.7.800.20260522-0510.jar` 630 252 B |
| **`org.eclipse.m2e.jdt`** | 2.5.200.20260602-0749 | m2e → JDT classpath 桥 | 同上，175 144 B |
| **`org.eclipse.m2e.apt.core`** | 2.3.100 | APT（Lombok 那条链） | 同上，56 663 B |
| **`org.eclipse.m2e.maven.runtime`** | **3.9.1600.20260522-0510** | **内嵌 Maven 运行时** | 同上，5 958 733 B |
| `org.eclipse.m2e.workspace.cli` | 0.4.0 | m2e CLI | 同上，17 901 B |
| `org.eclipse.buildship.core` / `.compat` + `org.gradle.toolingapi` | 3.1.12 / 8.9.0 | Gradle 支持 | 同上 |
| `wrapped.com.jetbrains.intellij.java.java-decompiler-engine` | 253.29346.240 | 反编译（`jdt://` 虚拟源码的可读来源之一） | 同上，940 444 B |
| `org.eclipse.jdt.apt.core` / `.pluggable.core` | 3.8.800 / 1.4.700 | 注解处理 | 同上 |
| `org.commonmark*` / `wrapped.com.vladsch.flexmark*` / `org.jsoup` | — | Javadoc hover 的 Markdown/HTML 渲染 | 同上 |

**明确不包含（本次逐项确认）**：

- **没有 Spring 支持的任何 bundle**：`config.ini` 里匹配 `spring` 的结果为 **0 条**；
  `plugins/` 目录名里也没有任何 `spring`（`Select-String -Path .artifacts\jdtls\config_win\config.ini -Pattern 'spring'` 无输出）。
  m2e 的 Spring 支持（Eclipse 的 m2e-apt / Spring IDE）从来不在 JDT LS 里程碑载荷里。
- 没有 `features/` 里的产品级功能：`features/` 只有 **1 个 jar** ——
  `org.eclipse.equinox.executable_3.8.3400.v20260804-1928.jar`（734 816 B），是启动器资源，不是 IDE 功能。
- 载荷里**没有捆绑 JDK**：`third_party/jdtls/manifest.json` 只声明 `minimumJavaVersion: 21`；
  载荷展开后没有 `jre/` 目录（`gpui/crates/java/src/jdtls.rs:449` 的 `jdtls_root.join("jre")` 是"官方载荷可能自带、
  我们这份没有"的兜底候选）。JDK 由 `third_party/jdk`（Temurin 21）单独提供。

### 2.6 载荷启动方式

**（a）上游自带的 wrapper 脚本**：`.artifacts/jdtls/bin/jdtls.ps1`（22 行，已读）：

```powershell
$javaExecutable = if ($env:JAVA_HOME) { Join-Path $env:JAVA_HOME "bin\java.exe" } else { "java" }   # :2
$jvmArguments.Add("-javaagent:$lombokAgent")                                                        # :6
$jvmArguments.Add("--add-modules=ALL-SYSTEM")                                                       # :7
$jvmArguments.Add("--add-opens=java.base/java.util=ALL-UNNAMED")                                    # :8
$jvmArguments.Add("--add-opens=java.base/java.lang=ALL-UNNAMED")                                    # :9
& $javaExecutable @jvmArguments "-Declipse.application=org.eclipse.jdt.ls.core.id1" `
  "-Declipse.product=org.eclipse.jdt.ls.core.product" "-Dosgi.bundles.defaultStartLevel=4" `
  "-Dlog.protocol=true" "-Dlog.level=ALL" "-jar" $launcherJar.FullName `
  "-configuration" $configuration @serverArguments                                                 # :21
```

`.artifacts/jdtls/bin/jdtls.bat` 只是转发到 `jdtls.ps1`（120 B）。

**（b）Lithe 实际用的方式：Core 结构化直启（不执行脚本）**。
`gpui/crates/java/src/session.rs:113-158` 送进 `lsp.startServer` 的字段（本文件已逐字读过）：

```json
{ "providerId": "java",
  "executablePath": "<jdtls>/bin/jdtls.bat", "arguments": [],
  "environment": { "JAVA_HOME": "<jdk 21+>" },
  "rootUri": "file:///D:/ws/", "workingDirectory": "D:\\ws",
  "runtimeExecutablePath": "<jdk>/bin/java.exe",
  "jdtlsLaunchResources": { "launcherJarPath": ..., "configurationDirectory": "...\\config_win",
                            "lombokAgentPath": ..., "javaDebugBundlePath": ...,
                            "javaExtensionBundlePaths": [ ... ] },
  "cacheDirectory": "%LOCALAPPDATA%\\Lithe\\cache\\language-servers",
  "workspaceFingerprint": "<java.jdtWorkspaceFingerprint 的不透明结果>",
  "javaRuntimes": [{ "homePath": "<jdk>", "version": "21.0.8" }] }
```

JVM 命令行由 **Core** 拼（`rust/lithe-core/src/lsp/languages/jdt.rs` 的 `direct_java_arguments`，
`gpui/crates/java/src/jdtls.rs:11-15` 明确「平台适配器不拼 JVM 参数」）。
`arguments: []` 是刻意的：有 `jdtlsLaunchResources` + `runtimeExecutablePath` 时 Core 自己拼
（`gpui/PLAN.md:954-955`、`shared/contracts/rust-core-api.md:1208-1217`）。

**（c）workspace 目录（`-data`）**：宿主只给 `cacheDirectory`，Core 自己拼两级
`<cacheDirectory>/jdtls/<workspaceKey>`（`gpui/crates/java/src/workspace.rs:15-23`、
`shared/contracts/rust-core-api.md:1241-1301`）。
`workspaceKey` = `sha256(workspaceRoot 归一化 + workspaceFingerprint)`（`lsp.jdtWorkspaceKey`）。
`workspaceFingerprint` 的输入是**根构建描述符的时间戳/大小**（`pom.xml` / `build.gradle` / `build.gradle.kts`）
+ **直接含 `pom.xml` 的子目录名** + `jdtlsVersion`（`gpui/crates/java/src/workspace.rs:33,130-198`）。
缓存保留 30 天（`java.jdtCacheRetention`）。

**（d）配置目录**：按平台选 `config_win` / `config_mac` / `config_mac_arm` / `config_linux`
（`gpui/crates/java/src/jdtls.rs:77-89`；载荷里 10 个 `config_*` 目录都在，包含 `config_ss_*` 无头变体）。

### 2.7 `.artifacts/jdtls` 与 `third_party/jdtls` 是否同一份

**是同一份载荷，但 `third_party/jdtls` 里没有二进制**：
`third_party/jdtls/manifest.json` 与 `.artifacts/jdtls/manifest.json` 的**大小完全相同（2122 B）**，
且 gpui 侧与 Windows host 都把 `third_party/jdtls/manifest.json` **编译进代码**作为版本兜底：
`gpui/crates/java/src/jdtls.rs:34`（`include_str!("../../../../third_party/jdtls/manifest.json")`）、
`windows/tauri/src-tauri/src/lsp.rs:36`（同）。`.artifacts/jdtls/manifest.json` 是 `prepare-jdtls.ps1` 拷进去的复本。
校验和可对照：`jdtls-1.61.0-338e7e73….tar.gz` 的文件名里嵌的哈希与 manifest 的 `archiveSHA256`
前 12 位一致（`338e7e73d618`）。**版本一致：1.61.0**。

### 2.8 `third_party/**` 下还有别的 Java 相关载荷吗

逐个列完（`Get-ChildItem third_party -Recurse -File` 只有 4 个文件）：

| 路径 | 是否 Java 相关 | 说明 |
| --- | --- | --- |
| `third_party/jdtls/manifest.json` | ✅ | JDT LS 1.61.0 + lombok + java-debug + java-test |
| `third_party/jdk/manifest.json` | ✅ | Temurin 21，四平台；本地**未下载**（`.artifacts/jdk` 不存在） |
| `third_party/dbx/manifest.json` | ❌ | 数据库工具载荷 |
| `third_party/README.md` | — | 策略说明 |

**没有**：Maven wrapper 载荷（无 `third_party/maven/**` / `mvnw`）、
没有捆绑 JDK 二进制、**没有 Spring Boot language server**。
Maven 本身也不在载荷里 —— 项目要跑 Maven 时用的是「项目自带 wrapper 或机器上装的 Maven」
（`rust/lithe-core/src/project/maven.rs` 的 `has_wrapper`、`windows/.../resolve-maven-toolchain.ts:55-75`）。
本机实测：`where.exe mvn` → `D:\ProgramData\maven\apache-maven-3.6.3\bin\mvn`。

### 2.9 本机 Java 环境（实测，会决定验证链路怎么写）

```
where.exe java            -> D:\ProgramData\java\jdk1.8.0_221\bin\java.exe
java -version             -> 1.8.0_221
$env:JAVA_HOME            -> D:\ProgramData\java\jdk1.8.0_221
D:\ProgramData\java\openjdk-21\bin\java.exe  -> 存在 (True)
where.exe mvn             -> D:\ProgramData\maven\apache-maven-3.6.3\bin\mvn(.cmd)
$env:LITHE_JDTLS_JAVA     -> (空)
```

⚠️ **PATH 上的 `java` 是 1.8，JDT LS 1.61 起不来**（`minimumJavaVersion: 21`）。
验证时必须显式给 `LITHE_JDTLS_JAVA=D:\ProgramData\java\openjdk-21\bin\java.exe`
（发现顺序见 `gpui/crates/java/src/jdtls.rs:426-460`；`gpui/HANDOFF.md:90-91`、
`gpui/PLAN.md:1019-1022` 都记了这条）。JDK 21 在 `%ProgramData%\java` 这一档，
正是 `installed_jdk_roots()` 枚举的目录之一（`jdtls.rs:544-546`），所以**产品路径本身能找到它**。

---

## 3. Maven：从「打开一个 pom.xml 项目」到「能补全 / 跳转 / 看到依赖」

### 3.1 打开项目到有语义能力之间的全部环节

| # | 环节 | 谁负责 | 现状 | 证据 |
| --- | --- | --- | --- | --- |
| 1 | 识别工作区是不是 Java 项目 | **Core** `java.workspacePolicy` | ✅ gpui 已用 | `gpui/crates/java/src/service.rs:289-327` |
| 2 | 找出 Maven reactor（哪个目录是 Maven 根、有哪些模块） | **Core** `maven.scan` | ⛔ gpui 未调 | `rust/lithe-core/src/project/maven.rs:792-826`；契约 `rust-core-api.md:1450-1473` |
| 3 | 解析 `pom.xml`（groupId/artifactId/packaging/modules/profiles/sourceRoots/hasWrapper） | **Core**（自己的 XML 解析，不是 m2e） | ✅ 已有 | `rust/lithe-core/src/project/maven.rs:734-826`（`declared_modules` / `scan`）；示例 fixture `shared/fixtures/maven/source-roots-v1.json` |
| 4 | 把 reactor/源根/profiles/Maven settings 告诉 JDT LS | **Core** `lsp.startServer{mavenContext}` | ✅ 已有，⛔ gpui 没传 | `rust/lithe-core/src/lsp/interface/engine.rs:965-1003`；`rust/lithe-core/src/project/maven.rs:241-287`（`jdt_configuration`） |
| 5 | 真正 import Maven 项目 + 建 classpath | **m2e（JDT LS 内部）** | ✅ 载荷里有 | `config.ini:7` 的 5 个 `org.eclipse.m2e.*`；`gpui/PLAN.md:1027-1030` 明确「JDT 自己的 m2e 导入仍在跑」 |
| 6 | 语义能力（补全/跳转/诊断/重构） | **JDT LS** | ✅ 能让它跑，⛔ gpui 只用 `definition` | `gpui/crates/java/src/service.rs:122-176` |
| 7 | 依赖树（`mvn dependency:tree`） | **Core** `maven.dependencyPlan` + 宿主跑进程 + `maven.dependencies` 解析 | ⛔ gpui 未接 | 契约 `rust-core-api.md:109-110`；`rust/lithe-core/src/project/maven.rs:159` |
| 8 | 构建诊断（编译错误映射到编辑器） | **Core** `maven.diagnostics` + JDT 自己的 `publishDiagnostics` | ⛔ gpui 未接 | 契约 `rust-core-api.md:111` |
| 9 | Maven 执行计划（`mvn` 参数数组，不用 shell） | **Core** `maven.launchPlan` | ⛔ gpui 未接 | `rust/lithe-core/src/project/maven.rs:147`；契约 `:1475-1480` |
| 10 | Maven 视图（模块树 / 生命周期 / Profiles / 依赖树 / 输出） | **gpui 自建（纯呈现）** | ⛔ 只有空态 | `gpui/crates/workbench/src/right_tool_window.rs:136-155` |

### 3.2 最小可用路径（M2，只动 gpui 侧，不碰 Core）

```
workspace.snapshot  →  maven.scan(root, paths)  →  组装 mavenContext(version 1)
        →  lsp.startServer{ ..., mavenContext }  →  等 stateChanged: ready
        →  需要时 lsp.request{ completion | definition | ... }
```

关键细节（都已核实）：

- **`mavenContext` 的字段**就是 `maven.launchPlan` 那个 version 1 context
  （`MavenLaunchContextRequest`，`rust/lithe-core/src/project/maven.rs:63-78`）：
  `{ version: 1, reactorPath, profiles[], settingsPath?, localRepositoryPath?, skipTests, mavenExecutablePath?, javaHomePath? }`。
  最小路径只需要 `{ version: 1, reactorPath, profiles: [], skipTests: false }`，
  `reactorPath` 取 `maven.scan` 的 `relativePath`（`"."` 表示工作区根）。
- **Core 会自动补 sourcePaths**：`jdt_configuration` 遍历 `declared_modules` 收集
  `mainJava / testJava / generatedMain / generatedTest` 四类源根，归一化成
  `java.project.sourcePaths`（`rust/lithe-core/src/project/maven.rs:249-285`）。
  **我们不需要自己解析 `pom.xml` 找源根**（这也是「不要自己造」清单里的一条）。
- **Core 会在 `ServiceReady` 之后**每个 Maven 项目发一条
  `java.project.updateSettings`（带 `org.eclipse.m2e.core.selectedProfiles`），
  最多 8 个在飞、其余排队，每项报 `running/succeeded/failed`，
  聚合状态通过 `lsp.waitEvents` 的 `mavenProfileTask` 事件给宿主（契约 `:1178-1195`）。
  → **gpui 需要的事件泵不只是补全要用，Maven 剖面进度也要用**。
- **`settingsPath` 走 `java.configuration.maven.userSettings`**；
  如果配置了 `localRepositoryPath`，Core 会在 `<cacheDirectory>` 下**物化一份 settings.xml**
  再交给 JDT（`engine.rs:990-992` 的 `materialized_maven_settings`），
  所以「JDT 用的本地仓库 == 命令行用的本地仓库」这条一致性是 Core 保证的。
- **现在 gpui 不传 `mavenContext` 的代价**（`gpui/PLAN.md:1027-1030` 已记）：
  Maven 工程的源根与剖面不由 Lithe 喂给 JDT（JDT 的 m2e 仍会自己导入），
  但**生成源根、显式 `<build>` 源根、选中的 profile、settings 里的镜像/本地仓库**这些
  「Lithe 才知道的东西」不会同步给 JDT。

### 3.3 依赖树（M3 的一部分）

走 Core 的三件套，宿主只负责跑进程与展示：

1. `maven.dependencyPlan{root, context, module}` → 得到 `{ executable: {toolchain:"project-maven"}, arguments[], workingDirectory, configurationFingerprint }`；
   里面写死了 `org.apache.maven.plugins:maven-dependency-plugin:3.8.1:tree`（`rust/lithe-core/src/project/maven.rs:51-52`）。
2. 宿主在 `workingDirectory` 起进程跑 `arguments`（**参数数组，不经 shell**）。
3. 输出（上限 500 000 字符）交 `maven.dependencies{modulePath, output}` →
   归一化成确定性的依赖树（上限 10 000 节点 / 深度 64，`maven.rs:53-55`）。

Windows 参考实现：`windows/tauri/src/features/maven/stores/maven.store.ts`（57 KB，
`scanMavenProject` 调用在 `:637`）、`windows/tauri/src/features/maven/api/maven-core-api.ts:35-58`。

### 3.4 Maven 项目是怎么被识别的（Windows 参考）

**不用 m2e 做识别，也不自己解析 `pom.xml`**：识别完全走 Core 的 `maven.scan`，
入口是 `windows/tauri/src/features/maven/stores/maven.store.ts:637` 的
`scanMavenProject(root, visiblePaths)`；返回 `null` 时把 `project` 置空、视图走
「未检测到 Maven 项目」空态（`:639-660`）。Core 侧的候选顺序写在契约里：
「`maven.scan` ... returns `null` when neither the root nor the supplied visible
workspace-relative paths contain a readable `pom.xml`. Candidates are tried in
shallowest-first order, with `/`-normalized lexical paths breaking ties, until one
parses successfully; a malformed candidate does not hide a valid nested project」
（`shared/contracts/rust-core-api.md:1450-1454`）。

Maven 安装（谁跑 `mvn`）由「项目 wrapper 优先、然后机器安装」决定，
且**这一步不启动任何进程**（`windows/tauri/src/features/maven/services/resolve-maven-toolchain.ts:24-37,55-75`）：
理由写在同文件 `:33-36` —— 这段跑在「打开第一个 Java 文件」的等待路径上，
一次 `mvn -version` 探测会拖慢每一次首开。

---

## 4. Spring / Spring Boot

### 4.1 事实盘点

| 事实 | 结论 | 证据 |
| --- | --- | --- |
| JDT LS 载荷里有 Spring 专属支持吗 | **没有**。`config.ini` 106 个 bundle 中匹配 `spring` 的 **0 条**；`plugins/` 无 spring jar | `.artifacts/jdtls/config_win/config.ini:7`；`Select-String -Pattern 'spring'` 无输出 |
| 仓库里有没有捆绑 / 引用 `spring-boot-language-server` | **没有**。全仓（`.md/.json/.swift/.ts/.rs/.ps1/.sh`）搜 `spring-boot-language-server` / `vscode-spring-boot` / `broadcom` → **0 命中** | 全仓 `Select-String -SimpleMatch` 无输出 |
| Lithe 有没有自己的 Spring 语义 | **有，而且很完整**：Core `spring.index` 会索引配置项、配置值（含 profile 与覆盖关系）、property 引用、bean、注入、端点、诊断 | `rust/lithe-core/src/languages/spring.rs`（2019 行，72 511 B）；`SpringIndexResponse` 字段见 `rust/lithe-core/src/protocol/...`（`spring.rs:3-7` 的 import） |
| macOS 怎么用 Spring | 全部走 Core `spring.index`，**没有 Spring Boot LS** | `macos/Sources/Lithe/Application/Features/SpringFeatureModel.swift:36`（`operations.springIndex(...)`）、`macos/Sources/Lithe/Models/AppModel/AppModel+LanguageEditing.swift:42,87-120`（hover / completion） |
| Windows 怎么用 Spring | 同样走 Core `spring.index`，用于 `@Value`/注入跳转 | `windows/tauri/src/features/spring/api/spring-index-api.ts:56-74`、`utils/spring-navigation.ts:44-133` |
| Spring Boot 的运行 | Core 的 Run 探测器认 `spring-boot-maven-plugin`，生成 `spring-boot.maven` 配置，goal 是 `spring-boot:run`，并支持 `-Dspring-boot.run.main-class` / `jvmArguments` / `arguments` | `rust/lithe-core/src/execution/detectors/maven.rs:17`（`("spring-boot-maven-plugin", "spring-boot.maven")`）、`execution/configuration.rs:2004-2008`、`:2034` |
| Spring Boot 端点视图 | macOS 有 `SpringEndpointsView`（列 `@RequestMapping` 路由，点击打开） | `macos/Sources/Lithe/Views/Run/SpringEndpointsView.swift`；macOS 命令 `spring-endpoints`（`macos/Sources/Lithe/Models/Keymap/LitheCommandCatalog.swift:54`） |
| gpui 侧的 Spring | **零**：`gpui/crates/*` 里搜 `spring` **0 命中** | `Get-ChildItem gpui\crates -Recurse -Include *.rs \| Select-String spring` 无输出 |

**`spring.index` 的输入**（`rust/lithe-core/src/languages/spring.rs:33-51`）：
`{ root, paths[], metadataRepository?, metadataRepositories[], refreshDependencyMetadata, textOverrides }`。
`metadataRepositories` 指向 `~/.m2/repository`，Core 会从里面的
`spring-configuration-metadata.json` / `additional-spring-configuration-metadata.json`
（`macos/.../SpringFeatureModel.swift:101-102`）读出**全部 Spring Boot 配置项的名称、类型、默认值、说明**。
→ **这意味着 Spring Boot 配置项的补全/提示事实已经躺在 `.m2` 里，我们不需要 Spring Boot LS 也能做**。

### 4.2 三条路对比

#### (a) 捆绑 `spring-boot-language-server`（`vscode-spring-boot` 的语言服务器）

| 维度 | 情况 |
| --- | --- |
| 上游 | spring-projects/spring-tools 的 STS4 语言服务器；官方有「Integrate Language Server Into Client」文档（[spring-tools wiki](https://github.com/spring-projects/spring-tools/wiki/Developer-Manual-Integrate-Language-Server-Into-Client)） |
| 许可证 | **未确认**（GitHub 在本机不可达：`web_fetch` 报 `URL hostname "github.com" resolves to a non-public IP address`；web_search 只返回二手页面）。**动手前必须逐包核对**（STS4 历史上是 EPL-2.0 + 部分包另有条款，"JAR 体积/条款随版本变化"有上游 issue：<https://github.com/spring-projects/spring-tools/issues/1293> 讨论过从 JAR 部署，<https://github.com/spring-projects/spring-tools/issues/1436> 讨论过 Boot LS 的 JAR 体积） |
| 体积 | **未确认**（同上，issue #1436 标题就是 "Boot LS JAR size"，但本机拿不到正文） |
| 启动方式 | **未确认**（同一份 wiki 文档本机拿不到）。STS4 的 LS 是 Java 进程 + LSP over stdio/socket，**这点与 `lsp.startServer` 的形状天然相容**，但具体命令行必须在能访问上游时核对 |
| 增量收益 | Spring 专有：`@ConfigurationProperties` 类型安全绑定、`@Autowired` 候选 bean、`spring.factories`、Boot 属性元数据的**服务端**校验与 quick-fix。**比 JDT LS + `spring.index` 明显更强** |
| 代价 | 多一个 JVM 常驻进程（内存/启动时间）、多一份载荷要审计与升级、多一个许可证面；且它的 Java 语义**仍依赖 JDT LS**（STS4 与 JDT LS 协作），不是替代关系 |

#### (b) 只靠 JDT LS（不做 Spring 专项）

| 做得好的 | 做不好的（用户能直接感到的差异） |
| --- | --- |
| Java 补全/跳转/诊断/重构/格式化全部可用（JDT 本体） | `application.properties` / `.yml` 里**没有 Spring 配置项补全**（JDT 不认识这些 key） |
| Maven 项目导入、classpath、依赖解析（m2e） | `@Value("${foo.bar}")` 里的 key 拼错**没有警告**，也没有「跳到定义」 |
| `@Autowired` / `@Component` 的**类型**解析（就是普通 Java 语义） | 补全 `@Autowired` 之后**不会按 bean 候选过滤**；`@Qualifier` 无提示 |
| Spring Boot 应用的**运行**（m2e + runConfig，见 §4.1） | 没有 endpoint 视图、没有 `@ConfigurationProperties` 的 key 校验、没有 `spring.factories` 感知 |

结论：**(b) 是「Java 开发体验完整、Spring 体验缺失」**。对"能开发 Spring Boot 项目"这件事，
用户当天最痛的是**属性文件没提示 + 属性 key 拼错没人管**，而不是 `@Autowired` 候选。

#### (c) 先做 Maven 项目模型（M2），Spring 专项后置

| 维度 | 情况 |
| --- | --- |
| 依赖的上游 | Core `maven.scan` + `lsp.startServer{mavenContext}` + JDT LS 的 m2e —— **全部已就绪** |
| gpui 侧工作量 | 小：1 个新模块（`maven.scan` 调用 + context 组装）+ `session.rs` 加一个字段 |
| 当天可见收益 | 打开真实 Spring Boot 多模块工程时，跨模块跳转 / 补全 / 生成源根**不再失效**；Maven profile 生效；`settings.xml` 的镜像与本地仓库与命令行一致 |
| 风险 | 低：不改 Core、不改载荷、不新增许可证面 |
| 不覆盖 | 属性文件补全 / bean 候选 / endpoint 视图 |

### 4.3 建议（与事实分开）

**建议：走 (c) → (b) → 视收益再评估 (a)**，理由是三条硬约束叠加：

1. **许可证/体积事实缺口**：(a) 需要的许可证条款与载荷体积**本次无法核实**（GitHub 不可达）。
   在 `develop-lithe` 的硬规则下，「复用最大的合规子系统」要求先有证据；证据不全就不能落地为载荷决策。
2. **仓库已有等价资产**：Core `spring.index` 已经能给出配置项（含默认值/说明）、配置值、
   property 引用、bean、注入、**端点**（`rust/lithe-core/src/languages/spring.rs:54` 起）。
   macOS 已经把这个索引接到了补全、hover、跳转、端点视图（`SpringFeatureModel.swift:36`、
   `AppModel+LanguageEditing.swift:42,87-120`、`SpringEndpointsView.swift`）。
   先复用这份资产，能覆盖 (b) 里列出的前两项最痛的缺口，且**零新增载荷**。
3. **顺序依赖**：(a) 的 Spring Boot LS **仍然需要 JDT LS 提供 Java 模型**。
   所以不管走哪条路，M2（Maven 项目模型接进 JDT）都是前置。先把前置做掉，
   (a) 与 (c) 的增量收益边界才会清楚。

**具体建议**（可执行口径）：

- **M5 做 `spring.index` 的接线，而不是新语言服务器**：补全菜单是同一个（上游 `CompletionProvider`），
  在 `.properties` / `.yml` 文件里再喂一路 `spring.index` 的数据源即可；
  `@Value` / 注入跳转照 `windows/tauri/src/features/spring/utils/spring-navigation.ts` 的语义
  （property ↔ value ↔ injection 三方互跳，`:44-133`）搬到 gpui。
- **只有在出现「`@ConfigurationProperties` 类型安全绑定 + bean 候选按类型过滤」这类明确诉求时**，
  才启动 (a) 的评估；评估必须带三张表：许可证逐包清单、载荷体积、以及"能不能作为 `lsp.startServer`
  的第二个 provider 被 Core 托管"。**未确认项见 §8。**

---

## 5. Core 命令清单（Java / Maven / 项目模型 / 运行配置 / LSP 相关）

来源：`shared/contracts/rust-core-api.md:79-211` 的 Commands 表（逐行读）。
「通用」= 多语言/多项目类型共用；「Java 专用」= 只服务 Java 生态。

### 5.1 Maven

| 命令 | 一句话 | 类型 | 契约行 |
| --- | --- | --- | --- |
| `maven.scan` | 解析 Maven 项目描述符，递归返回模块与 profiles | **Maven 专用** | `:107`（细节 `:1450-1473`） |
| `maven.launchPlan` | 由 versioned context 生成确定性的 Maven 调用（参数数组） | Maven 专用 | `:108`（`:1475+`） |
| `maven.dependencyPlan` | 为单个模块生成有界的依赖树调用 | Maven 专用 | `:109` |
| `maven.dependencies` | 把 dependency-plugin 的输出归一成确定性依赖树 | Maven 专用 | `:110` |
| `maven.diagnostics` | 从构建输出解析稳定的 Maven 编译器诊断 | Maven 专用 | `:111` |
| `maven.testResults` | 解析有界的 JUnit/Surefire 结果摘要与失败位置（还读 XML 报告） | Maven 专用 | `:112`（细节 `:212-281`） |

**没有** `java.*` / `maven.*` 之外的「解析 classpath」命令：classpath 的事实归 JDT/m2e，
Core 只做 `mavenContext` 的**归一化与配置下发**（§3.2）。

### 5.2 Java

| 命令 | 一句话 | 类型 | 契约行 |
| --- | --- | --- | --- |
| `java.workspacePolicy` | 判定 Java 工作区是否要激活，并给变更路径分类 | Java 专用 | `:138`（细节 `:1280-1291`） |
| `java.runMarkers` | 把 JDT 的 main/test 发现与 Maven 测试结果投成编辑器 Run 标记 | Java 专用 | `:139` |
| `java.jdtWorkspaceFingerprint` | 把平台的构建文件观察归一成可移植的 JDT 工作区指纹 | Java 专用 | `:140` |
| `java.jdtCacheRetention` | 从平台元数据里挑出过期的 JDT 工作区状态目录键 | Java 专用 | `:141` |
| `java.navigationMarkers` | 由有界的 JDT 语义请求解析带版本的 Java gutter 标记 | Java 专用 | `:147` |
| `java.resolveNavigation` | 把 gutter 标记解析成父类/实现位置 | Java 专用 | `:148` |
| `java.codeVision` | 返回 Java 声明的使用次数（code lens） | Java 专用 | `:155` |
| `java.className` | 由 Java 源码的包名 + 简单名解析运行时类名 | Java 专用 | `:156` |
| `java.sourceDefinition` | 在源码里定位 Java 类型/方法/字段声明 | Java 专用 | `:157` |
| `java.serverPort` | 解析 properties / YAML 里的 Spring server port | Java 专用（Spring 相关） | `:158` |
| `java.structure` | 解析 Java 的折叠、inlay hint 与可移植语法角色 | Java 专用 | `:159` |
| `spring.index` | 建确定性 Spring 配置/bean/注入/端点索引 | Java 专用（Spring） | `:160` |
| `mybatis.index` | 建确定性 MyBatis mapper 接口与 XML statement 索引 | Java 专用 | `:161` |

### 5.3 运行配置 / 启动计划

| 命令 | 一句话 | 类型 | 契约行 |
| --- | --- | --- | --- |
| `runConfig.inspect` | 只读检查 `.lithe` 运行文档的版本与陈旧度 | 通用（对 Java/Maven 有专用探测） | `:162` |
| `runConfig.generate` | 生成确定性的 Java/Maven 运行配置与工具链需求 | Java/Maven 相关 | `:163`（细节 `:1532-1546`） |
| `runConfig.resolve` | 合并 generated / project / local 三层并返回诊断 | 通用 | `:164`（`:1548+`） |
| `runConfig.updateOptions` | 应用类型化选项编辑，返回更新后的 project 或 local 文档 | 通用 | `:165` |
| `runConfig.saveEditorChanges` | 为一次编辑器保存准备 local 与可选 project 文档 | 通用 | `:166` |
| `runConfig.createUserConfiguration` | 校验类型化用户配置并返回更新后的文档 | 通用 | `:167` |
| `runConfig.createLaunchPlan` | 把一份生效配置投成平台中立的 Run/Debug 计划（支持 `mavenContext` 与 `javaLaunch`） | 通用（Java/Maven/Spring 路径明确） | `:168`（细节 `:1658-1690`） |

### 5.4 LSP（通用）

| 命令 | 一句话 | 类型 | 契约行 |
| --- | --- | --- | --- |
| `lsp.startServer` | 起一个 Rust 拥有的进程/会话并开始 initialize（**Java 可带 `mavenContext`**） | 通用 | `:136`（Java 细节 `:1162-1195`） |
| `lsp.jdtWorkspaceKey` | 推导确定性的 JDT LS 工作区状态目录键 | 通用（JDT 专用语义） | `:137` |
| `lsp.syncDocument` | 打开文档或应用全量/增量 `didChange`，版本单调 | 通用 | `:143` |
| `lsp.workspaceFilesChanged` | 把归一化的增删改文件发布给某个会话 | 通用 | `:144` |
| `lsp.closeDocument` | 关闭文档并清诊断 | 通用 | `:145` |
| `lsp.request` | 提交类型化语义请求，返回不透明 `operationId` | 通用 | `:146`（`:1336-1356`） |
| `lsp.cancelOperation` | 取消一个在飞语义操作 | 通用 | `:149` |
| `lsp.pollEvents` | 排空有序的生命周期/特性/诊断/结果/日志事件 | 通用 | `:150` |
| `lsp.waitEvents` | 阻塞到有事件或超时，然后排空 | 通用 | `:151` |
| `lsp.clearDiagnostics` | 清掉一个会话拥有的全部诊断 | 通用 | `:152` |
| `lsp.snapshot` | 返回诊断运行时快照（测试与控制面用） | 通用 | `:153` |
| `lsp.stopServer` / `lsp.destroyServer` | 有界优雅关闭 / 摘掉终态会话句柄 | 通用 | `:142` / `:154` |
| `lsp.applyTextEdits` | 应用 LSP UTF-16 文本编辑并校验范围 | 通用 | `:131` |
| `lsp.plainSnippet` | 把 LSP snippet 插入文本转成纯文本 | 通用 | `:132` |
| `lsp.builtinCompletions` | 轻量的当前文件标识符补全 | 通用（**降级路径**） | `:133` |
| `lsp.builtinHover` | 轻量的当前符号 hover 文本 | 通用（降级路径） | `:134` |
| `lsp.builtinNavigation` | 轻量的当前文件定义/引用位置 | 通用（降级路径） | `:135` |

### 5.5 `lsp.request` 支持哪些 `operation`（**关键：补全/诊断所需的语义请求 Core 都已实现**）

`rust/lithe-core/src/lsp/interface/engine.rs:273-321` 的 `LspSemanticOperation` 枚举逐条读完，
映射到 LSP 方法见 `engine.rs:3920-3936`，能力门见 `engine.rs:3938-3963`：

**completion / hover / definition / declaration / typeDefinition / references / implementation /
javaSuperImplementation / rename / formatting / codeActions / resolveCompletion / resolveCodeAction /
executeCommand / inlayHints / foldingRanges / semanticTokens / codeLens / virtualDocument /
javaEntrypoints / javaTestItems / javaMainMethods**

- `completion` -> `textDocument/completion`（`engine.rs:3940` 能力 `"completion"`）。
- **诊断**不走 `lsp.request`：JDT 的 `textDocument/publishDiagnostics` 被 Core 归一成
  `kind == "diagnostics"` 的事件（`engine.rs:4374`、`engine.rs:6528-6543`），
  经 `lsp.waitEvents` / `lsp.pollEvents` 出来。
- `javaEntrypoints` / `javaTestItems` / `javaMainMethods` 是 **`workspace/executeCommand`** 的可移植封装
  （`engine.rs:3926-3930`），分别调 Java Debug Server 的 `vscode.java.resolveMainClass` /
  Java Test 的 `vscode.java.test.findTestTypesAndMethods` / `vscode.java.resolveMainMethod`
  （契约 `:1343-1356`）。
- **没有 `textDocument/documentSymbol`**（`Select-String 'textDocument/documentSymbol'` 在 `engine.rs` 无命中）
  → 「大纲视图」要么走 `java.structure`，要么走 `workspace.searchEverywhere`。

---

## 6. 分步实现计划

每步 = **可独立交付** + 明确的验证方式 + 前置条件。遵守 `gpui/HANDOFF.md:37-70` 的纪律
（cargo 只跑改动范围、构建输出重定向到日志再 grep、不要用 PowerShell 管道接原生进程、
同一时刻只有一个代码写者、诊断走 `eprintln!`、验证要到交互级）。

### M1 — Java 补全（最高优先级，当天可见）

**交付**：在 `.java` 文件里敲字弹出补全菜单，方向键选择，Enter/Tab 落字；snippet 占位符被展开成纯文本。

**复用（不许自研）**：
- 上游 `gpui-base` 0.6.6 的补全子系统：`Lsp.completion_provider`
  （`gpui-base-0.6.6/src/input/editor/lsp/mod.rs:41`）、`CompletionProvider` trait
  （`lsp/completions.rs:40-103`）、菜单渲染与键盘交互
  （`gpui-component-0.6.6/src/input/popovers/completion_menu.rs`、`input/overlay.rs:188-197`）。
  触发是**自动的**：每次按键都会调 `handle_completion_trigger`
  （`gpui-base-0.6.6/src/input/editor/mod.rs:93-101`），**不需要新键位**。
  上游自己就有一个可抄的接线样例：`gpui-component-0.6.6/src/inspector.rs:104-107`
  （`state.lsp_mut().completion_provider = Some(provider); cx.notify();`）。
- Core：`lsp.request{operation:"completion"}` + `lsp.waitEvents` + `lsp.plainSnippet`（如需）。

**自建（gpui 侧，一个新文件，无 Java 语法）**：
- `CompletionProvider` 实现：`offset`（**字节**）→ `(line, utf16Column)`；
  调 `lsp.request`；等 `requestCompleted`；把结果转成 `lsp_types::CompletionItem`。
  三套列口径的换算**必须复用已有函数**：`gpui/crates/editor/src/navigation.rs` 的
  `core_position` / `editor_position`（`navigation.rs:39-49` 有口径表）。
- **事件泵**：现在 `Session::request` 只在"发完请求后"等事件（`session.rs:302-359`），
  空闲时没有事件消费者。补全/诊断/导入进度要求**持续**收 `lsp.waitEvents`，
  而且要按 `operationId` 分派（`session.rs:334-336` 已有这个判据，抬成常驻循环即可）。

**验证**（Windows 上怎么做、看什么）：
1. 前置：`LITHE_JDTLS_JAVA=D:\ProgramData\java\openjdk-21\bin\java.exe`
   （本机 PATH 上的 `java` 是 1.8，见 §2.9），工作区放一个含 `pom.xml` 的工程。
2. `cd gpui; cargo build --bin Lithe *> ..\.artifacts\pN\build.log; Write-Output "exit=$LASTEXITCODE"`
   —— **不要用管道**（`HANDOFF.md:42-49`）。
3. 启动 `Lithe.exe`，打开一个 `.java`，敲 `Str`：
   - 界面证据：`gpui/capture-screenshot.ps1` 截图 + `.artifacts/ui-type.ps1` 注入按键；
   - 日志证据（**读 `.err`**，`HANDOFF.md:64-65`）：期望看到补全请求的诊断行
     （新增 `S1_JAVA_COMPLETION uri=… count=… first=…` 之类，与既有 `S1_JAVA_*` 同格式，
     `gpui/PLAN.md:990-1006`）。
4. 用真实注入走一次"上/下/Enter"，然后断言编辑器正文里真的出现了落地文本
   （读盘或在 PowerShell 侧核对保存后的文件内容）。
5. 可选：gpui 现有那条选定式端到端测试（`gpui/crates/java/src/service.rs:512` 的
   `LITHE_GPUI_JDTLS_SMOKE`）可以照抄一份补全版本。

**前置条件**：JDK 21；载荷就绪（已就绪）；`lsp.request` 的 `completion` 已被 Core 实现（已确认）。
**风险**：`lsp_types` 版本的 `CompletionItem` 与 Core 的 JSON 形状要对齐（Core 返回的是 LSP 原样结构，
`shared/contracts/rust-core-api.md:1336-1342`）——**这条未逐字段验证**，见 §8。

---

### M2 — Maven 项目模型接进 JDT（`mavenContext`）

**交付**：打开真实 Spring Boot（多模块 / 有生成源根 / 有 profile）工程时，
`src/main/java`、`src/test/java`、`target/generated-sources/**` 都能补全与跳转；
选中 profile 后 JDT 用同一套 classpath；JDT 用的本地仓库/镜像与命令行一致。

**复用**：Core `maven.scan`（`rust/lithe-core/src/project/maven.rs:792`）
+ Core `lsp.startServer{mavenContext}`（`engine.rs:965-1003`）+ m2e（载荷内）。

**自建**：`maven.scan` 调用 + context 组装（放在 `gpui/crates/java/` 里，与 `workspace.rs` 同层），
`session.rs` 的 `SessionSpec` 加 `maven_context: Option<Value>` 并在 `start` 的 JSON 里加一个字段。

**验证**：
1. 造一个真 Maven 工程（本机有 `mvn 3.6.3`）：根 `pom.xml` + 两个子模块 + 一个
   `target/generated-sources/annotations`（或 `<build><sourceDirectory>` 显式指定）。
2. 启动后看新诊断行里的 `sourcePaths` 条数与 `reactorPath`；
   期望 `S1_JAVA_MAVEN reactorPath=. modules=2 sourcePaths=N`。
3. **反证**：把 context 关掉再开一次，确认「生成源根里的类补不出来」这个 bad case 会复现
   （只有能区分 before/after 的验证才算过，`HANDOFF.md:66-67`）。
4. 听 `mavenProfileTask` 事件：在 PowerShell 侧核对 `%LOCALAPPDATA%\Lithe\cache\language-servers\jdtls\<key>`
   的复用（`reuse=true`）与 JDT 日志里的 `cacheDisposition`（`gpui/PLAN.md:967` 的既有验证手法）。

**前置条件**：M1 的事件泵（M2 的 profile 进度/就绪判定都要它）。
**不做**：不解析 `pom.xml`（那是 Core 的 `maven.scan`）、不自己算 classpath（那是 m2e）。

---

### M3 — Maven 工具窗（模块树 / 生命周期 / Profiles / 依赖树）

**交付**：右栏 Maven 从「未检测到 Maven 项目」变成真视图：
模块树（含 sourceRoots 折叠项）、双击跑 `compile`/`package`/`test`、输出进底部、
依赖树（懒加载）、profile 勾选、构建诊断映射回编辑器。

**复用**：`maven.scan` / `maven.launchPlan` / `maven.dependencyPlan` / `maven.dependencies` /
`maven.diagnostics` / `maven.testResults`；宿主只负责起进程与展示。
呈现规格照 Windows：`windows/tauri/src/features/maven/components/maven-pane.tsx`（32 976 B）、
`maven-run-pane.tsx`、`maven-node-context-menu.tsx`、`maven-source-root-rows.tsx`。

**自建**：`gpui/crates/maven/`（数据层：`scan` + 进程会话 + 事件）+ 右栏三个视图。
底部需要能显示构建输出（现在只有终端，`workspace.rs:961`）。

**验证**：真实注入点模块树的运行图标 → 底部出现 Maven 输出 → PowerShell 侧核对
`target/classes` 时间戳变化 / `mvn` 进程确实起过（`Get-Process`）；
构建失败时核对诊断是否落到编辑器 gutter（`maven.diagnostics` 的 `{path,line,column,severity,message}`）。

**前置条件**：M2（否则 JDT 与 Maven 对源根的理解不一致）；底部面板要能容纳"构建输出"这一类视图。

---

### M4 — Run：Maven / Spring Boot 一键启动

**交付**：Run 工具窗列出探测器生成的配置（含 `spring-boot.maven:demo`），
点运行把应用跑起来、输出进底部、失败给出可读原因；能停。

**复用**：Core `runConfig.generate` / `runConfig.resolve` / `runConfig.createLaunchPlan`
（`shared/contracts/rust-core-api.md:162-168`、`:1532-1690`）；
Spring Boot 的 goal 与参数由 Core 生成（`execution/configuration.rs:2004-2008` 的
`spring-boot:run` + `spring-boot.run.jvmArguments` / `.arguments` / `.main-class`）。
**不许自研**：main class 发现归 `javaEntrypoints`（`lsp.request`），
探针**禁止**出现 `is_main_method` / `containsJavaMainMethod` 之类的本地扫描器 ——
`scripts/verify-java-semantic-ownership.mjs:11-18` 会把它们判为违规并让校验失败。

**自建**：Run 视图 + 宿主进程启动（PTY 或普通子进程）+ 输出流 + 停止。

**验证**：真实注入点绿三角 → `Get-Process java` 出现 → 底部出现 Spring Boot banner 与
`Started DemoApplication` → 点停止后 `java` 进程消失（**并确认没有孤儿**，`HANDOFF.md:70`）。

**前置条件**：M2（classpath/源根一致）；宿主进程能力（gpui 已有终端，可复用其 PTY 经验）。

---

### M5 — Spring 补全 / 跳转 / 端点视图

**交付**：
- `application.properties` / `application.yml` 里补全 Spring 配置项，带类型、默认值与说明；
- `@Value("${...}")` 与配置项之间双向跳转；`@Autowired` / 构造注入跳到 bean 定义；
- Spring Endpoints 视图（列 `@RequestMapping` 路由，点开跳源码）。

**复用**：Core `spring.index`（`rust/lithe-core/src/languages/spring.rs:54`；
`metadataRepositories` 指 `~/.m2/repository` 读 `spring-configuration-metadata.json`，
`macos/.../SpringFeatureModel.swift:101-102`）+ **M1 的同一个补全菜单**（再加一路数据源）
+ 跳转语义照 `windows/tauri/src/features/spring/utils/spring-navigation.ts:44-133`。

**自建**：一个 Spring 索引的调度器（防抖 + `textOverrides` 传未保存正文，
照 `windows/tauri/src/features/spring/hooks/use-spring-index.ts:42-100` 的 300 ms 防抖与取消语义）
+ 端点视图。

**验证**：在 `.properties` 里敲 `spring.datasource.` 看到补全项与默认值；
把某个 key 改错确认 `spring.index` 的 diagnostics 报出来
（`rust/lithe-core/src/languages/spring.rs` 的 `SpringDiagnosticResponse`）；
`@Value` 上 F12 跳到配置项行。

**前置条件**：M1（同一个补全 provider 骨架）；`~/.m2/repository` 里有 Spring Boot 依赖
（`refreshDependencyMetadata` 首次扫描会给 60 s 超时，`use-spring-index.ts:65`）。

---

## 7. 「不要自己造」清单

这些**必须留给上游引擎**；自研会同时违反 `develop-lithe` 的复用规则和仓库已有的守卫脚本。
（`scripts/verify-java-semantic-ownership.mjs:11-18` 把 `is_main_method`、`main_declaration_index`、
`main_class_exists`、`containsJavaMainMethod`、`java.runConfigurations`、`java.testMethods`
列为**禁止出现的 token**，扫描 `rust/lithe-core/src`、`macos/Sources`、`windows/tauri/src`。）

| 不许造 | 归属 | 证据 |
| --- | --- | --- |
| Java 符号表、类型解析、补全/诊断/重构 | **JDT LS 1.61.0** | 载荷 `org.eclipse.jdt.ls.core_1.61.0…jar`；`gpui/crates/java/src/lib.rs:8-13` |
| Maven 项目导入与 classpath | **m2e**（JDT LS 内） | `.artifacts/jdtls/config_win/config.ini:7`；`gpui/PLAN.md:1027-1030` |
| `pom.xml` 解析 / reactor / 模块 / sourceRoots / profile 清单 | **Core `maven.scan`** | `rust/lithe-core/src/project/maven.rs:734-826` |
| 源根 → JDT 的配置下发 | **Core `lsp.startServer{mavenContext}`** | `engine.rs:965-1003`；`maven.rs:241-287` |
| main class / 测试类发现 | **Java Debug Server / Java Test**（经 `lsp.request` 的 `javaEntrypoints` / `javaTestItems` / `javaMainMethods`） | 契约 `:1343-1356`；守卫脚本 `verify-java-semantic-ownership.mjs` |
| 运行配置的生成与合并 | **Core `runConfig.*`** | 契约 `:162-168` |
| Maven 命令行构造 | **Core `maven.launchPlan` / `dependencyPlan`** | `rust/lithe-core/src/project/maven.rs:147,159` |
| Surefire/JUnit 结果解析 | **Core `maven.testResults`** | `rust/lithe-core/src/project/maven_test_reports.rs`；契约 `:212-281` |
| 补全菜单（布局/键盘/滚动/snippet） | **上游 `gpui-base` / `gpui-component`** | `gpui-base-0.6.6/src/input/editor/lsp/completions.rs`、`gpui-component-0.6.6/src/input/popovers/completion_menu.rs` |
| LSP 进程/组帧/request id/文档版本/超时/诊断状态 | **Rust Core** | `shared/contracts/rust-core-api.md:1162-1220`；`gpui/crates/java/src/session.rs:1-6` |
| Spring 语义索引（配置项/bean/注入/端点） | **Core `spring.index`** | `rust/lithe-core/src/languages/spring.rs:54`；`HANDOFF.md:123-124` |
| JDT 工作区索引缓存与复用 | **Core 三条命令 + 平台只做观察** | 契约 `:1241-1301`；`gpui/crates/java/src/workspace.rs:5-13` |

**gpui 只允许做**：编排（有界生命周期、取消、陈旧结果防护）、资源预算、
跨平台契约的稳定化、呈现（视图/交互）、以及端到端工作流。

---

## 8. 未确认的点 + 下一步怎么确认

| # | 未确认 | 影响 | 怎么确认 |
| --- | --- | --- | --- |
| 1 | 载荷体积口径：`HANDOFF.md:90` / `PLAN.md:923` 写 61.7 MB，本次实测展开后 **76.1 MB** | 打包/更新器体积预算 | 找 61.7 MB 的来源（可能在某个验证脚本或 release notes 里）；或明确改成"整个载荷 76.1 MB、plugins 目录 X MB"并统一口径。命令：`Get-ChildItem .artifacts\jdtls -Recurse -File \| Measure-Object Length -Sum` |
| 2 | `spring-boot-language-server` 的**许可证逐包清单**与**载荷体积** | 决定 §4.2(a) 能不能落地 | 本机 `github.com` 不可达（`web_fetch` 报 non-public IP）。需在有网络的环境查 STS4 的公式化许可证与最新 JAR 体积；上游 issue [#1293](https://github.com/spring-projects/spring-tools/issues/1293)（JAR 部署）、[#1436](https://github.com/spring-projects/spring-tools/issues/1436)（Boot LS JAR size）是入口 |
| 3 | `spring-boot-language-server` 的**确切启动命令**（jar 路径 / stdin-stdout / socket、JDK 要求） | 决定它能不能作为第二个 provider 交给 Core 托管 | 同一份 STS4 wiki（本机不可达）；或在一台能访问上游的机器上装 `vscjava.vscode-spring-boot` 并读它的 `package.json` / 日志 |
| 4 | Core `lsp.request{operation:"completion"}` 的**响应 JSON 具体形状**（是否直接给 `lsp_types::CompletionItem` 的 camelCase 结构、`textEdit` / `insertTextFormat` 怎么给） | M1 的适配器要按这个写 | 读 `rust/lithe-core/src/lsp/interface/engine.rs` 里 completion 的归一化分支（本次只读了 operation 枚举与能力表，没读结果归一化）；或跑一次真实 JDT 抓 `requestCompleted` 事件 |
| 5 | Core 的 `lsp.request` 是否支持**多个语义请求并发**（补全 + 诊断 + gutter marker 同时） | M1 的事件泵设计 | 读 `engine.rs` 的 pending 表与按 `operationId` 分派逻辑；已有 `java.navigationMarkers` 的批量机制（`engine.rs:3965-3974` 的 `JavaNavigationMarkerBatch`）可作参考 |
| 6 | gpui 的补全菜单在**深/浅主题**与 125% DPI 下的实际表现 | 用户体验（维护者的第一口径） | 真机截图（`gpui/capture-screenshot.ps1`，注意物理像素 ×1.25，`HANDOFF.md:74`） |
| 7 | `.artifacts/jdtls` 里那份 `manifest.json` 与 `third_party/jdtls/manifest.json` 是否**永远**同源 | 升级流程 | 读 `scripts/prepare-jdtls.ps1` 里拷贝 manifest 的那几行（本次只读了它与哈希/license 相关的行） |
| 8 | `spring.index` 的**增量/性能**表现（大工程、依赖元数据首次扫描） | M5 的调度策略 | 在真实 Spring Boot 工程上测一次 `refreshDependencyMetadata=true` 的耗时（Windows 侧首次给 60 s 超时，`use-spring-index.ts:65`） |
| 9 | 本机是否能在 `.artifacts/jdk` 准备一份捆绑 JDK 21 | 验证链路是否还需要 `LITHE_JDTLS_JAVA` | 跑 `scripts/prepare-jdk.ps1`（**本次未跑**，属写操作）；或继续用 `D:\ProgramData\java\openjdk-21` |
| 10 | gpui 有没有可复用的**PTY/子进程宿主**给 M4 用 | M4 工作量 | 读 `gpui/crates/terminal/` 的进程与 PTY 实现（本次只确认了底部面板有 `BottomPaneKind::Terminal`，没读终端 crate） |

---

## 附：本文引用的关键坐标速查

```
载荷
  third_party/jdtls/manifest.json:2,5,7,12,19,27     版本/许可证/lombok/java-debug/java-test/最低 Java 21
  third_party/jdk/manifest.json:2,6,11,16,21         Temurin 21 四平台
  .artifacts/jdtls/config_win/config.ini:7           osgi.bundles（106 个，含 5 个 m2e，0 个 spring）
  .artifacts/jdtls/bin/jdtls.ps1:2,6-9,21            wrapper 启动方式
  .artifacts/jdtls/features/                         只有 1 个 equinox executable jar

gpui
  gpui/crates/java/src/lib.rs:8-13                   边界声明（无 LSP 协议、无 Java 语法）
  gpui/crates/java/src/jdtls.rs:34                   include_str! third_party/jdtls/manifest.json
  gpui/crates/java/src/jdtls.rs:74,426-460           minimumJavaVersion=21 + JDK 发现顺序
  gpui/crates/java/src/workspace.rs:33,130-198       指纹输入（pom.xml/build.gradle + 直接 Maven 模块）
  gpui/crates/java/src/session.rs:113-158            lsp.startServer 的完整请求（无 mavenContext）
  gpui/crates/java/src/service.rs:122-176            只用 definition
  gpui/crates/editor/src/navigation.rs:39-49         三套列口径表
  gpui/crates/workbench/src/right_tool_window.rs:136-155  Maven 空态
  gpui/crates/workbench/src/workspace.rs:961         底部只有 Terminal
  gpui/PLAN.md:842-852,925-1038                      阶段 10 两批的规格与边界
  gpui/HANDOFF.md:13-20,37-70,90-91,118-124          优先级/纪律/环境事实/队列

Core
  rust/lithe-core/src/lsp/interface/engine.rs:273-321        LspSemanticOperation 全清单
  rust/lithe-core/src/lsp/interface/engine.rs:965-1003       mavenContext → JdtMavenConfiguration
  rust/lithe-core/src/lsp/languages/jdt.rs:94-109            JdtMavenConfiguration 字段
  rust/lithe-core/src/project/maven.rs:63-78                  MavenLaunchContextRequest v1
  rust/lithe-core/src/project/maven.rs:241-287                jdt_configuration（源根归一化）
  rust/lithe-core/src/project/maven.rs:792-826                maven.scan
  rust/lithe-core/src/languages/spring.rs:33-51,54            spring.index
  rust/lithe-core/src/execution/configuration.rs:2004-2008    spring-boot:run
  rust/lithe-core/src/execution/detectors/maven.rs:17         spring-boot-maven-plugin → spring-boot.maven
  scripts/verify-java-semantic-ownership.mjs:11-18            禁止本地 main/test 扫描器的守卫

契约
  shared/contracts/rust-core-api.md:79-211                    Commands 表
  shared/contracts/rust-core-api.md:1162-1195                 LSP 运行时边界 + mavenContext 语义
  shared/contracts/rust-core-api.md:1336-1356                 lsp.request 支持的 operation
  shared/contracts/rust-core-api.md:1450-1473                 maven.scan
  shared/contracts/rust-core-api.md:1532-1546                 runConfig.generate

上游（cargo registry，非仓库）
  D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-base-0.6.6\src\input\editor\lsp\
  D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-component-0.6.6\src\input\popovers\completion_menu.rs
  D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-component-0.6.6\src\inspector.rs:104-107
```
