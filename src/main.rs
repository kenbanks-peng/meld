mod chart;
mod config;
mod paths;
mod render;
mod skill;
mod syntax;

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::{CommandFactory, Parser, Subcommand};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(
    version,
    subcommand_negates_reqs = true,
    args_conflicts_with_subcommands = true,
    about = "Mark Elements Down: convert component-enhanced Markdown to standalone HTML"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Input .meld file (output defaults to the same path with .html extension)
    #[arg(required_unless_present = "init")]
    input: Option<PathBuf>,
    /// Output file, or - for stdout
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Alternative TOML config (default: $XDG_CONFIG_HOME/meld/config.toml)
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// Directory of component .html templates (overrides configured templates)
    #[arg(long)]
    components: Option<PathBuf>,
    /// Override the document title
    #[arg(long)]
    title: Option<String>,
    /// Export editable config, CSS and all component templates into a new directory
    #[arg(long, conflicts_with_all = ["input", "output", "config", "components", "title"])]
    init: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create global project storage and print absolute source_dir/output_dir as JSON
    Paths {
        /// Project directory (defaults to the current directory)
        #[arg(long, default_value = ".")]
        project: PathBuf,
    },
    /// Print concise agent instructions for authoring and rendering Meld files
    Skill {
        /// Alternative TOML config (default: $XDG_CONFIG_HOME/meld/config.toml)
        #[arg(short, long)]
        config: Option<PathBuf>,
        /// Directory of component .html templates (overrides configured templates)
        #[arg(long)]
        components: Option<PathBuf>,
    },
}

fn run(cli: Cli, default_config: PathBuf) -> Result<()> {
    if let Some(command) = cli.command {
        match command {
            Commands::Paths { project } => {
                let paths = paths::ProjectPaths::create(&project)?;
                let mut stdout = io::stdout().lock();
                serde_json::to_writer(&mut stdout, &paths)?;
                writeln!(stdout)?;
            }
            Commands::Skill { config, components } => {
                let setup = config::Setup::load(
                    config.as_deref().unwrap_or(&default_config),
                    components.as_deref(),
                )?;
                io::stdout()
                    .lock()
                    .write_all(skill::guide(&setup).as_bytes())?;
            }
        }
        return Ok(());
    }
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
    let setup = config::Setup::load(
        cli.config.as_deref().unwrap_or(&default_config),
        cli.components.as_deref(),
    )?;
    let source =
        fs::read_to_string(&input).with_context(|| format!("reading {}", input.display()))?;
    let renderer = render::Renderer::new(&setup)?;
    let body = renderer.render(&source)?;
    let css = renderer.css();
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
    let hash = STANDARD.encode(Sha256::digest(css.as_bytes()));
    let html = format!(
        "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'sha256-{hash}'; base-uri 'none'; form-action 'none'\">\n<title>{}</title>\n<style>{}</style>\n</head>\n<body><main><article>{body}</article></main></body>\n</html>\n",
        render::escape(lang),
        render::escape(title),
        css
    );
    if output.as_os_str() == "-" {
        io::stdout().lock().write_all(html.as_bytes())?;
    } else {
        fs::write(&output, html).with_context(|| format!("writing {}", output.display()))?;
    }
    Ok(())
}

fn main() {
    let default_config = match config::Setup::ensure_default() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("error: {err:#}");
            std::process::exit(1);
        }
    };
    if std::env::args_os().len() == 1 {
        Cli::command().print_help().expect("failed to print help");
        return;
    }
    if let Err(err) = run(Cli::parse(), default_config) {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
