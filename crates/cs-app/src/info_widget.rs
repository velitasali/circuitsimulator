//! Simulation info card. Colours are hex strings, not `QColor`.

use qtbridge::qobject;

pub struct InfoWidget {
    sim_time: String,
    target_speed: String,
    real_speed: String,
    sim_load: String,
    gui_load: String,
    fps: String,
    over_load: String,
    over_loaded: bool,
    mcu_device: String,
    mcu_name: String,
    has_mcu: bool,
    slider_value: i32,
    panel_background: String,
    panel_border: String,
    text_color: String,
    accent_color: String,
}

impl Default for InfoWidget {
    fn default() -> Self {
        let st = cs_engine::settings::get();
        let dark = crate::macos_host::apply_theme(&st.theme);
        let mut s = Self {
            sim_time: "00:00:00 s  000 ms  000 µs  000 ns  000 ps ".into(),
            target_speed: "100.00 %".into(),
            real_speed: "00.00 %".into(),
            sim_load: "00.00 %".into(),
            gui_load: "00.00 %".into(),
            fps: "0".into(),
            over_load: String::new(),
            over_loaded: false,
            mcu_device: "---".into(),
            mcu_name: "---".into(),
            has_mcu: false,
            slider_value: 150,
            panel_background: String::new(),
            panel_border: String::new(),
            text_color: String::new(),
            accent_color: String::new(),
        };
        s.compute_theme_colors(dark);
        s
    }
}

pub fn format_circ_time(t_step: u64) -> String {
    let mut step = (t_step as f64) / 1e6;
    let hours = (step / 3600e6) as i64;
    step -= (hours as f64) * 3600e6;
    let mins = (step / 60e6) as i64;
    step -= (mins as f64) * 60e6;
    let secs = (step / 1e6) as i64;
    step -= (secs as f64) * 1e6;
    let m_secs = (step / 1e3) as i64;
    step -= (m_secs as f64) * 1e3;
    let u_secs = step as i64;
    step -= u_secs as f64;
    let n_secs = (step * 1e3) as i64;
    step -= (n_secs as f64) / 1e3;
    step += 1e-7;
    let p_secs = (step * 1e6) as i64;

    format!(
        "{:02}:{:02}:{:02} s  {:03} ms  {:03} µs  {:03} ns  {:03} ps ",
        hours, mins, secs, m_secs, u_secs, n_secs, p_secs
    )
}

#[qobject(Singleton, ConvertToCamelCase)]
impl InfoWidget {
    qproperty!("simTime", Read = sim_time, Notify = sim_time_changed);
    qproperty!(
        "targetSpeed",
        Read = target_speed,
        Notify = target_speed_changed
    );
    qproperty!("realSpeed", Read = real_speed, Notify = rate_changed);
    qproperty!("simLoad", Read = sim_load, Notify = rate_changed);
    qproperty!("guiLoad", Read = gui_load, Notify = rate_changed);
    qproperty!("fps", Read = fps, Notify = rate_changed);
    qproperty!("overLoad", Read = over_load, Notify = rate_changed);
    qproperty!("overLoaded", Read = over_loaded, Notify = rate_changed);
    qproperty!("mcuDevice", Read = mcu_device, Notify = mcu_changed);
    qproperty!("mcuName", Read = mcu_name, Notify = mcu_changed);
    qproperty!("hasMcu", Read = has_mcu, Notify = mcu_changed);
    qproperty!(
        "sliderValue",
        Read = slider_value,
        Write = set_slider_value,
        Notify = speed_changed
    );
    qproperty!(
        "panelBackground",
        Read = panel_background,
        Notify = theme_changed
    );
    qproperty!("panelBorder", Read = panel_border, Notify = theme_changed);
    qproperty!("textColor", Read = text_color, Notify = theme_changed);
    qproperty!("accentColor", Read = accent_color, Notify = theme_changed);

    fn sim_time(&self) -> String {
        self.sim_time.clone()
    }
    fn target_speed(&self) -> String {
        self.target_speed.clone()
    }
    fn real_speed(&self) -> String {
        self.real_speed.clone()
    }
    fn sim_load(&self) -> String {
        self.sim_load.clone()
    }
    fn gui_load(&self) -> String {
        self.gui_load.clone()
    }
    fn fps(&self) -> String {
        self.fps.clone()
    }
    fn over_load(&self) -> String {
        self.over_load.clone()
    }
    fn over_loaded(&self) -> bool {
        self.over_loaded
    }
    fn mcu_device(&self) -> String {
        self.mcu_device.clone()
    }
    fn mcu_name(&self) -> String {
        self.mcu_name.clone()
    }
    fn has_mcu(&self) -> bool {
        self.has_mcu
    }
    fn slider_value(&self) -> i32 {
        self.slider_value
    }
    fn panel_background(&self) -> String {
        self.panel_background.clone()
    }
    fn panel_border(&self) -> String {
        self.panel_border.clone()
    }
    fn text_color(&self) -> String {
        self.text_color.clone()
    }
    fn accent_color(&self) -> String {
        self.accent_color.clone()
    }

