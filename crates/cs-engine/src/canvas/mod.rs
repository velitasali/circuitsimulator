//! Circuit canvas: scene + hit-test + viewport. No Qt.
//!
//! Input is item-coordinate mouse/wheel matching `CircuitCanvas`.
//! QML draws from the scene snapshot; this module owns selection, wires, and
//! the viewport transform.

pub(crate) mod clipboard;
pub(crate) mod dirty;
pub(crate) mod drag;
pub mod draw;
pub mod events;
pub mod export;
pub mod files;
mod geom;
pub(crate) mod history;
pub(crate) mod pin;
pub(crate) mod place;
pub(crate) mod props;
pub mod scene;
pub(crate) mod sim_sync;
pub(crate) mod tooltip;
mod viewport;
mod wire;
pub(crate) mod wire_tool;

pub use crate::CircSettings;
pub use dirty::DirtySet;
pub use draw::{Align, Color, Draw, PaintCtx, Palette};
pub use events::{
    BUTTON_LEFT, BUTTON_MIDDLE, BUTTON_RIGHT, Change, CursorKind, KEY_0, KEY_A, KEY_BACKSPACE,
    KEY_C, KEY_DELETE, KEY_DOWN, KEY_EQUAL, KEY_ESCAPE, KEY_L, KEY_LEFT, KEY_MINUS, KEY_PLUS,
    KEY_R, KEY_RIGHT, KEY_UP, KEY_V, KEY_X, KEY_Y, KEY_Z, MOD_ALT, MOD_CTRL, MOD_META, MOD_SHIFT,
};
pub use export::{
    Pixmap, parse_hex, png_bytes, render_viewport, render_viewport_mut,
    render_viewport_regions_mut, save_image, svg_string,
};
pub use files::MemoryItemInfo;
pub use geom::{Point, Rect, map_rect, snap_to_grid4, to_grid};
pub use pin::{PIN_HIT_RADIUS, Pin, PinDirection, PinGeom};
pub use place::PlaceKind;
pub use scene::{
    CanvasOverflowInfo, Item, Part, PropGroup, PropRow, Scene, resistor_hit_rect,
    resistor_selection_rect,
};
pub use viewport::{
    DEFAULT_SCENE_HEIGHT, DEFAULT_SCENE_WIDTH, MAX_ZOOM, MIN_ZOOM, Viewport, ZOOM_STEP,
};
pub use wire::{Wire, WireHit};

use drag::Drag;
use history::{History, Snapshot};
use viewport::Viewport as Vp;

#[inline]
pub(crate) fn with_pin_id<R>(comp_id: &str, suffix: &str, f: impl FnOnce(&str) -> R) -> R {
    let len = comp_id.len() + 1 + suffix.len();
    if len <= 128 {
        let mut buf = [0u8; 128];
        buf[..comp_id.len()].copy_from_slice(comp_id.as_bytes());
        buf[comp_id.len()] = b'-';
        buf[comp_id.len() + 1..len].copy_from_slice(suffix.as_bytes());
        if let Ok(s) = std::str::from_utf8(&buf[..len]) {
            return f(s);
        }
    }
    f(&format!("{comp_id}-{suffix}"))
}

#[inline]
pub(crate) fn with_pin_id_idx<R>(
    comp_id: &str,
    prefix: &str,
    idx: impl std::fmt::Display,
    f: impl FnOnce(&str) -> R,
) -> R {
    use std::io::Write;
    let mut buf = [0u8; 128];
    let mut cursor = std::io::Cursor::new(&mut buf[..]);
    if write!(cursor, "{comp_id}-{prefix}{idx}").is_ok() {
        let len = cursor.position() as usize;
        if let Ok(s) = std::str::from_utf8(&buf[..len]) {
            return f(s);
        }
    }
    f(&format!("{comp_id}-{prefix}{idx}"))
}

