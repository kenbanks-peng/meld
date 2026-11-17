use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

pub const CSS: &str = include_str!("../assets/theme.css");
pub const TEMPLATES: &[(&str, &str)] = &[
    (
        "masthead",
        include_str!("../assets/components/masthead.html"),
    ),
    ("hero", include_str!("../assets/components/hero.html")),
    ("grid", include_str!("../assets/components/grid.html")),
    ("metric", include_str!("../assets/components/metric.html")),
    ("panel", include_str!("../assets/components/panel.html")),
    ("summary", include_str!("../assets/components/summary.html")),
    ("callout", include_str!("../assets/components/callout.html")),
    ("badge", include_str!("../assets/components/badge.html")),
    ("status", include_str!("../assets/components/status.html")),
    ("muted", include_str!("../assets/components/muted.html")),
    ("footer", include_str!("../assets/components/footer.html")),
    ("chart", include_str!("../assets/components/chart.html")),
];

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Component {
    pub template: Option<String>,
    pub inline: Option<bool>,
    pub required: Vec<String>,
    pub defaults: BTreeMap<String, String>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Theme {
    pub variables: BTreeMap<String, String>,
    pub css: Option<String>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub title: Option<String>,
    pub lang: Option<String>,
    pub theme: Theme,
    pub components: BTreeMap<String, Component>,
}

pub struct Setup {
    pub config: Config,
    pub definitions: BTreeMap<String, Component>,
    pub templates: BTreeMap<String, String>,
    pub css: String,
}

fn builtin_definitions() -> BTreeMap<String, Component> {
    TEMPLATES
        .iter()
        .map(|(name, _)| {
            let mut c = Component::default();
            let (required, defaults): (&[&str], &[(&str, &str)]) = match *name {
                "hero" => (&["title"], &[("eyebrow", "")]),
                "grid" => (&[], &[("columns", "2")]),
                "metric" => (&["label", "value"], &[("tone", "mauve")]),
                "panel" => (&["title"], &[("label", "")]),
                "callout" => (&["title"], &[("tone", "teal")]),
                "status" => (&[], &[("tone", "positive")]),
                "chart" => (&["title"], &[]),
                _ => (&[], &[]),
            };
            c.inline = Some(matches!(*name, "badge" | "status"));
            c.required = required.iter().map(|s| s.to_string()).collect();
            c.defaults = defaults
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            (name.to_string(), c)
        })
        .collect()
}

pub fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

impl Setup {
    pub fn load(path: Option<&Path>, component_dir: Option<&Path>) -> Result<Self> {
        let config: Config = match path {
            Some(p) => toml::from_str(
                &fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?,
            )
            .with_context(|| format!("parsing {}", p.display()))?,
            None => Config::default(),
        };
        let base = path.and_then(Path::parent).unwrap_or(Path::new("."));
        let mut definitions = builtin_definitions();
        let mut templates: BTreeMap<String, String> = TEMPLATES
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        for (name, custom) in &config.components {
            anyhow::ensure!(valid_name(name), "invalid component name: {name}");
            if let Some(file) = &custom.template {
                templates.insert(
                    name.clone(),
                    fs::read_to_string(base.join(file))
                        .with_context(|| format!("reading template for @{name}"))?,
                );
            }
            let definition = definitions.entry(name.clone()).or_default();
            if let Some(inline) = custom.inline {
                definition.inline = Some(inline);
            }
            if !custom.required.is_empty() {
                definition.required = custom.required.clone();
            }
            definition.defaults.extend(custom.defaults.clone());
        }
        if let Some(dir) = component_dir {
            for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
                let p = entry?.path();
                if p.extension().and_then(|s| s.to_str()) == Some("html") {
                    let name = p
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .context("invalid template filename")?;
                    anyhow::ensure!(valid_name(name), "invalid component name: {name}");
                    templates.insert(name.to_string(), fs::read_to_string(&p)?);
                    definitions.entry(name.to_string()).or_default();
                }
            }
        }
        for name in definitions.keys() {
            anyhow::ensure!(
                templates.contains_key(name),
                "component @{name} needs a template"
            );
        }
        let mut css = CSS.to_string();
        if !config.theme.variables.is_empty() {
            css.push_str("\n:root {\n");
            for (key, value) in &config.theme.variables {
                if !valid_name(key) || value.contains([';', '{', '}', '<', '>']) {
                    bail!("invalid theme variable: {key}");
                }
                css.push_str(&format!("  --{key}: {value};\n"));
            }
            css.push_str("}\n");
        }
        if let Some(file) = &config.theme.css {
            css.push_str(&fs::read_to_string(base.join(file)).context("reading theme CSS")?);
        }
        anyhow::ensure!(
            !css.to_ascii_lowercase().contains("</style"),
            "CSS must not contain </style"
        );
        Ok(Self {
            config,
            definitions,
            templates,
            css,
        })
    }

    pub fn init(dir: &Path) -> Result<()> {
        // Refuse existing destinations rather than silently replacing user templates.
        fs::create_dir(dir)
            .with_context(|| format!("creating {} (must not exist)", dir.display()))?;
        fs::create_dir(dir.join("components"))?;
        fs::write(dir.join("theme.css"), "/* Additional trusted CSS. */\n")?;
        let mut config = Config {
            title: Some("My report".into()),
            lang: Some("en".into()),
            ..Config::default()
        };
        config.theme.css = Some("theme.css".into());
        config
            .theme
            .variables
            .insert("mauve".into(), "#cba6f7".into());
        config.components = builtin_definitions();
        for (name, template) in TEMPLATES {
            fs::write(
                dir.join("components").join(format!("{name}.html")),
                template,
            )?;
            config.components.get_mut(*name).unwrap().template =
                Some(format!("components/{name}.html"));
        }
        fs::write(dir.join("config.toml"), toml::to_string_pretty(&config)?)?;
        Ok(())
    }
}
