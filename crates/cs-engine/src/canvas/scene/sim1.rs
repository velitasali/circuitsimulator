//! SIM1 file format import/export, clipboard serialization, circuit lowering, and ID allocation.

use super::Scene;
use super::item::Item;
use super::wires::pin_belongs;
use crate::Circuit;
use crate::canvas::geom::Point;
use crate::canvas::wire::Wire;
use crate::circ1;
use crate::components::*;

fn remap_pin(pin: &str, map: &std::collections::HashMap<String, String>) -> String {
    for (old, new) in map {
        if pin_belongs(pin, std::slice::from_ref(old)) {
            return format!("{}{}", new, &pin[old.len()..]);
        }
    }
    pin.to_string()
}

fn hidden_mcu_sim1(id: &str, m: &crate::mcu::McuComp) -> String {
    let mut attrs = String::new();
    if m.device.freq > 0.0 {
        attrs.push_str(&format!(
            " Frequency=\"{}\"",
            crate::units::format_si(m.device.freq, "Hz")
        ));
    }
    attrs.push_str(&format!(
        " ForceFreq=\"{}\"",
        if m.force_freq { "true" } else { "false" }
    ));
    if let Some(p) = &m.firmware {
        attrs.push_str(&format!(" Program=\"{p}\""));
    }
    attrs.push_str(&format!(
        " Auto_Load=\"{}\"",
        if m.auto_load { "true" } else { "false" }
    ));
    attrs.push_str(&format!(
        " savePGM=\"{}\"",
        if m.save_pgm { "true" } else { "false" }
    ));
    if m.save_pgm {
        let pgm: String = m
            .device
            .flash_words()
            .iter()
            .map(|w| format!("{w},"))
            .collect();
        attrs.push_str(&format!(" pgm=\"{pgm}\""));
    }
    format!("<item itemtype=\"MCU\" CircId=\"{id}\"{attrs} />\n")
}

impl Scene {
    pub fn selection_to_sim1(&self) -> Option<String> {
        if !self.any_selected() {
            return None;
        }
        let mut s = Scene::new();
        s.settings = self.settings.clone();
        for it in &self.items {
            if it.selected {
                s.items.push(it.clone());
            }
        }
        for w in &self.wires {
            if w.closed() && w.selected {
                s.wires.push(w.clone());
            }
        }
        if s.items.is_empty() && s.wires.is_empty() {
            return None;
        }
        Some(s.to_sim1())
    }

    pub fn paste_sim1(&mut self, src: &str, delta: Point) -> crate::Result<bool> {
        self.paste_sim1_with(src, delta, &crate::subcircuit::SubcSearch::default())
    }

    pub fn paste_sim1_with(
        &mut self,
        src: &str,
        delta: Point,
        search: &crate::subcircuit::SubcSearch,
    ) -> crate::Result<bool> {
        let incoming = Scene::from_sim1_with(src, search)?;
        if incoming.items.is_empty() && incoming.wires.is_empty() {
            return Ok(false);
        }
        self.clear_selection();
        let mut id_map = std::collections::HashMap::new();
        for mut it in incoming.items {
            let old = it.id.clone();
            let new_id = match &mut it.kind {
                Part::Subcircuit(subc) => {
                    let id = format!("{}-{}", subc.device, self.next_subc);
                    self.next_subc += 1;
                    id
                }
                Part::Mcu(mcu) => {
                    let device = crate::subcircuit::device_from_id(&old);
                    let id = format!("{device}-{}", self.next_mcu);
                    self.next_mcu += 1;
                    mcu.mcu.rebind_id(&id);
                    id
                }
                k => self.alloc_id(k),
            };
            id_map.insert(old, new_id.clone());
            it.id = new_id;
            it.x += delta.x;
            it.y += delta.y;
            it.selected = true;
            if it.label.is_empty() {
                it.label = it.id.clone();
            }
            self.add_saved_item(it);
        }
        for mut w in incoming.wires {
            if !w.closed() {
                continue;
            }
            let start = remap_pin(&w.start_pin, &id_map);
            let Some(end_old) = w.end_pin.clone() else {
                continue;
            };
            let end = remap_pin(&end_old, &id_map);
            if start == w.start_pin || end == end_old {
                continue;
            }
            w.id = format!("Connector-{}", self.next_wire);
            self.next_wire += 1;
            w.start_pin = start;
            w.end_pin = Some(end);
            w.translate(delta.x, delta.y);
            w.selected = true;
            self.wires.push(w);
        }
        Ok(true)
    }