/// C++ `CurrentWidget::setSliderValue`: `pow(slider/150, 4.5)`. Default
/// slider 150 yields speed 1.0.
pub fn curr_speed_from_slider(slider: i32) -> f64 {
    (f64::from(slider.clamp(1, 1000)) / 150.0).powf(4.5)
}

/// C++ `Connector::updateStep` wrap of `m_step` into `[0, 8)`.
pub(crate) fn wrap_chevron_step(mut step: f64) -> f64 {
    if step.abs() > 8.0 {
        step %= 8.0;
    }
    if step < 0.0 {
        step += 8.0;
    }
    step
}

#[derive(Clone, Debug)]
pub struct Canvas {
    pub(crate) viewport: Vp,
    pub(crate) scene: Scene,
    pub(crate) drag: Drag,
    pub(crate) banding: bool,
    pub(crate) band_rect: Rect,
    pub(crate) cursor: CursorKind,
    pub(crate) cursor_moved: bool,
    pub(crate) last_item: Point,
    pub(crate) last_scene: Point,
    pub(crate) show_grid: bool,
    pub(crate) show_scroll: bool,
    pub(crate) show_component_rect: bool,
    pub(crate) hovered_pin: Option<String>,
    pub(crate) sim_running: bool,
    pub(crate) pin_volts: std::collections::HashMap<String, f64>,
    pub(crate) wire_currents: std::collections::HashMap<String, f64>,
    pub(crate) pin_directions: std::collections::HashMap<String, PinDirection>,
    pub(crate) pin_pullups: std::collections::HashSet<String>,
    pub(crate) sim_error: Option<String>,
    pub(crate) sim_warning: Option<String>,
    pub(crate) sim_warning_ticks: i32,
    pub(crate) anim_tick: u32,
    pub(crate) sim_tick: u32,
    pub(crate) file_path: Option<String>,
    pub(crate) running: Option<crate::Circuit>,
    pub(crate) readings: std::collections::HashMap<String, crate::instruments::ReadingView>,
    pub(crate) scope_traces: crate::plot::PlotBuffer,
    pub(crate) la_traces: crate::plot::PlotBuffer,
    pub(crate) scope_time_div: f64,
    pub(crate) scope_time_pos: f64,
    pub(crate) scope_volt_div: [f64; 4],
    pub(crate) scope_volt_pos: [f64; 4],
    pub(crate) scope_tracks: i32,
    pub(crate) scope_hidden: [bool; 4],
    pub(crate) scope_trigger: i32,
    pub(crate) scope_trig_level: f64,
    pub(crate) scope_filter: f64,
    pub(crate) la_time_div: f64,
    pub(crate) la_time_pos: f64,
    pub(crate) la_trigger: i32,
    pub(crate) la_threshold_r: f64,
    pub(crate) la_threshold_f: f64,
    pub(crate) history: History,
    pub(crate) pending: Option<Snapshot>,
    pub(crate) clipboard: Option<(String, Point)>,
    pub(crate) prop_uid: Option<String>,
    pub(crate) prop_open: bool,
    pub(crate) open_prop_uids: Vec<String>,
    pub(crate) last_opened_prop_uid: Option<String>,
    pub(crate) saved_sim1: String,
    /// Override for tests; empty means `catalog::standard()` at lookup time.
    pub(crate) catalog: crate::catalog::Catalog,
    pub(crate) audio_sink: Option<std::sync::Arc<std::sync::Mutex<dyn crate::audio::AudioSink>>>,
    pub(crate) hit_label_info: Option<(String, bool)>,
    pub(crate) hit_pin_id: Option<String>,
    pub(crate) last_wheel_item: Option<(String, f64, String, f64)>,
    /// C++ `Linker::m_selecComp`: uid of the component currently linking others.
    pub(crate) linking_from: Option<String>,
    /// linker uid -> linked component uids.
    pub(crate) linked: std::collections::HashMap<String, Vec<String>>,
    pub(crate) probe_last_high: std::collections::HashMap<String, bool>,
    pub(crate) overload_states:
        std::collections::HashMap<String, crate::overload::ItemOverloadState>,
    pub(crate) overload_log: Vec<crate::overload::OverloadLogEntry>,
    pub(crate) overload_escalated: bool,
    pub(crate) pressed_push_id: Option<String>,
    pub(crate) dirty: dirty::DirtySet,
    /// Last quantized analog glow (LED / lamp / RGB) so intensity noise
    /// without a visible step does not dirty the item.
    pub(crate) analog_visual: std::collections::HashMap<String, i32>,
    /// C++ `CurrentWidget::speed()`: `(slider/150)^4.5`. Multiplies wire
    /// current each frame to advance chevron phase.
    pub(crate) curr_speed: f64,
    /// C++ `Connector::m_step` per wire, wrapped to `[0, 8)`.
    pub(crate) wire_chevron_step: std::collections::HashMap<String, f64>,
    pub(crate) active_device_id: Option<String>,
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Canvas {
    pub fn new() -> Self {
        Self::empty()
    }

    pub fn with_demo_divider() -> Self {
        let mut c = Self::empty();
        c.scene.add_demo_divider();
        c.saved_sim1 = c.scene.to_sim1();
        c
    }

    pub fn empty() -> Self {
        let scene = Scene::new();
        let saved_sim1 = scene.to_sim1();
        Self {
            viewport: Vp::default(),
            scene,
            drag: Drag::None,
            banding: false,
            band_rect: Rect::default(),
            cursor: CursorKind::Arrow,
            cursor_moved: false,
            last_item: Point::zero(),
            last_scene: Point::zero(),
            show_grid: true,
            show_scroll: false,
            show_component_rect: false,
            hovered_pin: None,
            sim_running: false,
            pin_volts: std::collections::HashMap::new(),
            wire_currents: std::collections::HashMap::new(),
            pin_directions: std::collections::HashMap::new(),
            pin_pullups: std::collections::HashSet::new(),
            sim_error: None,
            sim_warning: None,
            sim_warning_ticks: 0,
            anim_tick: 0,
            sim_tick: 0,
            file_path: None,
            running: None,
            readings: std::collections::HashMap::new(),
            scope_traces: crate::instruments::empty_scope_traces(),
            la_traces: crate::instruments::empty_la_traces(),
            scope_time_div: crate::SCOPE_TIME_DIV_DEFAULT,
            scope_time_pos: 0.0,
            scope_volt_div: [1.0; 4],
            scope_volt_pos: [0.0; 4],
            scope_tracks: 1,
            scope_hidden: [false; 4],
            scope_trigger: -1,
            scope_trig_level: 0.0,
            scope_filter: 0.0,
            la_time_div: crate::SCOPE_TIME_DIV_DEFAULT,
            la_time_pos: 0.0,
            la_trigger: 0,
            la_threshold_r: crate::LA_THRESHOLD_DEFAULT,
            la_threshold_f: crate::LA_THRESHOLD_DEFAULT,
            history: History::new(),
            pending: None,
            clipboard: None,
            prop_uid: None,
            prop_open: false,
            open_prop_uids: Vec::new(),
            last_opened_prop_uid: None,
            saved_sim1,
            catalog: crate::catalog::Catalog::new(),
            audio_sink: None,
            hit_label_info: None,
            hit_pin_id: None,
            last_wheel_item: None,
            linking_from: None,
            linked: std::collections::HashMap::new(),
            probe_last_high: std::collections::HashMap::new(),
            overload_states: std::collections::HashMap::new(),
            overload_log: Vec::new(),
            overload_escalated: false,
            pressed_push_id: None,
            dirty: dirty::DirtySet::default(),
            analog_visual: std::collections::HashMap::new(),
            curr_speed: 1.0,
            wire_chevron_step: std::collections::HashMap::new(),
            active_device_id: None,
        }
    }

    pub fn active_device_id(&self) -> Option<&str> {
        self.active_device_id.as_deref()
    }

    pub fn set_active_device_id(&mut self, id: Option<String>) {
        if self.active_device_id != id {
            self.active_device_id = id;
            self.dirty.mark_full();
        }
    }

    pub fn active_top_level_item_id(&self) -> Option<String> {
        let devices = self.collect_programmable_devices(self.active_device_id.as_deref());
        let active_dev = devices.into_iter().find(|d| d.is_active)?;
        if let Some((top_id, _)) = active_dev.id.split_once('/') {
            Some(top_id.to_string())
        } else {
            Some(active_dev.id)
        }
    }

    pub fn is_item_active(&self, item_id: &str) -> bool {
        self.active_top_level_item_id().as_deref() == Some(item_id)
    }

    pub fn take_dirty(&mut self) -> dirty::DirtySet {
        std::mem::take(&mut self.dirty)
    }

    pub fn peek_dirty(&self) -> &dirty::DirtySet {
        &self.dirty
    }

    pub fn mark_item_dirty(&mut self, id: &str) {
        self.dirty.mark_item(id);
    }

    pub fn mark_wire_dirty(&mut self, id: &str) {
        self.dirty.mark_wire(id);
    }

    pub fn mark_full_dirty(&mut self) {
        self.dirty.mark_full();
    }

    pub fn restore_dirty(&mut self, dirty: dirty::DirtySet) {
        self.dirty.merge(dirty);
    }

    pub fn take_last_wheel_item(&mut self) -> Option<(String, f64, String, f64)> {
        self.last_wheel_item.take()
    }

    pub fn set_audio_sink(
        &mut self,
        sink: std::sync::Arc<std::sync::Mutex<dyn crate::audio::AudioSink>>,
    ) {
        if let Ok(mut s) = sink.lock() {
            s.resume();
        }
        self.audio_sink = Some(sink.clone());
        if let Some(circuit) = self.running.as_mut() {
            circuit.set_audio_sink(sink);
        }
    }

    pub fn viewport(&self) -> &Vp {
        &self.viewport
    }

    pub fn viewport_mut(&mut self) -> &mut Vp {
        &mut self.viewport
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }

    pub fn circ_settings(&self) -> &crate::CircSettings {
        self.scene.settings()
    }

    pub fn circ_time(&self) -> u64 {
        self.running.as_ref().map(|c| c.circ_time).unwrap_or(0)
    }

    pub fn mcu_info(&self) -> (String, String, bool) {
        for it in self.scene.items() {
            if matches!(&it.kind, Part::Mcu(_) | Part::QemuDevice(_)) {
                let clean = crate::package::clean_chip_label(it.pkg_name());
                let device = if !clean.is_empty() {
                    clean.to_string()
                } else {
                    it.kind.type_name().to_string()
                };
                let name = it.label.clone();
                return (
                    device,
                    if name.is_empty() { it.id.clone() } else { name },
                    true,
                );
            }
        }
        (String::new(), String::new(), false)
    }

    pub fn set_circ_settings(&mut self, s: crate::CircSettings) -> Change {
        let cur = self.scene.settings();
        let visual_changed = s.width != cur.width
            || s.height != cur.height
            || s.animate_logic != cur.animate_logic
            || s.animate_curr != cur.animate_curr
            || s.ansi != cur.ansi;
        let size_changed = s.width != cur.width || s.height != cur.height;
        *self.scene.settings_mut() = s;
        let mut c = Change {
            settings: true,
            ..Change::default()
        };
        if visual_changed {
            self.dirty.mark_full();
        }
        if size_changed {
            let w = self.scene.settings().width as f64;
            let h = self.scene.settings().height as f64;
            if self.viewport.set_scene_size(w, h) {
                c.viewport = true;
            }
        }
        if let Some(circuit) = self.running.as_mut() {
            let new_dt = self.scene.settings().analog_dt();
            if (circuit.dt - new_dt).abs() > 1e-15 {
                circuit.dt = new_dt;
                let _ = circuit.re_solve();
            }
            circuit.max_nl_steps = self.scene.settings().nl_steps;
            circuit.ps_per_sec = self.scene.settings().ps_per_sec();
        }
        c
    }

    pub fn begin_circ_settings_edit(&mut self) {
        if self.pending.is_none() {
            self.pending = Some(self.snapshot());
        }
    }

    pub fn commit_circ_settings_edit(&mut self) -> Change {
        let mut c = Change::default();
        if self.commit_pending() {
            c.history = true;
            c.settings = true;
        }
        c
    }

    pub fn update_circ_settings(&mut self, f: impl FnOnce(&mut crate::CircSettings)) -> Change {
        let mut s = self.scene.settings().clone();
        f(&mut s);
        if *self.scene.settings() == s {
            return Change::default();
        }
        if self.pending.is_none() {
            self.push_undo();
        }
        let mut c = self.set_circ_settings(s);
        if self.pending.is_none() {
            c.history = true;
        }
        c
    }

    pub fn show_grid(&self) -> bool {
        self.show_grid
    }

    pub fn set_show_grid(&mut self, show: bool) -> bool {
        if self.show_grid == show {
            return false;
        }
        self.show_grid = show;
        true
    }

    pub fn show_scroll(&self) -> bool {
        self.show_scroll
    }

    pub fn set_show_scroll(&mut self, show: bool) -> bool {
        if self.show_scroll == show {
            return false;
        }
        self.show_scroll = show;
        true
    }

    pub fn show_component_rect(&self) -> bool {
        self.show_component_rect
    }

    pub fn set_show_component_rect(&mut self, show: bool) -> bool {
        if self.show_component_rect == show {
            return false;
        }
        self.show_component_rect = show;
        true
    }

    pub fn banding(&self) -> bool {
        self.banding
    }

    pub fn cursor(&self) -> CursorKind {
        self.cursor
    }

    pub fn last_scene(&self) -> Point {
        if self.cursor_moved {
            self.last_scene
        } else {
            self.viewport.center()
        }
    }

    pub fn cursor_moved(&self) -> bool {
        self.cursor_moved
    }

    fn sync_last_scene(&mut self) {
        if self.cursor_moved {
            self.last_scene = self.viewport.map_to_circuit(self.last_item);
        } else {
            self.last_scene = self.viewport.center();
        }
    }

    pub fn last_item(&self) -> Point {
        self.last_item
    }

    pub fn hovered_pin(&self) -> Option<&str> {
        self.hovered_pin.as_deref()
    }

    pub fn sim_running(&self) -> bool {
        self.sim_running
    }

    pub fn set_sim_running(&mut self, running: bool) {
        self.sim_running = running;
    }

    pub fn sim_error(&self) -> Option<&str> {
        self.sim_error.as_deref()
    }

    pub fn sim_warning(&self) -> Option<&str> {
        self.sim_warning.as_deref()
    }

    pub fn pin_voltage(&self, pin: &str) -> Option<f64> {
        self.pin_volts.get(pin).copied()
    }

    #[inline]
    pub fn item_pin_voltage(&self, item_id: &str, suffix: &str) -> Option<f64> {
        with_pin_id(item_id, suffix, |pin_id| {
            self.pin_volts.get(pin_id).copied()
        })
    }

    pub fn set_pin_voltage(&mut self, pin: impl Into<String>, v: f64) {
        self.pin_volts.insert(pin.into(), v);
    }

    pub fn clear_pin_voltages(&mut self) {
        self.pin_volts.clear();
    }

    pub fn pin_volts(&self) -> &std::collections::HashMap<String, f64> {
        &self.pin_volts
    }

    pub fn pin_directions(&self) -> &std::collections::HashMap<String, PinDirection> {
        &self.pin_directions
    }

    pub fn pin_pullups(&self) -> &std::collections::HashSet<String> {
        &self.pin_pullups
    }

    pub fn pin_direction(&self, pin_id: &str) -> Option<PinDirection> {
        self.pin_directions.get(pin_id).copied()
    }

    pub fn is_pin_pullup(&self, pin_id: &str) -> bool {
        self.pin_pullups.contains(pin_id)
    }

    pub fn running_circuit(&self) -> Option<&crate::Circuit> {
        self.running.as_ref()
    }

    pub fn running_circuit_mut(&mut self) -> Option<&mut crate::Circuit> {
        self.running.as_mut()
    }

    pub fn mcu_set_pin_mode(&mut self, pin_id: &str, mode: crate::digital::PinMode) -> bool {
        if let Some(c) = &mut self.running {
            c.mcu_set_pin_mode(pin_id, mode)
        } else {
            false
        }
    }

    pub fn mcu_set_pin_pullup(&mut self, pin_id: &str, pullup: bool) -> bool {
        if let Some(c) = &mut self.running {
            c.mcu_set_pin_pullup(pin_id, pullup)
        } else {
            false
        }
    }

    pub fn wire_current(&self, id: &str) -> f64 {
        self.wire_currents.get(id).copied().unwrap_or(0.0)
    }

    pub fn pin_current(&self, pin: &str) -> Option<f64> {
        self.running
            .as_ref()
            .and_then(|c| c.current_out_of_pin(pin))
    }

    #[inline]
    pub fn item_pin_current(&self, item_id: &str, suffix: &str) -> Option<f64> {
        with_pin_id(item_id, suffix, |pin_id| self.pin_current(pin_id))
    }

    pub fn anim_tick(&self) -> u32 {
        self.anim_tick
    }

    /// C++ `CurrentWidget::setSliderValue`: `speed = (slider/150)^4.5`.
    pub fn set_curr_speed_slider(&mut self, slider: i32) {
        self.curr_speed = curr_speed_from_slider(slider);
    }

    pub fn curr_speed(&self) -> f64 {
        self.curr_speed
    }

    /// C++ `Connector::m_step` for this wire, in `[0, 8)`.
    pub fn chevron_step(&self, id: &str) -> f64 {
        self.wire_chevron_step.get(id).copied().unwrap_or(0.0)
    }

    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    pub fn file_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }

    pub fn is_modified(&self) -> bool {
        self.scene.to_sim1() != self.saved_sim1
    }

    pub fn mark_saved(&mut self) {
        self.saved_sim1 = self.scene.to_sim1();
    }

    pub fn is_idle(&self) -> bool {
        matches!(self.drag, Drag::None) && !self.scene.drawing()
    }

    pub fn can_paste(&self) -> bool {
        self.clipboard.is_some()
    }

    pub fn has_selection(&self) -> bool {
        self.scene.any_selected()
    }

    pub fn has_item_selection(&self) -> bool {
        self.scene.items().iter().any(|it| it.selected)
    }

    pub fn has_wire_selection(&self) -> bool {
        self.scene.wires().iter().any(|w| w.selected)
    }

    pub fn prop_open(&self) -> bool {
        self.prop_open || !self.open_prop_uids.is_empty()
    }

    pub fn prop_uid(&self) -> Option<&str> {
        self.prop_uid
            .as_deref()
            .or_else(|| self.open_prop_uids.last().map(|s| s.as_str()))
    }

    pub fn open_prop_uids(&self) -> &[String] {
        &self.open_prop_uids
    }

    pub fn last_opened_prop_uid(&self) -> Option<&str> {
        self.last_opened_prop_uid
            .as_deref()
            .or_else(|| self.prop_uid())
    }

