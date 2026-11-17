use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum Node {
    Markdown(String),
    Component {
        name: String,
        attrs: BTreeMap<String, String>,
        children: Vec<Node>,
        line: usize,
    },
}

/// Parse key=value pairs; quoted values support backslash escapes.
pub fn attributes(s: &str) -> Result<BTreeMap<String, String>> {
    let mut attrs = BTreeMap::new();
    let mut chars = s.chars().peekable();
    loop {
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }
        let key: String = std::iter::from_fn(|| {
            chars
                .peek()
                .copied()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .inspect(|_| {
                    chars.next();
                })
        })
        .collect();
        anyhow::ensure!(
            !key.is_empty() && chars.next() == Some('='),
            "expected key=value"
        );
        let mut value = String::new();
        if chars.peek() == Some(&'"') || chars.peek() == Some(&'\'') {
            let quote = chars.next().unwrap();
            loop {
                match chars.next() {
                    Some(c) if c == quote => break,
                    Some('\\') => value.push(chars.next().context("unfinished escape")?),
                    Some(c) => value.push(c),
                    None => bail!("unterminated quoted attribute"),
                }
            }
            anyhow::ensure!(
                chars.peek().is_none_or(|c| c.is_whitespace()),
                "expected whitespace after quoted attribute"
            );
        } else {
            while chars.peek().is_some_and(|c| !c.is_whitespace()) {
                value.push(chars.next().unwrap());
            }
            anyhow::ensure!(!value.is_empty(), "empty unquoted attribute");
        }
        anyhow::ensure!(
            attrs.insert(key.clone(), value).is_none(),
            "duplicate attribute: {key}"
        );
    }
    Ok(attrs)
}

pub fn name_end(s: &str) -> usize {
    s.bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
        .count()
}

pub fn parse(source: &str) -> Result<Vec<Node>> {
    let lines: Vec<&str> = source.lines().collect();
    parse_lines(&lines, 0, lines.len(), 0, 0)
}

fn indent(s: &str) -> usize {
    s.bytes().take_while(|b| *b == b' ').count()
}

fn parse_lines(
    lines: &[&str],
    start: usize,
    end: usize,
    base: usize,
    depth: usize,
) -> Result<Vec<Node>> {
    anyhow::ensure!(depth < 64, "component nesting exceeds 64 levels");
    let mut nodes = Vec::new();
    let mut raw = String::new();
    let mut i = start;
    let mut fence: Option<(u8, usize)> = None;
    while i < end {
        let line = lines[i];
        anyhow::ensure!(
            !line.starts_with('\t'),
            "line {}: use spaces for indentation",
            i + 1
        );
        let text = if line.trim().is_empty() {
            ""
        } else {
            anyhow::ensure!(
                indent(line) >= base,
                "line {}: component bodies need two-space indentation",
                i + 1
            );
            &line[base..]
        };
        let trimmed = text.trim_start();
        let marker = trimmed.as_bytes().first().copied().unwrap_or(0);
        let count = trimmed.bytes().take_while(|b| *b == marker).count();
        if text.len() - trimmed.len() <= 3 && matches!(marker, b'`' | b'~') && count >= 3 {
            if let Some((m, n)) = fence {
                if m == marker && count >= n && trimmed[count..].trim().is_empty() {
                    fence = None;
                }
            } else {
                fence = Some((marker, count));
            }
            raw.push_str(text);
            raw.push('\n');
            i += 1;
            continue;
        }
        if fence.is_none() && text.starts_with('@') {
            let n = name_end(&text[1..]);
            let tail = &text[1 + n..];
            if n > 0 && (tail.is_empty() || tail.starts_with(char::is_whitespace)) {
                if !raw.is_empty() {
                    nodes.push(Node::Markdown(std::mem::take(&mut raw)));
                }
                let name = text[1..1 + n].to_string();
                let attrs = attributes(tail).with_context(|| format!("line {}: @{name}", i + 1))?;
                let mut next = i + 1;
                while next < end && (lines[next].trim().is_empty() || indent(lines[next]) > base) {
                    next += 1;
                }
                let children = parse_lines(lines, i + 1, next, base + 2, depth + 1)?;
                nodes.push(Node::Component {
                    name,
                    attrs,
                    children,
                    line: i + 1,
                });
                i = next;
                continue;
            }
        }
        raw.push_str(text);
        raw.push('\n');
        i += 1;
    }
    if !raw.is_empty() {
        nodes.push(Node::Markdown(raw));
    }
    Ok(nodes)
}
