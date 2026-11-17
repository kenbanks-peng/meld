use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::TempDir;

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_meld"))
        .args(args)
        .output()
        .unwrap()
}
fn success(args: &[&str]) -> String {
    let out = cli(args);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}
fn render(source: &str) -> String {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("test.meld");
    fs::write(&input, source).unwrap();
    success(&[input.to_str().unwrap(), "-o", "-"])
}
fn failure(source: &str, message: &str) {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("test.meld");
    fs::write(&input, source).unwrap();
    let out = cli(&[input.to_str().unwrap(), "-o", "-"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains(message),
        "{:?}",
        out
    );
}

#[test]
fn no_arguments_prints_help_successfully() {
    let out = cli(&[]);
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    assert_eq!(String::from_utf8(out.stdout).unwrap(), success(&["--help"]));
}

#[test]
fn cli_uses_meld_branding() {
    assert_eq!(
        success(&["--version"]),
        concat!("meld ", env!("CARGO_PKG_VERSION"), "\n")
    );
    let help = success(&["--help"]);
    assert!(help.contains("Mark Elements Down"));
    assert!(help.contains("Usage: meld "));
    assert!(help.contains("Input .meld file"));
}

#[test]
fn options_without_input_still_fail() {
    assert!(!cli(&["--title", "Hello"]).status.success());
}

#[test]
fn demo_has_all_components_and_chart() {
    let html = success(&[
        "demo.meld",
        "-o",
        "-",
        "--title",
        "Engineering pulse · Q3 2026",
    ]);
    for expected in [
        "Engineering pulse.",
        "class=\"grid cols-4\"",
        "class=\"metric tone-teal\"",
        "99.97<span class=\"metric-unit\">%</span>",
        "class=\"status warning\"",
        "<strong>API gateway</strong>",
        "<svg ",
        "viewBox=\"0 0 960 320\"",
        "Q3 2026, Jun / Sep: 86",
        "<details class=\"chart-data\">",
        "Scope &amp; methodology",
    ] {
        assert!(html.contains(expected), "missing {expected}");
    }
    assert_eq!(html.matches("<rect ").count(), 6);
    assert!(!html.contains("@status"));
    assert!(!html.contains("@panel"));
}

#[test]
fn nested_markdown_and_inline_components() {
    let html = render(
        "@panel title=\"Hello\"\n  @grid columns=2\n    @summary\n      **Bold** and @badge[A **bold** badge] with @status(tone=warning)[watch].\n\n      - one\n      - two\n",
    );
    for expected in [
        "<section",
        "cols-2",
        "<strong>Bold</strong>",
        "<span class=\"badge\">A <strong>bold</strong> badge</span>",
        "<span class=\"status warning\">watch</span>",
        "<li>two</li>",
    ] {
        assert!(html.contains(expected), "missing {expected}: {html}");
    }
}

#[test]
fn code_and_escaped_components_stay_literal() {
    let html = render(
        "```meld\n@not-a-component\n@badge[code]\n```\n\n`@badge[code]` and \\@badge[literal]\n\n    @badge[indented code]\n",
    );
    assert!(!html.contains("<span class=\"badge\">"));
    assert!(html.contains("@not-a-component"));
    assert!(html.contains("@badge[literal]"));
}

#[test]
fn unsafe_document_input_is_escaped() {
    let html = render(
        "@hero title=\"<script>alert(1)</script>\"\n  <script>alert(2)</script>\n\n  @badge[<img src=x onerror=alert(3)>]\n\n[bad](javascript:alert%281%29)\n",
    );
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<img"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("href=\"#\""));
    assert!(html.contains("Content-Security-Policy"));
}

#[test]
fn diagnostics() {
    failure("@unknown\n  hello\n", "line 1");
    failure("@hero\n  text\n", "requires attribute title");
    failure("@grid columns=9\n", "columns must be");
    failure("@hero title=\"unfinished\n", "unterminated");
    failure("@hero title=a title=b\n", "duplicate attribute");
    failure("@hero title=a\n x\n", "two-space indentation");
    failure("@badge[unfinished", "unterminated");
    failure("@status(tone=warning)[x", "unterminated");
    failure(
        "@chart title=x\n  A | B\n  --- | ---\n  row | NaN\n",
        "finite",
    );
    failure(
        "@chart title=x\n  A | B\n  --- | ---\n  row | -1\n",
        "nonnegative",
    );
    failure(
        "@chart title=x\n  A | B\n  --- | ---\n  row | x\n",
        "invalid chart number",
    );
}