    #[qsignal]
    fn sim_time_changed(&mut self);
    #[qsignal]
    fn target_speed_changed(&mut self);
    #[qsignal]
    fn rate_changed(&mut self);
    #[qsignal]
    fn mcu_changed(&mut self);
    #[qsignal]
    fn speed_changed(&mut self);
    #[qsignal]
    fn theme_changed(&mut self);

    #[qslot]
    fn sync_sim(
        &mut self,
        circ_time_ps: f64,
        target_speed: f64,
        rate: f64,
        sim_load: f64,
        gui_load: f64,
        fps: i32,
        mcu_device: String,
        mcu_name: String,
        has_mcu: bool,
    ) {
        let new_time = format_circ_time(circ_time_ps.max(0.0) as u64);
        if self.sim_time != new_time {
            self.sim_time = new_time;
            self.sim_time_changed();
        }

        let new_target = format_percent(target_speed);
        if self.target_speed != new_target {
            self.target_speed = new_target;
            self.target_speed_changed();
        }

        let (r_speed, s_load, g_load, f_str, o_load, is_overloaded) = if rate < 0.0 {
            let msg = if (rate + 1.0).abs() < 1e-3 {
                cs_engine::i18n::tr("Speed: Debugger")
            } else {
                cs_engine::i18n::tr("Circuit ERROR!!!")
            };
            (
                msg,
                "00.00 %".to_string(),
                "00.00 %".to_string(),
                fps.to_string(),
                String::new(),
                false,
            )
        } else {
            let s_rate = format_percent(rate);
            let over_loaded = sim_load > 101.0;
            let mut o_load = String::new();
            let final_sim_load = if over_loaded {
                let over_load = sim_load - 100.0;
                o_load = format_percent(over_load);
                100.0
            } else {
                sim_load
            };
            let s_load = format_percent(final_sim_load);
            let g_load = format_percent(gui_load);
            (s_rate, s_load, g_load, fps.to_string(), o_load, over_loaded)
        };

        if self.real_speed != r_speed
            || self.sim_load != s_load
            || self.gui_load != g_load
            || self.fps != f_str
            || self.over_load != o_load
            || self.over_loaded != is_overloaded
        {
            self.real_speed = r_speed;
            self.sim_load = s_load;
            self.gui_load = g_load;
            self.fps = f_str;
            self.over_load = o_load;
            self.over_loaded = is_overloaded;
            self.rate_changed();
        }

        let dev = if has_mcu { mcu_device } else { "---".into() };
        let nm = if has_mcu { mcu_name } else { "---".into() };
        if self.mcu_device != dev || self.mcu_name != nm || self.has_mcu != has_mcu {
            self.mcu_device = dev;
            self.mcu_name = nm;
            self.has_mcu = has_mcu;
            self.mcu_changed();
        }
    }

    fn compute_theme_colors(&mut self, dark: bool) {
        use cs_engine::theme::{ColorId, ColorTheme};
        let (bg_r, bg_g, bg_b, _) = ColorTheme::get_rgba(ColorId::TreeItemBg1, dark);
        self.panel_background = format!("#dc{bg_r:02x}{bg_g:02x}{bg_b:02x}");

        let (b_r, b_g, b_b, _) = ColorTheme::get_rgba(ColorId::TreeItemText1, dark);
        self.panel_border = format!("#64{b_r:02x}{b_g:02x}{b_b:02x}");

        self.text_color = ColorTheme::get_hex(ColorId::ComponentText, dark);
        self.accent_color = ColorTheme::get_hex(ColorId::ItemHovered, dark);
    }

    #[qslot]
    fn update_theme_colors(&mut self, dark: bool) {
        self.compute_theme_colors(dark);
        self.theme_changed();
    }

    fn set_slider_value(&mut self, v: i32) {
        let v = v.clamp(1, 1000);
        if self.slider_value == v {
            return;
        }
        self.slider_value = v;
        self.speed_changed();
    }
}

pub fn format_percent(val: f64) -> String {
    format!("{:05.2} %", val)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speed_percent_formatting() {
        assert_eq!(format_percent(100.0), "100.00 %");
        assert_eq!(format_percent(50.0), "50.00 %");
        assert_eq!(format_percent(1.0), "01.00 %");
        assert_eq!(format_percent(0.0), "00.00 %");
        assert_eq!(format_percent(1000.0), "1000.00 %");
    }

    #[test]
    fn test_format_circ_time() {
        let t = format_circ_time(1_000_000_000_000); // 1 sec
        assert!(t.contains("00:00:01 s"));
    }
}
