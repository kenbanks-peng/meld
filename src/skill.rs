use crate::config::Setup;
use std::{collections::BTreeMap, fmt::Write};

fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn guide(setup: &Setup) -> String {
    let mut out = include_str!("../assets/skill.md").to_string();
    if setup.definitions.is_empty() {
        out.push_str("\nNo components configured; use ordinary Markdown.\n");
        return out;
    }
    out.push_str("\nComponent | Mode | Required | Defaults\n--- | --- | --- | ---\n");
    let mut blocks = BTreeMap::new();
    let mut inlines = Vec::new();
    for (name, def) in &setup.definitions {
        let inline = def.inline.unwrap_or(false);
        let mode = if inline { "inline" } else { "block" };
        let defaults = def
            .defaults
            .iter()
            .map(|(key, value)| format!("{key}={}", quote(value)))
            .collect::<Vec<_>>()
            .join(", ");
        // Keep arbitrary design values from breaking the Markdown table.
        let cell = |value: &str| {
            value
                .replace('&', "&amp;")
                .replace('|', "&#124;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('\n', "&#10;")
                .replace('\r', "&#13;")
        };
        writeln!(
            out,
            "@{name} | {mode} | {} | {}",
            cell(&def.required.join(", ")),
            cell(&defaults)
        )
        .unwrap();

        let attrs = def
            .required
            .iter()
            .map(|key| {
                let value = match (name.as_str(), key.as_str()) {
                    ("hero", "title") => "Weekly report",
                    ("panel", "title") => "Delivery",
                    ("callout", "title") => "Next step",
                    ("chart", "title") => "Deployments",
                    ("metric", "label") => "Deployments",
                    ("metric", "value") => "24",
                    ("grid", "columns") => "2",
                    _ => key,
                };
                format!("{key}={}", quote(value))
            })
            .collect::<Vec<_>>()
            .join(" ");
        if inline {
            let args = if attrs.is_empty() {
                String::new()
            } else {
                format!("({attrs})")
            };
            let body = match name.as_str() {
                "badge" => "Week 12",
                "status" => "On track",
                _ => "Example",
            };
            inlines.push(format!("@{name}{args}[{body}]"));
        } else {
            let args = if attrs.is_empty() {
                String::new()
            } else {
                format!(" {attrs}")
            };
            let body = match name.as_str() {
                "masthead" => "**Platform team**\n\nWeek 12",
                "hero" => "Smaller releases, faster feedback.",
                "grid" | "panel" => "",
                "metric" => "Up from 18 last week.",
                "summary" => "**Release time fell by 30%.** All services met their targets.",
                "callout" => "- Finish the cache rollout.\n- Measure build times.",
                "chart" => "Week | Deployments\n--- | ---\n11 | 18\n12 | 24",
                "muted" => "Production only; excludes rollbacks.",
                "footer" => "Platform team · Internal",
                _ => "Example **Markdown**.",
            };
            blocks.insert(name.as_str(), format!("@{name}{args}\n{}", indent(body)));
        }
    }
    if setup.definitions.contains_key("grid") {
        out.push_str("\nGrid: columns must be 1–4.\n");
    }
    if setup.definitions.contains_key("chart") {
        out.push_str("\nChart body: Markdown table; first column is categories, followed by 1–6\nfinite, nonnegative numeric series. Use plain numbers, without units or separators.\n");
    }

    // Assemble one report, nesting only components declared in block mode.
    for (parent, children) in [
        ("grid", &["metric", "callout"][..]),
        ("panel", &["summary", "chart", "muted"][..]),
    ] {
        if blocks.contains_key(parent) {
            let nested = children
                .iter()
                .filter_map(|name| blocks.remove(name))
                .collect::<Vec<_>>();
            if !nested.is_empty() {
                blocks
                    .get_mut(parent)
                    .unwrap()
                    .push_str(&indent(&nested.join("\n\n")));
            }
        }
    }
    let mut example = String::new();
    for name in ["masthead", "hero"] {
        if let Some(block) = blocks.remove(name) {
            writeln!(example, "{block}\n").unwrap();
        }
    }
    if !inlines.is_empty() {
        writeln!(example, "{}\n", inlines.join(" ")).unwrap();
    }
    for name in ["grid", "panel"] {
        if let Some(block) = blocks.remove(name) {
            writeln!(example, "{block}\n").unwrap();
        }
    }
    let footer = blocks.remove("footer");
    for block in blocks.values().chain(footer.iter()) {
        writeln!(example, "{block}\n").unwrap();
    }
    // Attribute values may contain fence characters.
    let fence = "~".repeat(example.chars().filter(|c| *c == '~').count().max(2) + 1);
    writeln!(out, "\n## Complete example\n\nAdapt the content and use only the components your document needs.\n\n{fence}meld\n{}\n{fence}", example.trim_end()).unwrap();
    out
}

fn indent(body: &str) -> String {
    body.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("  {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
