const SOURCE = "https://github.com/ThomGardiner/sigil/blob/main/";

const playgroundMissing = new Set([
  "chart", "badge", "code-edit", "color-field", "combobox", "command-menu",
  "date-field", "dot", "axis-drag", "drag-ghost", "floating-text", "gauge",
  "html", "icon", "image-rgba", "image-bytes", "markdown", "markdown-edit",
  "mermaid", "heatmap", "validated-field", "radio-group", "surface", "table",
  "terminal", "toast", "toast-queue", "transport", "turn-strip", "path",
  "virtual-list-bind", "app-shell", "kv-grid",
]);

const exampleMissing = new Set([
  "chart", "code-edit", "color-field", "combobox", "command-menu", "date-field",
  "floating-text", "html", "icon", "image-rgba", "image-bytes",
  "markdown", "markdown-edit", "mermaid", "heatmap", "validated-field",
  "toast", "toast-queue", "turn-strip", "kv-grid",
]);

const previews = {
  button: "button_primary.png",
  checkbox: "checkbox.png",
  chip: "chips.png",
  dropdown: "dropdown.png",
  gauge: "gauge.png",
  progress: "progress.png",
  "radio-group": "radio_group.png",
  slider: "slider.png",
  tabs: "tabs.png",
  "text-field": "text_field.png",
  toggle: "toggle.png",
};

function component(category, row) {
  const [slug, name, description, code, file, _visualFixture = null, note = null] = row;
  const coverage = !playgroundMissing.has(slug)
    ? "playground"
    : exampleMissing.has(slug) ? "tests" : "example";
  return {
    slug,
    name,
    category,
    description,
    code,
    file: `crates/widgets/src/${file}`,
    source: `${SOURCE}crates/widgets/src/${file}`,
    preview: previews[slug] ?? null,
    coverage,
    note,
  };
}

