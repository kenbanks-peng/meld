# Write a Meld file

Write UTF-8 Markdown plus components in `report.meld`. Render with
`meld report.meld` to create `report.html`; fix any reported errors and rerun.

## Syntax

- Blocks: `@name key=value` on its own line. Indent the body and nested
  components **two spaces** per level; dedent to close. Use spaces, not tabs.
- Inline: `@name[Markdown]` or `@name(key=value)[Markdown]` within text.
- Attributes are strings. Quote values containing spaces: `title="Weekly report"`.
  Single or double quotes work; backslashes escape characters inside quotes.
- Bodies support Markdown: headings, emphasis, lists, links, tables, and code.
  Code and escaped `\@name` stay literal. Raw HTML displays as text.
- Write content, not an HTML wrapper, front matter, CSS, or JavaScript.
  Use only the components below, in their listed mode. Required attributes
  must be supplied; omit defaults unless changing them.

## Components
