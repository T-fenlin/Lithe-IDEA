//! `spring.index` 的**输入清单**：从工作区根收集要交给 Core 索引的文件路径。
//!
//! ## 为什么这一步必须存在（这就是那个 bug）
//!
//! `spring.index` 的 `paths` 是 `#[serde(default)] Vec<String>`
//! （`rust/lithe-core/src/languages/spring.rs:36-51`），而 Core **只**对请求里给出的
//! 工作区相对路径做索引：Java 源码来自 `paths` 里扩展名为 `java` 的条目
//! （同文件 `:82-89` → `endpoint_index` `:1477-1552`），配置文档来自同一批路径
//! （`:93-111`）。不传 `paths` = 一个工作区文件都不扫，响应里只剩
//! `built_in_properties()`（`:324-386`）那批内置属性 —— 实测诊断行是
//! `S1_SPRING index=ok endpoints=0 beans=0 properties=10 values=0 refs=0 injections=0`，
//! 看起来"索引成功"，其实 `endpoints` 恒为 0。
//!
//! 所以"要扫哪些文件"是 gpui 侧**不可省略**的职责，Core 无法替我们猜：Core 拿不到
//! 工作区文件树（那是平台适配层的输入），也不该为了猜它去自己遍历工作区。
//!
//! ## 口径来源：Windows 参考实现（只读）
//!
//! 同一段逻辑在 Windows 产品里是
//! `windows/tauri/src/features/spring/utils/spring-index-paths.ts`：
//! 文件名判据 `isSpringIndexPath`（`:3-9`）+ `isSpringConfigurationPath`（`:11-17`），
//! 相对路径与排序 `workspaceRelativeSpringPath`（`:19-21`）+ `collectSpringIndexPaths`（`:23-31`）。
//! 本模块逐条对齐它，只补两件它靠前端文件树天然获得、而 gpui 侧必须自己做的事：
//!
//! 1. **排除构建产物目录** —— 参考实现的输入是已经过滤过的文件树
//!    （`use-spring-index.ts:46-50` 调 `getAllProjectFiles()`）；
//! 2. **有界遍历** —— gpui 侧直接在真实文件系统上递归，没有上限就会在 monorepo 里
//!    把打开面板的那一帧卡住。

use std::path::{Component, Path, PathBuf};

/// 单次索引最多交给 Core 的路径条数。
///
/// Core 会**逐个读文件内容**（`rust/lithe-core/src/languages/spring.rs:54-136` 的
/// `source_content`），而 `spring.index` 是在打开右工具窗的同一帧里同步发的
/// （`crate::workspace` 的懒扫路径）：几万个 Java 文件的工作区没有上限就是面板卡死。
/// 5000 是"够覆盖真实单模块/多模块工程、又不至于让单次索引超过秒级"的数量级；
/// 截断时 `S1_SPRING` 会打 `truncated=true`，所以"扫少了"永远是可核对的事实而不是猜测。
pub const MAX_SPRING_PATHS: usize = 5_000;

/// 单次收集最多读多少个目录项（含没有命中的文件与目录）。
///
/// 路径上限只约束"交出多少条"，不约束"花多久"：一个只有几十个 `.java`、但混着
/// 上百万依赖文件的目录（例如误把 `~/.m2/repository` 当工作区根打开）会在够到路径上限
/// **之前**就把遍历拖死。两个边界约束的是两件不同的事，必须分开设。
pub const MAX_SCANNED_ENTRIES: usize = 200_000;

/// 收集结论。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpringSourcePaths {
    /// 工作区相对路径、`/` 分隔、升序 —— Core 要求确定性输入，本层负责给出。
    pub paths: Vec<String>,
    /// 清单是否被截断（路径上限或扫描预算先到）。
    ///
    /// 为真表示"这不是完整工作区"，调用方应当把它打进诊断行而不是默默当成完整结果。
    pub truncated: bool,
}

