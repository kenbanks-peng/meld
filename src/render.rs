use crate::{
    config::Setup,
    syntax::{self, Node},
};
use anyhow::{Context, Result, bail};
use minijinja::{AutoEscape, Environment, UndefinedBehavior, context};
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd, html};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub struct Renderer<'a> {
    setup: &'a Setup,
    env: Environment<'a>,
    styles: RefCell<BTreeSet<String>>,
}

impl<'a> Renderer<'a> {
    pub fn new(setup: &'a Setup) -> Result<Self> {
        let mut env = Environment::new();
        env.set_auto_escape_callback(|_| AutoEscape::Html);
        env.set_undefined_behavior(UndefinedBehavior::Strict);
        for (name, template) in &setup.templates {
            env.add_template(name, template)?;
        }
        Ok(Self {
            setup,
            env,
            styles: RefCell::default(),
        })
    }

    pub fn render(&self, source: &str) -> Result<String> {
        self.styles.borrow_mut().clear();
        self.nodes(&syntax::parse(source)?, 0)
    }

    pub fn css(&self) -> String {
        self.styles
            .borrow()
            .iter()
            .fold(self.setup.css.clone(), |mut css, rule| {
                css.push('\n');
                css.push_str(rule);
                css
            })
    }

    fn nodes(&self, nodes: &[Node], depth: usize) -> Result<String> {
        let mut out = String::new();
        for node in nodes {
            match node {
                Node::Markdown(s) => out.push_str(&self.markdown(s, false, depth)?),
                Node::Component {
                    name,
                    attrs,
                    children,
                    line,
                } => {
                    let result = (|| {
                        let def = self
                            .setup
                            .definitions
                            .get(name)
                            .with_context(|| format!("unknown component @{name}"))?;
                        anyhow::ensure!(
                            !def.inline.unwrap_or(false),
                            "@{name} uses inline syntax: @{name}[text]"
                        );
                        let content = self.nodes(children, depth + 1)?;
                        let source = if name == "chart" {
                            let mut raw = String::new();
                            for child in children {
                                match child {
                                    Node::Markdown(s) => raw.push_str(s),
                                    _ => bail!("chart body must be a plain Markdown table"),
                                }
                            }
                            raw
                        } else {
                            String::new()
                        };
                        self.component(name, attrs, &content, &source)
                    })();
                    out.push_str(&result.with_context(|| format!("line {line}: @{name}"))?);
                }
            }
        }
        Ok(out)
    }

    fn component(
        &self,
        name: &str,
        attrs: &BTreeMap<String, String>,
        content: &str,
        raw: &str,
    ) -> Result<String> {
        let def = self
            .setup
            .definitions
            .get(name)
            .with_context(|| format!("unknown component @{name}"))?;
        let mut values = def.defaults.clone();
        values.extend(attrs.clone());
        for key in &def.required {
            anyhow::ensure!(values.contains_key(key), "@{name} requires attribute {key}");
        }
        if name == "grid" {
            let columns = values.get("columns").context("grid requires columns")?;
            let columns: usize = columns
                .parse()
                .ok()
                .filter(|n| *n > 0)
                .context("grid columns must be a positive integer")?;
            values.insert("columns".into(), columns.to_string());
            self.styles.borrow_mut().insert(format!(
                ".grid.cols-{columns} {{ --grid-columns: {columns}; --grid-tablet-columns: {}; }}",
                columns.min(2)
            ));
        }
        let value = values.get("value").map(String::as_str).unwrap_or("");
        let split = value
            .char_indices()
            .find(|(_, c)| !c.is_ascii_digit() && *c != '.' && *c != '-')
            .map(|(i, _)| i)
            .unwrap_or(value.len());
        let (number, unit) = value.split_at(split);
        let chart = if name == "chart" {
            let (html, styles) =
                crate::chart::render(raw, values.get("title").map(String::as_str).unwrap_or(""))?;
            self.styles.borrow_mut().extend(styles);
            html
        } else {
            String::new()
        };
        self.env
            .get_template(name)?
            .render(context! {
                attrs => values, content => content, number => number, unit => unit, chart => chart
            })
            .with_context(|| format!("rendering @{name}"))
    }

