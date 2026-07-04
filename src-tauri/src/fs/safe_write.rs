//! 生成ファイルの書き込み。
//! `protected` が付いたファイル（実装部）は既存なら上書きしない（要求仕様 ※1 / F-5）。

use std::path::Path;

use serde::Serialize;

use crate::codegen::GeneratedFile;

#[derive(Debug, Default, Serialize)]
pub struct WriteReport {
    /// 書き込んだファイル（ワークスペースからの相対パス）
    pub written: Vec<String>,
    /// 保護により書き込みをスキップしたファイル
    pub skipped: Vec<String>,
}

pub fn write_files(root: &Path, files: &[GeneratedFile]) -> Result<WriteReport, String> {
    let mut report = WriteReport::default();
    for file in files {
        let path = root.join(&file.rel_path);
        let rel = file.rel_path.to_string_lossy().to_string();
        if file.protected && path.exists() {
            report.skipped.push(rel);
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("ディレクトリ作成に失敗しました ({}): {e}", rel))?;
        }
        std::fs::write(&path, &file.content)
            .map_err(|e| format!("書き込みに失敗しました ({}): {e}", rel))?;
        report.written.push(rel);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("arcsyn_safe_write_test")
            .join(name);
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn file(rel: &str, content: &str, protected: bool) -> GeneratedFile {
        GeneratedFile {
            rel_path: PathBuf::from(rel),
            content: content.to_string(),
            protected,
        }
    }

    #[test]
    fn writes_new_files_with_parent_dirs() {
        let root = temp_root("new");
        let report = write_files(&root, &[file("a/b/impl.py", "v1", true)]).unwrap();
        assert_eq!(report.written, vec!["a/b/impl.py"]);
        assert_eq!(
            std::fs::read_to_string(root.join("a/b/impl.py")).unwrap(),
            "v1"
        );
    }

    #[test]
    fn protected_file_is_not_overwritten() {
        let root = temp_root("protected");
        write_files(&root, &[file("impl.py", "user code", true)]).unwrap();
        let report = write_files(&root, &[file("impl.py", "regenerated", true)]).unwrap();
        assert_eq!(report.skipped, vec!["impl.py"]);
        assert!(report.written.is_empty());
        assert_eq!(
            std::fs::read_to_string(root.join("impl.py")).unwrap(),
            "user code"
        );
    }

    #[test]
    fn unprotected_file_is_regenerated() {
        let root = temp_root("regen");
        write_files(&root, &[file("interfaces.py", "v1", false)]).unwrap();
        let report = write_files(&root, &[file("interfaces.py", "v2", false)]).unwrap();
        assert_eq!(report.written, vec!["interfaces.py"]);
        assert_eq!(
            std::fs::read_to_string(root.join("interfaces.py")).unwrap(),
            "v2"
        );
    }
}