const groups = [
  ["Content & foundation", [
    ["label", "Label", "Styled text for interface copy, values, and headings.", "label(\"Ready\")", "lib.rs"],
    ["icon", "Icon", "A glyph with inherited size, tone, and icon-font styling.", "icon(\"⌕\")", "icon.rs", "icon_primary_tone.png"],
    ["path", "Path", "Build and fill or stroke vector paths in the retained tree.", "path(48.0, 48.0)\n    .move_to(8.0, 40.0)\n    .line_to(24.0, 8.0)\n    .line_to(40.0, 40.0)\n    .close()", "vector.rs", "vector_path_shapes.png"],
    ["image-rgba", "Image RGBA", "Display decoded RGBA pixels with accessible alternate text.", "image_rgba(width, height, pixels).map(|image| image.alt(\"Preview\"))", "image.rs"],
    ["image-bytes", "Image bytes", "Decode encoded image bytes before placing them in the tree.", "image_bytes(bytes).map(|image| image.alt(\"Preview\"))", "image.rs", null, "Requires the image-decode feature."],
    ["html", "HTML", "Render the framework's supported HTML subset as native content.", "html(source)", "html.rs"],
    ["markdown", "Markdown", "Render Markdown through Sigil's owned text and layout pipeline.", "markdown(source)", "markdown.rs"],
    ["markdown-edit", "Markdown edit", "Edit Markdown source with rendered-document behavior.", "markdown_edit(text.bind())", "markdown.rs"],
    ["mermaid", "Mermaid", "Render supported Mermaid diagrams as native vector content.", "mermaid(source)", "mermaid.rs"],
  ]],
  ["Actions & status", [
    ["button", "Button", "Trigger an action with hover, pressed, focus, and disabled states.", "button(\"Save\").activate(|| save())", "button.rs", "button_default.png"],
    ["icon-button", "Icon button", "Compact glyph action with an accessible label.", "icon_button(\"⌕\", || open()).accessible(\"Search\")", "button.rs", "button_focused.png"],
    ["chip", "Chip", "Small semantic label for state, category, or metadata.", "chip(\"Stable\", Tone::Success)", "chip.rs", "chip_success.png"],
    ["badge", "Badge", "Compact count or status indicator built on the chip primitive.", "badge(\"12\", Tone::Info)", "chip.rs", "chip_info.png"],
    ["dot", "Dot", "Minimal semantic status marker with configurable tone.", "dot(Tone::Success)", "dot.rs", "dot_success.png"],
    ["progress", "Progress", "Linear determinate progress for a bounded operation.", "progress(value, 1.0)", "progress.rs", "progress_half.png"],
    ["gauge", "Gauge", "Compact value display for monitored ranges and warning thresholds.", "gauge(value).max(100.0).label(\"CPU\").warn(0.2)", "gauge.rs", "gauge_low_warn.png"],
    ["toast", "Toast", "Display one transient message from a reactive optional string.", "toast(message)", "toast.rs"],
    ["toast-queue", "Toast queue", "Own and render several transient messages in order.", "toast_queue(queue)", "toast_queue.rs"],
  ]],
  ["Inputs", [
    ["checkbox", "Checkbox", "Toggle a boolean value with keyboard and pointer input.", "checkbox(checked).label(\"Remember me\")", "checkbox.rs", "checkbox_default.png"],
    ["toggle", "Toggle", "Switch a boolean setting with an animated state change.", "toggle(enabled).label(\"Notifications\")", "toggle.rs", "toggle_hover_settled.png"],
    ["slider", "Slider", "Select a numeric value across a bounded range.", "slider(volume, 0.0..=1.0)", "slider.rs", "slider_default.png"],
    ["text-field", "Text field", "Single-line text input with selection, focus, and IME support.", "text_field(name).placeholder(\"Project name\")", "text_field.rs", "text_field_default.png"],
    ["text-edit", "Text edit", "Multiline editor with selection, scrolling, and IME support.", "text_edit(notes)", "text_edit.rs", "text_edit_multiline.png"],
    ["number-field", "Number field", "Numeric entry with parsing, focus, and bounded values.", "number_field(count, 0..=100)", "number_field.rs", "number_field_default.png"],
    ["color-field", "Color field", "Edit and validate an RGBA color value.", "color_field(color)", "color_field.rs"],
    ["date-field", "Date field", "Structured date input with field-level validation.", "date_field(date)", "date_field.rs"],
    ["dropdown", "Dropdown", "Choose one value from a compact anchored option list.", "dropdown(selected, options)", "dropdown.rs", "dropdown_default.png"],
    ["combobox", "Combobox", "Filter and choose from a larger set of options.", "combobox(Combobox::new(options).placeholder(\"Choose…\"))", "combobox.rs"],
    ["radio-group", "Radio group", "Choose one value from a visible set of labeled options.", "radio_group(selected)\n    .option(\"light\", \"Light\")\n    .option(\"dark\", \"Dark\")", "radio_group.rs", "radio_group_three_options_first_selected.png"],
  ]],
  ["Navigation & overlays", [
    ["tabs", "Tabs", "Switch between peer views with focus and keyboard navigation.", "tabs(active, vec![\"Files\".into(), \"Search\".into()])", "tabs.rs", "tabs_default.png"],
    ["menu", "Menu", "Anchored actions, separators, disabled items, and nested submenus.", "menu(open).label(\"File\").item(MenuItem::new(\"Save\").select(|| save()))", "menu.rs", "menu_open.png"],
    ["menu-bar", "Menu bar", "Desktop-style top-level menus backed by the menu primitive.", "menu_bar().menu(\"File\", [MenuItem::new(\"Open\").select(|| open())])", "menu/bar.rs"],
    ["command-menu", "Command menu", "Keyboard-driven nested action menu with disabled rows and cancellation.", "command_menu().item(\"Open\", || open())", "command_menu.rs", "command_menu_open.png"],
    ["tooltip", "Tooltip", "Contextual text anchored to hover or keyboard focus.", "tooltip(\"Refresh data\").child(button(\"Refresh\"))", "tooltip.rs", "tooltip_with_keyboard_shortcut.png"],
    ["modal", "Modal", "Focused overlay with backdrop, dismissal, and focus containment.", "modal(open).title(\"Settings\").child(content)", "modal.rs", "modal_open.png"],
    ["sheet", "Sheet", "Edge-mounted modal surface for secondary tasks.", "sheet(open).side(Side::Right).child(content)", "modal.rs", "modal_dismissing.png"],
    ["hint-overlay", "Hint overlay", "Temporary in-context keyboard hints over existing content.", "patterns::hint_overlay(&hints)", "patterns.rs", "hint_overlay_default.png"],
  ]],
  ["Data & complex views", [
    ["chart", "Chart", "Plot one or more bar or line series with labels and semantic tones.", "chart(values).line().labels(labels)", "chart.rs"],
    ["table", "Table", "Virtualized rows with columns, sorting, selection, and horizontal scroll.", "table(state)", "table.rs", "table_default.png"],
    ["virtual-list", "Virtual list", "Render only the visible window of a large collection.", "virtual_list(items, |index, item| row(index, item))", "virtual_list/mod.rs", "virtual_list_default.png"],
    ["virtual-list-bind", "Virtual list bind", "Bind a changing collection to the virtualized list window.", "sigil::widgets::virtual_list_bind(items, row)", "virtual_list/mod.rs", "virtual_list_items_bind.png", "Public in sigil::widgets; not re-exported directly by the prelude."],
    ["code-edit", "Code edit", "Code-oriented editor with caret, line chrome, and synchronized scrolling.", "code_edit(lines, caret)", "code_edit.rs"],
    ["terminal", "Terminal", "Render windowed terminal rows and send typed input to a host session.", "terminal(lines, on_input)", "terminal.rs"],
  ]],
  ["Layout & composition", [
    ["panel", "Panel", "Group content on a themed surface with optional title and elevation.", "panel().title(\"Inspector\").child(content)", "panel.rs", "panel_default.png"],
    ["scroll", "Scroll", "Clip and scroll content while retaining input and geometry state.", "scroll(content, 800.0)", "scroll.rs", "scroll_default.png"],
    ["splitter", "Splitter", "Resize adjacent regions along a horizontal or vertical axis.", "splitter(SplitterOrientation::Horizontal)\n    .child(left, 280.0)\n    .child(right, 720.0)", "splitter.rs", "splitter_horizontal_default.png"],
    ["surface", "Surface", "Reserve a layout hole for a native view identified by a host key.", "surface(1)", "surface.rs"],
    ["card", "Card", "A conventional padded surface for a discrete content group.", "card(content)", "patterns.rs"],
    ["sidebar", "Sidebar", "Place content in a fixed-width navigation surface.", "sidebar(navigation)", "patterns.rs"],
    ["avatar", "Avatar", "Compact identity mark built from short initials.", "avatar(\"TG\")", "patterns.rs"],
    ["field", "Field", "Pair a label, control, hint, and required state.", "field(\"Name\", text_field(name.bind()))", "patterns.rs", "pattern_field.png"],
    ["validated-field", "Validated field", "Add inline validation feedback to the field pattern.", "validated_field(\"Email\", text_field(email), error)", "patterns.rs", "validated_field_with_error.png"],
    ["app-shell", "App shell", "Arrange top chrome, work area, status, and an optional modal.", "patterns::app_shell(top, work, status, None::<NodeBuilder>)", "patterns.rs", "pattern_app_shell.png"],
    ["kv-grid", "Key/value grid", "Align several static labeled values into a compact inspection grid.", "patterns::kv_grid(&[(\"Status\", \"Ready\")])", "patterns.rs", "pattern_kv_grid.png"],
    ["kv-row", "Key/value row", "One labeled value row for metadata and inspectors.", "kv_row(\"Status\", label(\"Ready\"))", "composition.rs", "kv_row_default.png"],
    ["status-bar", "Status bar", "Compact application status and metadata strip.", "status_bar(items)", "composition.rs", "status_bar_default.png"],
    ["divider-horizontal", "Horizontal divider", "Separate stacked regions with a one-dimensional rule.", "divider_horizontal()", "divider.rs", "divider_horizontal_default.png"],
    ["divider-vertical", "Vertical divider", "Separate adjacent regions with a one-dimensional rule.", "divider_vertical()", "divider.rs", "divider_vertical_default.png"],
  ]],
  ["Interaction & special", [
    ["interactive", "Interactive", "Opt a plain node into hover, pressed, focus, and disabled style states.", "interactive(container()).class(\"p-4 hover:bg-accent-9\")", "interactive.rs"],
    ["axis-drag", "Axis drag", "Attach capture-based pointer motion along one axis to a node.", "axis_drag(handle, Axis::X, snapshot, on_delta, on_end)", "drag.rs"],
    ["drag-ghost", "Drag ghost", "Render transient feedback while moving content between targets.", "drag_ghost(drag_state, |payload| preview(payload))", "drag_drop.rs"],
    ["floating-text", "Floating text", "Render short-lived animated text from a reactive spawn stream.", "floating_text(spawn)", "floating_text.rs"],
    ["heatmap", "Heatmap", "Generate a color field for debugging or dense scalar data.", "sigil::widgets::overlay::heatmap(width, height, |x, y| sample(x, y))", "overlay/heatmap.rs", null, "Public in sigil::widgets::overlay; not re-exported directly by the prelude."],
    ["transport", "Transport", "Playback controls with reactive progress and seeking.", "transport(playing, progress, toggle).seek(seek)", "transport.rs"],
    ["turn-strip", "Turn strip", "Show an ordered set of turns with one active entry.", "turn_strip(entries).active(active)", "turn_strip.rs", "turn_strip_active_highlight.png"],
  ]],
];

