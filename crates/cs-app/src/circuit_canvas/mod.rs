//! QML façade for the rust canvas. Scene/hit-test live in `cs_engine::canvas`.

pub mod files;
pub mod interaction;
pub mod palette;
pub mod plots;
pub mod props;
pub mod repaint;
pub mod selection;

use crate::path_util::strip_file_url;
use cs_engine::backup;
use cs_engine::canvas::{Canvas, Change};
use cs_engine::settings;
use cs_engine::units;
use files::PendingReplace;
use qtbridge::qobject;
use serde_json::{Value, json};

pub struct CircuitCanvas {
    pub(crate) inner: Canvas,
    pub(crate) dark: bool,
    pub(crate) grid_color: String,
    pub(crate) canvas_color: String,
    pub(crate) viewport_color: String,
    pub(crate) band_color: String,
    pub(crate) body_color: String,
    pub(crate) border_color: String,
    pub(crate) wire_color: String,
    pub(crate) pin_high_color: String,
    pub(crate) pin_low_color: String,
    pub(crate) meter_display_color: String,
    pub(crate) pin_open_high_color: String,
    pub(crate) pin_open_low_color: String,
    pub(crate) pin_input_high_color: String,
    pub(crate) pin_input_low_color: String,
    pub(crate) pin_out_high_color: String,
    pub(crate) pin_out_low_color: String,
    pub(crate) shape_body_color: String,
    pub(crate) prop_label_color: String,
    pub(crate) prop_header_color: String,
    pub(crate) mcu_label_bg: String,
    pub(crate) mcu_label_text: String,
    pub(crate) mcu_label_border: String,
    pub(crate) chip_body_color: String,
    pub(crate) chip_label_color: String,
    pub(crate) coords_label_text: String,
    pub(crate) tunnel_color1: String,
    pub(crate) tunnel_color2: String,
    pub(crate) tunnel_color3: String,
    pub(crate) scope_channel1_color: String,
    pub(crate) scope_channel2_color: String,
    pub(crate) scope_channel3_color: String,
    pub(crate) scope_channel4_color: String,
    pub(crate) la_channel1_color: String,
    pub(crate) la_channel2_color: String,
    pub(crate) la_channel3_color: String,
    pub(crate) la_channel4_color: String,
    pub(crate) la_channel5_color: String,
    pub(crate) la_channel6_color: String,
    pub(crate) la_channel7_color: String,
    pub(crate) la_channel8_color: String,
    pub(crate) msg_ok_bg: String,
    pub(crate) msg_ok_text: String,
    pub(crate) msg_warn_bg: String,
    pub(crate) msg_warn_text: String,
    pub(crate) msg_error_bg: String,
    pub(crate) msg_error_text: String,
    pub(crate) sim_pause_tint: String,
    pub(crate) toggle_button_bg: String,
    pub(crate) mem_data_bg: String,
    pub(crate) mem_data_text: String,
    pub(crate) mem_type_text: String,
    pub(crate) mem_val_text: String,
    pub(crate) mcu_pc_val1_text: String,
    pub(crate) mcu_pc_val2_text: String,
    pub(crate) border_width: f64,
    pub(crate) wire_width: f64,
    pub(crate) bus_wire_width: f64,
    pub(crate) component_fill_alpha: f64,
    pub(crate) component_symbol_alpha: f64,
    pub(crate) hover_html: String,
    pub(crate) hover_visible: bool,
    pub(crate) hover_x: f64,
    pub(crate) hover_y: f64,
    pub(crate) sim_load: f64,
    pub(crate) gui_load: f64,
    pub(crate) real_speed: f64,
    pub(crate) real_fps: i32,
    pub(crate) last_ref_time: Option<std::time::Instant>,
    pub(crate) last_circ_time: u64,
    pub(crate) gui_time_ns: u64,
    pub(crate) frame_count: u32,
    pub(crate) pending_sim_log: Vec<String>,
    pub(crate) active_device_id: Option<String>,
    pub(crate) pending_replace: PendingReplace,
    pub(crate) project_path: String,
    pub(crate) initial_loaded: bool,
    pub(crate) last_warnings_count: usize,
    pub(crate) last_warnings_crashed: bool,
    pub(crate) dpr: f64,
    pub(crate) last_nav_raster: Option<std::time::Instant>,
}

impl Default for CircuitCanvas {
    fn default() -> Self {
        let st = settings::get();
        let mut inner = Canvas::new();
        inner.set_show_grid(st.draw_grid);
        inner.set_show_scroll(st.show_scroll);
        let mut s = Self {
            inner,
            dark: false,
            active_device_id: None,
            sim_load: 0.0,
            gui_load: 0.0,
            real_speed: 0.0,
            real_fps: st.fps.clamp(1, 100) as i32,
            last_ref_time: None,
            last_circ_time: 0,
            gui_time_ns: 0,
            frame_count: 0,
            pending_sim_log: Vec::new(),
            pending_replace: PendingReplace::None,
            last_warnings_count: 0,
            last_warnings_crashed: false,
            dpr: 1.0,
            last_nav_raster: None,
            grid_color: String::new(),
            canvas_color: String::new(),
            viewport_color: String::new(),
            band_color: String::new(),
            body_color: String::new(),
            border_color: String::new(),
            wire_color: String::new(),
            pin_high_color: String::new(),
            pin_low_color: String::new(),
            meter_display_color: String::new(),
            pin_open_high_color: String::new(),
            pin_open_low_color: String::new(),
            pin_input_high_color: String::new(),
            pin_input_low_color: String::new(),
            pin_out_high_color: String::new(),
            pin_out_low_color: String::new(),
            shape_body_color: String::new(),
            prop_label_color: String::new(),
            prop_header_color: String::new(),
            mcu_label_bg: String::new(),
            mcu_label_text: String::new(),
            mcu_label_border: String::new(),
            chip_body_color: String::new(),
            chip_label_color: String::new(),
            coords_label_text: String::new(),
            tunnel_color1: String::new(),
            tunnel_color2: String::new(),
            tunnel_color3: String::new(),
            scope_channel1_color: String::new(),
            scope_channel2_color: String::new(),
            scope_channel3_color: String::new(),
            scope_channel4_color: String::new(),
            la_channel1_color: String::new(),
            la_channel2_color: String::new(),
            la_channel3_color: String::new(),
            la_channel4_color: String::new(),
            la_channel5_color: String::new(),
            la_channel6_color: String::new(),
            la_channel7_color: String::new(),
            la_channel8_color: String::new(),
            msg_ok_bg: String::new(),
            msg_ok_text: String::new(),
            msg_warn_bg: String::new(),
            msg_warn_text: String::new(),
            msg_error_bg: String::new(),
            msg_error_text: String::new(),
            sim_pause_tint: String::new(),
            toggle_button_bg: String::new(),
            mem_data_bg: String::new(),
            mem_data_text: String::new(),
            mem_type_text: String::new(),
            mem_val_text: String::new(),
            mcu_pc_val1_text: String::new(),
            mcu_pc_val2_text: String::new(),
            border_width: cs_engine::theme::COMPONENT_BORDER_WIDTH,
            wire_width: cs_engine::theme::WIRE_WIDTH,
            bus_wire_width: cs_engine::theme::BUS_WIRE_WIDTH,
            component_fill_alpha: cs_engine::theme::COMPONENT_FILL_ALPHA,
            component_symbol_alpha: cs_engine::theme::COMPONENT_SYMBOL_ALPHA,
            hover_html: String::new(),
            hover_visible: false,
            hover_x: 0.0,
            hover_y: 0.0,
            project_path: String::new(),
            initial_loaded: false,
        };
        s.apply_palette();
        s
    }
}

impl CircuitCanvas {
    fn guard_replace(&mut self, next: PendingReplace) -> bool {
        if !self.inner.is_modified() {
            return false;
        }
        self.pending_replace = next;
        self.request_save_before_replace();
        true
    }

    fn finish_pending_replace(&mut self) {
        let pending = std::mem::take(&mut self.pending_replace);
        match pending {
            PendingReplace::None => {}
            PendingReplace::New => self.replace_with_new(),
            PendingReplace::Load(path) => self.load_path_now(&path),
        }
    }

    fn replace_with_new(&mut self) {
        let c = files::replace_with_new(&mut self.inner, &self.project_path);
        self.apply(c);
        self.file_path_changed();
        self.circ_settings_changed();
        self.canvas_overflow_changed();
        self.last_warnings_count = 0;
        self.last_warnings_crashed = false;
        self.warnings_changed();
        self.root_circuit_loaded();
    }

    fn load_path_now(&mut self, path: &str) {
        match files::load_path_now(&mut self.inner, &self.project_path, path) {
            Ok(c) => {
                self.apply(c);
                self.file_path_changed();
                self.canvas_overflow_changed();
                self.last_warnings_count = 0;
                self.last_warnings_crashed = false;
                self.warnings_changed();
                self.root_circuit_loaded();
            }
            Err(_) => {
                self.flush_sim_logs();
            }
        }
    }

    fn apply_circuit_draft(&mut self, draft: backup::CircuitDraft) {
        match files::apply_circuit_draft(&mut self.inner, &self.project_path, draft) {
            Ok(Some(c)) => {
                self.apply(c);
                self.file_path_changed();
                self.flush_sim_logs();
            }
            Ok(None) => {}
            Err(_) => {}
        }
    }

    fn persist_circuit_draft(&mut self) {
        if !self.inner.is_idle() {
            return;
        }
        self.persist_circuit_draft_now();
    }

    fn persist_circuit_draft_now(&mut self) {
        files::persist_circuit_draft_now(&self.inner, &self.project_path);
    }

    fn apply_palette(&mut self) {
        palette::update_palette_colors(self);
    }

    fn flush_sim_logs(&mut self) {
        let logs = cs_engine::logging::drain_simulation();
        if !logs.is_empty() {
            self.pending_sim_log.extend(logs);
            self.sim_log_changed();
        }
    }

    fn apply(&mut self, c: Change) {
        self.flush_sim_logs();
        if c.zoom {
            self.zoom_changed();
        }
        if c.center {
            self.center_changed();
        }
        if c.viewport {
            self.viewport_changed();
        }
        if c.zoom || c.center || c.viewport {
            self.coords_changed();
            if self.hover_visible {
                self.hover_visible = false;
                self.hover_changed();
            }
        }
        if c.band {
            self.band_changed();
        }
        if c.items {
            self.ensure_active_device();
            self.items_changed();
            self.devices_changed();
        }
        if c.pause_sim {
            self.request_pause_sim();
        }
        if c.hovered_pin {
            self.hovered_pin_changed();
        }
        if c.wires {
            self.wires_changed();
        }
        if c.cursor {
            self.cursor_changed();
        }
        if c.sim {
            self.sim_changed();
        }
        if c.anim {
            self.anim_changed();
        }
        if c.history {
            self.history_changed();
            self.persist_circuit_draft();
        }
        if c.props {
            self.props_changed();
        }
        if c.open_props {
            self.open_prop_dialogs_changed();
            if let Some(uid) = self.inner.last_opened_prop_uid() {
                let uid_str = uid.to_string();
                self.request_bring_prop_to_front(uid_str);
            }
        }
        if c.plots {
            self.traces_changed();
        }
        if c.settings {
            self.circ_settings_changed();
            self.scene_rect_changed();
        }
        if c.items || c.wires || c.settings || c.history {
            self.canvas_overflow_changed();
        }
        if c.open_scope {
            self.request_show_scope();
        }
        if c.open_la {
            self.request_show_la();
        }
        if c.open_terminal {
            self.request_show_terminal();
        }
        if let Some((mon_id, title)) = &c.open_serial_mon {
            self.request_open_serial_mon(mon_id.clone(), title.clone());
        }
        let view_nav = (c.center || c.zoom)
            && !c.band
            && !c.items
            && !c.wires
            && !c.sim
            && !c.anim
            && !c.settings;

        if view_nav {
            let vp = self.inner.viewport();
            let center = vp.center();
            if !crate::native_canvas::pixmap_covers_view(
                center.x,
                center.y,
                vp.zoom(),
                vp.view_size().0,
                vp.view_size().1,
            ) {
                let due = self
                    .last_nav_raster
                    .is_none_or(|t| t.elapsed() >= std::time::Duration::from_millis(32));
                if due {
                    self.inner.mark_full_dirty();
                    let dirty = self.inner.take_dirty();
                    self.render_canvas();
                    self.record_repaint_event(&dirty, true, c);
                }
            }
        } else {
            let dirty = self.inner.take_dirty();
            let (vw, vh) = self.inner.viewport().view_size();
            if vw <= 0.0 || vh <= 0.0 {
                self.inner.restore_dirty(dirty);
                return;
            }
            let force_full = c.zoom || c.center || c.viewport || dirty.full;
            let scene_rects = if force_full {
                Vec::new()
            } else {
                dirty.scene_rects(&self.inner)
            };
            let fallback_full = scene_rects.is_empty() && (c.items || c.wires) && !dirty.full;
            let painted_full;
            if force_full || fallback_full {
                self.render_canvas();
                painted_full = true;
            } else if !scene_rects.is_empty() {
                if !self.render_canvas_regions(&scene_rects) {
                    self.render_canvas();
                    painted_full = true;
                } else {
                    painted_full = false;
                }
            } else {
                painted_full = false;
            }
            if force_full || fallback_full || !scene_rects.is_empty() || c.band {
                self.record_repaint_event(&dirty, painted_full, c);
            }
        }
    }

