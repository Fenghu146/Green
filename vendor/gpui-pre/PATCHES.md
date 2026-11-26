# Green's patches to gpui-pre

Upstream: `gpui-pre` =0.3.14, as ElyOffice's `vendor/gpui-pre` keeps it. Green
adds one thing:

## `shape-count` feature

`src/text_system.rs` counts every `TextSystem::shape_line` call in a global
`AtomicUsize`, with `TextSystem::shaped_lines()` and `reset_shaped_lines()`.
It compiles in only with the `shape-count` feature (off by default), so release
builds keep no counter on the shaping path.

Why: S2 proves that folding and unfolding change the visible set without
re-shaping any text — the count must read 0 across the measurement window.

Rebase note: this is the third hand-maintained patch point when GPUI moves up
(the component-library fork and Green's own changes are the other two).