export const components = groups.flatMap(([category, rows]) =>
  rows.map((row) => component(category, row))
);

export const categories = groups.map(([name, rows]) => ({ name, count: rows.length }));

export const patterns = [
  { slug: "application-shell", name: "Application shell", description: "Top chrome, optional navigation, resizable work area, and persistent status.", components: ["app-shell", "menu-bar", "splitter", "status-bar"] },
  { slug: "validated-form", name: "Validated form", description: "Labeled controls with hints, required state, and inline errors.", components: ["field", "validated-field", "text-field", "button"] },
  { slug: "data-workspace", name: "Data workspace", description: "Virtualized records with sorting, selection, filters, and detail panes.", components: ["table", "virtual-list", "splitter", "kv-grid"] },
  { slug: "command-workflow", name: "Command workflow", description: "Keyboard-first discovery with nested actions and transient feedback.", components: ["command-menu", "menu", "toast-queue", "tooltip"] },
  { slug: "media-controls", name: "Media controls", description: "Playback state, seeking, elapsed progress, and compact status.", components: ["transport", "slider", "progress", "icon-button"] },
];

export const showcases = [
  {
    slug: "workbench",
    name: "Workbench",
    description: "A dense desktop workspace with navigation, a work log, document content, composition controls, and status chrome.",
    screenshot: "workbench.png",
    dimensions: "1180 × 780 logical pixels",
    alt: "Sigil Workbench running in a native macOS window with a project sidebar, document thread, controls, and composer",
    command: "cargo run -p workbench",
  },
  {
    slug: "widget-playground",
    name: "Widget Playground",
    description: "The native component inventory showing form, display, navigation, container, and reactive states in one scrollable window.",
    screenshot: "widget-playground.png",
    dimensions: "1100 × 800 logical pixels",
    alt: "Sigil Widget Playground running in a native macOS window with form and display widgets",
    command: "cargo run -p widget-playground",
  },
  {
    slug: "todo",
    name: "Todo",
    description: "A compact native application with GPU menu chrome, editable tasks, filtering, persistence, file dialogs, and a system tray.",
    screenshot: "todo.png",
    dimensions: "720 × 560 logical pixels",
    alt: "Sigil Todo running in a native macOS window with task input, filters, task rows, and status controls",
    command: "cargo run -p todo",
  },
];

