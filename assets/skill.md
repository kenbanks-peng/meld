# Meld agent skill

Generate standalone HTML reports by writing UTF-8 `.meld` files: Markdown
plus components. No front matter, HTML wrapper, JavaScript, or external assets needed.

## Author
- Write the content directly to a `.meld` file; use Markdown headings, lists,
  links, tables, emphasis, and fenced code.
- Block component: `@name key=value` on its own line. Indent its Markdown body
  and nested components by exactly two spaces per level; dedent to close.
- Attributes are strings; quote values containing spaces: `title="My report"`.
- Inline components: `@badge[Markdown]`, `@status(tone=warning)[Watch]`.
  Inline content supports Markdown and nested inline components.
- Use only known components and supply their required attributes:
  - `hero title=...` (optional `eyebrow`), `panel title=...` (optional `label`).
  - `grid columns=2` (1–4); `metric label=... value=...` (optional `tone`).
  - `callout title=...` (optional `tone`); `summary`, `muted`, `masthead`, `footer`.
  - Inline: `badge`, `status` (default `tone=positive`).
  - `chart title=...`: Markdown table body; first column is categories,
    next 1–6 columns are finite, nonnegative numbers.
- Raw HTML is displayed as text. Code spans/blocks and escaped `\@name` stay literal.

## Example
Write this as `report.meld`:

```meld
@hero title="Weekly report" eyebrow="Engineering"
  **Healthy overall.** @badge[On track]

@panel title="Overview"
  @grid columns=2
    @metric label="Uptime" value=99.97% tone=teal
      Above target.

    @summary
      @status(tone=warning)[Watch latency]

@chart title="Requests"
  Day | Requests
  --- | ---
  Mon | 120
  Tue | 150
```

## Render
Run `meld report.meld` to create `report.html`; use `-o path.html` for another
file or `-o -` for stdout. Set the page title with `--title "Weekly report"`.
Fix reported syntax/attribute errors and rerun; errors leave output untouched.

Every run seeds `$XDG_CONFIG_HOME/meld` (fallback `~/.config/meld`) if absent.
Edit its config.toml, theme.css, or components/*.html to customize normal renders.
Existing files are never overwritten; rendering has no embedded design fallback.
For alternative designs, run `meld --init design` (new directory), edit its
config/CSS/templates, then render with `--config design/config.toml` (no merging).
`--components directory` overrides templates by filename. Config asset paths
resolve beside the config. Load only trusted CSS/templates; template bodies use
`{{ content|safe }}`, attributes use `{{ attrs.title }}` (auto-escaped).
