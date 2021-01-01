//! Property inspection, reflection, and item/wire JSON serialization.

use cs_engine::canvas::{Canvas, Part, Point, Rect};
use serde_json::{Value, json};

pub(super) fn build_prop_groups_json(it: &cs_engine::canvas::Item) -> Value {
    let groups: Vec<Value> = it
        .prop_groups()
        .into_iter()
        .map(|group| {
            let rows: Vec<Value> = group
                .rows
                .into_iter()
                .map(|row| {
                    let is_showing = it.show_val && it.show_prop == row.name;
                    let can_show = !row.name.is_empty() && row.kind != "path" && row.kind != "bool";
                    let is_num = row.kind == "double"
                        || row.kind == "int"
                        || row.kind == "uint"
                        || row.kind == "num"
                        || row.kind == "number";
                    let raw_text = if row.kind == "bool" {
                        String::new()
                    } else {
                        it.prop_text(row.name).unwrap_or_default()
                    };
                    let base_unit = cs_engine::units::base_unit_for(row.unit);
                    let display_val = if is_num && !base_unit.is_empty() {
                        raw_text
                            .strip_suffix(base_unit)
                            .map(|s| s.trim().to_string())
                            .unwrap_or_else(|| raw_text.clone())
                    } else {
                        raw_text.clone()
                    };
                    let translated_caption = cs_engine::i18n::tr(row.caption);
                    let translated_info = cs_engine::i18n::tr(row.info);
                    let translated_options: Vec<String> = row
                        .options
                        .iter()
                        .map(|opt| cs_engine::i18n::tr(opt))
                        .collect();
                    let mut v = json!({
                        "name": row.name,
                        "kind": row.kind,
                        "caption": translated_caption,
                        "info": translated_info,
                        "unit": base_unit,
                        "options": translated_options,
                        "rawOptions": row.options,
                        "enabled": row.enabled,
                        "rowVisible": row.visible,
                        "showOnCanvas": is_showing,
                        "canShowOnCanvas": can_show,
                    });
                    if row.kind == "bool" {
                        v["boolValue"] = json!(it.prop_bool(row.name).unwrap_or(false));
                    } else {
                        v["text"] = json!(display_val);
                        v["fullText"] = json!(raw_text);
                    }
                    v
                })
                .collect();
            json!({
                "name": cs_engine::i18n::tr(group.name),
                "rows": rows,
            })
        })
        .collect();
    json!(groups)
}

pub(super) fn open_prop_dialogs_json(canvas: &Canvas) -> Value {
    let items = canvas.scene().items();
    let list: Vec<Value> = canvas
        .open_prop_uids()
        .iter()
        .filter_map(|uid| {
            items.iter().find(|it| &it.id == uid).map(|it| {
                let human_name = cs_engine::i18n::tr(it.human_name());
                let title = if it.id.starts_with(it.human_name()) {
                    let suffix = &it.id[it.human_name().len()..];
                    format!("{}{}", human_name, suffix)
                } else {
                    format!("{} ({})", human_name, it.id)
                };
                json!({
                    "uid": it.id,
                    "title": title,
                    "typeText": human_name,
                    "label": it.label,
                })
            })
        })
        .collect();
    Value::Array(list)
}

