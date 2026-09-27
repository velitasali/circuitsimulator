# Guidelines & Architecture for Agents

This repository is a schematic capture and circuit simulation application ported from C++ / Qt to Rust / QML (`cs-app`, `cs-engine`, `cs-mcu`, `cs-script`, `cs-qemu`).

## 1. Color Theme & Palettes

All colors must be sourced from the central Color Theme (`crates/cs-engine/src/theme.rs`), which provides parity with the C++ `ColorTheme` class:
- **Never** hardcode ad-hoc hex color codes in components or canvas fa&ccedil;ades.
- Qt Quick color strings follow the `#AARRGGBB` format when 8 digits are used (the leading 2 hex digits represent alpha).
- Shaders and canvas items query `ColorTheme` via `CircuitCanvas` palette properties.

## 2. Pins & Visual Geometry

- **Pin lengths and contact**: Pin stems must span precisely up to the component boundary (`pin.plen`), without gaps and without protruding underneath component bodies.
- **Logic Direction Arrows**:
  - Input pins draw an inward chevron (`[ (0, -2), (2, 0), (0, 2) ]`).
  - Output pins draw an outward chevron (`[ (2, -2), (0, 0), (2, 2) ]`).
  - Chevrons stay visible when connected, replacing the hollow tip ring.
- **Translucent vs Opaque Bodies**: Components with translucent fills (like Resistors, BJTs, Diodes) rely on exact pin lengths so that stems do not render underneath the translucent body.

## 3. Testing & Verification

- Run `cargo fmt --all`, `cargo fix` (to clean up unused imports/warnings), and `cargo test --workspace` when I tell you to commit if you haven't already run them previously, and don't commit if there are test failures.

## 4. Rust Idioms & Import Conventions

- **Types, Structs, Enums & Traits**: Always import at the top of the file via `use` and reference by short name (e.g. `use circuit_canvas::CircuitCanvas;` and `.register::<CircuitCanvas>()`). Avoid inline qualification unless resolving an explicit name conflict.
- **Free Functions**: Prefer importing the parent module rather than the function itself (e.g. `use std::fs;` followed by `fs::read(...)`).

## 5. Architectural Changes & User Consultation

- **Consult Before Choosing Local Workarounds vs. Architectural Extensions**: When implementing a fix or feature where an architectural extension (e.g., extending core traits like `Draw`, introducing new primitives, or updating shared pipelines) would make present and future usages significantly simpler and more contained, do not silently proceed with complex local workarounds.
- **Ask the user first**: Explain the options and trade-offs (e.g., localized workaround vs. core architectural foundation) so the user can choose whether to implement the architectural change.

## 6. Typography & Text Components (`AppText`)

- **Always Use `AppText` Over `Text`**: All UI text in QML must use `AppText` instead of Qt Quick's built-in `Text`.
  - Qt Quick's `Text` only queries the application font upon creation and fails to update reactively when font family or size changes at runtime. `AppText` binds `font.family: App.fontFamily` and defaults `font.pixelSize: App.fontNormal` dynamically.
- **Omit Redundant Default Sizes**: Since `font.pixelSize: App.fontNormal` is already the default in `AppText`, do **not** redundantly assign `font.pixelSize: App.fontNormal` on `AppText` instances. Only set `font.pixelSize` when using non-default sizes (`App.fontSmall`, `App.fontMicro`, `App.fontTiny`, `App.fontTitle`, or explicit numbers).
- **Font Family Overrides**: Monospace or custom font overrides (e.g., `font.family: "Menlo"` or `"Ubuntu Mono"`) should be set directly on `AppText`, which cleanly overrides the default `App.fontFamily`.
- **Exceptions**:
  - `AppIcon.qml` remains a direct `Text` element because it specifically binds to the `"Material Symbols Rounded"` icon font.
  - Enum constants (`Text.WordWrap`, `Text.RichText`, `Text.ElideRight`, `Text.AlignVCenter`, etc.) and `TextMetrics` continue to use the Qt `Text` namespace.
- **Composite `uiFont` Properties**: When dialogs or views declare a `readonly property font uiFont` used by controls (such as `AppComboBox`, `AppButton`, or `AppTextField`), always include `family: App.fontFamily` (e.g., `Qt.font({ family: App.fontFamily, pixelSize: App.fontNormal })`) so they react to global font family changes.