#[test]
fn export_edit_and_extend() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("design");
    success(&["--init", dir.to_str().unwrap()]);
    let config = dir.join("config.toml");
    let input = tmp.path().join("report.meld");
    fs::write(
        &input,
        "@hero title=Hello\n  @badge[one]\n\n@notice title=Warning\n  **Body**\n\n@pill[Two]\n",
    )
    .unwrap();
    fs::write(
        dir.join("components/badge.html"),
        "<b class=\"custom\">{{ content|safe }}</b>",
    )
    .unwrap();
    fs::write(
        dir.join("components/notice.html"),
        "<aside><h2>{{ attrs.title }}</h2>{{ content|safe }}</aside>",
    )
    .unwrap();
    fs::write(
        dir.join("components/pill.html"),
        "<i>{{ content|safe }}</i>",
    )
    .unwrap();
    let mut cfg = fs::read_to_string(&config).unwrap();
    cfg.push_str("\n[components.notice]\ntemplate = 'components/notice.html'\nrequired = ['title']\n\n[components.pill]\ntemplate = 'components/pill.html'\ninline = true\n");
    cfg = cfg.replace("#cba6f7", "#ff0000");
    fs::write(&config, cfg).unwrap();
    let html = success(&[
        input.to_str().unwrap(),
        "--config",
        config.to_str().unwrap(),
        "-o",
        "-",
    ]);
    for expected in [
        "--mauve: #ff0000",
        "<b class=\"custom\">one</b>",
        "<aside><h2>Warning</h2>",
        "<strong>Body</strong>",
        "<i>Two</i>",
    ] {
        assert!(html.contains(expected), "missing {expected}");
    }
    assert!(!cli(&["--init", dir.to_str().unwrap()]).status.success());
}

#[test]
fn template_directory_and_partial_config_override() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("badge.html"), "<em>{{ content|safe }}</em>").unwrap();
    fs::write(
        tmp.path().join("extra.html"),
        "<aside>{{ content|safe }}</aside>",
    )
    .unwrap();
    fs::write(
        tmp.path().join("metric.html"),
        "<h2>{{ attrs.label }} {{ attrs.tone }}</h2>",
    )
    .unwrap();
    fs::write(
        tmp.path().join("config.toml"),
        "[components.metric]\ntemplate='metric.html'\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("input.meld"),
        "@metric label=Test value=5\n\n@badge[X]\n\n@extra\n  Y\n",
    )
    .unwrap();
    let html = success(&[
        tmp.path().join("input.meld").to_str().unwrap(),
        "--components",
        tmp.path().to_str().unwrap(),
        "--config",
        tmp.path().join("config.toml").to_str().unwrap(),
        "-o",
        "-",
    ]);
    assert!(html.contains("<h2>Test mauve</h2>"));
    assert!(html.contains("<em>X</em>"));
    assert!(html.contains("<aside><p>Y</p>"));
}

#[test]
fn default_output_and_no_input_overwrite() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.meld");
    fs::write(&input, "# Hello").unwrap();
    success(&[input.to_str().unwrap()]);
    assert!(input.with_extension("html").exists());
    assert!(
        !cli(&[input.to_str().unwrap(), "-o", input.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(fs::read_to_string(&input).unwrap(), "# Hello");
}

#[test]
fn css_hash_matches_and_output_is_deterministic() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use sha2::{Digest, Sha256};
    let first = render("# Hi\n");
    assert_eq!(first, render("# Hi\n"));
    let css = first
        .split_once("<style>")
        .unwrap()
        .1
        .split_once("</style>")
        .unwrap()
        .0;
    let hash = STANDARD.encode(Sha256::digest(css.as_bytes()));
    assert!(first.contains(&format!("style-src 'sha256-{hash}'")));
}

#[test]
fn rejects_bad_config_and_css() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.meld");
    fs::write(&input, "Hello").unwrap();
    for config in [
        "[theme.variables]\ntext='red; bad'",
        "unknown=true",
        "[components.extra]\ninline=true",
    ] {
        fs::write(tmp.path().join("bad.toml"), config).unwrap();
        let out = cli(&[
            input.to_str().unwrap(),
            "-c",
            tmp.path().join("bad.toml").to_str().unwrap(),
        ]);
        assert!(!out.status.success());
        assert!(!Path::new(&input.with_extension("html")).exists());
    }
}
