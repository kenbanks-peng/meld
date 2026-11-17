use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::TempDir;

fn cli(args: &[&str]) -> Output {
    let config_home = TempDir::new().unwrap();
    Command::new(env!("CARGO_BIN_EXE_meld"))
        .env("XDG_CONFIG_HOME", config_home.path())
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
fn skill_prints_active_guide_without_working_directory_files() {
    let tmp = TempDir::new().unwrap();
    let config_home = TempDir::new().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_meld"))
        .env("XDG_CONFIG_HOME", config_home.path())
        .arg("skill")
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(out.stderr.is_empty());
    let guide = String::from_utf8(out.stdout).unwrap();
    assert!(guide.starts_with(include_str!("../assets/skill.md")));
    for name in [
        "badge", "callout", "chart", "footer", "grid", "hero", "masthead", "metric", "muted",
        "panel", "status", "summary",
    ] {
        assert!(guide.contains(&format!("@{name} | ")));
    }
    assert_eq!(
        guide
            .lines()
            .filter(|line| line.starts_with('@') && line.contains(" | "))
            .count(),
        12
    );
    for omitted in [
        "## Customize",
        "--init",
        "--config",
        "--components",
        "-o ",
        "MiniJinja",
        "XDG_CONFIG_HOME",
    ] {
        assert!(!guide.contains(omitted), "unexpected {omitted}");
    }
    assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 0);
    assert!(success(&["--help"]).contains("skill"));
    assert!(success(&["skill", "--help"]).contains("agent instructions"));
}

#[test]
fn skill_example_renders() {
    let guide = success(&["skill"]);
    let examples = skill_examples(&guide);
    assert_eq!(examples.len(), 1);
    let html = render(&examples.join("\n\n"));
    for expected in [
        "Weekly report",
        "cols-2",
        "status positive",
        "<svg ",
        "class=\"masthead\"",
        "class=\"hero\"",
        "class=\"metric tone-mauve\"",
        "class=\"panel\"",
        "class=\"summary\"",
        "class=\"callout tone-teal\"",
        "class=\"muted\"",
        "class=\"badge\"",
        "class=\"footer\"",
    ] {
        assert!(html.contains(expected), "missing {expected}");
    }
}

fn skill_examples(guide: &str) -> Vec<String> {
    use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
    let mut examples = Vec::new();
    let mut collecting = false;
    for event in Parser::new(guide) {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))
                if lang.as_ref() == "meld" =>
            {
                collecting = true;
                examples.push(String::new());
            }
            Event::End(TagEnd::CodeBlock) => collecting = false,
            Event::Text(text) if collecting => examples.last_mut().unwrap().push_str(&text),
            _ => {}
        }
    }
    examples
}