/// 收集 `root` 下要交给 `spring.index` 的工作区相对路径（生产上限见两个常量）。
pub fn collect_spring_source_paths(root: &Path) -> SpringSourcePaths {
    collect_bounded(root, MAX_SPRING_PATHS, MAX_SCANNED_ENTRIES)
}

/// 带上限的收集本体：两个上限都显式传入，测试才能用"5 个文件 + 上限 2"验证截断，
/// 而不必造 5001 个文件（那会让这条单测变成秒级的磁盘压力测试）。
fn collect_bounded(root: &Path, max_paths: usize, max_entries: usize) -> SpringSourcePaths {
    let mut candidates: Vec<String> = Vec::new();
    let mut truncated = false;
    let mut scanned = 0usize;
    // 显式栈而不是递归：目录深度由工作区决定，深层目录树上的递归会爆栈，
    // 而爆栈发生在 UI 线程上就是整个应用消失。
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    'walk: while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            // 读不了的目录（权限、遍历中被删除）跳过而不是让整次收集失败：
            // 少几个文件是可接受的降级，而"一个子目录读不了就让 Spring 面板整块空掉"不是。
            continue;
        };
        let mut children: Vec<(PathBuf, bool, bool)> = Vec::new();
        for entry in entries {
            scanned += 1;
            if scanned > max_entries {
                // 预算是硬边界：到点即停，绝不为一个畸形工作区继续扫。
                truncated = true;
                break 'walk;
            }
            let Ok(entry) = entry else { continue };
            // `DirEntry::file_type()` **不跟随软链**（std 文档口径）：指向祖先目录的软链
            // 因此既不是 `is_dir` 也不是 `is_file`，会被下面的判断自然剔掉，
            // 遍历不可能成环。跟随软链要么死循环，要么把 `~/.m2` 这类库外目录吸进来。
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            children.push((entry.path(), kind.is_dir(), kind.is_file()));
        }

        // 每层按路径排序，文件按**升序**当场处理、子目录按升序下探。
        // `read_dir` 的顺序由文件系统决定（NTFS / APFS / ext4 各不相同），不排序的话
        // 同一个工作区在不同机器上会给出不同的清单；更关键的是**截断发生时**
        // "留下哪 N 条"必须与文件系统无关，否则 Core 的输入不可复现。
        children.sort_by(|left, right| left.0.cmp(&right.0));
        // 子目录单独收集、**逆序**压栈：栈是 LIFO，逆序压才能升序出，
        // 于是"先本层文件、再按名字升序下探子目录"这个遍历序是确定的。
        let mut directories: Vec<PathBuf> = Vec::new();
        for (child, is_dir, is_file) in children {
            if is_dir {
                if !is_excluded_directory(&child) {
                    directories.push(child);
                }
                continue;
            }
            if !is_file {
                // 软链、设备文件、FIFO 等一律跳过（见上面 `file_type()` 的说明）。
                continue;
            }
            let Some(relative) = workspace_relative_path(root, &child) else {
                continue;
            };
            if !is_spring_index_path(&relative) {
                continue;
            }
            candidates.push(relative);
            if candidates.len() > max_paths {
                truncated = true;
                break 'walk;
            }
        }
        for directory in directories.into_iter().rev() {
            stack.push(directory);
        }
    }

    // 先排序、再截断：留下的永远是**已收集集合的排序序前 N 条**（遍历序只决定被截断时
    // 收集到了哪些，而它本身是确定的）。顺序即 Core 的输入，必须是确定的
    // —— 契约要求确定性输入输出（`shared/contracts/rust-core-api.md:1737-1745`）。
    candidates.sort();
    candidates.truncate(max_paths);
    SpringSourcePaths {
        paths: candidates,
        truncated,
    }
}