    pub fn prop_item(&self) -> Option<&Item> {
        self.prop_uid().and_then(|id| self.scene.item_by_id(id))
    }

    pub fn hit_label_uid(&self) -> Option<&str> {
        self.hit_label_info.as_ref().map(|(id, _)| id.as_str())
    }

    pub fn hit_label_is_val(&self) -> bool {
        self.hit_label_info
            .as_ref()
            .map(|(_, v)| *v)
            .unwrap_or(false)
    }

    pub fn has_label_hit(&self) -> bool {
        self.hit_label_info.is_some()
    }

    pub fn has_pin_hit(&self) -> bool {
        self.hit_pin_id.is_some()
    }

    pub fn hit_pin_id(&self) -> Option<&str> {
        self.hit_pin_id.as_deref()
    }

    pub fn is_hit_pin_inverted(&self) -> bool {
        self.hit_pin_id
            .as_deref()
            .map(|id| self.scene.is_pin_inverted(id))
            .unwrap_or(false)
    }

    pub fn is_pin_inverted(&self, pin_id: &str) -> bool {
        self.scene.is_pin_inverted(pin_id)
    }

    pub fn toggle_pin_inverted(&mut self, pin_id: &str) -> Change {
        let is_inv = self.scene.toggle_pin_inverted(pin_id);
        if let Some(c) = &mut self.running {
            c.set_pin_inverted(pin_id, is_inv);
        }
        Change {
            items: true,
            wires: true,
            ..Change::default()
        }
    }