pub(super) fn items_json(canvas: &Canvas) -> Value {
    let volts = canvas.pin_volts();
    let scene = canvas.scene();
    let readings = canvas.readings();
    let scene_items = scene.items();
    let mut arr: Vec<Value> = Vec::with_capacity(scene_items.len());
    for it in scene_items {
        let item_pins = it.pins();
        let mut pins: Vec<Value> = Vec::with_capacity(item_pins.len());
        let pkg = it.kind.package();
        for p in item_pins {
            let pin_id = if let Some(rest) = p.id.strip_prefix(&it.id) {
                rest.strip_prefix('-').unwrap_or(rest)
            } else {
                &p.id
            };
            let pkg_pin = pkg.and_then(|pkg| pkg.find_pin(pin_id));
            let (is_bus, inverted, unused, space, pin_type) = if let Some(pp) = pkg_pin {
                (
                    pp.is_bus(),
                    pp.inverted() ^ scene.inverted_pins.contains(&p.id),
                    pp.unused(),
                    pp.space,
                    pp.pin_type.clone(),
                )
            } else {
                (
                    p.is_bus,
                    scene.is_pin_inverted(&p.id),
                    p.unused,
                    0,
                    String::new(),
                )
            };
            let is_rst = pin_type.eq_ignore_ascii_case("rst");
            let is_un = unused
                || pin_type.eq_ignore_ascii_case("nc")
                || pin_type.eq_ignore_ascii_case("unused");
            let direction = if is_rst || is_un {
                ""
            } else if let Some(dir) = canvas.pin_direction(&p.id) {
                dir.as_str()
            } else {
                it.default_pin_direction(pin_id)
                    .map(|d| d.as_str())
                    .unwrap_or("")
            };
            let pullup = if canvas.sim_running() {
                canvas.is_pin_pullup(&p.id)
            } else {
                it.default_pin_pullup(pin_id) || scene.is_pin_pullup(&p.id)
            };
            let mut pin = json!({
                "id": p.id,
                "pinId": pin_id,
                "x": p.local.x,
                "y": p.local.y,
                "angle": p.angle,
                "length": p.length,
                "connected": scene.pin_connected(&p.id),
                "label": p.label,
                "isBus": is_bus,
                "inverted": inverted,
                "pullup": pullup,
                "unused": unused,
                "space": space,
                "pinType": pin_type,
                "direction": direction,
            });
            if let Some(v) = volts.get(&p.id) {
                pin["volt"] = json!(if *v > 2.5 { 5.0 } else { 0.0 });
            }
            pins.push(pin);
        }
        let pins_str = serde_json::to_string(&pins).unwrap_or_default();
        let rd = readings.get(&it.id);
        let ov = canvas.item_overload_state(&it.id);
        let (is_warning, is_crashed, warning_text) = match ov {
            Some(state) => (state.warning, state.crashed, state.reason.clone()),
            None => (false, false, String::new()),
        };
        let rect = it.body_rect();
        let (lx, ly) = it.label_pos();
        let (vx, vy) = it.val_pos();
        let local_expanded = rect.adjust(-24.0, -24.0, 24.0, 24.0);
        let mut bounds = cs_engine::canvas::map_rect(
            Point::new(it.x, it.y),
            local_expanded,
            it.rotation,
            it.hflip,
            it.vflip,
        );
        if it.show_id {
            bounds = bounds.united(Rect::new(it.x + lx - 20.0, it.y + ly - 10.0, 40.0, 20.0));
        }
        if it.show_val {
            bounds = bounds.united(Rect::new(it.x + vx - 20.0, it.y + vy - 10.0, 40.0, 20.0));
        }
        arr.push(json!({
            "uid": it.id,
            "sceneX": it.x,
            "sceneY": it.y,
            "warning": is_warning,
            "crashed": is_crashed,
            "warningText": warning_text,
            "boundsX": bounds.x,
            "boundsY": bounds.y,
            "boundsW": bounds.w,
            "boundsH": bounds.h,
            "selected": it.selected,
            "rotationDeg": it.rotation,
            "hflip": it.hflip,
            "vflip": it.vflip,
            "label": it.label,
            "showId": it.show_id,
            "labelX": lx,
            "labelY": ly,
            "labelRot": it.label_rot,
            "customLabelPos": it.custom_label_pos,
            "showVal": it.show_val,
            "showProp": it.show_prop,
            "valText": it.val_label_text(),
            "valX": vx,
            "valY": vy,
            "valRot": it.val_rot,
            "customValPos": it.custom_val_pos,
            "bodyX": rect.x,
            "bodyY": rect.y,
            "bodyW": rect.w,
            "bodyH": rect.h,
            "resistance": it.resistance(),
            "voltage": it.voltage(),
            "capacitance": it.capacitance(),
            "inductance": it.inductance(),
            "closed": it.closed(),
            "pnp": it.pnp(),
            "pChannel": it.p_channel(),
            "depletion": it.depletion(),
            "kind": it.kind.type_name(),
            "pkgW": it.pkg_w(),
            "pkgH": it.pkg_h(),
            "logicSymbol": it.logic_symbol(),
            "customColor": it.custom_color(),
            "bckgndColor": it.bckgnd_color(),
            "pkgName": it.pkg_name(),
            "subcType": it.subc_type(),
            "pins": pins,
            "pinsJson": pins_str,
            "reading": rd.map(|r| r.text.clone()).unwrap_or_default(),
            "readingMax": rd.map(|r| r.max_text.clone()).unwrap_or_default(),
            "readingAvg": rd.map(|r| r.avg_text.clone()).unwrap_or_default(),
            "readingExtra": rd.map(|r| r.extra.clone()).unwrap_or_default(),
            "readingHz": rd.map(|r| r.hz_text.clone()).unwrap_or_default(),
            "probeHigh": rd.map(|r| r.high).unwrap_or(false),
            "probeLow": rd.map(|r| r.low).unwrap_or(false),
            // Specific properties for delegates:
            "gateKind": it.gate_kind(),
            "numInputs": it.num_inputs(),
            "invertInputs": it.invert_inputs(),
            "invertOutput": it.invert_output(),
            "initHigh": it.init_high(),
            "tristate": it.tristate(),
            "small": it.small(),
            "ffKind": it.ff_kind(),
            "useRs": it.use_rs(),
            "trigger": it.trigger(),
            "channels": it.channels(),
            "useReset": it.use_reset(),
            "testInputs": it.test_inputs(),
            "testOutputs": it.test_outputs(),
            "period": it.period(),
            "freqKhz": it.freq_khz(),
            "waveType": it.wave_type(),
            "freqHz": it.freq_hz(),
            "amplitude": it.amplitude(),
            "offset": it.offset(),
            "duty": it.duty(),
            "sourceValue": it.source_value(),
            "minValue": it.min_value(),
            "maxValue": it.max_value(),
            "sourceRunning": it.source_running(),
            "controlPins": it.control_pins(),
            "currSource": it.is_curr_source(),
            "currControl": it.curr_control(),
            "gain": it.gain(),
            "normClose": it.norm_close(),
            "poles": it.poles(),
            "key": it.key(),
            "showButton": it.show_button(),
            "pressed": it.pressed(),
            "size": it.size(),
            "state": it.state(),
            "exclusive": it.exclusive(),
            "commonPin": it.common_pin(),
            "doubleThrow": it.double_throw(),
            "active": it.active(),
            "rows": it.rows(),
            "cols": it.cols(),
            "wiper": it.wiper(),
            "minR": it.min_r(),
            "maxR": it.max_r(),
            "bussed": it.bussed(),
            "lux": it.lux(),
            "rDark": it.r_dark(),
            "rLight": it.r_light(),
            "tempC": it.temp_c(),
            "r0": it.r0(),
            "beta": it.beta(),
            "t0C": it.t0_c(),
            "alpha": it.alpha(),
            "strain": it.strain_val(),
            "gaugeFactor": it.gauge_factor(),
            "minC": it.min_c(),
            "maxC": it.max_c(),
            "minL": it.min_l(),
            "maxL": it.max_l(),
            "inductance1": it.inductance1(),
            "inductance2": it.inductance2(),
            "coupling": it.coupling(),
            "vGateTh": it.v_gate_th(),
            "iHold": it.i_hold(),
            "vBreakover": it.v_breakover(),
            "muxOnResistance": it.on_resistance(),
            "commonAnode": it.common_anode(),
            "color": it.color_str(),
            "ledColor": it.color_str(),
            "ledLitColor": cs_engine::theme::ColorTheme::led_color_hex(it.color_str()).0,
            "ledUnlitColor": cs_engine::theme::ColorTheme::led_color_hex(it.color_str()).1,
            "dispWidth": it.disp_width(),
            "dispHeight": it.disp_height(),
            "dispRotate": it.disp_rotate(),
            "controlCode": it.control_code(),
            "controller": it.controller(),
            "contrast": it.contrast(),
            "bias": it.bias(),
            "grounded": it.grounded(),
            "transparent": it.transparent(),
            "rxMin": it.rx_min(),
            "rxMax": it.rx_max(),
            "ryMin": it.ry_min(),
            "ryMax": it.ry_max(),
            "xPos": it.x_pos(),
            "yPos": it.y_pos(),
            "padWidth": it.width_px(),
            "padHeight": it.height_px(),
            "stickX": it.stick_x(),
            "stickY": it.stick_y(),
            "btnDown": it.btn_down(),
            "dialVal": it.dial_val(),
            "btnClosed": it.btn_closed(),
            "distance": it.distance(),
            "useSlider": it.use_slider(),
            "model": it.model(),
            "temp": it.temp(),
            "humi": it.humi(),
            "tempInc": it.temp_inc(),
            "humiInc": it.humi_inc(),
            "rom": it.rom(),
            "timeUpdated": it.time_updated(),
            "rpmNominal": it.rpm_nominal(),
            "voltNominal": it.volt_nominal(),
            "speed": it.speed(),
            "angle": it.angle(),
            "bipolar": it.bipolar(),
            "minPulse": it.min_pulse(),
            "maxPulse": it.max_pulse(),
            "pos": it.pos(),
            "file": it.file(),
            "debug": it.debug(),
            "threshold": it.threshold(),
            "maxCurrent": it.max_current(),
            "buzzer": it.buzzer(),
            "volume": it.volume(),
            "frequency": it.frequency(),
            "impedance": it.impedance(),
            "ledIntensity": if matches!(&it.kind, Part::Led(_)) && canvas.sim_running() {
                let current = canvas.item_pin_current(&it.id, cs_engine::PIN_LEFT).unwrap_or(0.0).max(0.0);
                let max_i = it.max_current().max(0.001);
                let ratio = (current / max_i).max(0.0);
                if ratio > 0.0 { ratio.sqrt().min(1.5) } else { 0.0 }
            } else {
                0.0
            },
            "ledWarning": is_warning,
            "ledCrashed": is_crashed,
            "modules": it.modules(),
            "displays": it.modules(),
            "count": it.count(),
            "power": it.power(),
            "rCold": it.r_cold(),
            "tunnelName": it.tunnel_name(),
            "isBus": it.is_bus(),
            "busWidth": it.bus_width(),
            "pinsCount": it.pins_count(),
            "portName": it.port_name(),
            "baudRate": it.baud_rate(),
            "step": it.dial_step(),
            "shapeKind": it.shape_kind(),
            "shapeWidth": it.shape_width(),
            "shapeHeight": it.shape_height(),
            "shapeText": it.shape_text(),
            "chFreqs": if matches!(&it.kind, Part::Oscope(_) | Part::LogicAnalyzer(_)) {
                let text = rd.map(|r| r.text.as_str()).unwrap_or("");
                let parts: Vec<&str> = text.split(';').collect();
                json!(parts)
            } else {
                json!([])
            },
            "chTunnels": json!(it.ch_tunnels()),
            "itemVisible": it.is_visible(),
            "tunnelShow": it.tunnel_visible(),
            "probePause": it.probe_pause(),
        }));
    }
    Value::Array(arr)
}