/// 该文件是否要交给 `spring.index`：`spring-index-paths.ts:3-17` 的逐条移植。
///
/// 参考实现先 `getBaseName(filePath).toLowerCase()`（`:4`），所以这里也只比较小写文件名：
/// Windows 上 `Application.JAVA` 与 `application.java` 是同一个文件。
fn is_spring_index_path(relative: &str) -> bool {
    let name = relative
        .rsplit('/')
        .next()
        .unwrap_or(relative)
        .to_ascii_lowercase();
    if name.ends_with(".java") {
        return true;
    }
    // 两份元数据 JSON（`:6-7`）：Core 用它们补 `properties` 的文档与默认值
    // （`rust/lithe-core/src/languages/spring.rs:210-216`，按**文件名**判据，与目录无关）。
    if name == "spring-configuration-metadata.json"
        || name == "additional-spring-configuration-metadata.json"
    {
        return true;
    }
    is_spring_configuration_file(&name)
}

/// `isSpringConfigurationPath`（`spring-index-paths.ts:11-17`）：`application.properties` /
/// `application.yml` / `application.yaml` 及其 `application-<profile>.*` 变体。
///
/// 这些是 Core 产出 `values` 的唯一来源（`rust/lithe-core/src/languages/spring.rs:93-111`），
/// 不收它们就等于右工具窗永远看不到配置值 —— 参考实现收，所以这里也收。
fn is_spring_configuration_file(name: &str) -> bool {
    if name == "application.properties" || name == "application.yml" || name == "application.yaml" {
        return true;
    }
    match name.strip_prefix("application-") {
        Some(profile) => {
            profile.ends_with(".properties")
                || profile.ends_with(".yml")
                || profile.ends_with(".yaml")
        }
        None => false,
    }
}

/// 构建产物 / 版本控制 / IDE 状态目录。
///
/// 依据是 Windows 参考的文件树忽略表
/// （`windows/tauri/src/features/file-system/controllers/utils.ts:40-108` 的 `IGNORE_PATTERNS`，
/// 这里取其中"版本控制 / 构建产物 / IDE 状态"三类）；参考实现之所以不用自己写这段，
/// 是因为它的输入来自已过滤的文件树。gpui 侧直接在真实文件系统上递归，必须自己排除：
///
/// - `target/`、`build/`、`out/` 里可能同时存在**代码生成产物**（同一份注解的另一份副本），
///   送进 Core 会让端点在面板里重复出现；
/// - 一个 Maven `target/` 或 `node_modules/` 常有上万条目录项，不排除就会把
///   [`MAX_SCANNED_ENTRIES`] 的预算吃光，真正的工作区源码反而被截断掉。
///
/// 按**目录名**、大小写不敏感地比较：Windows 上 `Target` 与 `target` 是同一个目录，
/// 只比小写才不会漏。代价是 Linux 上名为 `Build/` 的目录也会被跳过 ——
/// 与"面板卡死/端点重复"相比这个代价可以接受。比较用 `eq_ignore_ascii_case`
/// 而不是先 `to_lowercase()`：这条路径在每次遍历命中目录时都要跑，不该额外分配。
fn is_excluded_directory(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    EXCLUDED_DIRECTORIES
        .iter()
        .any(|excluded| name.eq_ignore_ascii_case(excluded))
}

/// 排除清单（值一律小写，比较见 [`is_excluded_directory`]）。
const EXCLUDED_DIRECTORIES: &[&str] = &[
    // 版本控制
    ".git",
    ".svn",
    ".hg",
    ".bzr",
    // 构建产物与依赖（Java/Maven/Gradle、Node、Python、Go、Rust 常见输出目录）
    "target",
    "build",
    "out",
    "dist",
    "bin",
    "obj",
    "node_modules",
    "vendor",
    "coverage",
    ".gradle",
    ".mvn",
    "__pycache__",
    ".venv",
    "venv",
    ".next",
    ".nuxt",
    ".cache",
    // IDE 状态
    ".idea",
    ".vscode",
    ".vs",
];

