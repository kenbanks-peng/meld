use crate::render::escape;
use anyhow::{Context, Result};

pub fn render(source: &str, title: &str) -> Result<String> {
    let rows: Vec<Vec<&str>> = source
        .lines()
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            s.trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect()
        })
        .collect();
    anyhow::ensure!(
        rows.len() >= 3 && rows[0].len() >= 2,
        "chart needs a Markdown table with numeric series"
    );
    let columns = rows[0].len();
    anyhow::ensure!(columns <= 7, "chart supports at most six series");
    anyhow::ensure!(
        rows[1].len() == columns
            && rows[1]
                .iter()
                .all(|s| s.contains('-') && s.chars().all(|c| matches!(c, '-' | ':' | ' '))),
        "invalid chart table separator"
    );
    let mut data = Vec::new();
    let mut max = 0.0_f64;
    for row in &rows[2..] {
        anyhow::ensure!(row.len() == columns, "inconsistent chart column count");
        let values = row[1..]
            .iter()
            .map(|v| {
                let n: f64 = v
                    .parse()
                    .with_context(|| format!("invalid chart number: {v}"))?;
                anyhow::ensure!(
                    n.is_finite() && n >= 0.0,
                    "chart values must be finite and nonnegative"
                );
                Ok(n)
            })
            .collect::<Result<Vec<_>>>()?;
        for v in &values {
            max = max.max(*v);
        }
        data.push(values);
    }
    let scale = if max == 0.0 { 1.0 } else { max };
    let title = escape(title);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 960 320\" role=\"img\" aria-label=\"{title}\"><title>{title}</title>"
    );
    for tick in 0..=4 {
        let y = 270 - tick * 60;
        let value = scale * (tick as f64 / 4.0);
        svg.push_str(&format!("<line x1=\"65\" x2=\"905\" y1=\"{y}\" y2=\"{y}\" class=\"chart-grid\"/><text x=\"55\" y=\"{}\" text-anchor=\"end\" class=\"chart-label\">{value:.1}</text>", y+4));
    }
    let group = 840.0 / data.len() as f64;
    let width = group * 0.72 / (columns - 1) as f64;
    for (i, values) in data.iter().enumerate() {
        let x = 65.0 + i as f64 * group;
        svg.push_str(&format!(
            "<text x=\"{}\" y=\"295\" text-anchor=\"middle\" class=\"chart-label\">{}</text>",
            x + group / 2.0,
            escape(rows[i + 2][0])
        ));
        for (j, value) in values.iter().enumerate() {
            let h = (value / scale) * 240.0;
            svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{h}\" rx=\"3\" class=\"series-{j}\"><title>{}, {}: {value}</title></rect>",
                x + group*0.14 + j as f64*width, 270.0-h, width*0.85, escape(rows[0][j+1]), escape(rows[i+2][0])));
        }
    }
    svg.push_str("</svg><div class=\"chart-legend\">");
    for (i, series) in rows[0][1..].iter().enumerate() {
        svg.push_str(&format!(
            "<span class=\"series-{i}\">{}</span>",
            escape(series)
        ));
    }
    svg.push_str("</div>");
    Ok(svg)
}
