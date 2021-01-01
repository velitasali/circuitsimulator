//! Central ColorTheme bindings for CircuitCanvas palette properties.

use cs_engine::theme::{ColorId, ColorTheme};
use serde_json::{Value, json};

/// Update all color hex string fields according to theme brightness.
pub(super) fn update_palette_colors(canvas: &mut super::CircuitCanvas) {
    let dark = canvas.dark;
    canvas.viewport_color = ColorTheme::get_hex(ColorId::ViewportBackground, dark);
    canvas.canvas_color = ColorTheme::get_hex(ColorId::CanvasBackground, dark);
    canvas.grid_color = ColorTheme::get_hex(ColorId::CanvasGridDots, dark);
    canvas.band_color = ColorTheme::get_hex(ColorId::ItemHovered, dark);
    canvas.body_color = ColorTheme::get_hex(ColorId::ComponentBody, dark);
    canvas.border_color = ColorTheme::get_hex(ColorId::ComponentBorder, dark);
    canvas.wire_color = ColorTheme::get_hex(ColorId::WireOpen, dark);
    canvas.pin_high_color = ColorTheme::get_hex(ColorId::PinHigh, dark);
    canvas.pin_low_color = ColorTheme::get_hex(ColorId::PinLow, dark);
    canvas.meter_display_color = ColorTheme::get_hex(ColorId::MeterDisplayText, dark);
    canvas.pin_open_high_color = ColorTheme::get_hex(ColorId::PinOpenHigh, dark);
    canvas.pin_open_low_color = ColorTheme::get_hex(ColorId::PinOpenLow, dark);
    canvas.pin_input_high_color = ColorTheme::get_hex(ColorId::PinInputHigh, dark);
    canvas.pin_input_low_color = ColorTheme::get_hex(ColorId::PinInputLow, dark);
    canvas.pin_out_high_color = ColorTheme::get_hex(ColorId::PinOutHigh, dark);
    canvas.pin_out_low_color = ColorTheme::get_hex(ColorId::PinOutLow, dark);
    canvas.shape_body_color = ColorTheme::get_hex(ColorId::ShapeBody, dark);
    canvas.prop_label_color = ColorTheme::get_hex(ColorId::PropLabelText, dark);
    canvas.prop_header_color = ColorTheme::get_hex(ColorId::PropHeaderText, dark);
    canvas.mcu_label_bg = ColorTheme::get_hex(ColorId::McuLabelBg, dark);
    canvas.mcu_label_text = ColorTheme::get_hex(ColorId::McuLabelText, dark);
    canvas.mcu_label_border = ColorTheme::get_hex(ColorId::McuLabelBorder, dark);
    canvas.chip_body_color = ColorTheme::get_hex(ColorId::ChipBody, dark);
    canvas.chip_label_color = ColorTheme::get_hex(ColorId::ChipLabelText, dark);
    canvas.coords_label_text = ColorTheme::get_hex(ColorId::CoordsLabelText, dark);
    canvas.tunnel_color1 = ColorTheme::get_hex(ColorId::TunnelColor1, dark);
    canvas.tunnel_color2 = ColorTheme::get_hex(ColorId::TunnelColor2, dark);
    canvas.tunnel_color3 = ColorTheme::get_hex(ColorId::TunnelColor3, dark);
    canvas.scope_channel1_color = ColorTheme::get_hex(ColorId::ScopeChannel1, dark);
    canvas.scope_channel2_color = ColorTheme::get_hex(ColorId::ScopeChannel2, dark);
    canvas.scope_channel3_color = ColorTheme::get_hex(ColorId::ScopeChannel3, dark);
    canvas.scope_channel4_color = ColorTheme::get_hex(ColorId::ScopeChannel4, dark);
    canvas.la_channel1_color = ColorTheme::get_hex(ColorId::LaChannel1, dark);
    canvas.la_channel2_color = ColorTheme::get_hex(ColorId::LaChannel2, dark);
    canvas.la_channel3_color = ColorTheme::get_hex(ColorId::LaChannel3, dark);
    canvas.la_channel4_color = ColorTheme::get_hex(ColorId::LaChannel4, dark);
    canvas.la_channel5_color = ColorTheme::get_hex(ColorId::LaChannel5, dark);
    canvas.la_channel6_color = ColorTheme::get_hex(ColorId::LaChannel6, dark);
    canvas.la_channel7_color = ColorTheme::get_hex(ColorId::LaChannel7, dark);
    canvas.la_channel8_color = ColorTheme::get_hex(ColorId::LaChannel8, dark);
    canvas.msg_ok_bg = ColorTheme::get_hex(ColorId::MsgOkBg, dark);
    canvas.msg_ok_text = ColorTheme::get_hex(ColorId::MsgOkText, dark);
    canvas.msg_warn_bg = ColorTheme::get_hex(ColorId::MsgWarnBg, dark);
    canvas.msg_warn_text = ColorTheme::get_hex(ColorId::MsgWarnText, dark);
    canvas.msg_error_bg = ColorTheme::get_hex(ColorId::MsgErrorBg, dark);
    canvas.msg_error_text = ColorTheme::get_hex(ColorId::MsgErrorText, dark);
    canvas.sim_pause_tint = ColorTheme::get_hex(ColorId::SimPauseTint, dark);
    canvas.toggle_button_bg = ColorTheme::get_hex(ColorId::ToggleButtonBg, dark);
    canvas.mem_data_bg = ColorTheme::get_hex(ColorId::MemDataBg, dark);
    canvas.mem_data_text = ColorTheme::get_hex(ColorId::MemDataText, dark);
    canvas.mem_type_text = ColorTheme::get_hex(ColorId::MemTypeText, dark);
    canvas.mem_val_text = ColorTheme::get_hex(ColorId::MemValText, dark);
    canvas.mcu_pc_val1_text = ColorTheme::get_hex(ColorId::McuPcVal1Text, dark);
    canvas.mcu_pc_val2_text = ColorTheme::get_hex(ColorId::McuPcVal2Text, dark);
}

/// Helper for `ledColorPair` QSlot.
pub(super) fn led_color_pair(color_name: &str) -> Value {
    let (lit, unlit) = ColorTheme::led_color_hex(color_name);
    json!({ "lit": lit, "unlit": unlit })
}
