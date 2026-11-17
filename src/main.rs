mod chart;
mod config;
mod render;
mod syntax;

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::{CommandFactory, Parser};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(
    version,
    about = "Mark Elements Down: convert component-enhanced Markdown to standalone HTML"
)]
struct Cli {
    /// Input .meld file (output defaults to the same path with .html extension)
    #[arg(required_unless_present = "init")]
    input: Option<PathBuf>,
    /// Output file, or - for stdout
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// TOML config; relative asset paths resolve beside this file
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// Directory of component .html templates (overrides built-ins and config)
    #[arg(long)]
    components: Option<PathBuf>,
    /// Override the document title
    #[arg(long)]
    title: Option<String>,
    /// Export editable config, CSS and all component templates into a new directory
    #[arg(long, conflicts_with_all = ["input", "output", "config", "components", "title"])]
    init: Option<PathBuf>,
}

fn run(cli: Cli) -> Result<()> {
    if let Some(dir) = cli.init {
        return config::Setup::init(&dir);
    }
    let input = cli.input.context("missing input")?;
    let output = cli.output.unwrap_or_else(|| input.with_extension("html"));
    anyhow::ensure!(output != input, "input and output paths must differ");
    if output.exists() && input.exists() {
        anyhow::ensure!(
            fs::canonicalize(&output)? != fs::canonicalize(&input)?,
            "output resolves to input"
        );
    }
    let setup = config::Setup::load(cli.config.as_deref(), cli.components.as_deref())?;
    let source =
        fs::read_to_string(&input).with_context(|| format!("reading {}", input.display()))?;
    let body = render::Renderer::new(&setup)?.render(&source)?;
    let title = cli
        .title
        .as_deref()
        .or(setup.config.title.as_deref())
        .unwrap_or_else(|| {
            input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Document")
        });
    let lang = setup.config.lang.as_deref().unwrap_or("en");
    let hash = STANDARD.encode(Sha256::digest(setup.css.as_bytes()));
    let html = format!(
        "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'sha256-{hash}'; base-uri 'none'; form-action 'none'\">\n<title>{}</title>\n<style>{}</style>\n</head>\n<body><main><article>{body}</article></main></body>\n</html>\n",
        render::escape(lang),
        render::escape(title),
        setup.css
    );
    if output.as_os_str() == "-" {
        io::stdout().lock().write_all(html.as_bytes())?;
    } else {
        fs::write(&output, html).with_context(|| format!("writing {}", output.display()))?;
    }
    Ok(())
}

fn main() {
    if std::env::args_os().len() == 1 {
        Cli::command().print_help().expect("failed to print help");
        return;
    }
    if let Err(err) = run(Cli::parse()) {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
