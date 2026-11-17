# meld

**Mark Elements Down.**

Use Meld to supplement Markdown with component elements to produce self-contained HTML.

## Install and use

Requires Rust with edition 2024 support.

```sh
cargo install --path .
meld demo.meld --title "Engineering pulse · Q3 2026"
meld document.meld -o document.html
meld document.meld -o -                  # stdout
meld skill                            # authoring guide + active component examples
```

Without `-o`, output uses the input filename with an `.html` extension.
Errors leave the output untouched.

## Themes and components

Every run ensures `$XDG_CONFIG_HOME/meld` exists (falling back to
`~/.config/meld` when XDG_CONFIG_HOME is unset, empty, or relative). On first
use, Meld seeds `config.toml`, the full `theme.css`, and `components/*.html`.
Edit these files, then render normally:

```sh
meld document.meld
```

Rendering reads only external configuration, CSS, and templates. Embedded assets
are used only to seed new designs. Existing designs are never overwritten or
silently repaired; missing or invalid referenced files produce errors.

For an alternative design, export into a **new** directory:

```sh
meld --init design
meld demo.meld --config design/config.toml
```

`meld skill --config design/config.toml --components extra` documents the same\nactive components as rendering, including custom definitions and defaults.\n\n`--config` replaces the default design, rather than merging with it.
Theme and template paths resolve relative to the config, not the working directory.

A minimal standalone config (defines only `@notice`):

```toml
title = "My document"
lang = "en"

[theme.variables]
base = "#1e1e2e"
text = "#cdd6f4"
mauve = "#cba6f7"

[theme]
css = "theme.css" # optional; the complete stylesheet

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

Each `[components.NAME]` defines its template, required attributes, defaults,
and inline mode. The seeded config includes complete definitions for all starter
components. There are no implicit component definitions or inherited defaults.
Missing template attributes are errors.

Alternatively, `--components directory` loads all `*.html` files by component name,
overriding configured templates. New names default to block components.
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

The seeded stylesheet uses CSS variables. Override them in
`[theme.variables]` (applied after the stylesheet); edit the CSS for layout changes or
`:root { color-scheme: light; }`. The generated content-security policy permits
only the hashed stylesheet: no scripts, remote resources, or inline style attributes.

## Development

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```