    pub fn set_pin_inverted(&mut self, pin_id: &str, inverted: bool) -> Change {
        self.scene.set_pin_inverted(pin_id, inverted);
        if let Some(c) = &mut self.running {
            c.set_pin_inverted(pin_id, inverted);
        }
        Change {
            items: true,
            wires: true,
            ..Change::default()
        }
    }

    pub fn set_view_size(&mut self, w: f64, h: f64) -> Change {
        let before = self.viewport.center();
        if !self.viewport.set_view_size(w, h) {
            return Change::default();
        }
        self.sync_last_scene();
        let mut c = Change {
            viewport: true,
            ..Change::default()
        };
        if self.viewport.center() != before {
            c.center = true;
        }
        c.merge(self.update_hover(self.last_scene, 0));
        c
    }

    pub fn set_zoom(&mut self, z: f64) -> Change {
        let before = self.viewport.center();
        if !self.viewport.set_zoom(z) {
            return Change::default();
        }
        self.sync_last_scene();
        let mut c = Change {
            zoom: true,
            viewport: true,
            ..Change::default()
        };
        if self.viewport.center() != before {
            c.center = true;
        }
        c.merge(self.update_hover(self.last_scene, 0));
        c
    }

    pub fn set_center(&mut self, x: f64, y: f64) -> Change {
        if !self.viewport.set_center(Point::new(x, y)) {
            return Change::default();
        }
        self.sync_last_scene();
        let mut c = Change {
            center: true,
            viewport: true,
            ..Change::default()
        };
        if self.hovered_pin.take().is_some() {
            c.hovered_pin = true;
        }
        c
    }

