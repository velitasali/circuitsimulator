//! Canvas component property editing, package pin geometry, linker and monitors.

use crate::canvas::events::Change;
use crate::canvas::scene::Part;
use crate::canvas::{Canvas, Rect};
use crate::components::ComponentChange;

fn item_monitor_names(it: &crate::canvas::scene::Item) -> Vec<String> {
    match &it.kind {
        Part::Mcu(mcu) => {
            let list = mcu.mcu.device.trans_module_names();
            if list.is_empty() {
                vec!["USART".into(), "TWI".into(), "SPI".into()]
            } else {
                list
            }
        }
        Part::QemuDevice(qemu) => {
            let mut list: Vec<String> = qemu
                .qemu
                .usarts
                .iter()
                .enumerate()
                .map(|(i, _)| format!("USART{}", i + 1))
                .collect();
            list.extend(
                qemu.qemu
                    .spis
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("SPI{}", i + 1)),
            );
            list.extend(
                qemu.qemu
                    .twis
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("I2C{}", i + 1)),
            );
            if list.is_empty() {
                vec!["USART1".into()]
            } else {
                list
            }
        }
        Part::SerialPort(_) | Part::SerialTerm(_) | Part::Esp01(_) => {
            vec!["Serial Monitor".into()]
        }
        _ => Vec::new(),
    }
}

fn structural_text_prop(name: &str) -> bool {
    matches!(
        name,
        "Package"
            | "package"
            | "Power_Pins"
            | "Switch_Pins"
            | "Num_Inputs"
            | "numInputs"
            | "Channels"
            | "Poles"
            | "Size"
            | "Rows"
            | "Cols"
            | "Segments"
            | "Pins"
            | "Down"
            | "Width"
            | "Height"
            | "Count"
            | "Modules"
            | "ShowButton"
            | "Show_Button"
            | "show_button"
            | "NumDisplays"
            | "numDisplays"
            | "Vertical_Pins"
            | "vertical_pins"
    )
}

fn structural_bool_prop(name: &str) -> bool {
    matches!(
        name,
        "Power_Pins"
            | "Switch_Pins"
            | "Logic_Symbol"
            | "LogicSymbol"
            | "logic_symbol"
            | "Grounded"
            | "Use_RS"
            | "UseRS"
            | "Use_Reset"
            | "Tristate"
            | "Double_Throw"
            | "DT"
            | "Common_Pin"
            | "CommonPin"
            | "Bussed"
            | "PullUp"
            | "Control_Pins"
            | "Small"
            | "ShowButton"
            | "Show_Button"
            | "show_button"
            | "Vertical_Pins"
            | "vertical_pins"
    )
}

impl Canvas {
    pub fn open_properties(&mut self, uid: &str) -> Change {
        if self.scene.item_by_id(uid).is_none() {
            return Change::default();
        }
        self.prop_uid = Some(uid.to_string());
        self.prop_open = true;
        self.last_opened_prop_uid = Some(uid.to_string());
        if !self.open_prop_uids.iter().any(|id| id == uid) {
            self.open_prop_uids.push(uid.to_string());
        }
        Change {
            props: true,
            open_props: true,
            ..Change::default()
        }
    }

    pub fn open_selected_properties(&mut self) -> Change {
        let uid = self
            .scene
            .items()
            .iter()
            .find(|it| it.selected && !it.is_node())
            .map(|it| it.id.clone())
            .or_else(|| {
                self.scene
                    .items()
                    .iter()
                    .find(|it| it.selected)
                    .map(|it| it.id.clone())
            });
        match uid {
            Some(id) => self.open_properties(&id),
            None => Change::default(),
        }
    }

    pub fn close_property_dialog(&mut self, uid: &str) -> Change {
        if let Some(pos) = self.open_prop_uids.iter().position(|id| id == uid) {
            self.open_prop_uids.remove(pos);
        }
        if self.prop_uid.as_deref() == Some(uid) {
            self.prop_uid = self.open_prop_uids.last().cloned();
            self.prop_open = !self.open_prop_uids.is_empty();
        }
        if self.last_opened_prop_uid.as_deref() == Some(uid) {
            self.last_opened_prop_uid = None;
        }
        Change {
            props: true,
            ..Change::default()
        }
    }

    pub fn close_properties(&mut self) -> Change {
        if !self.prop_open && self.prop_uid.is_none() && self.open_prop_uids.is_empty() {
            return Change::default();
        }
        self.prop_open = false;
        self.prop_uid = None;
        self.open_prop_uids.clear();
        self.last_opened_prop_uid = None;
        Change {
            props: true,
            ..Change::default()
        }
    }