const app = (slug, kind, description, command = `cargo run -p ${slug}`, folder = slug) => ({
  slug,
  name: slug.split("-").map((word) => word[0].toUpperCase() + word.slice(1)).join(" "),
  kind,
  description,
  command,
  folder: `examples/${folder}`,
  source: `${SOURCE}examples/${folder}`,
});

export const apps = [
  app("access-log", "Performance", "Million-row live tape with 60 × 8 visible cells updated every 16 ms.", "cargo run --release -p access-log --bin access-log-sigil"),
  app("data-table", "Performance", "A parity build of GPUI 0.2.2's 10,000-row, 24-column data table.", "cargo run --release -p data-table"),
  app("widget-playground", "Inventory", "Interactive coverage for 32 public constructors and common states."),
  app("vertical-slice", "Full path", "Tree, layout, signals, events, focus, scene emission, and rendering in one dashboard."),
  app("todo", "Native app", "Persistence, native file dialogs, system tray, external links, and GPU menu chrome."),
  app("player", "Native app", "Audio playback, transport controls, reactive progress, and seeking.", "cargo run -p player -- /path/to/track"),
  app("pty-pane", "Native app", "A live terminal process beside ordinary widgets in a resizable split view."),
  app("strays", "TUI", "Terminal inventory of local servers, source trees, owners, ports, and ages."),
  app("api-explorer", "App study", "REST client layout with endpoint selection and routed request and response tabs."),
  app("file-browser", "App study", "Virtualized paths with preview, metadata, and hex panes."),
  app("chat-client", "App study", "Channels, messages, members, composition, split panes, tabs, and settings."),
  app("kanban-board", "App study", "Three columns, drag-and-drop cards, project navigation, and an edit modal."),
  app("workbench", "App study", "Dense desktop workspace with navigation, work log, composition, and diffs."),
  app("csv-viewer", "Utility", "Six-column virtualized trade-ticket table with fixed headers and selection."),
  app("log-tailer", "Utility", "Virtualized log viewer with filters, selection, and bottom-pinned scrolling."),
  app("regex-tester", "Utility", "Derived matches and inline parse errors from editable pattern and sample text."),
  app("scratchpad", "Utility", "Editable buffer with derived dirty state, save, revert, clear, and submit paths."),
  app("password-generator", "Utility", "Slider and character-class controls with derived output."),
  app("timer-stopwatch", "Utility", "Start, stop, lap, reset, and interval-driven time state."),
  app("hello-counter", "Foundation", "Minimum reactive window with one signal and one handler."),
  app("hello-events", "Foundation", "Pointer hit testing, keyboard focus, and event routing."),
  app("hello-style", "Foundation", "Style values, layout, and theme-driven rendering."),
  app("linear-row", "Foundation", "A small row-composition and layout proof."),
  app("hello-host", "Foundation", "Host lifecycle and native window integration."),
  app("hello-tree", "Foundation", "Retained tree construction and updates."),
  app("hello-layout", "Foundation", "Owned layout primitives and constraints."),
  app("hello-tui", "Foundation", "The same UI model hosted in a terminal."),
  app("hello-rect", "Internal", "Low-level rectangle rendering proof.", "cargo run -p hello-rect", "internals/hello-rect"),
];

export const screenshotBase = "assets/screenshots/";

export const coverageLabels = {
  playground: "In playground",
  example: "Runnable example",
  tests: "Test coverage only",
};
