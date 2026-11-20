# Write a Meld file

Use Meld to supplement Markdown with component elements to produce self-contained HTML.

Run `meld paths --project <project-directory>` (defaults to the current directory).
Meld creates global project storage and prints JSON containing absolute
`source_dir` and `output_dir` paths. Write UTF-8 Markdown plus components to
`<source_dir>/document.meld`, then render with
`meld "<source_dir>/document.meld" --output "<output_dir>/document.html"`.
Keep sources and HTML in these folders, not in the project repository.

## Syntax

- Blocks: `@name key=value` on its own line. Indent the body and nested
  components **two spaces** per level; dedent to close. Use spaces, not tabs.
- Inline: `@name[Markdown]` or `@name(key=value)[Markdown]` within text.
- Attributes are strings. Quote values containing spaces: `title="Getting started"`.
  Single or double quotes work; backslashes escape characters inside quotes.
- Bodies support Markdown: headings, emphasis, lists, links, tables, and code.
  Code and escaped `\@name` stay literal. Raw HTML displays as text.
- Write content, not an HTML wrapper, front matter, CSS, or JavaScript.
  Use only the components below, in their listed mode. Required attributes
  must be supplied; omit defaults unless changing them.

## Components
