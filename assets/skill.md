# Write and render

1. Run `meld paths --project <project-directory>` (default: current directory).
2. Read absolute `source_dir` and `output_dir` from the returned JSON.
3. Write UTF-8 Markdown plus components to `<source_dir>/document.meld`.
4. Run `meld "<source_dir>/document.meld" --output "<output_dir>/document.html"`.
5. Fix rendering errors before opening the HTML. Keep sources and HTML in the returned folders, not the repository.

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