    fn alloc_id(&mut self, part: &Part) -> String {
        let (next, prefix) = match part {
            Part::Resistor(_) => (&mut self.next_resistor, "Resistor"),
            Part::Battery(_) => (&mut self.next_battery, "Battery"),
            Part::Ground(_) => (&mut self.next_ground, "Ground"),
            Part::FixedVolt(_) => (&mut self.next_fixed, "FixedVolt"),
            Part::Node(_) => (&mut self.next_node, "Node"),
            Part::Capacitor(_) => (&mut self.next_capacitor, "Capacitor"),
            Part::ElCapacitor(_) => (&mut self.next_el_capacitor, "ElCapacitor"),
            Part::Inductor(_) => (&mut self.next_inductor, "Inductor"),
            Part::Switch(_) => (&mut self.next_switch, "Switch"),
            Part::Diode(_) => (&mut self.next_diode, "Diode"),
            Part::Led(_) => (&mut self.next_led, "Led"),
            Part::Bjt(_) => (&mut self.next_bjt, "Bjt"),
            Part::Mosfet(_) => (&mut self.next_mosfet, "Mosfet"),
            Part::OpAmp(_) => (&mut self.next_opamp, "OpAmp"),
            Part::Jfet(_) => (&mut self.next_jfet, "Jfet"),
            Part::Comparator(_) => (&mut self.next_comparator, "Comparator"),
            Part::VoltReg(_) => (&mut self.next_voltreg, "VoltReg"),
            Part::Probe(_) => (&mut self.next_probe, "Probe"),
            Part::Voltmeter(_) => (&mut self.next_voltmeter, "Voltmeter"),
            Part::Ammeter(_) => (&mut self.next_ammeter, "Ammeter"),
            Part::FreqMeter(_) => (&mut self.next_freqmeter, "FreqMeter"),
            Part::Oscope(_) => (&mut self.next_oscope, "Oscope"),
            Part::LogicAnalyzer(_) => (&mut self.next_lanalizer, "LogicAnalyzer"),
            _ => (&mut self.next_resistor, part.type_name()),
        };
        let id = format!("{prefix}-{next}");
        *next += 1;
        id
    }

    pub fn analog_dt(&self) -> f64 {
        self.settings.analog_dt()
    }

    pub fn to_circuit(&self) -> Circuit {
        let mut c = Circuit::new();
        c.dt = self.settings.analog_dt();
        c.max_nl_steps = self.settings.nl_steps;
        c.ps_per_sec = self.settings.ps_per_sec();
        for it in &self.items {
            it.kind.add_to_circuit(&it.id, &mut c);
        }
        for h in &self.hidden {
            c.add_comp(h.clone());
        }
        for w in &self.wires {
            if let Some(end) = &w.end_pin {
                c.connect(&w.start_pin, end);
            }
        }
        for pin_id in &self.inverted_pins {
            c.set_pin_inverted(pin_id, self.is_pin_inverted(pin_id));
        }
        c
    }

    pub fn from_sim1(src: &str) -> crate::Result<Self> {
        Self::from_sim1_with(src, &crate::subcircuit::SubcSearch::default())
    }