pub(super) fn wires_json(canvas: &Canvas) -> Value {
    let volts = canvas.pin_volts();
    let scene_wires = canvas.scene().wires();
    let mut arr: Vec<Value> = Vec::with_capacity(scene_wires.len());
    for w in scene_wires {
        let mut points: Vec<Value> = Vec::with_capacity(w.points.len());
        let mut points_str = String::with_capacity(w.points.len() * 24 + 2);
        points_str.push('[');
        for (idx, p) in w.points.iter().enumerate() {
            if idx > 0 {
                points_str.push(',');
            }
            points.push(json!({ "x": p.x, "y": p.y }));
            use std::fmt::Write;
            let _ = write!(points_str, "{{\"x\":{},\"y\":{}}}", p.x, p.y);
        }
        points_str.push(']');
        let wb = w.bounds().adjust(-12.0, -12.0, 12.0, 12.0);
        arr.push(json!({
            "uid": w.id,
            "points": points,
            "pointsJson": points_str,
            "boundsX": wb.x,
            "boundsY": wb.y,
            "boundsW": wb.w,
            "boundsH": wb.h,
            "selected": w.selected,
            "drawing": w.drawing(),
            "isBus": w.is_bus,
            "voltage": volts.get(&w.start_pin).copied().unwrap_or(0.0),
            "current": canvas.wire_current(&w.id),
        }));
    }
    Value::Array(arr)
}