    pub fn zoom_by(&mut self, factor: f64, anchor_item: Point) -> Change {
        if !self.viewport.zoom_by(factor, anchor_item) {
            return Change::default();
        }
        self.sync_last_scene();
        let mut c = Change::viewport_all();
        if self.hovered_pin.take().is_some() {
            c.hovered_pin = true;
        }
        c
    }

    pub fn zoom_in(&mut self) -> Change {
        if !self.viewport.zoom_in() {
            return Change::default();
        }
        self.sync_last_scene();
        let mut c = Change::viewport_all();
        c.merge(self.update_hover(self.last_scene, 0));
        c
    }

    pub fn zoom_out(&mut self) -> Change {
        if !self.viewport.zoom_out() {
            return Change::default();
        }
        self.sync_last_scene();
        let mut c = Change::viewport_all();
        c.merge(self.update_hover(self.last_scene, 0));
        c
    }

    pub fn zoom_one(&mut self) -> Change {
        let before = self.viewport.center();
        if !self.viewport.zoom_one() {
            return Change::default();
        }
        self.sync_last_scene();
        let mut c = Change {
            zoom: true,
            viewport: true,
            ..Change::default()
        };
        if self.viewport.center() != before {
            c.center = true;
        }
        c.merge(self.update_hover(self.last_scene, 0));
        c
    }