    pub fn set_prop_label_for(&mut self, uid: &str, text: String) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::document(uid), |scene| {
            if let Some(it) = scene.item_by_id_mut(&uid_s) {
                it.label = text;
                true
            } else {
                false
            }
        })
    }

    pub fn set_prop_label(&mut self, text: String) -> Change {
        let Some(uid) = self.prop_uid.clone() else {
            return Change::default();
        };
        self.set_prop_label_for(&uid, text)
    }

    pub fn set_prop_show_id_for(&mut self, uid: &str, show: bool) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::document(uid), |scene| {
            if let Some(it) = scene.item_by_id_mut(&uid_s) {
                it.show_id = show;
                true
            } else {
                false
            }
        })
    }

    pub fn set_prop_show_id(&mut self, show: bool) -> Change {
        let Some(uid) = self.prop_uid.clone() else {
            return Change::default();
        };
        self.set_prop_show_id_for(&uid, show)
    }

    pub fn set_show_prop_for(&mut self, uid: &str, name: String, show: bool) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::document(uid), |scene| {
            scene
                .item_by_id_mut(&uid_s)
                .map(|it| {
                    it.set_show_prop(&name, show);
                    true
                })
                .unwrap_or(false)
        })
    }

    pub fn set_show_prop(&mut self, name: String, show: bool) -> Change {
        let Some(uid) = self.prop_uid.clone() else {
            return Change::default();
        };
        self.set_show_prop_for(&uid, name, show)
    }

    pub fn set_prop_text_for(&mut self, uid: &str, name: String, text: String) -> Change {
        let spec = if structural_text_prop(&name) {
            ComponentChange::structural(uid)
        } else {
            ComponentChange::document(uid)
        };
        let uid = uid.to_string();
        self.apply_component_edit(spec, |scene| {
            scene
                .item_by_id_mut(&uid)
                .is_some_and(|it| it.set_prop_text(&name, &text))
        })
    }

    pub fn set_prop_text(&mut self, name: String, text: String) -> Change {
        let Some(uid) = self.prop_uid.clone() else {
            return Change::default();
        };
        self.set_prop_text_for(&uid, name, text)
    }

    pub fn set_prop_bool_for(&mut self, uid: &str, name: String, value: bool) -> Change {
        let spec = if structural_bool_prop(&name) {
            ComponentChange::structural(uid)
        } else {
            ComponentChange::document(uid)
        };
        let uid = uid.to_string();
        self.apply_component_edit(spec, |scene| {
            scene
                .item_by_id_mut(&uid)
                .is_some_and(|it| it.set_prop_bool(&name, value))
        })
    }

    pub fn set_prop_bool(&mut self, name: String, value: bool) -> Change {
        let Some(uid) = self.prop_uid.clone() else {
            return Change::default();
        };
        self.set_prop_bool_for(&uid, name, value)
    }

    pub fn toggle_item(&mut self, uid: &str) -> Change {
        let Some(idx) = self.scene.items().iter().position(|it| it.id == uid) else {
            return Change::default();
        };
        let center = self.scene.items()[idx].position();
        self.apply_component_edit(ComponentChange::document(uid), |scene| {
            scene.toggle_switch_at(idx, center).is_some()
        })
    }

    pub fn toggle_dip_switch(&mut self, uid: &str, switch_idx: usize) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::document(uid), |scene| {
            scene.toggle_dip_switch(&uid_s, switch_idx)
        })
    }

    pub fn set_dip_switch(&mut self, uid: &str, switch_idx: usize, on: bool) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::document(uid), |scene| {
            scene.set_dip_switch(&uid_s, switch_idx, on)
        })
    }

    pub fn set_push_state(&mut self, uid: &str, pressed: bool) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::live(uid), |scene| {
            scene.set_push_state(&uid_s, pressed)
        })
    }

    pub fn set_keypad_pressed(
        &mut self,
        uid: &str,
        row: usize,
        col: usize,
        pressed: bool,
    ) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::live(uid), |scene| {
            scene.set_keypad_pressed(&uid_s, row, col, pressed)
        })
    }

    pub fn set_touchpad_pos(&mut self, uid: &str, x: i32, y: i32) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::live(uid), |scene| {
            scene.set_touchpad_pos(&uid_s, x, y)
        })
    }

    pub fn reset_touchpad(&mut self, uid: &str) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::live(uid), |scene| {
            scene.reset_touchpad(&uid_s)
        })
    }

    pub fn set_joystick_pos(&mut self, uid: &str, x: f64, y: f64) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::continuous(uid), |scene| {
            scene.set_joystick_pos(&uid_s, x, y)
        })
    }

    pub fn set_joystick_button(&mut self, uid: &str, down: bool) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::continuous(uid), |scene| {
            scene.set_joystick_button(&uid_s, down)
        })
    }

    pub fn set_rotary_dial(&mut self, uid: &str, val: i32) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::continuous(uid), |scene| {
            scene.set_rotary_dial(&uid_s, val)
        })
    }

    pub fn set_rotary_button(&mut self, uid: &str, closed: bool) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::continuous(uid), |scene| {
            scene.set_rotary_button(&uid_s, closed)
        })
    }

    pub fn set_sr04_distance(&mut self, uid: &str, dist: f64) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::continuous(uid), |scene| {
            scene.set_sr04_distance(&uid_s, dist)
        })
    }

    pub fn step_sensor_temp(&mut self, uid: &str, up: bool) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::continuous(uid), |scene| {
            scene.step_sensor_temp(&uid_s, up)
        })
    }

    pub fn step_sensor_humi(&mut self, uid: &str, up: bool) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::continuous(uid), |scene| {
            scene.step_sensor_humi(&uid_s, up)
        })
    }

    pub fn set_pot_wiper(&mut self, uid: &str, wiper: f64) -> Change {
        let uid_s = uid.to_string();
        let c = self.apply_component_edit(ComponentChange::continuous(uid), |scene| {
            scene.set_pot_wiper(&uid_s, wiper)
        });
        if let Some(item) = self.scene.item_by_id(uid) {
            self.last_wheel_item = Some((
                uid.to_string(),
                item.source_value(),
                item.val_label_text(),
                item.wiper(),
            ));
        }
        c
    }

    pub fn set_dial_val(&mut self, uid: &str, val: f64) -> Change {
        let uid_s = uid.to_string();
        let c = self.apply_component_edit(ComponentChange::continuous(uid), |scene| {
            scene.set_dial_val(&uid_s, val)
        });
        if let Some(item) = self.scene.item_by_id(uid) {
            self.last_wheel_item = Some((
                uid.to_string(),
                item.source_value(),
                item.val_label_text(),
                item.wiper(),
            ));
        }
        c
    }

    pub fn add_package_pin(
        &mut self,
        uid: &str,
        angle: i32,
        x: i32,
        y: i32,
        id: &str,
        label: &str,
    ) -> Change {
        let uid_s = uid.to_string();
        let id = id.to_string();
        let label = label.to_string();
        self.apply_component_edit(ComponentChange::structural(uid), |scene| {
            scene.add_package_pin(&uid_s, angle, x, y, &id, &label)
        })
    }

    pub fn remove_package_pin(&mut self, uid: &str, pin_id: &str) -> Change {
        let uid_s = uid.to_string();
        let pin_id = pin_id.to_string();
        self.apply_component_edit(ComponentChange::structural(uid), |scene| {
            scene.remove_package_pin(&uid_s, &pin_id)
        })
    }

    pub fn update_package_pin(
        &mut self,
        uid: &str,
        old_pin_id: &str,
        new_pin: crate::package::PkgPin,
    ) -> Change {
        let uid_s = uid.to_string();
        let old_pin_id = old_pin_id.to_string();
        self.apply_component_edit(ComponentChange::structural(uid), |scene| {
            scene.update_package_pin(&uid_s, &old_pin_id, new_pin)
        })
    }

    pub fn generate_package_pins(
        &mut self,
        uid: &str,
        left: usize,
        right: usize,
        top: usize,
        bottom: usize,
        prefix: &str,
        start_index: usize,
        clear_first: bool,
    ) -> Change {
        let uid_s = uid.to_string();
        let prefix = prefix.to_string();
        self.apply_component_edit(ComponentChange::structural(uid), |scene| {
            scene.generate_package_pins(
                &uid_s,
                left,
                right,
                top,
                bottom,
                &prefix,
                start_index,
                clear_first,
            )
        })
    }

    pub fn set_package_dimensions(&mut self, uid: &str, width: i32, height: i32) -> Change {
        let uid_s = uid.to_string();
        self.apply_component_edit(ComponentChange::structural(uid), |scene| {
            scene.set_package_dimensions(&uid_s, width, height)
        })
    }

    pub fn set_package_footprint_dip(
        &mut self,
        uid: &str,
        name: &str,
        pin_count: usize,
        custom_width: Option<i32>,
    ) -> Change {
        let uid_s = uid.to_string();
        let name = name.to_string();
        self.apply_component_edit(ComponentChange::structural(uid), |scene| {
            scene.set_package_footprint_dip(&uid_s, &name, pin_count, custom_width)
        })
    }

    pub fn set_package_footprint_ls(
        &mut self,
        uid: &str,
        name: &str,
        inputs: &[&str],
        outputs: &[&str],
        top: &[&str],
        bottom: &[&str],
    ) -> Change {
        let uid_s = uid.to_string();
        let name = name.to_string();
        let inputs: Vec<String> = inputs.iter().map(|s| (*s).to_string()).collect();
        let outputs: Vec<String> = outputs.iter().map(|s| (*s).to_string()).collect();
        let top: Vec<String> = top.iter().map(|s| (*s).to_string()).collect();
        let bottom: Vec<String> = bottom.iter().map(|s| (*s).to_string()).collect();
        self.apply_component_edit(ComponentChange::structural(uid), |scene| {
            let inputs: Vec<&str> = inputs.iter().map(String::as_str).collect();
            let outputs: Vec<&str> = outputs.iter().map(String::as_str).collect();
            let top: Vec<&str> = top.iter().map(String::as_str).collect();
            let bottom: Vec<&str> = bottom.iter().map(String::as_str).collect();
            scene.set_package_footprint_ls(&uid_s, &name, &inputs, &outputs, &top, &bottom)
        })
    }

    pub fn load_package_xml(&mut self, uid: &str, xml: &str) -> Change {
        let uid_s = uid.to_string();
        let xml = xml.to_string();
        self.apply_component_edit(ComponentChange::structural(uid), |scene| {
            scene.load_package_xml(&uid_s, &xml)
        })
    }

    pub fn export_package_xml(&self, uid: &str) -> Option<String> {
        self.scene.export_package_xml(uid)
    }

    pub fn get_package_pin(&self, uid: &str, pin_id: &str) -> Option<crate::package::PkgPin> {
        let pkg = self.scene.package_ref(uid)?;
        pkg.find_pin(pin_id).cloned()
    }

    pub fn selected_item_uarts(&self) -> Vec<String> {
        self.selected_item_monitors()
    }

    /// C++ `m_transModules` names (USART, TWI, SPI) or QEMU USART ports.
    pub fn selected_item_monitors(&self) -> Vec<String> {
        if let Some(it) = self.selected_or_nested_mcu() {
            return item_monitor_names(it);
        }
        let Some(it) = self.scene.items().iter().find(|it| it.selected) else {
            return Vec::new();
        };
        if let Some(uid) = self.selected_nested_mcu_uid() {
            return self.monitors_for(&uid);
        }
        item_monitor_names(it)
    }

    pub fn monitors_for(&self, uid: &str) -> Vec<String> {
        if let Some(it) = self.scene.item_by_id(uid) {
            return item_monitor_names(it);
        }
        for c in self.scene.hidden() {
            if c.id == uid || uid.strip_suffix(&c.id).is_some_and(|p| p.ends_with('/')) {
                if let crate::elements::Kind::Mcu(mcu) = &c.kind {
                    return mcu.device.trans_module_names();
                }
            }
        }
        Vec::new()
    }

    pub fn linking_from(&self) -> Option<&str> {
        self.linking_from.as_deref()
    }

    pub fn start_linking(&mut self, uid: &str) {
        self.linking_from = Some(uid.to_string());
    }

    pub fn stop_linking(&mut self) {
        self.linking_from = None;
    }

    /// Toggle a link from `linking_from` to `target`. Empty target or same uid ends linking.
    pub fn complete_link(&mut self, target: &str) -> Change {
        let Some(from) = self.linking_from.clone() else {
            return Change::default();
        };
        if target.is_empty() || target == from {
            self.linking_from = None;
            return Change {
                cursor: true,
                ..Change::default()
            };
        }
        let entry = self.linked.entry(from).or_default();
        if let Some(i) = entry.iter().position(|id| id == target) {
            entry.remove(i);
        } else {
            entry.push(target.to_string());
        }
        Change {
            items: true,
            ..Change::default()
        }
    }

    pub(crate) fn refresh_props(&mut self) -> Change {
        let prev_len = self.open_prop_uids.len();
        self.open_prop_uids
            .retain(|id| self.scene.item_by_id(id).is_some());
        let open_props_changed = self.open_prop_uids.len() != prev_len;
        if let Some(uid) = &self.prop_uid {
            if self.scene.item_by_id(uid).is_none() {
                self.prop_uid = self.open_prop_uids.last().cloned();
                self.prop_open = !self.open_prop_uids.is_empty();
            }
        }
        if let Some(last) = &self.last_opened_prop_uid {
            if self.scene.item_by_id(last).is_none() {
                self.last_opened_prop_uid = None;
            }
        }
        Change {
            props: true,
            open_props: open_props_changed,
            ..Change::default()
        }
    }

    /// Rubber band in *item* coordinates. C++ `bandItemRect`.
    pub fn band_item_rect(&self) -> Rect {
        if !self.banding || self.band_rect.is_null() {
            return Rect::default();
        }
        self.viewport
            .map_rect_from_circuit(self.band_rect.normalized())
    }
}
