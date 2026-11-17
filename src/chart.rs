use crate::render::escape;
use anyhow::{Context, Result};

pub fn render(source: &str, title: &str) -> Result<(String, Vec<String>)> {
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
    anyhow::ensure!(
        rows[1].len() == columns
            && rows[1]
                .iter()
                .all(|s| s.contains('-') && s.chars().all(|c| matches!(c, '-' | ':' | ' '))),
        "invalid chart table separator"
    );
    let mut data = Vec::new();
    let mut min = 0.0_f64;
    let mut max = 0.0_f64;
    for row in &rows[2..] {
        anyhow::ensure!(row.len() == columns, "inconsistent chart column count");
        let values = row[1..]
            .iter()
            .map(|v| {
                let n: f64 = v
                    .parse()
                    .with_context(|| format!("invalid chart number: {v}"))?;
                anyhow::ensure!(n.is_finite(), "chart values must be finite");
                Ok(n)
            })
            .collect::<Result<Vec<_>>>()?;
        for v in &values {
            min = min.min(*v);
            max = max.max(*v);
        }
        data.push(values);
    }
    // Normalize before subtracting so even opposite finite extremes cannot overflow.
    let scale = min.abs().max(max).max(f64::MIN_POSITIVE);
    let low = min / scale;
    let high = if min == max { 1.0 } else { max / scale };
    let position = |value: f64| 270.0 - (value / scale - low) / (high - low) * 240.0;
    let baseline = position(0.0);
    let title = escape(title);
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 960 320\" role=\"img\" aria-label=\"{title}\"><title>{title}</title>"
    );
    for tick in 0..=4 {
        let y = 270 - tick * 60;
        let fraction = tick as f64 / 4.0;
        let value = min * (1.0 - fraction) + max * fraction;
        svg.push_str(&format!("<line x1=\"65\" x2=\"905\" y1=\"{y}\" y2=\"{y}\" class=\"chart-grid\"/><text x=\"55\" y=\"{}\" text-anchor=\"end\" class=\"chart-label\">{value:.1}</text>", y+4));
    }
    svg.push_str(&format!(
        "<line x1=\"65\" x2=\"905\" y1=\"{baseline}\" y2=\"{baseline}\" class=\"chart-grid\"/>"
    ));
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
            let y = position(*value);
            let h = (y - baseline).abs();
            svg.push_str(&format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{h}\" rx=\"3\" class=\"series-{j}\"><title>{}, {}: {value}</title></rect>",
                x + group*0.14 + width*0.075 + j as f64*width, y.min(baseline), width*0.85, escape(rows[0][j+1]), escape(rows[i+2][0])));
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
    let styles = (6..columns - 1)
        .map(|i| {
            let hue = (i as f64 * 137.507764) % 360.0;
            format!(".series-{i} {{ --series: hsl({hue:.3} 65% 72%); }}")
        })
        .collect();
    Ok((svg, styles))
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn category_labels_are_centered_over_bar_groups() {
        for series_count in [1, 2, 3] {
            let mut source = String::from("Category");
            for series in 0..series_count {
                source.push_str(&format!(" | Series {series}"));
            }
            source.push_str(&format!("\n---{}\n", " | ---".repeat(series_count)));
            for category in ["Unit tests", "Integration tests"] {
                source.push_str(category);
                source.push_str(&" | 3".repeat(series_count));
                source.push('\n');
            }

            let (html, _) = render(&source, "Test").unwrap();
            let attribute = |element: &str, name: &str| -> f64 {
                element
                    .split_once(&format!("{name}=\""))
                    .unwrap()
                    .1
                    .split('"')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap()
            };
            let labels: Vec<_> = html
                .split("<text ")
                .skip(1)
                .filter(|element| element.contains("text-anchor=\"middle\""))
                .map(|element| attribute(element, "x"))
                .collect();
            let bars: Vec<_> = html.split("<rect ").skip(1).collect();
            assert_eq!(labels.len(), 2);
            assert_eq!(bars.len(), 2 * series_count);
            for (label, group) in labels.iter().zip(bars.chunks(series_count)) {
                let left = attribute(group[0], "x");
                let last = group.last().unwrap();
                let right = attribute(last, "x") + attribute(last, "width");
                assert!((label - (left + right) / 2.0).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn finite_values_produce_finite_bounded_geometry() {
        for values in [
            vec![0.0, 0.0],
            vec![-10.0, -2.0],
            vec![-1.0, 1.0],
            vec![1.0, 10.0],
            vec![-f64::MAX, f64::MAX],
            vec![f64::from_bits(1), 0.0],
            vec![-f64::from_bits(1), f64::from_bits(1)],
        ] {
            let mut source = String::from("Category | Value\n--- | ---\n");
            for (i, value) in values.iter().enumerate() {
                source.push_str(&format!("{i} | {value}\n"));
            }
            let (html, _) = render(&source, "Test").unwrap();
            let bars: Vec<_> = html.split("<rect ").skip(1).collect();
            assert_eq!(bars.len(), values.len());
            for bar in bars {
                let attribute = |name: &str| -> f64 {
                    bar.split_once(&format!("{name}=\""))
                        .unwrap()
                        .1
                        .split('"')
                        .next()
                        .unwrap()
                        .parse()
                        .unwrap()
                };
                let y = attribute("y");
                let height = attribute("height");
                assert!(y.is_finite() && (30.0..=270.0).contains(&y), "{y}");
                assert!(
                    height.is_finite() && (0.0..=240.0).contains(&height),
                    "{height}"
                );
                assert!(y + height <= 270.0);
            }
        }
    }
}
