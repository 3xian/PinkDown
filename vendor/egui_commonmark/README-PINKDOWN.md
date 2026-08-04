# PinkDown renderer patch

This is `egui_commonmark` 0.21.1 with a deliberately small rendering-only patch.
The upstream `pulldown-cmark` event stream remains the sole Markdown parser.

PinkDown changes:

- responsive, wrapping table rows rendered from upstream `TableCell` events;
- fenced-code framing with PinkDown-compatible padding, radius, and shadow;
- optional `Heading1` through `Heading6` named egui text styles;
- heading colors sourced from existing egui visual roles.

No source-text splitting or secondary Markdown grammar is used. When upgrading the
crate, reapply the changes in `src/parsers/pulldown.rs` and run the workspace tests.
