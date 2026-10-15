//! 符号级语义动作：转到引用、重命名符号、格式化文档。
//!
//! 与补全 / 悬停 / 代码动作同一条分工：**取数**在这里（异步、带代次防过期），
//! **改文本**走组件库的 `CodeEditor::apply_edits`（整批一次撤销步、只读拒绝），
//! **落点**复用 `EditorPane` 自己的打开与定位。
//!
//! 三者的共同点是「结果不是一条位置」：引用是一组位置，重命名与格式化是一批编辑。
//! 所以这里只把服务端回来的形状翻成宿主认识的值，不做 UI、不碰 gpui 实体。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use lithe_gpui_java::{JavaLanguageService, JavaTarget, JavaTextEdit, JavaWorkspaceEdits};

use crate::navigation::core_position;

/// 「转到引用」取回来的一个位置：文件 + 行 + UTF-16 列（LSP 口径）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reference {
    pub(crate) path: PathBuf,
    pub(crate) line: u32,
    pub(crate) utf16_column: u32,
}

impl Reference {
    /// 菜单行文案：相对工作区的路径 + **1 基**行列（给人看的数，服务端与内部一律 0 基）。
    ///
    /// 相对路径优先：同一个工程里 `src/main/java/A.java` 比一整条绝对路径好认。
    pub(crate) fn label(&self, workspace_root: Option<&Path>) -> String {
        let display = workspace_root
            .and_then(|root| self.path.strip_prefix(root).ok())
            .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| self.path.to_string_lossy().replace('\\', "/"));
        format!("{display}:{}:{}", self.line + 1, self.utf16_column + 1)
    }
}

/// 转到引用：返回 `None` 表示这次请求已被更晚的一次顶掉。
pub(crate) async fn fetch_references(
    service: Arc<JavaLanguageService>,
    file_path: PathBuf,
    text: String,
    offset: usize,
    generation: Arc<AtomicU64>,
    mine: u64,
) -> Option<Vec<Reference>> {
    let started = Instant::now();
    let (line, utf16_column) = core_position(&text, offset);
    let file = file_path.display().to_string();

    let targets = match service.references(&file_path, &text, line as u32, utf16_column as u32) {
        Ok(targets) => targets,
        Err(error) => {
            println!("S1_EDITOR_REFERENCES file={file} count=0 result=failed error={error}");
            return Some(Vec::new());
        }
    };

    if generation.load(Ordering::SeqCst) != mine {
        println!("S1_EDITOR_REFERENCES file={file} result=stale reason=superseded");
        return None;
    }

    // 虚拟文档（反编译类）这一侧没有宿主落点：计数不等于展示，宁可不列出来。
    let mut virtual_targets = 0usize;
    let references: Vec<Reference> = targets
        .into_iter()
        .filter_map(|target| match target {
            JavaTarget::File {
                path,
                line,
                utf16_column,
            } => Some(Reference {
                path,
                line,
                utf16_column,
            }),
            JavaTarget::Virtual { .. } => {
                virtual_targets += 1;
                None
            }
        })
        .collect();

    println!(
        "S1_EDITOR_REFERENCES file={file} count={} virtual={virtual_targets} ms={}",
        references.len(),
        started.elapsed().as_millis()
    );
    Some(references)
}

/// 重命名符号：把新名字发给服务端，取回一批编辑（可能横跨多个文件）。
pub(crate) async fn fetch_rename(
    service: Arc<JavaLanguageService>,
    file_path: PathBuf,
    text: String,
    offset: usize,
    new_name: String,
    generation: Arc<AtomicU64>,
    mine: u64,
) -> Option<JavaWorkspaceEdits> {
    let started = Instant::now();
    let (line, utf16_column) = core_position(&text, offset);
    let file = file_path.display().to_string();

    let edits = match service.rename(
        &file_path,
        &text,
        line as u32,
        utf16_column as u32,
        &new_name,
    ) {
        Ok(edits) => edits,
        Err(error) => {
            println!("S1_EDITOR_RENAME file={file} result=failed error={error}");
            return Some(JavaWorkspaceEdits::default());
        }
    };

    if generation.load(Ordering::SeqCst) != mine {
        println!("S1_EDITOR_RENAME file={file} result=stale reason=superseded");
        return None;
    }

    println!(
        "S1_EDITOR_RENAME file={file} newName={new_name:?} edits={} other_files={} ms={}",
        edits.edits.len(),
        edits.other_file_edits,
        started.elapsed().as_millis()
    );
    Some(edits)
}

/// 格式化整篇文档：取回一批编辑，由调用方整批落进缓冲区。
pub(crate) async fn fetch_formatting(
    service: Arc<JavaLanguageService>,
    file_path: PathBuf,
    text: String,
    generation: Arc<AtomicU64>,
    mine: u64,
) -> Option<Vec<JavaTextEdit>> {
    let started = Instant::now();
    let file = file_path.display().to_string();

    let edits = match service.formatting(&file_path, &text) {
        Ok(edits) => edits,
        Err(error) => {
            println!("S1_EDITOR_FORMAT file={file} result=failed error={error}");
            return Some(Vec::new());
        }
    };

    if generation.load(Ordering::SeqCst) != mine {
        println!("S1_EDITOR_FORMAT file={file} result=stale reason=superseded");
        return None;
    }

    println!(
        "S1_EDITOR_FORMAT file={file} edits={} ms={}",
        edits.len(),
        started.elapsed().as_millis()
    );
    Some(edits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_prefer_the_workspace_relative_path_and_one_based_positions() {
        let root = PathBuf::from("D:/proj");
        let reference = Reference {
            path: PathBuf::from("D:/proj/src/main/java/App.java"),
            line: 11,
            utf16_column: 4,
        };
        assert_eq!(reference.label(Some(&root)), "src/main/java/App.java:12:5");
    }

    #[test]
    fn a_reference_outside_the_workspace_keeps_its_full_path() {
        let root = PathBuf::from("D:/proj");
        let reference = Reference {
            path: PathBuf::from("D:/jdk/src/java.base/List.java"),
            line: 0,
            utf16_column: 0,
        };
        assert_eq!(reference.label(Some(&root)), "D:/jdk/src/java.base/List.java:1:1");
    }

    #[test]
    fn labels_without_a_workspace_root_still_read_as_a_path() {
        let reference = Reference {
            path: PathBuf::from("D:/proj/App.java"),
            line: 2,
            utf16_column: 9,
        };
        assert_eq!(reference.label(None), "D:/proj/App.java:3:10");
    }
}
