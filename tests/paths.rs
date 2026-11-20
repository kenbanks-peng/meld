use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

fn command(home: &Path, cwd: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_meld"));
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .current_dir(cwd)
        .arg("paths");
    cmd
}

fn paths(cmd: &mut Command) -> (PathBuf, PathBuf) {
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    let source = PathBuf::from(json["source_dir"].as_str().unwrap());
    let output = PathBuf::from(json["output_dir"].as_str().unwrap());
    assert!(source.is_absolute() && source.is_dir());
    assert!(output.is_absolute() && output.is_dir());
    assert_eq!(source, output.join("source"));
    (source, output)
}

fn fails(out: Output, message: &str) {
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(message),
        "{out:?}"
    );
}

#[test]
fn creates_stable_global_paths_and_renders_without_project_files() {
    let home = TempDir::new().unwrap();
    let project = tempfile::Builder::new()
        .prefix("project-")
        .tempdir()
        .unwrap();
    let (source, output) = paths(&mut command(home.path(), project.path()));
    let canonical = fs::canonicalize(project.path()).unwrap();
    let hash = format!(
        "{:x}",
        Sha256::digest(canonical.as_os_str().as_encoded_bytes())
    );
    let expected = fs::canonicalize(home.path())
        .unwrap()
        .join("data/meld")
        .join(format!(
            "{}-{}",
            canonical.file_name().unwrap().to_str().unwrap(),
            &hash[..12]
        ));
    assert_eq!(output, expected);
    fs::write(source.join("report.meld"), "# Global report").unwrap();
    assert_eq!(
        paths(
            command(home.path(), project.path())
                .arg("--project")
                .arg(".")
        ),
        (source.clone(), output.clone())
    );
    assert!(source.join("report.meld").exists());

    let out = Command::new(env!("CARGO_BIN_EXE_meld"))
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .arg(source.join("report.meld"))
        .arg("--output")
        .arg(output.join("report.html"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(
        fs::read_to_string(output.join("report.html"))
            .unwrap()
            .contains("Global report")
    );
    assert_eq!(fs::read_dir(project.path()).unwrap().count(), 0);
}

#[test]
fn explicit_project_and_same_names_are_distinct() {
    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    for name in ["a/report", "b/report"] {
        fs::create_dir_all(work.path().join(name)).unwrap();
    }
    let (_, a) = paths(command(home.path(), work.path()).args(["--project", "a/report"]));
    let (_, b) = paths(
        command(home.path(), work.path())
            .arg("--project")
            .arg(work.path().join("b/report")),
    );
    assert_ne!(a, b);
    assert!(
        a.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("report-")
    );
    assert!(
        b.file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("report-")
    );
    assert_eq!(
        fs::read_dir(work.path().join("a/report")).unwrap().count(),
        0
    );
}

#[test]
fn falls_back_for_unset_empty_and_relative_data_home() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    let expected = fs::canonicalize(home.path())
        .unwrap()
        .join(".local/share/meld");
    for value in [None, Some(""), Some("relative")] {
        let mut cmd = command(home.path(), project.path());
        if let Some(value) = value {
            cmd.env("XDG_DATA_HOME", value);
        } else {
            cmd.env_remove("XDG_DATA_HOME");
        }
        let (_, output) = paths(&mut cmd);
        assert_eq!(output.parent().unwrap(), expected);
    }
    assert_eq!(fs::read_dir(project.path()).unwrap().count(), 0);
}

#[test]
fn rejects_invalid_project_and_blocked_storage() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    fails(
        command(home.path(), project.path())
            .args(["--project", "missing"])
            .output()
            .unwrap(),
        "resolving project",
    );
    fs::write(project.path().join("file"), "").unwrap();
    fails(
        command(home.path(), project.path())
            .args(["--project", "file"])
            .output()
            .unwrap(),
        "project must be a directory",
    );
    fs::write(home.path().join("data"), "").unwrap();
    fails(
        command(home.path(), project.path()).output().unwrap(),
        "creating",
    );
}

#[test]
fn requires_home_only_when_data_home_is_unusable() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    paths(command(home.path(), project.path()).env_remove("HOME"));
    fails(
        command(home.path(), project.path())
            .env_remove("HOME")
            .env_remove("XDG_DATA_HOME")
            .output()
            .unwrap(),
        "HOME is not set",
    );
}

#[cfg(unix)]
#[test]
fn symlink_aliases_share_storage_and_names_are_sanitized() {
    let home = TempDir::new().unwrap();
    let work = TempDir::new().unwrap();
    let project = work.path().join("project with spaces");
    fs::create_dir(&project).unwrap();
    let alias = work.path().join("alias");
    std::os::unix::fs::symlink(&project, &alias).unwrap();
    let direct = paths(
        command(home.path(), work.path())
            .arg("--project")
            .arg(&project),
    );
    let linked = paths(
        command(home.path(), work.path())
            .arg("--project")
            .arg(&alias),
    );
    assert_eq!(direct, linked);
    assert!(
        direct
            .1
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("project-with-spaces-")
    );
}

#[test]
fn rejects_render_options_and_documents_paths() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();
    assert!(
        !command(home.path(), project.path())
            .args(["--output", "x"])
            .output()
            .unwrap()
            .status
            .success()
    );
    let out = command(home.path(), project.path())
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let help = String::from_utf8(out.stdout).unwrap();
    assert!(help.contains("--project"));
    assert!(help.contains("JSON"));
}
