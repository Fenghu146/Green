# Note rendering: the rules this project follows

Status: draft for review. Scope: how a Markdown file on disk becomes the pixels in one
editor tab. Non-goals: export, plugins, and anything that writes to the file.

## 1. One file, one document

The editor reads a file as UTF-8 and renders it. There is no intermediate format, no
sidecar, no database. If a feature cannot be expressed in the file itself, it is a view
state and lives in memory only.

| Concern | Where it lives | Written back? |
| --- | --- | --- |
| Text | the file | yes, on save |
| Folding | memory, keyed by heading path | no |
| Outline | derived from headings on load | no |
| Scroll position | session file | no |

## 2. Blocks

A block is one of: heading, paragraph, list, quote, table, code fence, rule, or math.
Blocks are separated by blank lines. A block never spans two files, and nothing inside a
block is reflowed across block boundaries.

1. **Headings** are `#` through `######`; level decides the outline, nothing else.
2. **Paragraphs** keep hard breaks: two trailing spaces end a line inside the paragraph.
   A soft break is a rendering decision and never changes the file.
3. **Lists** nest by indentation. A paragraph inside a list item must be indented to the
   item's content column; anything less is a new item.
4. **Quotes** nest up to three levels; deeper input renders flat rather than fail.
5. **Tables** take their alignment from the delimiter row. A table without a delimiter
   row is a paragraph, as CommonMark says.

## 3. Inline

Inline spans are resolved per block, in one pass, from left to right:

- Emphasis: `*a*`, `**a**`, `***a***`, `~~a~~`.
- Code spans: backticks bind to the longest run, so `` `a ` b` `` keeps its inner backtick.
- Highlight: `==a==` is this project's one extension. It stays `==a==` in the file.
- Links: `[text](url)` and `[text][ref]`. A link whose URL is relative resolves against
  the document's directory, so a moved file breaks its own links on purpose.
- Math: `$inline$` renders with the body font size; `$$display$$` is its own block and is
  centered.

### 3.1 Escaping

A backslash escapes the next ASCII punctuation character and is removed from the output.
It does not escape a letter, and it does not survive a save: the file keeps the backslash.

### 3.2 Emphasis edge cases

The rules follow CommonMark's left/right-flanking definition, with one deviation: an
underscore inside a Chinese word is not emphasis, because `a_b` is far more often a
identifier than a half-written italic.

```text
输入                 输出
*斜体*               斜体
**粗体**             粗体
a\\_b                a_b
==高亮==             高亮（背景色）
```

## 4. Code

Fenced code blocks preserve whitespace exactly, including tabs and trailing spaces inside
the fence. The info string after the opening fence is lowercased and, if a highlighter
knows the language, used to colour tokens; unknown languages render plain.

Indented code (four spaces) is recognised only when it is not inside a list item.

## 5. Width and measure

The column is 48rem wide and centred. Long lines wrap at the column, not at the window:
a wider window gives more margin, not longer lines. Inline images scale down to the
column width and never scale up past their natural size.

## 6. Failures

Every case below renders the raw text rather than a blank area, and logs one line:

| Input | Behaviour |
| --- | --- |
| Unclosed fence | the rest of the file is code |
| Unclosed `$` | the `$` is text |
| Table with a ragged row | the short row is padded with empty cells |
| Image that does not load | the alt text, in a bordered box |

The rule behind the table: a note must always show something. Silent blank areas are how
people lose trust in an editor.
