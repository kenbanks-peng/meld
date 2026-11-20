use anyhow::{Context, Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
pub struct ProjectPaths {
    pub source_dir: PathBuf,
    pub output_dir: PathBuf,
}

impl ProjectPaths {
    pub fn create(project: &Path) -> Result<Self> {
        let project = fs::canonicalize(project)
            .with_context(|| format!("resolving project {}", project.display()))?;
        ensure!(project.is_dir(), "project must be a directory");
        let root = match env::var_os("XDG_DATA_HOME").filter(|s| !s.is_empty()) {
            Some(path) if Path::new(&path).is_absolute() => PathBuf::from(path),
            _ => {
                let home = PathBuf::from(
                    env::var_os("HOME")
                        .filter(|s| !s.is_empty())
                        .context("HOME is not set")?,
                );
                ensure!(home.is_absolute(), "HOME must be an absolute path");
                home.join(".local/share")
            }
        };
        // Canonical paths make relative paths and symlink aliases share storage.
        let hash = format!(
            "{:x}",
            Sha256::digest(project.as_os_str().as_encoded_bytes())
        );
        let name = project
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| "project".into());
        let name: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .take(80)
            .collect();
        let output_dir = root.join("meld").join(format!("{name}-{}", &hash[..12]));
        let source_dir = output_dir.join("source");
        fs::create_dir_all(&source_dir)
            .with_context(|| format!("creating {}", source_dir.display()))?;
        // Resolve aliases in the data root too, so returned paths are absolute.
        let output_dir = fs::canonicalize(output_dir)?;
        Ok(Self {
            source_dir: output_dir.join("source"),
            output_dir,
        })
    }
}
