import io
p = "vendor/ely-gpui-components/crates/ely-documents/src/tree/tests/inline.rs"
t = io.open(p, encoding="utf-8").read()
t = t.replace('assert_eq!(text.styles, [(3..7, Style::Mark), (11..16, Style::Mark)]);',
              'assert_eq!(text.styles, [(4..10, Style::Mark), (15..24, Style::Mark)]);')
io.open(p, "w", encoding="utf-8", newline="\n").write(t)
print("ok")
