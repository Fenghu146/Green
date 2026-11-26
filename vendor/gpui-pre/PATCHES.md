# Green's patches to gpui-pre

Upstream: `gpui-pre` =0.3.14, as ElyOffice's `vendor/gpui-pre` keeps it. Green
adds two things.

## `shape-count` feature

`src/text_system/line_layout.rs` counts every line that is **actually shaped**
— a new `LineLayout` from the platform shaper — in a global `AtomicUsize`, with
`WindowTextSystem::shaped_lines()` and `reset_shaped_lines()`. A layout cache
hit is not a shape call and does not count; the counter therefore measures text
re-layout, not text drawing.

It compiles in only with the `shape-count` feature (off by default), so release
builds keep no counter on the shaping path.

Why: S2 proves that folding and unfolding change the visible set without
re-shaping any text — the count must read 0 across the measurement window.

## Kept lines across a fold

`LineLayoutCache::keep_frame()` (`WindowTextSystem::keep_frame`) copies the lines
drawn in the previous frame into a bounded kept set (4096 lines per window).
`layout_wrapped_line` and `layout_line` look there before shaping. A document
calls it when it folds a section: the hidden blocks' layouts stay alive, so
unfolding draws the same text without shaping it again. Kept layouts are
dropped when the font generation changes, and the set is cleared when it grows
past its bound, so memory cannot run away.

Why: design §5.1 says hidden blocks keep their layout and unfolding reuses it;
without this the line cache lives only one frame.

Rebase note: this is the third hand-maintained patch point when GPUI moves up
(the component-library fork and Green's own changes are the other two).
