# Third-party source overrides

## glib 0.18.5

This directory contains the crates.io release of `glib` 0.18.5, licensed under MIT
(see `glib/LICENSE` and `glib/COPYRIGHT`), with the two-line memory-safety fix from
[gtk-rs-core commit 05dff0e](https://github.com/gtk-rs/gtk-rs-core/commit/05dff0ee696f9bcd8617cd48c4b812d046d440cb).
The original Cargo registry cache markers are omitted. Other upstream sources
are unchanged.

The fix makes the `g_variant_get_child` output pointer mutable and passes it as
`&mut p`. It addresses [GHSA-wrw7-89jp-8q8g](https://github.com/advisories/GHSA-wrw7-89jp-8q8g)
(RUSTSEC-2024-0429), which affects `VariantStrIter` under optimization.

Tauri 2's Linux GTK3 stack requires glib 0.18 and cannot use the fixed 0.20+
version without upstream compatibility changes. The workspace's `[patch.crates-io]`
selects this source for that existing dependency. Its version remains 0.18.5;
version-only scanners can continue reporting the advisory. Do not suppress the
advisory globally or claim that crates.io 0.18.5 is fixed.

CI runs `glib_variant_strings_backport` in the Linux GUI's release profile,
exercising forward and backward iteration, Unicode and empty strings. Keep this
override until the stable Tauri dependency graph uses a fixed glib version, then
remove both the override and vendored copy after checking the optimized test.