/// 工作区相对路径、`/` 分隔（契约要求的标识符口径）。
///
/// 非 UTF-8 文件名返回 `None`（= 跳过）：`paths` 是 JSON 字符串，有损转换后 Core 按该
/// 相对路径打开文件必然失败（`source_content`），送一条注定失败、又可能碰巧命中别的文件的
/// 假路径比少收一条更糟。
fn workspace_relative_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut text = String::new();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            continue;
        };
        let part = part.to_str()?;
        if !text.is_empty() {
            text.push('/');
        }
        text.push_str(part);
    }
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 级联临时目录：断言失败也会在 `Drop` 里删掉，不留工作区残骸。
    ///
    /// 名字用"进程号 + 进程内自增序号"而不是时间戳：序号已经能保证并发测试之间不撞名，
    /// 而时钟会让"同一进程重复跑"与"跨进程跑"产生不同的目录名，排查时反而更难对。
    struct TempTree(PathBuf);

    static NEXT_TEMP_TREE: AtomicU64 = AtomicU64::new(0);

    impl TempTree {
        fn new(label: &str) -> Self {
            let sequence = NEXT_TEMP_TREE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "lithe-gpui-spring-paths-{}-{sequence}-{label}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("临时工作区根");
            Self(root)
        }

        fn root(&self) -> &Path {
            &self.0
        }

        /// 写一个相对路径的文件（自动创建父目录）。
        fn write(&self, relative: &str, content: &str) {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("临时工作区父目录");
            }
            std::fs::write(&path, content).expect("临时工作区文件");
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// 文件名判据：对齐参考实现 `spring-index-paths.ts:3-17`。
    ///
    /// 两个方向都要守：该收的（`.java`、两份元数据 JSON、`application*` 配置）一条不漏，
    /// 不该收的（`.java.bak`、`application.json`、`Demo.kt`）一条不进 ——
    /// 多收会让 Core 白读文件，少收就是端点/配置值缺一截。
    #[test]
    fn spring_index_path_names_match_the_reference() {
        for accepted in [
            "Demo.java",
            "a/b/APPLICATION.JAVA",
            "application.properties",
            "application.yml",
            "application.yaml",
            "application-prod.properties",
            "application-prod.yml",
            "application-prod.yaml",
            "META-INF/spring-configuration-metadata.json",
            "META-INF/additional-spring-configuration-metadata.json",
        ] {
            assert!(is_spring_index_path(accepted), "{accepted} 应当被收集");
        }

        for rejected in [
            "Demo.java.bak",
            "Demo.kt",
            "application.json",
            "application-prod.txt",
            "spring-configuration-metadata.json.bak",
            "spring-metadata.json",
            "index.html",
        ] {
            assert!(!is_spring_index_path(rejected), "{rejected} 不应当被收集");
        }
    }

    /// 排除判据只看**目录名**且大小写不敏感：
    /// `Target`/`target` 都要排除，而名字里含 `target` 的源码目录（`targets`）不能误伤。
    #[test]
    fn exclusion_matches_directory_names_case_insensitively() {
        for excluded in [
            "target",
            "Target",
            "TARGET",
            "build",
            "out",
            ".git",
            "node_modules",
        ] {
            assert!(
                is_excluded_directory(Path::new(excluded)),
                "{excluded} 应当被排除"
            );
        }
        for kept in [
            "src",
            "main",
            "java",
            "targets",
            "node_modules_",
            "resource",
        ] {
            assert!(
                !is_excluded_directory(Path::new(kept)),
                "{kept} 不是构建产物目录，不该被排除"
            );
        }
    }

    /// 合规文件按"相对路径 + `/` 分隔 + 升序"输出，不合格的文件与目录一个不进。
    #[test]
    fn collects_relative_sorted_paths_and_skips_unrelated_files() {
        let tree = TempTree::new("extensions");
        tree.write(
            "src/main/java/demo/Application.java",
            "class Application {}\n",
        );
        tree.write(
            "src/main/resources/application.properties",
            "server.port=8080\n",
        );
        tree.write(
            "src/main/resources/application-prod.yml",
            "server:\n  port: 9090\n",
        );
        tree.write(
            "src/main/resources/META-INF/spring-configuration-metadata.json",
            "{}\n",
        );
        tree.write("notes.md", "# 不是 Spring 事实\n");
        tree.write(
            "src/main/java/demo/Application.java.bak",
            "class Application {}\n",
        );

        let collected = collect_spring_source_paths(tree.root());

        assert_eq!(
            collected.paths,
            vec![
                "src/main/java/demo/Application.java".to_string(),
                "src/main/resources/META-INF/spring-configuration-metadata.json".to_string(),
                "src/main/resources/application-prod.yml".to_string(),
                "src/main/resources/application.properties".to_string(),
            ],
            "升序 + 工作区相对路径 + `/` 分隔"
        );
        assert!(
            collected.paths.iter().all(|path| !path.contains('\\')),
            "必须是 `/` 分隔"
        );
        assert!(!collected.truncated, "没有到上限就不该报截断");
    }

    /// 构建产物与 VCS 目录里的 `.java` 一个都不许进清单。
    ///
    /// 这条守的是"面板慢/端点重复"两个真实故障：`target/generated-sources` 里的生成代码
    /// 会变成重复端点，而遍历 `node_modules`/`target` 会吃掉扫描预算。
    #[test]
    fn build_artifact_and_vcs_directories_are_not_scanned() {
        let tree = TempTree::new("excluded");
        tree.write("src/main/java/demo/Real.java", "class Real {}\n");
        for directory in ["target", "build", "out", ".git", "node_modules"] {
            tree.write(
                &format!("{directory}/generated/Generated.java"),
                "class Generated {}\n",
            );
            tree.write(
                &format!("{directory}/application.properties"),
                "generated=true\n",
            );
        }

        let collected = collect_spring_source_paths(tree.root());

        assert_eq!(
            collected.paths,
            vec!["src/main/java/demo/Real.java".to_string()],
            "被排除目录里的源码与配置都不该出现"
        );
    }

    /// 路径上限：截断到**排序序**的前 N 条，并置 `truncated`；没到上限就不置。
    #[test]
    fn the_path_cap_truncates_deterministically_and_reports_it() {
        let tree = TempTree::new("cap");
        for index in 0..5 {
            tree.write(&format!("src/File{index}.java"), "class Generated {}\n");
        }

        let capped = collect_bounded(tree.root(), 2, MAX_SCANNED_ENTRIES);
        assert_eq!(
            capped.paths,
            vec!["src/File0.java".to_string(), "src/File1.java".to_string()],
            "截断点必须落在排序序上，否则 Core 的输入不可复现"
        );
        assert!(capped.truncated, "截断了就要如实上报");

        let complete = collect_bounded(tree.root(), 5, MAX_SCANNED_ENTRIES);
        assert_eq!(complete.paths.len(), 5, "命中数等于上限时不算截断");
        assert!(!complete.truncated);
    }

    /// 扫描预算与路径上限是两件事：命中 0 条也能因为"目录项太多"而截断并停止。
    #[test]
    fn the_scan_budget_stops_a_tree_without_any_hits() {
        let tree = TempTree::new("budget");
        for index in 0..5 {
            tree.write(&format!("assets/asset-{index}.bin"), "x\n");
        }

        let collected = collect_bounded(tree.root(), MAX_SPRING_PATHS, 2);

        assert!(collected.paths.is_empty(), "没有合规文件就应当是空清单");
        assert!(collected.truncated, "预算用尽代表清单不完整，必须如实上报");
    }

    /// 空工作区与非存在根都给空清单且**不报截断**：这是 `spring::index` 仍然发请求、
    /// 视图仍然是"非 null 即有权重"的前提（面板靠内置属性显示非空态）。
    #[test]
    fn an_empty_or_missing_workspace_yields_no_paths() {
        let tree = TempTree::new("empty");
        tree.write("notes.md", "no java here\n");

        let empty = collect_spring_source_paths(tree.root());
        assert!(empty.paths.is_empty());
        assert!(!empty.truncated, "没有命中不等于被截断");

        let missing = tree.root().join("does-not-exist");
        let absent = collect_spring_source_paths(&missing);
        assert!(
            absent.paths.is_empty(),
            "不存在的根不该 panic，也不该编出路径"
        );
        assert!(!absent.truncated);
    }
}