    pub fn from_sim1_with(
        src: &str,
        search: &crate::subcircuit::SubcSearch,
    ) -> crate::Result<Self> {
        let parsed =
            circ1::parse_circ1(src).or_else(|_| crate::sim1::parse_legacy_to_circ1(src))?;
        let mut s = Scene::new();
        s.settings = parsed.circ.clone();

        for it in parsed.items {
            let (x, y) = it.graphic.pos.unwrap_or((0.0, 0.0));
            let mut item = Item::new(it.circ_id.clone(), x, y, it.part);
            item.rotation = it.graphic.rotation;
            item.hflip = it.graphic.hflip;
            item.vflip = it.graphic.vflip;
            item.label = it.graphic.label.unwrap_or_else(|| it.circ_id.clone());
            item.show_id = it.graphic.show_id;
            if let Some((lx, ly)) = it.graphic.label_pos {
                item.label_x = lx;
                item.label_y = ly;
                item.custom_label_pos = true;
            }
            item.label_rot = it.graphic.label_rot;
            item.show_val = it.graphic.show_val;
            item.show_prop = it.graphic.show_prop.unwrap_or_default();
            if let Some((vx, vy)) = it.graphic.val_pos {
                item.val_x = vx;
                item.val_y = vy;
                item.custom_val_pos = true;
            }
            item.val_rot = it.graphic.val_rot;

            // Handle Mcu, QemuDevice, Subcircuit view loading
            match &mut item.kind {
                Part::Mcu(mcu) => {
                    let dev = if !mcu.device.is_empty() && mcu.device != item.id {
                        crate::subcircuit::device_from_id(&mcu.device)
                    } else {
                        crate::subcircuit::device_from_id(&item.id)
                    };
                    let spec = crate::mcu::McuItemSpec {
                        device: dev.clone(),
                        frequency: if mcu.mcu.device.freq > 0.0 {
                            Some(mcu.mcu.device.freq)
                        } else {
                            None
                        },
                        force_freq: mcu.mcu.force_freq,
                        program: mcu.mcu.firmware.clone(),
                        auto_load: mcu.mcu.auto_load,
                        save_pgm: mcu.mcu.save_pgm,
                        pgm: if mcu.pgm.is_empty() {
                            None
                        } else {
                            Some(mcu.pgm.clone())
                        },
                        logic_symbol: mcu.logic_symbol,
                        package_name: if mcu.package.name.is_empty() {
                            None
                        } else {
                            Some(mcu.package.name.clone())
                        },
                    };
                    if let Ok(view) = crate::mcu::instantiate_view(&item.id, &spec, search) {
                        mcu.device = dev;
                        mcu.mcu = view.mcu;
                        mcu.packages = view.packages;
                        mcu.package = view.package;
                        mcu.logic_symbol = view.logic_symbol;
                    } else {
                        let packages = crate::mcu::load_packages(&dev, search);
                        let pkg = crate::mcu::canvas_package(
                            &mcu.mcu,
                            &item.id,
                            &packages,
                            mcu.logic_symbol,
                            spec.package_name.as_deref(),
                        );
                        mcu.device = dev;
                        mcu.packages = packages;
                        mcu.package = pkg;
                    }
                }
                Part::QemuDevice(qemu) => {
                    let mut view =
                        crate::qemu::instantiate_qemu_view(&item.id, &qemu.qemu.device, search);
                    view.qemu.firmware_dir = search.circuit_dir.clone();
                    if !qemu.qemu.firmware.is_empty() {
                        view.qemu.firmware = qemu.qemu.firmware.clone();
                    }
                    if !qemu.qemu.extra_args.is_empty() {
                        view.qemu.extra_args = qemu.qemu.extra_args.clone();
                    }
                    qemu.package = view.package;
                    qemu.packages = view.packages;
                    qemu.qemu = view.qemu;
                }
                Part::Subcircuit(subc) => {
                    if let Ok(view) = crate::subcircuit::load_view(
                        &item.id,
                        &subc.device,
                        search,
                        subc.logic_symbol,
                        Some(&subc.package.name),
                    ) {
                        subc.package = view.package;
                        subc.nested_src = view.nested_src;
                        subc.nested_path = view.nested_path;
                        subc.logic_symbol = view.logic_symbol;
                    }
                }
                Part::WaveGen(wg) => {
                    if wg.wav_data.is_none() && !wg.file.is_empty() {
                        let candidate = search.circuit_dir.as_ref().map(|d| d.join(&wg.file));
                        let resolved = candidate.filter(|p| p.exists());
                        let wav_res = if let Some(p) = resolved {
                            crate::wav::WavData::from_file(p)
                        } else {
                            crate::wav::WavData::from_file(&wg.file)
                        };
                        if let Ok(wav) = wav_res {
                            wg.wav_data = Some(wav.samples);
                        }
                    }
                }
                _ => {}
            }

            s.add_saved_item(item);
        }

        // Infer missing Pos from connector endpoints if any
        let mut pin_pos = std::collections::HashMap::new();
        for c in &parsed.connectors {
            if let Some(first) = c.points.first() {
                pin_pos.insert(c.start.clone(), Point::new(first.0, first.1));
            }
            if let Some(last) = c.points.last() {
                pin_pos.insert(c.end.clone(), Point::new(last.0, last.1));
            }
        }
        for it in &mut s.items {
            for p in it.pins() {
                if let Some(scene) = pin_pos.get(&p.id) {
                    let local = it.map_local(p.local);
                    it.x += scene.x - local.x;
                    it.y += scene.y - local.y;
                    break;
                }
            }
        }

        for (idx, c) in parsed.connectors.into_iter().enumerate() {
            let points: Vec<Point> = c.points.iter().map(|(x, y)| Point::new(*x, *y)).collect();
            let wire = Wire::from_saved(
                if c.id.is_empty() {
                    format!("Connector-{}", idx + 1)
                } else {
                    c.id
                },
                c.start,
                c.end,
                points,
            );
            s.add_saved_wire(wire);
        }

        Ok(s)
    }

    pub fn to_sim1(&self) -> String {
        let mut item_lines = Vec::with_capacity(self.items.len());
        for it in &self.items {
            let graphic = it.graphic_attrs();
            item_lines.push(it.kind.write_item(&it.id, &graphic));
        }
        for h in &self.hidden {
            if let crate::elements::Kind::Mcu(m) = &h.kind {
                item_lines.push(hidden_mcu_sim1(&h.id, m));
            }
        }

        let mut connectors = Vec::with_capacity(self.wires.len());
        for w in &self.wires {
            if let Some(end) = &w.end_pin {
                connectors.push(crate::sim1::ParsedConnector {
                    id: w.id.clone(),
                    start: w.start_pin.clone(),
                    end: end.clone(),
                    points: w.points.iter().map(|p| (p.x, p.y)).collect(),
                });
            }
        }

        circ1::write_circ1(&item_lines, &connectors, &self.settings)
    }
}