pub(super) fn selected_item_active(canvas: &Canvas, active_device_id: Option<&str>) -> bool {
    let uid = super::selection::selected_item_uid(canvas);
    if uid.is_empty() {
        return false;
    }
    let nested = canvas.selected_nested_mcu_uid().unwrap_or_default();
    let aid = match active_device_id {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => canvas
            .collect_programmable_devices(None)
            .first()
            .map(|d| d.uid.clone())
            .unwrap_or_default(),
    };
    if aid.is_empty() {
        return false;
    }
    aid == uid || (!nested.is_empty() && aid == nested)
}

pub(super) fn ensure_active_device(
    canvas: &Canvas,
    current_active_id: Option<&str>,
) -> Option<Option<String>> {
    let devices = canvas.collect_programmable_devices(current_active_id);
    if devices.is_empty() {
        if current_active_id.is_some() {
            return Some(None);
        }
        return None;
    }
    if let Some(id) = current_active_id {
        if devices.iter().any(|d| d.uid == id || d.id == id) {
            return None;
        }
    }
    let uid = devices[0].uid.clone();
    Some(Some(uid))
}

pub(super) fn package_pin_data_json(canvas: &Canvas, uid: &str, pin_id: &str) -> Value {
    if let Some(pin) = canvas.get_package_pin(uid, pin_id) {
        json!({
            "id": pin.id,
            "label": pin.label,
            "type": pin.pin_type,
            "xpos": pin.xpos,
            "ypos": pin.ypos,
            "angle": pin.angle,
            "length": pin.length,
            "space": pin.space,
            "inverted": pin.inverted(),
            "unused": pin.unused(),
            "isBus": pin.is_bus(),
            "isPoint": pin.is_point(),
        })
    } else {
        json!({})
    }
}
