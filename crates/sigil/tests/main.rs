#![allow(
    unused_crate_dependencies,
    reason = "the test binary does not use every package dependency"
)]

use sigil::{events, geometry};

mod events_support {
    pub(crate) mod input;
    pub(crate) mod mock_shell;
    pub(crate) mod routing_helpers;
}
#[path = "layout_stub/mod.rs"]
mod layout_stub;
#[path = "renderer_child_paint_repro/support.rs"]
mod paint_support;
#[path = "scene_support/mod.rs"]
mod scene_support;
#[path = "widget_support/mod.rs"]
mod widget_support;

mod serial {
    use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

    static PROFILE: Mutex<()> = Mutex::new(());
    static KEYFRAMES: RwLock<()> = RwLock::new(());
    static VEGA: RwLock<()> = RwLock::new(());

    pub(crate) fn profile() -> MutexGuard<'static, ()> {
        PROFILE.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn keyframes() -> RwLockReadGuard<'static, ()> {
        shared(&KEYFRAMES)
    }

    pub(crate) fn keyframes_alone() -> RwLockWriteGuard<'static, ()> {
        alone(&KEYFRAMES)
    }

    pub(crate) fn vega() -> RwLockReadGuard<'static, ()> {
        shared(&VEGA)
    }

    pub(crate) fn vega_alone() -> RwLockWriteGuard<'static, ()> {
        alone(&VEGA)
    }

    fn shared(lock: &'static RwLock<()>) -> RwLockReadGuard<'static, ()> {
        lock.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn alone(lock: &'static RwLock<()>) -> RwLockWriteGuard<'static, ()> {
        lock.write().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(all(unix, feature = "terminal", feature = "pty"))]
fn child_args(module: &str, test: &str) -> [String; 4] {
    let module = module.split_once("::").map_or(module, |(_, inner)| inner);
    [
        format!("{module}::{test}"),
        "--exact".to_owned(),
        "--nocapture".to_owned(),
        "--test-threads=1".to_owned(),
    ]
}

mod a11y_a11y_snapshots;
mod a11y_audit;
mod a11y_bounds_projection;
mod a11y_live_regions;
mod a11y_reactive_flow;
mod a11y_rows_and_fields;
mod a11y_transform_bounds;
#[cfg(feature = "a2ui")]
mod a2ui_catalog;
#[cfg(feature = "a2ui")]
mod a2ui_review;
mod allocation_reuse;
mod animated_goldens;
mod animation_runtime_integration;
mod api_one_spelling;
mod app_api_fixes;
mod app_dpi_integration;
mod app_error;
mod app_host_layout;
mod app_run_defects;
mod app_scale_handling;
mod app_snapshot;
mod app_text_measure;
mod app_tree_root;
mod binding_animate_engine;
mod blend;
mod builder_ergonomics;
mod card_title;
mod chart_negative;
mod chart_scale;
mod chart_vega;
mod chart_vega_axes;
mod chart_vega_paint;
mod click_present;
mod clipboard_image;
mod clips;
mod code_block_bound;
mod code_edit_interaction;
mod code_edit_paging;
mod command_menu;
mod design;
mod design_sweep;
mod diagrams_review;
mod diff_widget;
mod drag_axis;
mod drag_drop;
mod drag_drop_fast;
mod dropdown_border_click;
mod dropdown_command;
mod dropdown_overflow;
mod editor_swap_probe;
mod embed_route;
mod error_kind;
mod events_a11y_action_dispatch;
mod events_capture_requester;
mod events_perf;
mod events_routing;
mod events_routing_dpad;
mod events_routing_drag;
mod events_routing_focus;
mod events_routing_focus_scope;
mod events_routing_ime;
mod events_routing_keyboard;
mod events_routing_scroll;
mod events_style_reactive_integration;
mod field_deep_position;
mod file_drop_route;
mod filters;
mod flex_text_min_size;
mod frame_driver;
mod gauge;
mod golden_support_ticks;
mod hal_shell_logical;
mod host_frame_updates;
mod i18n_theme;
mod icons_svg_winding;
mod image_decode;
#[cfg(feature = "image-decode")]
mod image_jpeg;
mod image_load;
mod imaging_review;
mod imaging_review_wave2;
mod incremental_scroll_position;
mod input_api;
mod interactive_state;
mod kernel_accesskit_id_stability;
mod kernel_composite_component;
mod kernel_dirty_tracking;
mod kernel_extensions;
mod kernel_fragment_debug_asserts;
mod kernel_owner_disposal;
mod kernel_reactive_children;
mod kernel_reactive_scopes;
mod kernel_scale;
mod kernel_semantics_defaults;
mod kernel_tree_shape;
mod layer_cache_images;
mod layout_absolute_inset;
mod layout_attach_intent;
mod layout_block_layout;
mod layout_compute_basic;
mod layout_dispose;
mod layout_engine;
mod layout_fill;
mod layout_fixture_suite;
mod layout_flex_auto_cross_hug;
mod layout_flex_auto_main_hug;
mod layout_flex_block_item_grows;
mod layout_flex_clamp_redistribution;
mod layout_flex_cross_basis_column;
mod layout_flex_fill_content;
mod layout_flex_grow_sibling_no_stretch;
mod layout_flex_grow_with_children;
mod layout_flex_intrinsic_main;
mod layout_flex_intrinsic_no_stretch;
mod layout_flex_intrinsic_padding;
mod layout_flex_item_grow_not_child_grow;
mod layout_flex_layout;
mod layout_grid_fixture;
mod layout_hardening;
mod layout_incremental;
mod layout_incremental_cache_no_stale;
mod layout_incremental_position;
mod layout_invariants;
mod layout_overflow_storage;
mod layout_per_subtree_dispatch;
mod layout_profile;
mod layout_reactive_children_invalidation;
mod layout_reactive_position;
mod layout_reuse;
mod layout_style_integration;
mod layout_text_inheritance;
mod layout_text_measure;
mod link;
mod list_end_reached;
mod log_view;
mod markdown_links;
mod markdown_stream;
#[cfg(feature = "terminal")]
mod markdown_terminal;
mod masks;
mod math_coverage;
mod modal_announce;
mod modal_layout;
mod modal_scroll;
mod modal_trap;
mod motion;
mod motion_alloc;
mod motion_props;
mod motion_review;
mod number_field_commit;
mod paints;
mod patterns_status;
mod phase_c_reactive_attrs;
mod prelude_surface;
mod primitives;
mod progress_cells;
mod progress_command;
mod radio_group_keyboard;
mod range_slider;
mod reactive_bug_fixes;
mod reactive_bug_fixes_p1;
mod reactive_debug_tools;
mod reactive_derived;
mod reactive_drain_budget;
mod reactive_drawphase;
mod reactive_edge_cases;
mod reactive_panic_recovery;
mod reactive_props;
mod reactive_reactive_contract;
mod reactive_signal_ops;
mod reactive_trace_log;
mod reactive_trigger;
mod render_repro;
mod renderer_blur_command;
mod renderer_blur_pixel_check;
mod renderer_border_paint;
mod renderer_caller_device;
mod renderer_child_paint_repro;
mod renderer_cpu_parity;
mod renderer_damage;
mod renderer_decoration_command;
mod renderer_decoration_pixel_check;
mod renderer_differential;
mod renderer_emit_cull;
mod renderer_frame_upload;
mod renderer_glow_command;
mod renderer_glow_pixel_check;
mod renderer_glyph_pixel_check;
mod renderer_gpu_coverage_gate;
mod renderer_gradient_command;
mod renderer_group_opacity;
mod renderer_icon_command;
mod renderer_icon_pixel_check;
mod renderer_image_paint;
mod renderer_inner_shadow_command;
mod renderer_layer_command;
mod renderer_layer_pixel_check;
mod renderer_layer_pool_properties;
mod renderer_pattern_command;
mod renderer_pattern_pixel_check;
mod renderer_per_corner_radius_command;
mod renderer_raster_atlas_growth;
mod renderer_raster_text_pixel_check;
mod renderer_raster_throughput_bench;
mod renderer_render_scene;
mod renderer_renderer_contract;
mod renderer_scene_command;
mod renderer_scene_sweep;
mod renderer_shadow_class_paint;
mod renderer_shadow_command;
mod renderer_slug_extrema;
mod renderer_split_path_command;
mod renderer_split_path_pixel_check;
mod renderer_sprite_atlas_properties;
mod renderer_sprite_command;
mod renderer_sprite_pixel_check;
mod renderer_stroke_command;
mod renderer_style_integration;
mod renderer_svg_bugs;
mod renderer_tall_scene;
mod replay;
mod rtl_layout;
mod scroll_command;
mod scroll_position;
mod scroll_preserves_content_layout;
mod shell_dpi_scroll;
mod shell_shell_contract;
mod shortcut_and_tone;
mod slider_keyboard;
mod slider_pointer_extent;
mod splitter;
mod state_overlay_button;
mod state_overlay_widgets;
mod story;
mod style_ds1_scale_tokens;
mod style_fragment_debug_asserts;
mod style_precedence;
mod style_reactive;
mod style_reactive_composition;
mod style_state;
mod style_styled;
mod svg_conformance;
mod svg_conformance_depth;
mod svg_picture_fidelity;
mod svg_picture_perf;
mod svg_sharpness;
mod table_edit;
mod table_state;
mod table_view;
mod tabs_reactive_content;
mod tabs_routing;
mod tag_input;
mod tasks_scheduler_contract;
mod tasks_scheduler_spawn;
mod tasks_task;
#[cfg(feature = "pty")]
mod terminal_command;
#[cfg(feature = "terminal")]
mod terminal_host;
#[cfg(all(feature = "terminal", feature = "pty"))]
mod terminal_images;
#[cfg(feature = "terminal")]
mod terminal_keys;
#[cfg(feature = "terminal")]
mod terminal_marks;
#[cfg(all(feature = "terminal", feature = "pty"))]
mod terminal_probe;
#[cfg(feature = "pty")]
mod terminal_session;
mod text_clip;
mod text_edit;
mod text_edit_caret;
mod text_edit_focus;
mod text_edit_ime;
mod text_edit_scroll;
mod text_edit_selection;
mod text_field_caret_style;
mod text_field_editing;
mod text_field_focus;
mod text_field_ime;
mod text_integration;
mod text_nowrap;
mod text_wrap;
mod text_wrap_fit;
mod theme_file_i18n;
mod time_field;
mod tooltip;
mod top_layer_culling;
mod transform3d;
mod two_hosts;
mod vector_fit;
mod vector_no_raster;
mod vector_path;
mod vega_engine;
mod view;
mod virtual_list_bind_layout;
mod virtual_list_integration;
mod virtual_list_keyboard;
mod virtual_list_prepend;
#[cfg(feature = "terminal")]
mod virtual_list_resize;
mod virtual_list_scene;
mod virtual_list_viewport;
mod visual_goldens;
mod visual_goldens_dpi;
mod visual_goldens_extra;
mod visual_goldens_wave2;
mod web_font_list;
mod widget_gaps;
mod widget_paint_coverage;
mod widget_states;
mod widget_styled;
mod widget_text_presence;
mod widget_width_override;
mod widgets;
mod widgets_widget_hygiene;
mod zoom_and_paint;
