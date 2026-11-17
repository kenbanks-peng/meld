# meld

**Mark Elements Down.**

Rust CLI that converts component-enhanced Markdown (`.meld`) into standalone,
responsive HTML. Includes the demo’s Catppuccin Mocha theme, Markdown tables,
and accessible SVG bar charts. No browser JavaScript or network assets.

## Install and use

Requires Rust with edition 2024 support.

```sh
cargo install --path .
meld demo.meld --title "Engineering pulse · Q3 2026"
meld report.meld -o report.html
meld report.meld -o -                  # stdout
```

Without `-o`, output uses the input filename with an `.html` extension.
Errors leave the output untouched.

## Themes and components

Export an editable starter design into a **new** directory:

```sh
meld --init design
meld demo.meld --config design/config.toml
```

Edit `design/config.toml`, `design/theme.css`, or any template in
`design/components/`. Theme and template paths resolve relative to the config,
not the working directory. Configuration is explicit; no files are auto-discovered.

A minimal config:

```toml
title = "My report"
lang = "en"

[theme.variables]
base = "#1e1e2e"
text = "#cdd6f4"
mauve = "#cba6f7"

[theme]
css = "extra.css" # optional; appended after the built-in stylesheet

[components.notice]
template = "notice.html"
required = ["title"]

[components.notice.defaults]
tone = "teal"
```

`notice.html` (MiniJinja syntax):

```html
<aside class="callout tone-{{ attrs.tone }}">
  <h3>{{ attrs.title }}</h3>
  {{ content|safe }}
</aside>
```

Then use:

```text
@notice title="Heads up"
  **Markdown** content, including nested components.
```

Attributes are strings. Template attributes are HTML-escaped automatically.
`content` is already-rendered HTML: use `{{ content|safe }}` to insert it.
Templates and CSS are **trusted application code**; do not load untrusted designs
or mark attribute values safe.

Override a built-in by using its name in `[components.NAME]`. Defaults and inline
mode are inherited; supplied defaults merge, and a nonempty `required` list
replaces the inherited list. Missing template attributes are errors.

Alternatively, `--components directory` loads all `*.html` files by component name,
overriding configured and built-in templates. New names default to block components.
Set `inline = true` in their config to support `@name[content]`.

## Meld syntax

Block components start on their own line. Indent their bodies by **two spaces**
per nesting level. Bodies support Markdown: headings, lists, links, code, tables,
and inline components.

```text
@panel label="01 / Overview" title="Summary"
  @grid columns=2
    @metric label="Uptime" value=99.97% tone=teal
      Above target.

    @summary
      **All systems operational.** @badge[On track]

  @status(tone=warning)[Watch latency]
```

Quote attribute values containing spaces with single or double quotes.
Inside quoted values, backslashes escape the following character.
Code spans, fenced/indented code blocks, and escaped `\@name[...]` remain literal.
Inline components support Markdown and nesting, but are not expanded inside links
or raw HTML. Raw Markdown HTML is displayed as text.

Built-ins:

| Component | Required attributes | Defaults / notes |
| --- | --- | --- |
| `masthead`, `footer` | — | Flex layout for paragraphs |
| `hero` | `title` | `eyebrow=""` |
| `grid` | — | `columns=2`; supports 1–4 |
| `metric` | `label`, `value` | `tone=mauve`; numeric suffix becomes a unit |
| `panel` | `title` | `label=""` |
| `summary`, `muted` | — | Subdued text |
| `callout` | `title` | `tone=teal` |
| `badge` | — | Inline: `@badge[Text]` |
| `status` | — | Inline; `tone=positive` |
| `chart` | `title` | Markdown table body; first column is categories |

Charts support 1–6 numeric series with finite, nonnegative values, including zero.
The rendered SVG includes a legend and an expandable data table.
Metric templates also receive `number` and `unit`; chart templates receive `chart`
(the generated SVG and legend).

The embedded stylesheet uses CSS variables from `assets/theme.css`.
Override them in `[theme.variables]`; use custom CSS for layout changes or
`:root { color-scheme: light; }`. The generated content-security policy permits
only the hashed stylesheet: no scripts, remote resources, or inline style attributes.

## Development

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```