    fn record_repaint_event(
        &mut self,
        dirty: &cs_engine::canvas::DirtySet,
        painted_full: bool,
        c: Change,
    ) {
        let band_rect = self.inner.band_item_rect();
        if let Some((rects, is_full, sub_rects)) =
            repaint::check_and_record_repaints(&self.inner, band_rect, c, dirty, painted_full)
        {
            self.repaint_recorded(rects, is_full, sub_rects);
        }
    }

    fn update_hover_tooltip(&mut self, x: f64, y: f64) {
        if let Some(html) = interaction::hover_tooltip_at(&self.inner, x, y) {
            let html_changed = self.hover_html != html || !self.hover_visible;
            let pos_changed = (self.hover_x - x).abs() > 1.0 || (self.hover_y - y).abs() > 1.0;
            self.hover_html = html;
            self.hover_visible = true;
            self.hover_x = x;
            self.hover_y = y;
            if html_changed {
                self.hover_changed();
            }
            if pos_changed {
                self.hover_pos_changed();
            }
        } else if self.hover_visible {
            self.hover_visible = false;
            self.hover_changed();
        }
    }

    pub fn render_canvas(&mut self) {
        self.last_nav_raster = Some(std::time::Instant::now());
        repaint::render_canvas(&self.inner, self.dark, self.dpr);
    }