    pub fn zoom_to_fit(&mut self) -> Change {
        let mut r = self.scene.items_bounding_rect();
        if r.is_null() {
            return Change::default();
        }
        r = r.adjust(-20.0, -20.0, 20.0, 20.0);
        if !self.viewport.zoom_to_rect(r) {
            return Change::default();
        }
        self.sync_last_scene();
        let mut c = Change::viewport_all();
        c.merge(self.update_hover(self.last_scene, 0));
        c
    }

    pub fn zoom_selected(&mut self) -> Change {
        let mut r = self.scene.selected_rect();
        if r.is_null() {
            return Change::default();
        }
        r = r.adjust(-20.0, -20.0, 20.0, 20.0);
        if !self.viewport.zoom_to_rect(r) {
            return Change::default();
        }
        self.sync_last_scene();
        let mut c = Change::viewport_all();
        c.merge(self.update_hover(self.last_scene, 0));
        c
    }

    pub fn focus_component(&mut self, id: &str) -> Change {
        let Some(idx) = self.scene.items().iter().position(|it| it.id == id) else {
            return Change::default();
        };
        let rect = self.scene.items()[idx].full_rect();
        let center = rect.center();
        let items_changed = self.select_only(idx);
        let center_changed = self.viewport.set_center(center);
        self.sync_last_scene();
        let mut c = Change {
            items: items_changed,
            center: center_changed,
            viewport: center_changed,
            ..Change::default()
        };
        c.merge(self.update_hover(self.last_scene, 0));
        c
    }