#[test]
fn skill_tracks_custom_config_and_template_directory() {
    let tmp = TempDir::new().unwrap();
    let config = tmp.path().join("config.toml");
    let templates = tmp.path().join("templates");
    fs::create_dir(&templates).unwrap();
    fs::write(
        tmp.path().join("notice.html"),
        "<aside>{{ attrs.title }} {{ attrs.tone }} {{ content|safe }}</aside>",
    )
    .unwrap();
    fs::write(
        tmp.path().join("pill.html"),
        "<i>{{ attrs.label }} {{ content|safe }}</i>",
    )
    .unwrap();
    fs::write(templates.join("extra.html"), "<b>{{ content|safe }}</b>").unwrap();
    fs::write(
        templates.join("notice.html"),
        "<section>{{ attrs.title }} {{ content|safe }}</section>",
    )
    .unwrap();
    fs::write(
        &config,
        r#"
[components.notice]
template = "notice.html"
required = ["title"]
[components.notice.defaults]
tone = 'say "hello" \\ again'
[components.pill]
template = "pill.html"
inline = true
required = ["label"]
"#,
    )
    .unwrap();
    let args = [
        "skill",
        "--config",
        config.to_str().unwrap(),
        "--components",
        templates.to_str().unwrap(),
    ];
    let guide = success(&args);
    assert_eq!(skill_examples(&guide).len(), 1);
    assert!(guide.contains("@notice | block | title |"));
    assert!(guide.contains("@pill | inline | label |"));
    assert!(guide.contains("@extra | block |"));
    assert!(!guide.contains("@hero"));
    let input = tmp.path().join("report.meld");
    fs::write(&input, skill_examples(&guide).join("\n\n")).unwrap();
    let html = success(&[
        input.to_str().unwrap(),
        "--config",
        config.to_str().unwrap(),
        "--components",
        templates.to_str().unwrap(),
        "-o",
        "-",
    ]);
    assert!(html.contains("<section>title"));
    assert!(html.contains("<i>label"));
    assert!(html.contains("<b>"));
    fs::remove_file(templates.join("extra.html")).unwrap();
    assert!(!success(&args).contains("@extra"));
    fs::write(&config, "").unwrap();
    let empty = success(&["skill", "-c", config.to_str().unwrap()]);
    assert!(empty.contains("No components configured"));
    assert!(skill_examples(&empty).is_empty());
    fs::write(&config, "invalid TOML").unwrap();
    let out = cli(&["skill", "-c", config.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
}

#[test]
fn skill_tracks_changes_to_default_design() {
    let tmp = TempDir::new().unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_meld"))
            .env("XDG_CONFIG_HOME", tmp.path())
            .arg("skill")
            .output()
            .unwrap()
    };
    assert!(run().status.success());
    fs::write(tmp.path().join("meld/only.html"), "{{ content|safe }}").unwrap();
    fs::write(
        tmp.path().join("meld/config.toml"),
        "[components.only]\ntemplate = 'only.html'\ninline = true\n",
    )
    .unwrap();
    let out = run();
    assert!(out.status.success());
    let guide = String::from_utf8(out.stdout).unwrap();
    assert_eq!(skill_examples(&guide).len(), 1);
    assert!(guide.contains("@only[Example]"));
    assert!(!guide.contains("@badge"));
}

#[test]
fn skill_rejects_render_arguments() {
    for args in [
        vec!["skill", "report.meld"],
        vec!["skill", "-o", "report.html"],
        vec!["--title", "Report", "skill"],
        vec!["--init", "design", "skill"],
    ] {
        assert!(!cli(&args).status.success(), "{args:?}");
    }
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
fn template_directory_overrides_external_config() {
    let tmp = TempDir::new().unwrap();
    let design = tmp.path().join("design");
    success(&["--init", design.to_str().unwrap()]);
    let cfg = fs::read_to_string(design.join("config.toml"))
        .unwrap()
        .replace("components/metric.html", "../metric.html");
    fs::write(design.join("config.toml"), cfg).unwrap();
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
        tmp.path().join("input.meld"),
        "@metric label=Test value=5\n\n@badge[X]\n\n@extra\n  Y\n",
    )
    .unwrap();
    let html = success(&[
        tmp.path().join("input.meld").to_str().unwrap(),
        "--components",
        tmp.path().to_str().unwrap(),
        "--config",
        design.join("config.toml").to_str().unwrap(),
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

#[test]
fn default_design_is_seeded_and_customizations_are_reused() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("nested/config");
    let input = tmp.path().join("input.meld");
    fs::write(&input, "@badge[Hello]").unwrap();
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_meld"))
            .env("XDG_CONFIG_HOME", &root)
            .arg(&input)
            .args(["-o", "-"])
            .output()
            .unwrap()
    };
    let first = run();
    assert!(first.status.success(), "{first:?}");
    let design = root.join("meld");
    assert!(design.join("config.toml").is_file());
    assert_eq!(
        fs::read_to_string(design.join("theme.css")).unwrap(),
        include_str!("../assets/theme.css")
    );
    fs::write(
        design.join("components/badge.html"),
        "<b>{{ content|safe }}</b>",
    )
    .unwrap();
    fs::write(design.join("theme.css"), "/* custom only */").unwrap();
    let cfg = fs::read_to_string(design.join("config.toml"))
        .unwrap()
        .replace("My report", "Custom title");
    fs::write(design.join("config.toml"), &cfg).unwrap();
    for _ in 0..2 {
        let out = run();
        assert!(out.status.success(), "{out:?}");
        let html = String::from_utf8(out.stdout).unwrap();
        assert!(html.contains("<b>Hello</b>"));
        assert!(html.contains("<title>Custom title</title>"));
        assert!(html.contains("/* custom only */"));
        assert!(!html.contains("--base:"));
        assert_eq!(fs::read_to_string(design.join("config.toml")).unwrap(), cfg);
    }
    fs::remove_file(design.join("components/badge.html")).unwrap();
    let out = run();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("reading template for @badge"));
    assert!(!design.join("components/badge.html").exists());
    fs::remove_file(design.join("config.toml")).unwrap();
    assert!(!run().status.success());
    assert!(!design.join("config.toml").exists());
}

#[test]
fn home_fallback_seeds_even_when_printing_help() {
    for xdg in [None, Some(""), Some("relative/path")] {
        let tmp = TempDir::new().unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_meld"));
        command
            .env("HOME", tmp.path())
            .env_remove("XDG_CONFIG_HOME")
            .arg("--help");
        if let Some(value) = xdg {
            command.env("XDG_CONFIG_HOME", value);
        }
        let out = command.output().unwrap();
        assert!(out.status.success(), "{out:?}");
        assert!(tmp.path().join(".config/meld/config.toml").is_file());
    }
}

#[test]
fn alternative_config_has_no_implicit_components_or_css() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.meld");
    let config = tmp.path().join("config.toml");
    fs::write(&config, "").unwrap();
    fs::write(&input, "# Plain").unwrap();
    let args = [
        input.to_str().unwrap(),
        "-c",
        config.to_str().unwrap(),
        "-o",
        "-",
    ];
    let html = success(&args);
    assert!(html.contains("<style></style>"));
    fs::write(&input, "@badge[Hello]").unwrap();
    let out = cli(&args);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown component @badge"));
}

#[test]
fn existing_invalid_default_is_not_overwritten() {
    let tmp = TempDir::new().unwrap();
    let design = tmp.path().join("meld");
    fs::create_dir(&design).unwrap();
    fs::write(design.join("config.toml"), "not valid TOML").unwrap();
    let input = tmp.path().join("input.meld");
    fs::write(&input, "# Hello").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_meld"))
        .env("XDG_CONFIG_HOME", tmp.path())
        .arg(&input)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(
        fs::read_to_string(design.join("config.toml")).unwrap(),
        "not valid TOML"
    );
    assert!(!design.join("theme.css").exists());
}