    fn render_canvas_regions(&mut self, scene_rects: &[cs_engine::canvas::Rect]) -> bool {
        self.last_nav_raster = Some(std::time::Instant::now());
        repaint::render_canvas_regions(&self.inner, self.dark, self.dpr, scene_rects)
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl CircuitCanvas {
    qproperty!(
        "projectPath",
        Read = project_path,
        Write = set_project_path,
        Notify = project_path_changed
    );
    qproperty!("zoom", Read = zoom, Write = set_zoom, Notify = zoom_changed);
    qproperty!(
        "centerX",
        Read = center_x,
        Write = set_center_x,
        Notify = center_changed
    );
    qproperty!(
        "centerY",
        Read = center_y,
        Write = set_center_y,
        Notify = center_changed
    );
    qproperty!("sceneRect", Read = scene_rect, Notify = scene_rect_changed);
    qproperty!(
        "canvasOverflow",
        Read = canvas_overflow,
        Notify = canvas_overflow_changed
    );
    qproperty!(
        "canvasOverflowCount",
        Read = canvas_overflow_count,
        Notify = canvas_overflow_changed
    );
    qproperty!(
        "canvasRequiredWidth",
        Read = canvas_required_width,
        Notify = canvas_overflow_changed
    );
    qproperty!(
        "canvasRequiredHeight",
        Read = canvas_required_height,
        Notify = canvas_overflow_changed
    );
    qproperty!(
        "canvasCurrentWidth",
        Read = canvas_current_width,
        Notify = canvas_overflow_changed
    );
    qproperty!(
        "canvasCurrentHeight",
        Read = canvas_current_height,
        Notify = canvas_overflow_changed
    );
    qproperty!(
        "visibleRect",
        Read = visible_rect,
        Notify = viewport_changed
    );
    qproperty!("cullMinX", Read = cull_min_x, Notify = coords_changed);
    qproperty!("cullMaxX", Read = cull_max_x, Notify = coords_changed);
    qproperty!("cullMinY", Read = cull_min_y, Notify = coords_changed);
    qproperty!("cullMaxY", Read = cull_max_y, Notify = coords_changed);
    qproperty!("vScrollPos", Read = v_scroll_pos, Notify = viewport_changed);
    qproperty!(
        "vScrollSize",
        Read = v_scroll_size,
        Notify = viewport_changed
    );
    qproperty!("hScrollPos", Read = h_scroll_pos, Notify = viewport_changed);
    qproperty!(
        "hScrollSize",
        Read = h_scroll_size,
        Notify = viewport_changed
    );
    qproperty!("banding", Read = banding, Notify = band_changed);
    qproperty!("bandRect", Read = band_rect, Notify = band_changed);
    qproperty!("items", Read = items, Notify = items_changed);
    qproperty!("wires", Read = wires, Notify = wires_changed);
    qproperty!("simRunning", Read = sim_running, Notify = sim_changed);
    qproperty!("simError", Read = sim_error, Notify = sim_changed);
    qproperty!("simWarning", Read = sim_warning, Notify = sim_changed);
    qproperty!("animTick", Read = anim_tick, Notify = anim_changed);
    qproperty!("filePath", Read = file_path, Notify = file_path_changed);
    qproperty!("fileName", Read = file_name, Notify = file_path_changed);
    qproperty!(
        "suggestImagePath",
        Read = suggest_image_path,
        Notify = file_path_changed
    );
    qproperty!(
        "suggestImageUrl",
        Read = suggest_image_url,
        Notify = file_path_changed
    );
    qproperty!("modified", Read = is_modified, Notify = history_changed);
    qproperty!("canUndo", Read = can_undo, Notify = history_changed);
    qproperty!("canRedo", Read = can_redo, Notify = history_changed);
    qproperty!("canPaste", Read = can_paste, Notify = history_changed);
    qproperty!("hasSelection", Read = has_selection, Notify = items_changed);
    qproperty!(
        "hasItemSelection",
        Read = has_item_selection,
        Notify = items_changed
    );
    qproperty!(
        "hasWireSelection",
        Read = has_wire_selection,
        Notify = wires_changed
    );
    qproperty!("hitLabelUid", Read = hit_label_uid, Notify = items_changed);
    qproperty!(
        "hitLabelIsVal",
        Read = hit_label_is_val,
        Notify = items_changed
    );
    qproperty!("hasLabelHit", Read = has_label_hit, Notify = items_changed);
    qproperty!("hitPinId", Read = hit_pin_id, Notify = items_changed);
    qproperty!(
        "hitPinInverted",
        Read = hit_pin_inverted,
        Notify = items_changed
    );
    qproperty!("hasPinHit", Read = has_pin_hit, Notify = items_changed);
    qproperty!(
        "selectedItemUid",
        Read = selected_item_uid,
        Notify = items_changed
    );
    qproperty!(
        "selectedItemType",
        Read = selected_item_type,
        Notify = items_changed
    );
    qproperty!(
        "selectedItemUarts",
        Read = selected_item_uarts,
        Notify = items_changed
    );
    qproperty!(
        "selectedItemMonitors",
        Read = selected_item_monitors,
        Notify = items_changed
    );
    qproperty!(
        "selectedItemActive",
        Read = selected_item_active,
        Notify = items_changed
    );
    qproperty!(
        "selectedHasFlash",
        Read = selected_has_flash,
        Notify = items_changed
    );
    qproperty!(
        "selectedHasEeprom",
        Read = selected_has_eeprom,
        Notify = items_changed
    );
    qproperty!(
        "selectedHasFirmware",
        Read = selected_has_firmware,
        Notify = items_changed
    );
    qproperty!(
        "selectedNestedMcuUid",
        Read = selected_nested_mcu_uid,
        Notify = items_changed
    );
    qproperty!("linkingFrom", Read = linking_from, Notify = items_changed);
    qproperty!(
        "hitPinIsPackage",
        Read = hit_pin_is_package,
        Notify = items_changed
    );
    qproperty!(
        "selectedTunnelVisible",
        Read = selected_tunnel_visible,
        Notify = items_changed
    );
    qproperty!(
        "selectedProbePause",
        Read = selected_probe_pause,
        Notify = items_changed
    );
    qproperty!(
        "selectedHasSdImage",
        Read = selected_has_sd_image,
        Notify = items_changed
    );
    qproperty!(
        "selectedHasImageData",
        Read = selected_has_image_data,
        Notify = items_changed
    );
    qproperty!(
        "subcircuitTree",
        Read = subcircuit_tree,
        Notify = file_path_changed
    );
    qproperty!(
        "programmableDevices",
        Read = programmable_devices,
        Notify = devices_changed
    );
    qproperty!(
        "activeDeviceId",
        Read = active_device_id,
        Write = set_active_device_id,
        Notify = active_device_changed
    );
    qproperty!("propOpen", Read = prop_open, Notify = props_changed);
    qproperty!(
        "openPropDialogs",
        Read = open_prop_dialogs,
        Notify = open_prop_dialogs_changed
    );
    qproperty!("selectedUid", Read = selected_uid, Notify = props_changed);
    qproperty!(
        "propItemType",
        Read = prop_item_type,
        Notify = props_changed
    );
    qproperty!(
        "propTypeText",
        Read = prop_type_text,
        Notify = props_changed
    );
    qproperty!(
        "propDescription",
        Read = prop_description,
        Notify = props_changed
    );
    qproperty!("propLabel", Read = prop_label, Notify = props_changed);
    qproperty!("propShowId", Read = prop_show_id, Notify = props_changed);
    qproperty!("propAnyInfo", Read = prop_any_info, Notify = props_changed);
    qproperty!("propGroups", Read = prop_groups, Notify = props_changed);
    qproperty!("gridColor", Read = grid_color, Notify = appearance_changed);
    qproperty!(
        "canvasColor",
        Read = canvas_color,
        Notify = appearance_changed
    );
    qproperty!(
        "viewportColor",
        Read = viewport_color,
        Notify = appearance_changed
    );
    qproperty!("bandColor", Read = band_color, Notify = appearance_changed);
    qproperty!("bodyColor", Read = body_color, Notify = appearance_changed);
    qproperty!(
        "borderColor",
        Read = border_color,
        Notify = appearance_changed
    );
    qproperty!("wireColor", Read = wire_color, Notify = appearance_changed);
    qproperty!(
        "pinHighColor",
        Read = pin_high_color,
        Notify = appearance_changed
    );
    qproperty!(
        "pinLowColor",
        Read = pin_low_color,
        Notify = appearance_changed
    );
    qproperty!(
        "meterDisplayColor",
        Read = meter_display_color,
        Notify = appearance_changed
    );
    qproperty!("borderWidth", Read = border_width, Constant);
    qproperty!("wireWidth", Read = wire_width, Constant);
    qproperty!("busWireWidth", Read = bus_wire_width, Constant);
    qproperty!("componentFillAlpha", Read = component_fill_alpha, Constant);
    qproperty!(
        "componentSymbolAlpha",
        Read = component_symbol_alpha,
        Constant
    );
    qproperty!(
        "pinOpenHighColor",
        Read = pin_open_high_color,
        Notify = appearance_changed
    );
    qproperty!(
        "pinOpenLowColor",
        Read = pin_open_low_color,
        Notify = appearance_changed
    );
    qproperty!(
        "pinInputHighColor",
        Read = pin_input_high_color,
        Notify = appearance_changed
    );
    qproperty!(
        "pinInputLowColor",
        Read = pin_input_low_color,
        Notify = appearance_changed
    );
    qproperty!(
        "pinOutHighColor",
        Read = pin_out_high_color,
        Notify = appearance_changed
    );
    qproperty!(
        "pinOutLowColor",
        Read = pin_out_low_color,
        Notify = appearance_changed
    );
    qproperty!(
        "shapeBodyColor",
        Read = shape_body_color,
        Notify = appearance_changed
    );
    qproperty!(
        "propLabelTextColor",
        Read = prop_label_color,
        Notify = appearance_changed
    );
    qproperty!(
        "propHeaderTextColor",
        Read = prop_header_color,
        Notify = appearance_changed
    );
    qproperty!(
        "mcuLabelBg",
        Read = mcu_label_bg,
        Notify = appearance_changed
    );
    qproperty!(
        "mcuLabelText",
        Read = mcu_label_text,
        Notify = appearance_changed
    );
    qproperty!(
        "mcuLabelBorder",
        Read = mcu_label_border,
        Notify = appearance_changed
    );
    qproperty!(
        "chipBodyColor",
        Read = chip_body_color,
        Notify = appearance_changed
    );
    qproperty!(
        "chipLabelColor",
        Read = chip_label_color,
        Notify = appearance_changed
    );
    qproperty!(
        "coordsLabelTextColor",
        Read = coords_label_text,
        Notify = appearance_changed
    );
    qproperty!(
        "tunnelColor1",
        Read = tunnel_color1,
        Notify = appearance_changed
    );
    qproperty!(
        "tunnelColor2",
        Read = tunnel_color2,
        Notify = appearance_changed
    );
    qproperty!(
        "tunnelColor3",
        Read = tunnel_color3,
        Notify = appearance_changed
    );
    qproperty!("msgOkBg", Read = msg_ok_bg, Notify = appearance_changed);
    qproperty!("msgOkText", Read = msg_ok_text, Notify = appearance_changed);
    qproperty!("msgWarnBg", Read = msg_warn_bg, Notify = appearance_changed);
    qproperty!(
        "msgWarnText",
        Read = msg_warn_text,
        Notify = appearance_changed
    );
    qproperty!(
        "msgErrorBg",
        Read = msg_error_bg,
        Notify = appearance_changed
    );
    qproperty!(
        "msgErrorText",
        Read = msg_error_text,
        Notify = appearance_changed
    );
    qproperty!(
        "simPauseTint",
        Read = sim_pause_tint,
        Notify = appearance_changed
    );
    qproperty!(
        "toggleButtonBg",
        Read = toggle_button_bg,
        Notify = appearance_changed
    );
    qproperty!("memDataBg", Read = mem_data_bg, Notify = appearance_changed);
    qproperty!(
        "memDataText",
        Read = mem_data_text,
        Notify = appearance_changed
    );
    qproperty!(
        "memTypeText",
        Read = mem_type_text,
        Notify = appearance_changed
    );
    qproperty!(
        "memValText",
        Read = mem_val_text,
        Notify = appearance_changed
    );
    qproperty!(
        "mcuPcVal1Text",
        Read = mcu_pc_val1_text,
        Notify = appearance_changed
    );
    qproperty!(
        "mcuPcVal2Text",
        Read = mcu_pc_val2_text,
        Notify = appearance_changed
    );
    qproperty!(
        "scopeChannel1Color",
        Read = scope_channel1_color,
        Notify = appearance_changed
    );
    qproperty!(
        "scopeChannel2Color",
        Read = scope_channel2_color,
        Notify = appearance_changed
    );
    qproperty!(
        "scopeChannel3Color",
        Read = scope_channel3_color,
        Notify = appearance_changed
    );
    qproperty!(
        "scopeChannel4Color",
        Read = scope_channel4_color,
        Notify = appearance_changed
    );
    qproperty!(
        "laChannel1Color",
        Read = la_channel1_color,
        Notify = appearance_changed
    );
    qproperty!(
        "laChannel2Color",
        Read = la_channel2_color,
        Notify = appearance_changed
    );
    qproperty!(
        "laChannel3Color",
        Read = la_channel3_color,
        Notify = appearance_changed
    );
    qproperty!(
        "laChannel4Color",
        Read = la_channel4_color,
        Notify = appearance_changed
    );
    qproperty!(
        "laChannel5Color",
        Read = la_channel5_color,
        Notify = appearance_changed
    );
    qproperty!(
        "laChannel6Color",
        Read = la_channel6_color,
        Notify = appearance_changed
    );
    qproperty!(
        "laChannel7Color",
        Read = la_channel7_color,
        Notify = appearance_changed
    );
    qproperty!(
        "laChannel8Color",
        Read = la_channel8_color,
        Notify = appearance_changed
    );
    qproperty!(
        "dark",
        Read = dark,
        Write = set_dark,
        Notify = appearance_changed
    );
    qproperty!(
        "showGrid",
        Read = show_grid,
        Write = set_show_grid,
        Notify = appearance_changed
    );
    qproperty!(
        "showScroll",
        Read = show_scroll,
        Write = set_show_scroll,
        Notify = show_scroll_changed
    );
    qproperty!(
        "showComponentRects",
        Read = show_component_rects,
        Write = set_show_component_rects,
        Notify = appearance_changed
    );
    qproperty!(
        "showComponentRect",
        Read = show_component_rects,
        Write = set_show_component_rects,
        Notify = appearance_changed
    );
    qproperty!("cursor", Read = cursor, Notify = cursor_changed);
    qproperty!("cursorX", Read = cursor_x, Notify = coords_changed);
    qproperty!("cursorY", Read = cursor_y, Notify = coords_changed);
    qproperty!("scopeTraces", Read = scope_traces, Notify = traces_changed);
    qproperty!("laTraces", Read = la_traces, Notify = traces_changed);
    qproperty!("timeUnits", Read = time_units, Constant);
    qproperty!(
        "sceneWidth",
        Read = scene_width,
        Write = set_scene_width,
        Notify = circ_settings_changed
    );
    qproperty!(
        "sceneHeight",
        Read = scene_height,
        Write = set_scene_height,
        Notify = circ_settings_changed
    );
    qproperty!(
        "animateLogic",
        Read = animate_logic,
        Write = set_animate_logic,
        Notify = circ_settings_changed
    );
    qproperty!(
        "animateCurr",
        Read = animate_curr,
        Write = set_animate_curr,
        Notify = circ_settings_changed
    );
    qproperty!(
        "ansiSymbols",
        Read = ansi_symbols,
        Write = set_ansi_symbols,
        Notify = circ_settings_changed
    );
    qproperty!(
        "speedPercent",
        Read = speed_percent,
        Write = set_speed_percent,
        Notify = circ_settings_changed
    );
    qproperty!(
        "speedLabel",
        Read = speed_label,
        Notify = circ_settings_changed
    );
    qproperty!(
        "step",
        Read = circ_step,
        Write = set_circ_step,
        Notify = circ_settings_changed
    );
    qproperty!(
        "stepUnit",
        Read = step_unit,
        Write = set_step_unit,
        Notify = circ_settings_changed
    );
    qproperty!(
        "reactStep",
        Read = react_step,
        Write = set_react_step,
        Notify = circ_settings_changed
    );
    qproperty!(
        "reactStepUnit",
        Read = react_step_unit,
        Write = set_react_step_unit,
        Notify = circ_settings_changed
    );
    qproperty!("realStep", Read = real_step, Notify = circ_settings_changed);
    qproperty!(
        "realStepUnit",
        Read = real_step_unit,
        Notify = circ_settings_changed
    );
    qproperty!(
        "nlSteps",
        Read = nl_steps,
        Write = set_nl_steps,
        Notify = circ_settings_changed
    );
    qproperty!("resets", Read = resets, Notify = circ_settings_changed);
    qproperty!("simCircTime", Read = sim_circ_time, Notify = sim_changed);
    qproperty!("simMcuDevice", Read = sim_mcu_device, Notify = sim_changed);
    qproperty!("simMcuName", Read = sim_mcu_name, Notify = sim_changed);
    qproperty!("simHasMcu", Read = sim_has_mcu, Notify = sim_changed);
    qproperty!("simLoad", Read = sim_load_val, Notify = sim_changed);
    qproperty!("simGuiLoad", Read = sim_gui_load_val, Notify = sim_changed);
    qproperty!(
        "simRealSpeed",
        Read = sim_real_speed_val,
        Notify = sim_changed
    );
    qproperty!("simFps", Read = sim_fps_val, Notify = sim_changed);
    qproperty!("hoverHtml", Read = hover_html, Notify = hover_changed);
    qproperty!("hoverVisible", Read = hover_visible, Notify = hover_changed);
    qproperty!("hoverX", Read = hover_x, Notify = hover_pos_changed);
    qproperty!("hoverY", Read = hover_y, Notify = hover_pos_changed);
    qproperty!(
        "hoveredPin",
        Read = hovered_pin_id,
        Notify = hovered_pin_changed
    );
    qproperty!(
        "warningsText",
        Read = warnings_text,
        Notify = warnings_changed
    );
    qproperty!(
        "warningsCrashed",
        Read = warnings_crashed,
        Notify = warnings_changed
    );
    qproperty!(
        "overloadLog",
        Read = overload_log,
        Notify = warnings_changed
    );

    #[qsignal]
    fn warnings_changed(&mut self);
    #[qsignal]
    fn project_path_changed(&mut self);
    #[qsignal]
    fn hovered_pin_changed(&mut self);
    #[qsignal]
    fn hover_changed(&mut self);
    #[qsignal]
    fn hover_pos_changed(&mut self);
    #[qsignal]
    fn zoom_changed(&mut self);
    #[qsignal]
    fn center_changed(&mut self);
    #[qsignal]
    fn viewport_changed(&mut self);
    #[qsignal]
    fn scene_rect_changed(&mut self);
    #[qsignal]
    fn canvas_overflow_changed(&mut self);
    #[qsignal]
    fn band_changed(&mut self);
    #[qsignal]
    fn items_changed(&mut self);
    #[qsignal]
    fn wires_changed(&mut self);
    #[qsignal]
    fn sim_changed(&mut self);
    #[qsignal]
    fn anim_changed(&mut self);
    #[qsignal]
    fn file_path_changed(&mut self);
    #[qsignal]
    fn appearance_changed(&mut self);
    #[qsignal]
    fn show_scroll_changed(&mut self);
    #[qsignal]
    fn cursor_changed(&mut self);
    #[qsignal]
    fn coords_changed(&mut self);
    #[qsignal]
    fn history_changed(&mut self);
    #[qsignal]
    fn props_changed(&mut self);
    #[qsignal]
    fn open_prop_dialogs_changed(&mut self);
    #[qsignal]
    fn request_show_properties(&mut self);
    #[qsignal]
    fn request_bring_prop_to_front(&mut self, uid: String);
    #[qsignal]
    fn request_show_scope(&mut self);
    #[qsignal]
    fn request_show_la(&mut self);
    #[qsignal]
    fn request_show_terminal(&mut self);
    #[qsignal]
    fn request_open_serial_mon(&mut self, mon_id: String, title: String);
    #[qsignal]
    fn traces_changed(&mut self);
    #[qsignal]
    fn circ_settings_changed(&mut self);
    #[qsignal]
    fn sim_log_changed(&mut self);
    #[qsignal]
    fn devices_changed(&mut self);
    #[qsignal]
    fn active_device_changed(&mut self);
    #[qsignal]
    fn request_open_file(&mut self, path: String);
    #[qsignal]
    fn request_show_editor(&mut self);
    #[qsignal]
    fn request_pause_sim(&mut self);
    #[qsignal]
    fn request_upload_run(&mut self, debug: bool);
    #[qsignal]
    fn repaint_recorded(&mut self, rects: Value, is_full: bool, sub_rects: Value);
    #[qsignal]
    fn item_value_changed(&mut self, uid: String, source_val: f64, val_text: String, wiper: f64);
    #[qsignal]
    fn request_save_before_replace(&mut self);
    #[qsignal]
    fn request_save_as_then_replace(&mut self);
    #[qsignal]
    fn root_circuit_loaded(&mut self);
    #[qsignal]
    fn open_subcircuit_requested(&mut self, path: String, label: String);

    fn subcircuit_tree(&self) -> Value {
        plots::subcircuit_tree_json(&self.inner)
    }

    fn programmable_devices(&self) -> Value {
        plots::programmable_devices_json(&self.inner, self.active_device_id.as_deref())
    }

    fn active_device_id(&self) -> String {
        self.active_device_id.clone().unwrap_or_default()
    }

    fn set_active_device_id(&mut self, id: String) {
        let changed = self.active_device_id.as_deref() != Some(&id);
        if changed {
            self.active_device_id = if id.is_empty() { None } else { Some(id) };
            self.inner
                .set_active_device_id(self.active_device_id.clone());
            self.active_device_changed();
            self.devices_changed();
            self.items_changed();
            self.render_canvas();
        }
    }

    /// C++ `Mcu::setupMcu`: the first programmable device becomes active if none is.
    fn ensure_active_device(&mut self) {
        if let Some(target) =
            props::ensure_active_device(&self.inner, self.active_device_id.as_deref())
        {
            self.active_device_id = target;
            self.inner
                .set_active_device_id(self.active_device_id.clone());
            self.active_device_changed();
            self.devices_changed();
            self.render_canvas();
        }
    }

    #[qslot]
    fn take_sim_log_line(&mut self) -> String {
        self.flush_sim_logs();
        if self.pending_sim_log.is_empty() {
            String::new()
        } else {
            self.pending_sim_log.remove(0)
        }
    }

    #[qslot]
    fn led_color_pair(&self, color_name: String) -> Value {
        palette::led_color_pair(&color_name)
    }

    fn warnings_text(&self) -> String {
        plots::warnings_text(&self.inner)
    }

    fn warnings_crashed(&self) -> bool {
        plots::warnings_crashed(&self.inner)
    }

    fn overload_log(&self) -> Value {
        plots::overload_log_json(&self.inner)
    }

    #[qslot]
    fn activate_overload_entry(&mut self, index: i32) {
        if index < 0 {
            return;
        }
        let log = self.inner.overload_log();
        if let Some(entry) = log.get(index as usize) {
            let uid = entry.comp_uid.clone();
            let c = self.inner.select_and_center_item(&uid);
            self.apply(c);
        }
    }

    #[qslot]
    fn select_and_center(&mut self, uid: String) {
        let c = self.inner.select_and_center_item(&uid);
        self.apply(c);
    }

    #[qslot]
    fn clear_overload_log(&mut self) {
        self.inner.clear_overload_log();
        self.last_warnings_count = 0;
        self.last_warnings_crashed = false;
        self.warnings_changed();
    }

    fn border_width(&self) -> f64 {
        self.border_width
    }

    fn wire_width(&self) -> f64 {
        self.wire_width
    }

    fn bus_wire_width(&self) -> f64 {
        self.bus_wire_width
    }

    fn component_fill_alpha(&self) -> f64 {
        self.component_fill_alpha
    }

    fn component_symbol_alpha(&self) -> f64 {
        self.component_symbol_alpha
    }

    fn zoom(&self) -> f64 {
        self.inner.viewport().zoom()
    }

    fn set_zoom(&mut self, z: f64) {
        let c = self.inner.set_zoom(z);
        self.apply(c);
    }

    fn center_x(&self) -> f64 {
        self.inner.viewport().center().x
    }

    fn set_center_x(&mut self, x: f64) {
        let y = self.inner.viewport().center().y;
        let c = self.inner.set_center(x, y);
        self.apply(c);
    }

    fn center_y(&self) -> f64 {
        self.inner.viewport().center().y
    }

    fn set_center_y(&mut self, y: f64) {
        let x = self.inner.viewport().center().x;
        let c = self.inner.set_center(x, y);
        self.apply(c);
    }

    fn scene_rect(&self) -> Value {
        repaint::rect_json(self.inner.viewport().scene_rect())
    }

    fn canvas_overflow(&self) -> bool {
        self.inner.canvas_overflow_info().has_overflow
    }

    fn canvas_overflow_count(&self) -> i32 {
        self.inner.canvas_overflow_info().overflowing_count as i32
    }

    fn canvas_required_width(&self) -> i32 {
        self.inner.canvas_overflow_info().required_width
    }

    fn canvas_required_height(&self) -> i32 {
        self.inner.canvas_overflow_info().required_height
    }

    fn canvas_current_width(&self) -> i32 {
        self.inner.canvas_overflow_info().current_width
    }

    fn canvas_current_height(&self) -> i32 {
        self.inner.canvas_overflow_info().current_height
    }

    fn visible_rect(&self) -> Value {
        repaint::rect_json(self.inner.viewport().visible_rect())
    }

    fn cull_margin(&self) -> f64 {
        repaint::cull_margin(self.inner.viewport().zoom())
    }

    fn cull_min_x(&self) -> f64 {
        let vp = self.inner.viewport();
        repaint::cull_min_x(vp.visible_rect(), vp.zoom())
    }

    fn cull_max_x(&self) -> f64 {
        let vp = self.inner.viewport();
        repaint::cull_max_x(vp.visible_rect(), vp.zoom())
    }

    fn cull_min_y(&self) -> f64 {
        let vp = self.inner.viewport();
        repaint::cull_min_y(vp.visible_rect(), vp.zoom())
    }

    fn cull_max_y(&self) -> f64 {
        let vp = self.inner.viewport();
        repaint::cull_max_y(vp.visible_rect(), vp.zoom())
    }

    fn v_scroll_pos(&self) -> f64 {
        let vp = self.inner.viewport();
        repaint::v_scroll_pos(vp.scene_rect(), vp.visible_rect())
    }

    fn v_scroll_size(&self) -> f64 {
        let vp = self.inner.viewport();
        repaint::v_scroll_size(vp.scene_rect(), vp.visible_rect())
    }

    fn h_scroll_pos(&self) -> f64 {
        let vp = self.inner.viewport();
        repaint::h_scroll_pos(vp.scene_rect(), vp.visible_rect())
    }

    fn h_scroll_size(&self) -> f64 {
        let vp = self.inner.viewport();
        repaint::h_scroll_size(vp.scene_rect(), vp.visible_rect())
    }

    fn banding(&self) -> bool {
        self.inner.banding()
    }

    fn band_rect(&self) -> Value {
        repaint::rect_json(self.inner.band_item_rect())
    }

    fn hovered_pin_id(&self) -> String {
        self.inner.hovered_pin().unwrap_or_default().to_string()
    }

    fn items(&self) -> Value {
        props::items_json(&self.inner)
    }

    fn wires(&self) -> Value {
        props::wires_json(&self.inner)
    }

    fn sim_running(&self) -> bool {
        self.inner.sim_running()
    }

    fn sim_error(&self) -> String {
        self.inner.sim_error().unwrap_or("").to_string()
    }

    fn sim_warning(&self) -> String {
        self.inner.sim_warning().unwrap_or("").to_string()
    }

    fn anim_tick(&self) -> i32 {
        self.inner.anim_tick() as i32
    }

    fn project_path(&self) -> String {
        self.project_path.clone()
    }

    fn set_project_path(&mut self, new_project: String) {
        let new_project = cs_engine::project::normalize_path(&new_project);
        if self.initial_loaded && self.project_path == new_project {
            return;
        }
        let is_first = !self.initial_loaded;
        self.initial_loaded = true;

        if !is_first {
            self.flush_draft();
            let defaults = cs_engine::settings::get().to_circ();
            let c = self.inner.new_circuit_with(defaults);
            self.apply(c);
            self.file_path_changed();
            self.circ_settings_changed();
        }

        let from_cli = crate::pending::OPEN_CIRCUIT.get().is_some() && is_first;

        self.project_path = new_project.clone();
        self.project_path_changed();

        if from_cli {
            if let Some(cli) = crate::pending::OPEN_CIRCUIT.get() {
                let path = crate::path_util::strip_file_url(cli);
                if std::path::Path::new(&path).exists() {
                    self.load_path_now(&path);
                }
            }
            if self.inner.file_path().is_some() {
                cs_engine::project::update_session(&new_project, |s| {
                    s.circuit_file = self.inner.file_path().map(|p| p.to_string());
                });
            }
        } else if !crate::pending::no_project() {
            if let Some(mut sess) = cs_engine::project::get_session(&new_project) {
                cs_engine::project::cleanup_missing_files(&mut sess);
                if let Some(circuit) = sess.circuit_file {
                    if std::path::Path::new(&circuit).exists() {
                        self.load_path_now(&circuit);
                    }
                }
            }
        }
    }

    fn file_path(&self) -> String {
        self.inner.file_path().unwrap_or("").to_string()
    }

    fn file_name(&self) -> String {
        self.inner.file_name()
    }

    fn suggest_image_path(&self) -> String {
        self.inner.suggest_image_path()
    }

    fn suggest_image_url(&self) -> String {
        files::suggest_image_url(&self.inner.suggest_image_path())
    }

    fn is_modified(&self) -> bool {
        self.inner.is_modified()
    }

    fn can_undo(&self) -> bool {
        self.inner.can_undo()
    }

    fn can_redo(&self) -> bool {
        self.inner.can_redo()
    }

    fn can_paste(&self) -> bool {
        self.inner.can_paste()
    }

    fn has_selection(&self) -> bool {
        self.inner.has_selection()
    }

    fn has_item_selection(&self) -> bool {
        self.inner.has_item_selection()
    }

    fn has_wire_selection(&self) -> bool {
        self.inner.has_wire_selection()
    }

    fn hit_label_uid(&self) -> String {
        self.inner.hit_label_uid().unwrap_or("").to_string()
    }

    fn hit_label_is_val(&self) -> bool {
        self.inner.hit_label_is_val()
    }

    fn has_label_hit(&self) -> bool {
        self.inner.has_label_hit()
    }

    fn has_pin_hit(&self) -> bool {
        self.inner.has_pin_hit()
    }

    fn hit_pin_id(&self) -> String {
        self.inner.hit_pin_id().unwrap_or("").to_string()
    }

    fn hit_pin_inverted(&self) -> bool {
        self.inner.is_hit_pin_inverted()
    }

    fn prop_open(&self) -> bool {
        self.inner.prop_open()
    }

    fn selected_item_uid(&self) -> String {
        selection::selected_item_uid(&self.inner)
    }

    fn selected_item_type(&self) -> String {
        selection::selected_item_type(&self.inner)
    }

    fn selected_item_uarts(&self) -> Value {
        let list = self.inner.selected_item_monitors();
        json!(list)
    }

    fn selected_item_monitors(&self) -> Value {
        json!(self.inner.selected_item_monitors())
    }

    fn selected_item_active(&self) -> bool {
        props::selected_item_active(&self.inner, self.active_device_id.as_deref())
    }

    fn selected_has_flash(&self) -> bool {
        self.inner.selected_has_flash()
    }

    fn selected_has_eeprom(&self) -> bool {
        self.inner.selected_has_eeprom()
    }

    fn selected_has_firmware(&self) -> bool {
        self.inner.selected_has_firmware()
    }

    fn selected_nested_mcu_uid(&self) -> String {
        self.inner.selected_nested_mcu_uid().unwrap_or_default()
    }

    fn linking_from(&self) -> String {
        self.inner.linking_from().unwrap_or("").to_string()
    }

    fn selected_tunnel_visible(&self) -> bool {
        selection::selected_tunnel_visible(&self.inner)
    }

    fn selected_probe_pause(&self) -> bool {
        selection::selected_probe_pause(&self.inner)
    }

    fn selected_has_sd_image(&self) -> bool {
        selection::selected_has_sd_image(&self.inner)
    }

    fn selected_has_image_data(&self) -> bool {
        selection::selected_has_image_data(&self.inner)
    }

    fn hit_pin_is_package(&self) -> bool {
        selection::hit_pin_is_package(&self.inner)
    }

    fn selected_uid(&self) -> String {
        self.inner
            .prop_item()
            .map(|it| it.id.clone())
            .unwrap_or_default()
    }

    fn prop_item_type(&self) -> String {
        self.inner
            .prop_item()
            .map(|it| it.kind.type_name().to_string())
            .unwrap_or_default()
    }

    fn prop_type_text(&self) -> String {
        self.inner
            .prop_item()
            .map(|it| cs_engine::i18n::tr(it.human_name()))
            .unwrap_or_default()
    }

    fn prop_description(&self) -> String {
        self.inner
            .prop_item()
            .map(|it| cs_engine::i18n::tr(it.description()))
            .unwrap_or_default()
    }

    fn prop_label(&self) -> String {
        self.inner
            .prop_item()
            .map(|it| it.label.clone())
            .unwrap_or_default()
    }

    fn prop_show_id(&self) -> bool {
        self.inner.prop_item().is_some_and(|it| it.show_id)
    }

    fn prop_any_info(&self) -> bool {
        self.inner.prop_item().is_some_and(|it| {
            !it.description().is_empty() || it.prop_rows().iter().any(|r| !r.info.is_empty())
        })
    }

    fn open_prop_dialogs(&self) -> Value {
        props::open_prop_dialogs_json(&self.inner)
    }

    fn prop_groups(&self) -> Value {
        let Some(it) = self.inner.prop_item() else {
            return json!([]);
        };
        props::build_prop_groups_json(it)
    }

    #[qslot]
    fn prop_type_text_for(&self, uid: String) -> String {
        if uid.is_empty() {
            return self.prop_type_text();
        }
        self.inner
            .scene()
            .item_by_id(&uid)
            .map(|it| cs_engine::i18n::tr(it.human_name()))
            .unwrap_or_default()
    }

    #[qslot]
    fn prop_description_for(&self, uid: String) -> String {
        if uid.is_empty() {
            return self.prop_description();
        }
        self.inner
            .scene()
            .item_by_id(&uid)
            .map(|it| cs_engine::i18n::tr(it.description()))
            .unwrap_or_default()
    }

    #[qslot]
    fn prop_label_for(&self, uid: String) -> String {
        if uid.is_empty() {
            return self.prop_label();
        }
        self.inner
            .scene()
            .item_by_id(&uid)
            .map(|it| it.label.clone())
            .unwrap_or_default()
    }

    #[qslot]
    fn prop_show_id_for(&self, uid: String) -> bool {
        if uid.is_empty() {
            return self.prop_show_id();
        }
        self.inner
            .scene()
            .item_by_id(&uid)
            .is_some_and(|it| it.show_id)
    }

    #[qslot]
    fn prop_any_info_for(&self, uid: String) -> bool {
        if uid.is_empty() {
            return self.prop_any_info();
        }
        self.inner.scene().item_by_id(&uid).is_some_and(|it| {
            !it.description().is_empty() || it.prop_rows().iter().any(|r| !r.info.is_empty())
        })
    }

    #[qslot]
    fn prop_groups_for(&self, uid: String) -> Value {
        let it = if uid.is_empty() {
            self.inner.prop_item()
        } else {
            self.inner.scene().item_by_id(&uid)
        };
        let Some(it) = it else {
            return json!([]);
        };
        props::build_prop_groups_json(it)
    }

    fn show_grid(&self) -> bool {
        self.inner.show_grid()
    }

    fn set_show_grid(&mut self, show: bool) {
        if self.inner.set_show_grid(show) {
            self.appearance_changed();
        }
    }

    fn show_scroll(&self) -> bool {
        self.inner.show_scroll()
    }

    fn set_show_scroll(&mut self, show: bool) {
        if self.inner.set_show_scroll(show) {
            self.show_scroll_changed();
        }
    }

    fn show_component_rects(&self) -> bool {
        self.inner.show_component_rect()
    }

    fn set_show_component_rects(&mut self, show: bool) {
        if self.inner.set_show_component_rect(show) {
            self.appearance_changed();
            self.render_canvas();
        }
    }

    fn time_units(&self) -> Vec<String> {
        cs_engine::settings::TIME_UNITS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    }
    fn scene_width(&self) -> i32 {
        self.inner.circ_settings().width
    }
    fn set_scene_width(&mut self, w: i32) {
        let c = self
            .inner
            .update_circ_settings(|s| s.width = w.clamp(1, 10_000));
        self.apply(c);
    }
    fn scene_height(&self) -> i32 {
        self.inner.circ_settings().height
    }
    fn set_scene_height(&mut self, h: i32) {
        let c = self
            .inner
            .update_circ_settings(|s| s.height = h.clamp(1, 10_000));
        self.apply(c);
    }
    fn animate_logic(&self) -> bool {
        self.inner.circ_settings().animate_logic
    }
    fn set_animate_logic(&mut self, v: bool) {
        let c = self.inner.update_circ_settings(|s| s.animate_logic = v);
        self.apply(c);
    }
    fn animate_curr(&self) -> bool {
        self.inner.circ_settings().animate_curr
    }
    fn set_animate_curr(&mut self, v: bool) {
        let c = self.inner.update_circ_settings(|s| s.animate_curr = v);
        self.apply(c);
    }
    fn ansi_symbols(&self) -> bool {
        self.inner.circ_settings().ansi
    }
    fn set_ansi_symbols(&mut self, v: bool) {
        let c = self.inner.update_circ_settings(|s| s.ansi = v);
        self.apply(c);
    }
    fn speed_percent(&self) -> f64 {
        self.inner.circ_settings().speed_percent()
    }
    fn set_speed_percent(&mut self, p: f64) {
        let c = self.inner.update_circ_settings(|s| s.set_speed_percent(p));
        self.apply(c);
    }
    fn speed_label(&self) -> String {
        self.inner.circ_settings().speed_label()
    }
    fn circ_step(&self) -> i32 {
        self.inner.circ_settings().steps_ps as i32
    }
    fn set_circ_step(&mut self, step: i32) {
        let c = self
            .inner
            .update_circ_settings(|s| s.set_step(step.max(1) as u64));
        self.apply(c);
    }
    fn step_unit(&self) -> i32 {
        self.inner.circ_settings().step_unit()
    }
    fn set_step_unit(&mut self, unit: i32) {
        let c = self.inner.update_circ_settings(|s| s.set_step_unit(unit));
        self.apply(c);
    }
    fn react_step(&self) -> i32 {
        self.inner.circ_settings().react_step_value() as i32
    }
    fn set_react_step(&mut self, n: i32) {
        let c = self.inner.update_circ_settings(|s| {
            let unit = s.react_step_unit();
            s.set_react(n.max(1) as u64, unit);
        });
        self.apply(c);
    }
    fn react_step_unit(&self) -> i32 {
        self.inner.circ_settings().react_step_unit()
    }
    fn set_react_step_unit(&mut self, unit: i32) {
        let c = self.inner.update_circ_settings(|s| {
            let v = s.react_step_value();
            s.set_react(v, unit);
        });
        self.apply(c);
    }
    fn real_step(&self) -> i32 {
        if let Some(c) = self.inner.running_circuit() {
            units::split_time(c.analog_ps()).0 as i32
        } else {
            self.react_step()
        }
    }
    fn real_step_unit(&self) -> i32 {
        if let Some(c) = self.inner.running_circuit() {
            units::split_time(c.analog_ps()).1
        } else {
            self.react_step_unit()
        }
    }
    fn nl_steps(&self) -> i32 {
        self.inner.circ_settings().nl_steps as i32
    }
    fn set_nl_steps(&mut self, n: i32) {
        let c = self
            .inner
            .update_circ_settings(|s| s.nl_steps = n.max(0) as u32);
        self.apply(c);
    }
    fn resets(&self) -> Value {
        let cur = self.inner.circ_settings();
        let def = cs_engine::settings::get().to_circ();
        let on_off = |on: bool| {
            if on {
                cs_engine::i18n::tr("On")
            } else {
                cs_engine::i18n::tr("Off")
            }
        };
        json!({
            "width": { "isDefault": cur.width == def.width, "text": format!("{} px", def.width) },
            "height": { "isDefault": cur.height == def.height, "text": format!("{} px", def.height) },
            "animLogic": { "isDefault": cur.animate_logic == def.animate_logic, "text": on_off(def.animate_logic) },
            "animCurr": { "isDefault": cur.animate_curr == def.animate_curr, "text": on_off(def.animate_curr) },
            "ansi": { "isDefault": cur.ansi == def.ansi, "text": on_off(def.ansi) },
            "speed": {
                "isDefault": cur.step_size == def.step_size && cur.steps_ps == def.steps_ps,
                "text": format!("{:.2}%", def.speed_percent())
            },
            "reactStep": {
                "isDefault": cur.react_step_ps == def.react_step_ps,
                "text": format!("{} {}", def.react_step_value(),
                    cs_engine::settings::TIME_UNITS.get(def.react_step_unit() as usize).unwrap_or(&"ps"))
            },
            "nlSteps": { "isDefault": cur.nl_steps == def.nl_steps, "text": def.nl_steps.to_string() },
        })
    }

    fn sim_circ_time(&self) -> f64 {
        self.inner.circ_time() as f64
    }

    fn sim_mcu_device(&self) -> String {
        self.inner.mcu_info().0
    }

    fn sim_mcu_name(&self) -> String {
        self.inner.mcu_info().1
    }

    fn sim_has_mcu(&self) -> bool {
        self.inner.mcu_info().2
    }

    #[qslot]
    fn begin_circ_settings_edit(&mut self) {
        self.inner.begin_circ_settings_edit();
    }

    #[qslot]
    fn commit_circ_settings_edit(&mut self) {
        let c = self.inner.commit_circ_settings_edit();
        self.apply(c);
    }

    #[qslot]
    fn restore_default(&mut self, key: String) {
        let def = cs_engine::settings::get().to_circ();
        let c = self.inner.update_circ_settings(|s| match key.as_str() {
            "width" => s.width = def.width,
            "height" => s.height = def.height,
            "animLogic" => s.animate_logic = def.animate_logic,
            "animCurr" => s.animate_curr = def.animate_curr,
            "ansi" => s.ansi = def.ansi,
            "nlSteps" => s.nl_steps = def.nl_steps,
            "speed" => {
                s.step_size = def.step_size;
                s.steps_ps = def.steps_ps;
            }
            "reactStep" => s.react_step_ps = def.react_step_ps,
            _ => {}
        });
        self.apply(c);
    }

    fn cursor(&self) -> String {
        self.inner.cursor().as_str().into()
    }

    fn cursor_x(&self) -> i32 {
        self.inner.last_scene().x.round() as i32
    }

    fn cursor_y(&self) -> i32 {
        self.inner.last_scene().y.round() as i32
    }

    fn grid_color(&self) -> String {
        self.grid_color.clone()
    }
    fn canvas_color(&self) -> String {
        self.canvas_color.clone()
    }
    fn viewport_color(&self) -> String {
        self.viewport_color.clone()
    }
    fn band_color(&self) -> String {
        self.band_color.clone()
    }
    fn body_color(&self) -> String {
        self.body_color.clone()
    }
    fn border_color(&self) -> String {
        self.border_color.clone()
    }
    fn wire_color(&self) -> String {
        self.wire_color.clone()
    }
    fn pin_high_color(&self) -> String {
        self.pin_high_color.clone()
    }
    fn pin_low_color(&self) -> String {
        self.pin_low_color.clone()
    }
    fn meter_display_color(&self) -> String {
        self.meter_display_color.clone()
    }
    fn pin_open_high_color(&self) -> String {
        self.pin_open_high_color.clone()
    }
    fn pin_open_low_color(&self) -> String {
        self.pin_open_low_color.clone()
    }
    fn pin_input_high_color(&self) -> String {
        self.pin_input_high_color.clone()
    }
    fn pin_input_low_color(&self) -> String {
        self.pin_input_low_color.clone()
    }
    fn pin_out_high_color(&self) -> String {
        self.pin_out_high_color.clone()
    }
    fn pin_out_low_color(&self) -> String {
        self.pin_out_low_color.clone()
    }
    fn shape_body_color(&self) -> String {
        self.shape_body_color.clone()
    }
    fn prop_label_color(&self) -> String {
        self.prop_label_color.clone()
    }
    fn prop_header_color(&self) -> String {
        self.prop_header_color.clone()
    }
    fn mcu_label_bg(&self) -> String {
        self.mcu_label_bg.clone()
    }
    fn mcu_label_text(&self) -> String {
        self.mcu_label_text.clone()
    }
    fn mcu_label_border(&self) -> String {
        self.mcu_label_border.clone()
    }
    fn chip_body_color(&self) -> String {
        self.chip_body_color.clone()
    }
    fn chip_label_color(&self) -> String {
        self.chip_label_color.clone()
    }
    fn coords_label_text(&self) -> String {
        self.coords_label_text.clone()
    }
    fn tunnel_color1(&self) -> String {
        self.tunnel_color1.clone()
    }
    fn tunnel_color2(&self) -> String {
        self.tunnel_color2.clone()
    }
    fn tunnel_color3(&self) -> String {
        self.tunnel_color3.clone()
    }
    fn msg_ok_bg(&self) -> String {
        self.msg_ok_bg.clone()
    }
    fn msg_ok_text(&self) -> String {
        self.msg_ok_text.clone()
    }
    fn msg_warn_bg(&self) -> String {
        self.msg_warn_bg.clone()
    }
    fn msg_warn_text(&self) -> String {
        self.msg_warn_text.clone()
    }
    fn msg_error_bg(&self) -> String {
        self.msg_error_bg.clone()
    }
    fn msg_error_text(&self) -> String {
        self.msg_error_text.clone()
    }
    fn sim_pause_tint(&self) -> String {
        self.sim_pause_tint.clone()
    }
    fn toggle_button_bg(&self) -> String {
        self.toggle_button_bg.clone()
    }
    fn mem_data_bg(&self) -> String {
        self.mem_data_bg.clone()
    }
    fn mem_data_text(&self) -> String {
        self.mem_data_text.clone()
    }
    fn mem_type_text(&self) -> String {
        self.mem_type_text.clone()
    }
    fn mem_val_text(&self) -> String {
        self.mem_val_text.clone()
    }
    fn mcu_pc_val1_text(&self) -> String {
        self.mcu_pc_val1_text.clone()
    }
    fn mcu_pc_val2_text(&self) -> String {
        self.mcu_pc_val2_text.clone()
    }
    fn scope_channel1_color(&self) -> String {
        self.scope_channel1_color.clone()
    }
    fn scope_channel2_color(&self) -> String {
        self.scope_channel2_color.clone()
    }
    fn scope_channel3_color(&self) -> String {
        self.scope_channel3_color.clone()
    }
    fn scope_channel4_color(&self) -> String {
        self.scope_channel4_color.clone()
    }
    fn la_channel1_color(&self) -> String {
        self.la_channel1_color.clone()
    }
    fn la_channel2_color(&self) -> String {
        self.la_channel2_color.clone()
    }
    fn la_channel3_color(&self) -> String {
        self.la_channel3_color.clone()
    }
    fn la_channel4_color(&self) -> String {
        self.la_channel4_color.clone()
    }
    fn la_channel5_color(&self) -> String {
        self.la_channel5_color.clone()
    }
    fn la_channel6_color(&self) -> String {
        self.la_channel6_color.clone()
    }
    fn la_channel7_color(&self) -> String {
        self.la_channel7_color.clone()
    }
    fn la_channel8_color(&self) -> String {
        self.la_channel8_color.clone()
    }
    fn dark(&self) -> bool {
        self.dark
    }
    fn hover_html(&self) -> String {
        self.hover_html.clone()
    }
    fn hover_visible(&self) -> bool {
        self.hover_visible
    }
    fn hover_x(&self) -> f64 {
        self.hover_x
    }
    fn hover_y(&self) -> f64 {
        self.hover_y
    }

    fn set_dark(&mut self, dark: bool) {
        if self.dark == dark {
            return;
        }
        self.dark = dark;
        self.apply_palette();
        self.appearance_changed();
        self.render_canvas();
    }

    #[qslot]
    fn set_dpr(&mut self, dpr: f64) {
        let d = dpr.max(1.0);
        if (self.dpr - d).abs() > 1e-4 {
            self.dpr = d;
            self.render_canvas();
        }
    }

    #[qslot]
    fn render(&mut self) {
        self.render_canvas();
    }

    #[qslot]
    fn settle_pan(&mut self) {
        let meta = crate::native_canvas::current_canvas_meta();
        let vp = self.inner.viewport();
        let dx = (vp.center().x - meta.center_x).abs() * vp.zoom();
        let dy = (vp.center().y - meta.center_y).abs() * vp.zoom();
        let zoom_changed = (vp.zoom() - meta.zoom).abs() > 1e-4;
        if dx > 1.0 || dy > 1.0 || zoom_changed {
            self.render_canvas();
        }
    }

    #[qslot]
    fn log(&mut self, msg: String) {
        cs_engine::sim_log::push(msg);
        self.flush_sim_logs();
    }

    #[qslot]
    fn set_view_size(&mut self, w: f64, h: f64) {
        let c = self.inner.set_view_size(w, h);
        self.apply(c);
    }

    #[qslot]
    fn record_repaint(&mut self, rects: Value, is_full: bool) {
        self.repaint_recorded(rects, is_full, json!([]));
    }

    #[qslot]
    fn toggle_repaint_overlay(&mut self) {
        cs_engine::settings::edit(|s| {
            s.repaint_overlay = !s.repaint_overlay;
        });
    }

    #[qslot]
    fn toggle_pin_inverted(&mut self, pin_id: String) {
        let c = self.inner.toggle_pin_inverted(&pin_id);
        self.apply(c);
        self.items_changed();
    }

    #[qslot]
    fn set_pin_inverted(&mut self, pin_id: String, inverted: bool) {
        let c = self.inner.set_pin_inverted(&pin_id, inverted);
        self.apply(c);
        self.items_changed();
    }

    #[qslot]
    fn mouse_press(&mut self, button: i32, x: f64, y: f64, mods: i32) {
        let c = self.inner.mouse_press(button as u32, x, y, mods as u32);
        self.coords_changed();
        self.apply(c);
        if self.hover_visible {
            self.hover_visible = false;
            self.hover_changed();
        }
    }

    #[qslot]
    fn mouse_move(&mut self, x: f64, y: f64, buttons: i32, mods: i32) {
        let c = self.inner.mouse_move(x, y, buttons as u32, mods as u32);
        self.coords_changed();
        self.apply(c);
        if buttons == 0 {
            self.update_hover_tooltip(x, y);
        } else if self.hover_visible {
            self.hover_visible = false;
            self.hover_changed();
        }
    }

    #[qslot]
    fn mouse_release(&mut self, button: i32, x: f64, y: f64, mods: i32) {
        let c = self.inner.mouse_release(button as u32, x, y, mods as u32);
        self.coords_changed();
        self.apply(c);
    }

    #[qslot]
    fn mouse_cancel(&mut self) {
        let c = self.inner.mouse_cancel();
        self.apply(c);
        if self.hover_visible {
            self.hover_visible = false;
            self.hover_changed();
        }
    }

    #[qslot]
    fn wheel(
        &mut self,
        pixel_dx: f64,
        pixel_dy: f64,
        angle_dx: f64,
        angle_dy: f64,
        x: f64,
        y: f64,
        mods: i32,
    ) {
        let c = self
            .inner
            .wheel(pixel_dx, pixel_dy, angle_dx, angle_dy, x, y, mods as u32);
        if let Some((uid, val, text, wiper)) = self.inner.take_last_wheel_item() {
            self.item_value_changed(uid, val, text, wiper);
        }
        self.apply(c);
    }

    #[qslot]
    fn pinch_zoom(&mut self, factor: f64, x: f64, y: f64) {
        let c = self.inner.pinch_zoom(factor, x, y);
        self.apply(c);
    }

    #[qslot]
    fn handle_key(&mut self, code: i32, mods: i32) {
        let c = self.inner.key_press(code, mods as u32);
        if let Some((uid, val, text, wiper)) = self.inner.take_last_wheel_item() {
            self.item_value_changed(uid, val, text, wiper);
        }
        self.apply(c);
    }

    #[qslot]
    fn zoom_in(&mut self) {
        let c = self.inner.zoom_in();
        self.apply(c);
    }

    #[qslot]
    fn zoom_out(&mut self) {
        let c = self.inner.zoom_out();
        self.apply(c);
    }

    #[qslot]
    fn zoom_one(&mut self) {
        let c = self.inner.zoom_one();
        self.apply(c);
    }

    #[qslot]
    fn zoom_to_fit(&mut self) {
        let c = self.inner.zoom_to_fit();
        self.apply(c);
    }

    #[qslot]
    fn zoom_selected(&mut self) {
        let c = self.inner.zoom_selected();
        self.apply(c);
    }

    #[qslot]
    fn auto_fit_canvas(&mut self) {
        let c = self.inner.auto_fit_canvas(200.0);
        self.apply(c);
    }

    #[qslot]
    fn center_circuit(&mut self) {
        let c = self.inner.center_circuit();
        self.apply(c);
    }

    #[qslot]
    fn select_overflowing(&mut self) {
        let c = self.inner.select_overflowing_items();
        self.apply(c);
    }

    #[qslot]
    fn focus_component(&mut self, id: String) {
        let c = self.inner.focus_component(&id);
        self.apply(c);
    }

    #[qslot]
    fn add_resistor(&mut self) {
        self.add_component("Resistor".into());
    }

    #[qslot]
    fn add_component(&mut self, typ: String) {
        let p = self.inner.last_scene();
        let c = self.inner.add_component_at(&typ, p);
        self.apply(c);
    }

    #[qslot]
    fn add_all_components(&mut self) {
        let p = self.inner.last_scene();
        let c = self.inner.add_all_components_at(p);
        self.apply(c);
    }

    /// Drop a component at item (view) coordinates. Mime is `name,compType` or
    /// just `compType`.
    #[qslot]
    fn drop_at(&mut self, mime: String, x: f64, y: f64) {
        let scene = interaction::drop_scene_point(&self.inner, x, y);
        let c = self.inner.add_component_at(&mime, scene);
        self.apply(c);
    }

    #[qslot]
    fn power_on(&mut self) {
        let c = self.inner.power_on();
        self.apply(c);
        self.warnings_changed();
    }

    #[qslot]
    fn power_off(&mut self) {
        let c = self.inner.power_off();
        self.apply(c);
        let cur_count = self.inner.overload_log().len();
        let cur_crashed = self.inner.overload_log().iter().any(|e| e.crashed);
        self.last_warnings_count = cur_count;
        self.last_warnings_crashed = cur_crashed;
        self.warnings_changed();
    }

    #[qslot]
    fn pause_qemu(&mut self) {
        self.inner.pause_qemu();
    }

    #[qslot]
    fn resume_qemu(&mut self) {
        self.inner.resume_qemu();
    }

    #[qslot]
    fn set_curr_speed_slider(&mut self, v: i32) {
        self.inner.set_curr_speed_slider(v);
    }

    #[qslot]
    fn tick(&mut self) {
        let t0 = std::time::Instant::now();
        let c = self.inner.tick();
        let t_sim = t0.elapsed();
        self.apply(c);
        let cur_count = self.inner.overload_log().len();
        let cur_crashed = self.inner.overload_log().iter().any(|e| e.crashed);
        if cur_count != self.last_warnings_count || cur_crashed != self.last_warnings_crashed {
            self.last_warnings_count = cur_count;
            self.last_warnings_crashed = cur_crashed;
            self.warnings_changed();
        }
        if self.inner.take_overload_escalated() {
            crate::macos_host::beep();
        }
        if self.hover_visible {
            let last_item = self.inner.last_item();
            self.update_hover_tooltip(last_item.x, last_item.y);
        }
        let total = t0.elapsed();

        // Calculate Simulation Load & GUI Load matching C++ SimulIDE
        let fps = cs_engine::settings::get().fps.clamp(1, 100) as f64;
        let expected_frame_dur_s = 1.0 / fps;
        let sim_ratio = (t_sim.as_secs_f64() / expected_frame_dur_s) * 100.0;
        self.sim_load = (self.sim_load + sim_ratio) * 0.5;
        self.gui_time_ns = self.gui_time_ns.saturating_add(total.as_nanos() as u64);
        self.frame_count = self.frame_count.saturating_add(1);

        let now = std::time::Instant::now();
        if let Some(last_ref) = self.last_ref_time {
            let elapsed = last_ref.elapsed();
            if elapsed >= std::time::Duration::from_secs(1) {
                let elapsed_secs = elapsed.as_secs_f64();
                self.gui_load = ((self.gui_time_ns as f64 / 1e9) / elapsed_secs) * 100.0;
                self.gui_time_ns = 0;
                let cur_circ_time = self.inner.circ_time();
                let circ_time_delta_s =
                    (cur_circ_time.saturating_sub(self.last_circ_time)) as f64 / 1e12;
                self.real_speed = (circ_time_delta_s / elapsed_secs) * 100.0;
                self.last_circ_time = cur_circ_time;
                self.real_fps = (self.frame_count as f64 / elapsed_secs).round() as i32;
                self.frame_count = 0;
                self.last_ref_time = Some(now);
            }
        } else {
            self.last_ref_time = Some(now);
            self.last_circ_time = self.inner.circ_time();
        }
    }

    fn sim_load_val(&self) -> f64 {
        self.sim_load
    }

    fn sim_gui_load_val(&self) -> f64 {
        self.gui_load
    }

    fn sim_real_speed_val(&self) -> f64 {
        self.real_speed
    }

    fn sim_fps_val(&self) -> i32 {
        self.real_fps
    }

    /// Drain MCU monitor pokes and publish RAM / PC / STATUS.
    #[qslot]
    fn sync_mcu(&mut self) {
        let c = self.inner.sync_mcu();
        self.apply(c);
    }

    #[qslot]
    fn poke_mcu_ram(&mut self, address: i32, val: i32) {
        if address < 0 {
            return;
        }
        let c = self.inner.poke_mcu_ram(address as u16, val as u8);
        self.apply(c);
    }

    #[qslot]
    fn poke_mcu_flash(&mut self, address: i32, val: i32) {
        if address < 0 {
            return;
        }
        let c = self.inner.poke_mcu_flash(address as u32, val as u16);
        self.apply(c);
    }

    #[qslot]
    fn poke_mcu_eeprom(&mut self, address: i32, val: i32) {
        if address < 0 {
            return;
        }
        let c = self.inner.poke_mcu_eeprom(address as u16, val as u8);
        self.apply(c);
    }

    fn scope_traces(&self) -> Value {
        plots::traces_json(self.inner.scope_traces())
    }

    fn la_traces(&self) -> Value {
        plots::traces_json(self.inner.la_traces())
    }

    #[qslot]
    fn set_scope_time_div(&mut self, v: f64) {
        let c = self.inner.set_scope_time_div(v);
        self.apply(c);
    }

    #[qslot]
    fn set_scope_time_pos(&mut self, v: f64) {
        let c = self.inner.set_scope_time_pos(v);
        self.apply(c);
    }

    #[qslot]
    fn set_scope_volt_div(&mut self, ch: i32, v: f64) {
        if ch >= 0 && ch < 4 {
            let c = self.inner.set_scope_volt_div(ch as usize, v);
            self.apply(c);
        }
    }

    #[qslot]
    fn set_scope_volt_pos(&mut self, ch: i32, v: f64) {
        if ch >= 0 && ch < 4 {
            let c = self.inner.set_scope_volt_pos(ch as usize, v);
            self.apply(c);
        }
    }

    #[qslot]
    fn set_scope_tracks(&mut self, tracks: i32) {
        let c = self.inner.set_scope_tracks(tracks);
        self.apply(c);
    }

    #[qslot]
    fn set_scope_hidden(&mut self, ch: i32, hide: bool) {
        if ch >= 0 && ch < 4 {
            let c = self.inner.set_scope_hidden(ch as usize, hide);
            self.apply(c);
        }
    }

    #[qslot]
    fn set_scope_trigger(&mut self, ch: i32) {
        let c = self.inner.set_scope_trigger(ch);
        self.apply(c);
    }

    #[qslot]
    fn set_scope_trig_level(&mut self, level: f64) {
        let c = self.inner.set_scope_trig_level(level);
        self.apply(c);
    }

    #[qslot]
    fn set_scope_filter(&mut self, v: f64) {
        let c = self.inner.set_scope_filter(v);
        self.apply(c);
    }

    #[qslot]
    fn auto_scale_scope(&mut self, ch: i32) -> Value {
        if ch < 0 || ch >= 4 {
            return json!({ "valid": false });
        }
        if let Some(res) = self.inner.auto_scale_scope(ch as usize) {
            json!({
                "valid": true,
                "voltDiv": res.volt_div,
                "voltPos": res.volt_pos,
                "timeDiv": res.time_div.unwrap_or(0.0),
            })
        } else {
            json!({ "valid": false })
        }
    }

    #[qslot]
    fn auto_scale_scope_all(&mut self) -> Value {
        let res = self.inner.auto_scale_scope_all();
        let chs: Vec<Value> = res
            .channels
            .into_iter()
            .map(|opt| {
                if let Some(r) = opt {
                    json!({
                        "valid": true,
                        "voltDiv": r.volt_div,
                        "voltPos": r.volt_pos,
                        "timeDiv": r.time_div.unwrap_or(0.0),
                    })
                } else {
                    json!({ "valid": false })
                }
            })
            .collect();
        json!({
            "channels": chs,
            "timeDiv": res.time_div.unwrap_or(0.0),
        })
    }

    #[qslot]
    fn set_la_time_div(&mut self, v: f64) {
        let c = self.inner.set_la_time_div(v);
        self.apply(c);
    }

    #[qslot]
    fn set_la_time_pos(&mut self, v: f64) {
        let c = self.inner.set_la_time_pos(v);
        self.apply(c);
    }

    #[qslot]
    fn set_la_trigger(&mut self, ch: i32) {
        let c = self.inner.set_la_trigger(ch);
        self.apply(c);
    }

    #[qslot]
    fn set_la_thresholds(&mut self, rise: f64, fall: f64) {
        let c = self.inner.set_la_thresholds(rise, fall);
        self.apply(c);
    }

    #[qslot]
    fn new_circuit(&mut self) {
        if self.guard_replace(PendingReplace::New) {
            return;
        }
        self.replace_with_new();
    }

    #[qslot]
    fn close_circuit(&mut self) {
        if self.guard_replace(PendingReplace::New) {
            return;
        }
        self.replace_with_new();
    }

    #[qslot]
    fn load_path(&mut self, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        if self.guard_replace(PendingReplace::Load(path.clone())) {
            return;
        }
        self.load_path_now(&path);
    }

    #[qslot]
    fn load_circ(&mut self, url: String) {
        self.load_path(url);
    }

    #[qslot]
    fn load_subcircuit(&mut self, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        match files::load_path_now(&mut self.inner, &self.project_path, &path) {
            Ok(c) => {
                self.apply(c);
                self.file_path_changed();
                self.canvas_overflow_changed();
                self.last_warnings_count = 0;
                self.last_warnings_crashed = false;
                self.warnings_changed();
            }
            Err(_) => {
                self.flush_sim_logs();
            }
        }
    }

    #[qslot]
    fn save_path(&mut self, url: String) {
        let raw_path = strip_file_url(&url);
        match files::save_path(&mut self.inner, &self.project_path, &raw_path) {
            Ok(path) => {
                if path.is_empty() {
                    return;
                }
                self.flush_sim_logs();
                self.file_path_changed();
                self.history_changed();
                if !matches!(self.pending_replace, PendingReplace::None) {
                    self.finish_pending_replace();
                }
            }
            Err(_) => {
                self.flush_sim_logs();
            }
        }
    }

    #[qslot]
    fn save(&mut self) {
        if let Some(path) = self.inner.file_path().map(|s| s.to_string()) {
            self.save_path(path);
        }
    }

    #[qslot]
    fn restore_draft(&mut self) {
        if let Some(draft) = files::restore_draft_draft(&self.inner, &self.project_path) {
            self.apply_circuit_draft(draft);
        }
    }

    #[qslot]
    fn flush_draft(&mut self) {
        self.persist_circuit_draft_now();
        let circuit_file = self.inner.file_path().map(|s| s.to_string());
        cs_engine::project::update_session(&self.project_path, |s| {
            s.circuit_file = circuit_file;
        });
    }

    #[qslot]
    fn confirm_replace_save(&mut self) {
        if self.inner.file_path().is_none() {
            self.request_save_as_then_replace();
            return;
        }
        self.save();
    }

    #[qslot]
    fn confirm_replace_discard(&mut self) {
        let path = self
            .inner
            .file_path()
            .map(|s| s.to_string())
            .unwrap_or_default();
        backup::clear_circuit_for(&path, &self.project_path);
        self.finish_pending_replace();
    }

    #[qslot]
    fn confirm_replace_cancel(&mut self) {
        self.pending_replace = PendingReplace::None;
    }

    #[qslot]
    fn set_item_label_pos(&mut self, uid: String, lx: f64, ly: f64) {
        if let Some(it) = self.inner.scene_mut().item_by_id_mut(&uid) {
            it.label_x = lx;
            it.label_y = ly;
            it.custom_label_pos = true;
            self.items_changed();
            self.history_changed();
            self.persist_circuit_draft();
        }
    }

    #[qslot]
    fn set_item_val_label_pos(&mut self, uid: String, vx: f64, vy: f64) {
        if let Some(it) = self.inner.scene_mut().item_by_id_mut(&uid) {
            it.val_x = vx;
            it.val_y = vy;
            it.custom_val_pos = true;
            self.items_changed();
            self.history_changed();
            self.persist_circuit_draft();
        }
    }

    #[qslot]
    fn rotate_item_label(&mut self, uid: String, angle: i32) {
        let c = self.inner.rotate_item_label(&uid, angle);
        self.apply(c);
    }

    #[qslot]
    fn rotate_item_val_label(&mut self, uid: String, angle: i32) {
        let c = self.inner.rotate_item_val_label(&uid, angle);
        self.apply(c);
    }

    #[qslot]
    fn mouse_double_click(&mut self, button: i32, x: f64, y: f64, mods: i32) {
        let c = self
            .inner
            .mouse_double_click(button as u32, x, y, mods as u32);
        self.apply(c);
    }

    #[qslot]
    fn undo(&mut self) {
        let c = self.inner.undo();
        self.apply(c);
    }

    #[qslot]
    fn redo(&mut self) {
        let c = self.inner.redo();
        self.apply(c);
    }

    #[qslot]
    fn copy_selection(&mut self) {
        if self.inner.copy_selection() {
            self.history_changed();
        }
    }

    #[qslot]
    fn cut_selection(&mut self) {
        let c = self.inner.cut_selection();
        self.apply(c);
    }

    #[qslot]
    fn remove_selection(&mut self) {
        let c = self.inner.delete_selected();
        self.apply(c);
    }

    #[qslot]
    fn select_all(&mut self) {
        let c = self.inner.select_all();
        self.apply(c);
    }

    #[qslot]
    fn paste_at_cursor(&mut self) {
        let c = self.inner.paste_at_cursor();
        self.apply(c);
    }

    #[qslot]
    fn rotate_cw(&mut self) {
        let c = self.inner.rotate_cw();
        self.apply(c);
    }

    #[qslot]
    fn rotate_ccw(&mut self) {
        let c = self.inner.rotate_ccw();
        self.apply(c);
    }

    #[qslot]
    fn rotate_180(&mut self) {
        let c = self.inner.rotate_selected(180.0);
        self.apply(c);
    }

    #[qslot]
    fn flip_h(&mut self) {
        let c = self.inner.flip_h();
        self.apply(c);
    }

    #[qslot]
    fn flip_v(&mut self) {
        let c = self.inner.flip_v();
        self.apply(c);
    }

    #[qslot]
    fn open_selected_properties(&mut self) {
        let c = self.inner.open_selected_properties();
        self.apply(c);
    }

    #[qslot]
    fn close_properties(&mut self) {
        let c = self.inner.close_properties();
        self.apply(c);
    }

    #[qslot]
    fn open_properties(&mut self, uid: String) {
        let c = self.inner.open_properties(&uid);
        self.apply(c);
    }

    #[qslot]
    fn close_property_dialog(&mut self, uid: String) {
        let c = self.inner.close_property_dialog(&uid);
        self.apply(c);
    }

    #[qslot]
    fn set_prop_label_for(&mut self, uid: String, text: String) {
        let c = self.inner.set_prop_label_for(&uid, text);
        self.apply(c);
    }

    #[qslot]
    fn set_prop_show_id_for(&mut self, uid: String, show: bool) {
        let c = self.inner.set_prop_show_id_for(&uid, show);
        self.apply(c);
    }

    #[qslot]
    fn set_show_prop_for(&mut self, uid: String, name: String, show: bool) {
        let c = self.inner.set_show_prop_for(&uid, name, show);
        self.apply(c);
    }

    #[qslot]
    fn set_prop_text_for(&mut self, uid: String, name: String, text: String) {
        let c = self.inner.set_prop_text_for(&uid, name, text);
        self.apply(c);
    }

    #[qslot]
    fn set_prop_bool_for(&mut self, uid: String, name: String, value: bool) {
        let c = self.inner.set_prop_bool_for(&uid, name, value);
        self.apply(c);
    }

    #[qslot]
    fn set_prop_label(&mut self, text: String) {
        let c = self.inner.set_prop_label(text);
        self.apply(c);
    }

    #[qslot]
    fn set_prop_show_id(&mut self, show: bool) {
        let c = self.inner.set_prop_show_id(show);
        self.apply(c);
    }

    #[qslot]
    fn set_show_prop(&mut self, name: String, show: bool) {
        let c = self.inner.set_show_prop(name, show);
        self.apply(c);
    }

    #[qslot]
    fn set_prop_text(&mut self, name: String, text: String) {
        let c = self.inner.set_prop_text(name, text);
        self.apply(c);
    }

    #[qslot]
    fn set_prop_bool(&mut self, name: String, value: bool) {
        let c = self.inner.set_prop_bool(name, value);
        self.apply(c);
    }

    #[qslot]
    fn save_image(&mut self, url: String) {
        let path = strip_file_url(&url);
        let pal = cs_engine::canvas::Palette::from_hex(
            &self.canvas_color,
            &self.grid_color,
            &self.body_color,
            &self.border_color,
            &self.wire_color,
            &self.pin_high_color,
            &self.pin_low_color,
            &self.band_color,
        );
        if let Err(e) = self.inner.save_image(&path, &pal) {
            eprintln!("save image: {e}");
        }
    }

    #[qslot]
    fn upload_firmware(&mut self, path: String) {
        let path = strip_file_url(&path);
        if path.is_empty() {
            return;
        }
        let c = self
            .inner
            .upload_firmware_to(&path, self.active_device_id.as_deref());
        self.apply(c);
    }

    #[qslot]
    fn firmware_source(&mut self, uid: String) -> String {
        let uid = if uid.is_empty() {
            self.selected_item_uid()
        } else {
            uid
        };
        if !uid.is_empty() {
            self.set_active_device_id(uid.clone());
        }
        self.inner
            .firmware_source_for(&uid)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    #[qslot]
    fn edit_firmware(&mut self, uid: String) {
        let uid = if !uid.is_empty() {
            uid
        } else if !self.selected_item_uid().is_empty() {
            self.selected_item_uid()
        } else if let Some(aid) = &self.active_device_id {
            aid.clone()
        } else {
            let devs = self.inner.collect_programmable_devices(None);
            devs.first().map(|d| d.id.clone()).unwrap_or_default()
        };
        if !uid.is_empty() {
            self.set_active_device_id(uid.clone());
        }
        let firm_path = self.inner.firmware_path_for(&uid);
        let src = self.inner.firmware_source_for(&uid);

        if let Some(src_path) = src {
            let path_str = src_path.to_string_lossy().into_owned();
            self.request_open_file(path_str);
        } else {
            if let Some(fp) = firm_path {
                let msg_prefix = cs_engine::i18n::tr(
                    "Edit firmware: No source file found next to the firmware:",
                );
                cs_engine::logging::log_sim(format!("{msg_prefix}\n{}", fp.display()));
            } else {
                cs_engine::logging::log_sim(cs_engine::i18n::tr(
                    "Edit firmware: No firmware loaded.",
                ));
            }
            self.flush_sim_logs();
            self.request_show_editor();
        }
    }

    #[qslot]
    fn open_subcircuit(&mut self, uid: String) {
        if let Some(it) = self.inner.scene().item_by_id(&uid) {
            if let cs_engine::canvas::Part::Subcircuit(sub) = &it.kind {
                let label = if !it.label.is_empty() {
                    it.label.clone()
                } else if !sub.device.is_empty() {
                    sub.device.clone()
                } else {
                    it.id.clone()
                };
                let target_path = sub.nested_path.clone().or_else(|| {
                    cs_engine::subcircuit::resolve_subc(
                        &sub.device,
                        &cs_engine::subcircuit::SubcSearch::from_circuit_path(
                            self.inner.file_path(),
                        ),
                    )
                    .and_then(|r| r.path.map(|p| p.to_string_lossy().into_owned()))
                });
                if let Some(path) = target_path {
                    self.open_subcircuit_requested(path, label);
                }
            }
        }
    }

    #[qslot]
    fn load_wav_file(&mut self, item_id: String, path: String) {
        let path = strip_file_url(&path);
        if path.is_empty() {
            return;
        }
        let c = self.inner.load_wav_file(&item_id, &path);
        self.apply(c);
    }

    #[qslot]
    fn toggle_item(&mut self, uid: String) {
        let c = self.inner.toggle_item(&uid);
        self.apply(c);
    }

    #[qslot]
    fn toggle_dip_switch(&mut self, uid: String, index: i32) {
        let c = self.inner.toggle_dip_switch(&uid, index as usize);
        self.apply(c);
    }

    #[qslot]
    fn set_dip_switch(&mut self, uid: String, index: i32, on: bool) {
        let c = self.inner.set_dip_switch(&uid, index as usize, on);
        self.apply(c);
    }

    #[qslot]
    fn set_push_state(&mut self, uid: String, pressed: bool) {
        let c = self.inner.set_push_state(&uid, pressed);
        self.apply(c);
    }

    #[qslot]
    fn set_push_button_pressed(&mut self, uid: String, pressed: bool) {
        let c = self.inner.set_push_state(&uid, pressed);
        self.apply(c);
    }

    #[qslot]
    fn set_keypad_key_pressed(&mut self, uid: String, row: usize, col: usize, pressed: bool) {
        let c = self.inner.set_keypad_pressed(&uid, row, col, pressed);
        self.apply(c);
    }

    #[qslot]
    fn set_pot_wiper(&mut self, uid: String, wiper: f64) {
        let c = self.inner.set_pot_wiper(&uid, wiper);
        if let Some((uid, val, text, wiper)) = self.inner.take_last_wheel_item() {
            self.item_value_changed(uid, val, text, wiper);
        }
        self.apply(c);
    }

    #[qslot]
    fn set_touchpad_pos(&mut self, uid: String, x: i32, y: i32) {
        let c = self.inner.set_touchpad_pos(&uid, x, y);
        self.apply(c);
    }

    #[qslot]
    fn reset_touchpad(&mut self, uid: String) {
        let c = self.inner.reset_touchpad(&uid);
        self.apply(c);
    }

    #[qslot]
    fn set_joystick_pos(&mut self, uid: String, x: f64, y: f64) {
        let c = self.inner.set_joystick_pos(&uid, x, y);
        self.apply(c);
    }

    #[qslot]
    fn set_joystick_button(&mut self, uid: String, down: bool) {
        let c = self.inner.set_joystick_button(&uid, down);
        self.apply(c);
    }

    #[qslot]
    fn set_rotary_dial(&mut self, uid: String, val: i32) {
        let c = self.inner.set_rotary_dial(&uid, val);
        self.apply(c);
    }

    #[qslot]
    fn set_rotary_button(&mut self, uid: String, closed: bool) {
        let c = self.inner.set_rotary_button(&uid, closed);
        self.apply(c);
    }

    #[qslot]
    fn set_sr04_distance(&mut self, uid: String, dist: f64) {
        let c = self.inner.set_sr04_distance(&uid, dist);
        self.apply(c);
    }

    #[qslot]
    fn step_sensor_temp(&mut self, uid: String, up: bool) {
        let c = self.inner.step_sensor_temp(&uid, up);
        self.apply(c);
    }

    #[qslot]
    fn step_sensor_humi(&mut self, uid: String, up: bool) {
        let c = self.inner.step_sensor_humi(&uid, up);
        self.apply(c);
    }

    #[qslot]
    fn set_dial_val(&mut self, uid: String, val: f64) {
        let c = self.inner.set_dial_val(&uid, val);
        if let Some((uid, val, text, wiper)) = self.inner.take_last_wheel_item() {
            self.item_value_changed(uid, val, text, wiper);
        }
        self.apply(c);
    }

    #[qslot]
    fn import_path(&mut self, url: String) {
        let path = strip_file_url(&url);
        match std::fs::read_to_string(&path) {
            Ok(src) => {
                let c = self.inner.import_sim1(&src);
                self.apply(c);
            }
            Err(e) => eprintln!("import circuit: {e}"),
        }
    }

    #[qslot]
    fn add_package_pin(
        &mut self,
        uid: String,
        angle: i32,
        x: i32,
        y: i32,
        id: String,
        label: String,
    ) {
        let c = self.inner.add_package_pin(&uid, angle, x, y, &id, &label);
        self.apply(c);
    }

    #[qslot]
    fn remove_package_pin(&mut self, uid: String, pin_id: String) {
        let c = self.inner.remove_package_pin(&uid, &pin_id);
        self.apply(c);
    }

    #[qslot]
    fn update_package_pin(
        &mut self,
        uid: String,
        old_pin_id: String,
        new_id: String,
        label: String,
        ptype: String,
        xpos: i32,
        ypos: i32,
        angle: i32,
        length: i32,
        space: i32,
    ) {
        let new_pin = cs_engine::package::PkgPin {
            id: new_id,
            label,
            pin_type: ptype,
            xpos,
            ypos,
            angle,
            length,
            space,
        };
        let c = self.inner.update_package_pin(&uid, &old_pin_id, new_pin);
        self.apply(c);
    }

    #[qslot]
    fn generate_package_pins(
        &mut self,
        uid: String,
        left: i32,
        right: i32,
        top: i32,
        bottom: i32,
        prefix: String,
        start_index: i32,
        clear_first: bool,
    ) {
        let c = self.inner.generate_package_pins(
            &uid,
            left.max(0) as usize,
            right.max(0) as usize,
            top.max(0) as usize,
            bottom.max(0) as usize,
            &prefix,
            start_index.max(0) as usize,
            clear_first,
        );
        self.apply(c);
    }

    #[qslot]
    fn set_package_dimensions(&mut self, uid: String, width: i32, height: i32) {
        let c = self.inner.set_package_dimensions(&uid, width, height);
        self.apply(c);
    }

    #[qslot]
    fn set_package_footprint_dip(
        &mut self,
        uid: String,
        name: String,
        pin_count: i32,
        custom_width: i32,
    ) {
        let cw = if custom_width > 0 {
            Some(custom_width)
        } else {
            None
        };
        let c = self
            .inner
            .set_package_footprint_dip(&uid, &name, pin_count.max(2) as usize, cw);
        self.apply(c);
    }

    #[qslot]
    fn set_package_footprint_ls(
        &mut self,
        uid: String,
        name: String,
        inputs: String,
        outputs: String,
        top_pins: String,
        bottom_pins: String,
    ) {
        let inps: Vec<&str> = inputs
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let outs: Vec<&str> = outputs
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let tops: Vec<&str> = top_pins
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let bots: Vec<&str> = bottom_pins
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let c = self
            .inner
            .set_package_footprint_ls(&uid, &name, &inps, &outs, &tops, &bots);
        self.apply(c);
    }

    #[qslot]
    fn load_package_file(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        match std::fs::read_to_string(&path) {
            Ok(xml) => {
                let c = self.inner.load_package_xml(&uid, &xml);
                self.apply(c);
            }
            Err(e) => eprintln!("load package file: {e}"),
        }
    }

    #[qslot]
    fn save_package_file(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        if let Some(xml) = self.inner.export_package_xml(&uid) {
            if let Err(e) = std::fs::write(&path, xml) {
                eprintln!("save package file: {e}");
            }
        }
    }

    #[qslot]
    fn load_firmware(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        if !uid.is_empty() {
            self.set_active_device_id(uid.clone());
        }
        let c = self.inner.load_firmware_path(&uid, &path);
        self.apply(c);
        self.flush_sim_logs();
    }

    #[qslot]
    fn reload_firmware(&mut self, uid: String) {
        if !uid.is_empty() {
            self.set_active_device_id(uid.clone());
        }
        let c = self.inner.reload_firmware(&uid);
        self.apply(c);
        self.flush_sim_logs();
    }

    #[qslot]
    fn load_eeprom(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        let c = self.inner.load_eeprom_file(&uid, &path);
        self.apply(c);
        self.flush_sim_logs();
    }

    #[qslot]
    fn save_eeprom(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        let _ = self.inner.save_eeprom_file(&uid, &path);
        self.flush_sim_logs();
    }

    #[qslot]
    fn load_item_data(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        let c = self.inner.load_item_data_file(&uid, &path);
        self.apply(c);
        self.flush_sim_logs();
    }

    #[qslot]
    fn save_item_data(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        let _ = self.inner.save_item_data_file(&uid, &path);
        self.flush_sim_logs();
    }

    #[qslot]
    fn memory_table_rows(&self, uid: String) -> Value {
        plots::memory_table_rows_json(&self.inner, &uid)
    }

    #[qslot]
    fn memory_item_info(&self, uid: String) -> Value {
        let Some(info) = self.inner.memory_item_info(&uid) else {
            return Value::Null;
        };
        json!({
            "id": info.id,
            "title": info.title,
            "isRom": info.is_rom,
            "cellBytes": info.cell_bytes,
            "wordBytes": info.word_bytes,
            "data": info.data,
            "totalBytes": info.data.len(),
        })
    }

    #[qslot]
    fn set_memory_byte(&mut self, uid: String, addr: i32, val: i32) {
        if addr >= 0 && val >= 0 {
            self.inner.set_memory_byte(&uid, addr as usize, val as u8);
        }
    }

    #[qslot]
    fn set_memory_bytes(&mut self, uid: String, data: Value) {
        if let Value::Array(arr) = data {
            let bytes: Vec<u8> = arr
                .iter()
                .filter_map(|v| v.as_u64().map(|n| n as u8))
                .collect();
            self.inner.set_memory_bytes(&uid, &bytes);
        }
    }

    #[qslot]
    fn get_memory_bytes(&self, uid: String) -> Value {
        if let Some(bytes) = self.inner.get_memory_bytes(&uid) {
            json!(bytes)
        } else {
            Value::Null
        }
    }

    #[qslot]
    fn load_functions(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        let c = self.inner.load_functions_file(&uid, &path);
        self.apply(c);
    }

    #[qslot]
    fn save_functions(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        let _ = self.inner.save_functions_file(&uid, &path);
    }

    #[qslot]
    fn load_image_file(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        let c = self.inner.load_image_file(&uid, &path);
        self.apply(c);
    }

    #[qslot]
    fn save_image_item(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        let _ = self.inner.save_image_item(&uid, &path);
    }

    #[qslot]
    fn load_sd_image(&mut self, uid: String, url: String) {
        let path = strip_file_url(&url);
        if path.is_empty() {
            return;
        }
        let c = self.inner.load_sd_image(&uid, &path);
        self.apply(c);
    }

    #[qslot]
    fn eject_sd_card(&mut self, uid: String) {
        let c = self.inner.eject_sd_card(&uid);
        self.apply(c);
    }

    #[qslot]
    fn hide_tunnel_group(&mut self, uid: String) {
        let c = self.inner.set_tunnel_group_visible(&uid, false);
        self.apply(c);
    }

    #[qslot]
    fn show_tunnel_group(&mut self, uid: String) {
        let c = self.inner.set_tunnel_group_visible(&uid, true);
        self.apply(c);
    }

    #[qslot]
    fn rename_tunnel_group(&mut self, uid: String, name: String) {
        let c = self.inner.rename_tunnel_group(&uid, &name);
        self.apply(c);
    }

    #[qslot]
    fn toggle_probe_pause(&mut self, uid: String) {
        let c = self.inner.toggle_probe_pause(&uid);
        self.apply(c);
    }

    #[qslot]
    fn test_unit_truth(&self, uid: String) -> Value {
        plots::test_unit_truth_json(&self.inner, &uid)
    }

    #[qslot]
    fn start_linking(&mut self, uid: String) {
        self.inner.start_linking(&uid);
        self.items_changed();
    }

    #[qslot]
    fn complete_link(&mut self) {
        let target = self.selected_item_uid();
        let c = self.inner.complete_link(&target);
        self.apply(c);
    }

    #[qslot]
    fn stop_linking(&mut self) {
        self.inner.stop_linking();
        self.items_changed();
    }

    #[qslot]
    fn upload_and_run(&mut self, uid: String, debug: bool) {
        if !uid.is_empty() {
            self.set_active_device_id(uid.clone());
        }
        self.edit_firmware(uid);
        self.request_upload_run(debug);
    }

    #[qslot]
    fn get_package_pin_data(&self, uid: String, pin_id: String) -> Value {
        props::package_pin_data_json(&self.inner, &uid, &pin_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use files::ensure_circ1_path;

    #[test]
    fn test_ensure_circ1_path() {
        assert_eq!(ensure_circ1_path("circuit.sim1"), "circuit.circ1");
        assert_eq!(
            ensure_circ1_path("/path/to/my_circuit.sim1"),
            "/path/to/my_circuit.circ1"
        );
        assert_eq!(ensure_circ1_path("circuit.sim2"), "circuit.circ1");
        assert_eq!(
            ensure_circ1_path("/path/to/my_circuit.sim2"),
            "/path/to/my_circuit.circ1"
        );
        assert_eq!(ensure_circ1_path("circuit.circ1"), "circuit.circ1");
        assert_eq!(
            ensure_circ1_path("/path/to/my_circuit.circ1"),
            "/path/to/my_circuit.circ1"
        );
        assert_eq!(ensure_circ1_path("circuit"), "circuit.circ1");
        assert_eq!(
            ensure_circ1_path("/path/to/my_circuit"),
            "/path/to/my_circuit.circ1"
        );
        assert_eq!(ensure_circ1_path("circuit.circ"), "circuit.circ.circ1");
        assert_eq!(ensure_circ1_path(""), "");
    }

    #[test]
    fn test_cull_bounds_and_item_wire_bounds() {
        let mut cc = CircuitCanvas::default();
        cc.inner.set_view_size(1000.0, 800.0);
        assert!(cc.cull_min_x() < cc.cull_max_x());
        assert!(cc.cull_min_y() < cc.cull_max_y());
        let vr = cc.visible_rect();
        let vr_w = vr["w"].as_f64().unwrap_or(0.0);
        let vr_h = vr["h"].as_f64().unwrap_or(0.0);
        assert!(vr_w > 0.0);
        assert!(vr_h > 0.0);
        assert!(cc.cull_max_x() - cc.cull_min_x() > vr_w);
        assert!(cc.cull_max_y() - cc.cull_min_y() > vr_h);

        let _ = cc.inner.load_sim1(
            r#"<circuit version="1.0.0">
  <item itemtype="Resistor" CircId="R1" Pos="50,100" Resistance="1000 Ω"/>
  <item itemtype="Connector" CircId="c1" startpinid="R1-lPin" endpinid="R1-rPin" pointList="50,100,150,100"/>
</circuit>"#,
            None,
        );
        let items_val = cc.items();
        let items_arr = items_val.as_array().expect("items must be array");
        assert_eq!(items_arr.len(), 1);
        let it0 = &items_arr[0];
        assert!(it0.get("boundsX").is_some());
        assert!(it0.get("boundsY").is_some());
        let bw = it0["boundsW"].as_f64().unwrap_or(0.0);
        let bh = it0["boundsH"].as_f64().unwrap_or(0.0);
        assert!(bw > 0.0, "boundsW must be positive");
        assert!(bh > 0.0, "boundsH must be positive");

        let wires_val = cc.wires();
        let wires_arr = wires_val.as_array().expect("wires must be array");
        assert_eq!(wires_arr.len(), 1);
        let w0 = &wires_arr[0];
        assert!(w0.get("boundsX").is_some());
        assert!(w0.get("boundsY").is_some());
        let w_bw = w0["boundsW"].as_f64().unwrap_or(0.0);
        let w_bh = w0["boundsH"].as_f64().unwrap_or(0.0);
        assert!(w_bw > 0.0, "wire boundsW must be positive");
        assert!(w_bh > 0.0, "wire boundsH must be positive");
    }

    #[test]
    fn test_show_component_rects_property() {
        let mut cc = CircuitCanvas::default();
        let initial = cc.show_component_rects();
        assert_eq!(initial, cc.inner.show_component_rect());

        cc.inner.set_show_component_rect(true);
        assert!(cc.show_component_rects());
        assert!(cc.inner.show_component_rect());

        cc.inner.set_show_component_rect(false);
        assert!(!cc.show_component_rects());
        assert!(!cc.inner.show_component_rect());
    }

    #[test]
    fn test_no_project_does_not_restore_draft() {
        let cc = CircuitCanvas::default();
        let _ = crate::pending::NO_PROJECT.set(true);
        assert!(files::restore_draft_draft(&cc.inner, "").is_none());
    }
}