    pub fn canvas_overflow_info(&self) -> CanvasOverflowInfo {
        self.scene.canvas_overflow_info()
    }

    pub fn auto_fit_canvas(&mut self, margin: f64) -> Change {
        let ibr = self.scene.items_bounding_rect();
        if ibr.is_null() || (self.scene.items().is_empty() && self.scene.wires().is_empty()) {
            return Change::default();
        }
        let max_abs_x = ibr.left().abs().max(ibr.right().abs());
        let max_abs_y = ibr.top().abs().max(ibr.bottom().abs());
        let m = margin.max(40.0);
        let req_w_raw = ((max_abs_x + m) * 2.0).ceil() as i32;
        let req_h_raw = ((max_abs_y + m) * 2.0).ceil() as i32;
        let cur_w = self.scene.settings().width;
        let cur_h = self.scene.settings().height;
        let new_w = (((req_w_raw + 99) / 100) * 100)
            .max(cur_w)
            .clamp(100, 10_000);
        let new_h = (((req_h_raw + 99) / 100) * 100)
            .max(cur_h)
            .clamp(100, 10_000);

        if new_w == cur_w && new_h == cur_h {
            return Change::default();
        }

        self.push_undo();
        let mut s = self.scene.settings().clone();
        s.width = new_w;
        s.height = new_h;
        let mut c = self.set_circ_settings(s);
        c.history = true;
        c
    }

    pub fn center_circuit(&mut self) -> Change {
        let ibr = self.scene.items_bounding_rect();
        if ibr.is_null() || (self.scene.items().is_empty() && self.scene.wires().is_empty()) {
            return Change::default();
        }
        let center = ibr.center();
        let dx = (-center.x / 8.0).round() * 8.0;
        let dy = (-center.y / 8.0).round() * 8.0;
        if dx == 0.0 && dy == 0.0 {
            return Change::default();
        }
        self.push_undo();
        let (actual_dx, actual_dy) = self.scene.center_all();
        if actual_dx == 0.0 && actual_dy == 0.0 {
            self.history.undo.pop();
            return Change::default();
        }
        let mut c = Change {
            items: true,
            wires: true,
            history: true,
            ..Change::default()
        };
        self.sync_last_scene();
        c.merge(self.update_hover(self.last_scene, 0));
        c
    }

    pub fn select_overflowing_items(&mut self) -> Change {
        let changed = self.scene.select_overflowing();
        if !changed {
            return Change::default();
        }
        self.sync_last_scene();
        let mut c = Change {
            items: true,
            wires: true,
            ..Change::default()
        };
        c.merge(self.update_hover(self.last_scene, 0));
        c
    }
}

#[cfg(test)]
mod tests;