    fn markdown(&self, source: &str, inline: bool, depth: usize) -> Result<String> {
        anyhow::ensure!(depth < 64, "component nesting exceeds 64 levels");
        let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
        // Code, raw HTML and links remain literal rather than becoming component invocations.
        let mut protected = Vec::new();
        let mut block_start = None;
        for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
            match event {
                Event::Start(Tag::CodeBlock(_)) => block_start = Some(range.start),
                Event::End(TagEnd::CodeBlock) => {
                    if let Some(start) = block_start.take() {
                        protected.push(start..range.end);
                    }
                }
                Event::Code(_)
                | Event::Html(_)
                | Event::InlineHtml(_)
                | Event::Start(Tag::Link { .. } | Tag::Image { .. }) => protected.push(range),
                _ => {}
            }
        }
        let mut prefix = "MELDTOKEN".to_string();
        while source.contains(&prefix) {
            prefix.push('X');
        }
        let mut replacements = Vec::new();
        let mut expanded = String::new();
        let mut i = 0;
        while i < source.len() {
            let rest = &source[i..];
            let escaped = source[..i]
                .bytes()
                .rev()
                .take_while(|b| *b == b'\\')
                .count()
                % 2
                == 1;
            if rest.starts_with('@') && !escaped && !protected.iter().any(|r| r.contains(&i)) {
                let n = syntax::name_end(&rest[1..]);
                let name = &rest[1..1 + n];
                let mut pos = 1 + n;
                let mut attrs = BTreeMap::new();
                if n > 0 && rest[pos..].starts_with('(') {
                    let end =
                        closing(rest, pos, '(', ')').context("unterminated inline attributes")?;
                    attrs = syntax::attributes(&rest[pos + 1..end])?;
                    pos = end + 1;
                }
                if n > 0 && rest[pos..].starts_with('[') {
                    let end = closing(rest, pos, '[', ']')
                        .with_context(|| format!("unterminated @{name} content"))?;
                    let def = self
                        .setup
                        .definitions
                        .get(name)
                        .with_context(|| format!("unknown component @{name}"))?;
                    anyhow::ensure!(def.inline.unwrap_or(false), "@{name} is a block component");
                    let content = self.markdown(&rest[pos + 1..end], true, depth + 1)?;
                    replacements.push(self.component(name, &attrs, &content, "")?);
                    expanded.push_str(&format!("{prefix}{}END", replacements.len() - 1));
                    i += end + 1;
                    continue;
                }
            }
            let c = rest.chars().next().unwrap();
            expanded.push(c);
            i += c.len_utf8();
        }
        let mut events = Vec::new();
        for event in Parser::new_ext(&expanded, options) {
            match event {
                Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) if inline => {}
                Event::Html(s) | Event::InlineHtml(s) => events.push(Event::Text(s)),
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) => {
                    events.push(Event::Start(Tag::Link {
                        link_type,
                        dest_url: safe_url(dest_url),
                        title,
                        id,
                    }));
                }
                Event::Start(Tag::Image {
                    link_type,
                    dest_url,
                    title,
                    id,
                }) => {
                    events.push(Event::Start(Tag::Image {
                        link_type,
                        dest_url: safe_url(dest_url),
                        title,
                        id,
                    }));
                }
                Event::Text(s) if s.contains(&prefix) => {
                    let mut remaining = s.as_ref();
                    while let Some(start) = remaining.find(&prefix) {
                        events.push(Event::Text(remaining[..start].to_string().into()));
                        let token = &remaining[start + prefix.len()..];
                        let end = token.find("END").context("invalid component placeholder")?;
                        let index: usize = token[..end].parse()?;
                        events.push(Event::InlineHtml(replacements[index].clone().into()));
                        remaining = &token[end + 3..];
                    }
                    events.push(Event::Text(remaining.to_string().into()));
                }
                other => events.push(other),
            }
        }
        let mut out = String::new();
        html::push_html(&mut out, events.into_iter());
        Ok(out)
    }
}

fn safe_url(url: CowStr<'_>) -> CowStr<'_> {
    let normalized: String = url
        .chars()
        .filter(|c| !c.is_control() && !c.is_whitespace())
        .collect();
    let lower = normalized.to_ascii_lowercase();
    if lower.starts_with("//")
        || lower.starts_with("\\\\")
        || (lower.contains(':')
            && !["https:", "http:", "mailto:"]
                .iter()
                .any(|p| lower.starts_with(p)))
    {
        "#".into()
    } else {
        url
    }
}

fn closing(s: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0;
    let mut quote = None;
    let mut escaped = false;
    for (offset, c) in s[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if open == '(' {
            if let Some(q) = quote {
                if c == q {
                    quote = None;
                }
                continue;
            }
            if c == '"' || c == '\'' {
                quote = Some(c);
                continue;
            }
        }
        if c == open {
            depth += 1;
        }
        if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(start + offset);
            }
        }
    }
    None
}
