//! Pin-edge and device-event updates for the Phase 5 displays and sensors.

use crate::digital::Drive;
use crate::elements::Kind;

pub(super) fn on_pins(kind: &mut Kind, id: &str, v: impl Fn(&str) -> f64, t: u64) -> Drive {
    let pin = |suffix: &str| v(&format!("{id}-{suffix}"));
    match kind {
        Kind::Max72xx { modules, fb, .. } => {
            let gnd = pin("gnd");
            let high = |suffix: &str| pin(suffix) - gnd > 2.0;
            let changed = fb.step(*modules, high("din"), high("clk"), high("cs"));
            Drive::stamp(changed)
        }
        Kind::Ws2812 {
            count,
            rst_time_ns,
            t0h_ns,
            t1h_ns,
            fb,
            ..
        } => {
            let high = pin("din") - pin("gnd") > 2.0;
            let (changed, arm) = fb.on_level(high, t, *count, *t0h_ns, *t1h_ns, *rst_time_ns);
            match arm {
                Some(at) => Drive::arm(at, changed, true),
                None => Drive::stamp(changed),
            }
        }
        Kind::Sr04 {
            distance,
            use_slider,
            trigger_high,
            echo_high,
            echo,
        } => echo.on_pins(
            trigger_high,
            echo_high,
            *distance,
            *use_slider,
            pin("vccpin"),
            pin("gndpin"),
            pin("trigpin"),
            pin("inpin"),
            t,
        ),
        Kind::Dht22 {
            model,
            temp,
            humi,
            out_state,
            pin_driven,
            link,
        } => link.on_pins(model, *temp, *humi, out_state, pin_driven, pin("inPin"), t),
        Kind::Ds18b20 {
            temp, dq_low, wire, ..
        } => wire.on_pins(*temp, dq_low, pin("inPin"), t),
        Kind::Ds1621 {
            temp,
            th,
            tl,
            tout_high,
            sda_low,
            bus,
            ..
        } => bus.on_pins(
            *temp,
            th,
            tl,
            tout_high,
            sda_low,
            pin("inPin1"),
            pin("inPin0"),
            pin("inPin2") > 2.5,
            pin("inPin3") > 2.5,
            pin("inPin4") > 2.5,
        ),
        Kind::Ds1307 {
            time_updated,
            sqw_freq,
            sqw_enabled,
            sqw_state,
            sda_low,
            bus,
            ..
        } => bus.on_pins(
            *time_updated,
            sqw_freq,
            sqw_enabled,
            sqw_state,
            sda_low,
            pin("PinSCL"),
            pin("PinSDA"),
            t,
        ),
        Kind::Esp01 {
            baud_rate, uart, ..
        } => {
            let rx_high = pin("pin1") > 2.0;
            uart.on_pins(*baud_rate, rx_high, t)
        }
        Kind::SerialPort {
            baud_rate, uart, ..
        } => {
            let rx_high = pin("rx") > 2.0;
            uart.on_pins(id, *baud_rate, rx_high, t)
        }
        Kind::SerialTerm {
            baud_rate, uart, ..
        } => {
            let rx_high = pin("rx") > 2.0;
            uart.on_pins(id, *baud_rate, rx_high, t)
        }
        Kind::SdCard { spi, .. } => {
            let cs_high = pin("PinCS") > 2.0;
            let sck_high = pin("PinCK") > 2.0;
            let di_high = pin("PinDI") > 2.0;
            let changed = spi.step(cs_high, sck_high, di_high);
            Drive::stamp(changed)
        }
        Kind::TftDisplay { tft, .. } => {
            let cs_high = pin("PinCS") > 2.0;
            let dc_high = pin("PinDC") > 2.0;
            let sck_high = pin("PinCK") > 2.0;
            let di_high = pin("PinDI") > 2.0;
            let rs_high = pin("PinRS") > 2.0;
            tft.step(cs_high, dc_high, sck_high, di_high, rs_high);
            Drive::QUIET
        }
        Kind::Pcf8833Display { pcf, .. } => {
            let cs_high = pin("PinCS") > 2.0;
            let clk_high = pin("PinCLK") > 2.0;
            let data_high = pin("PinDATA") > 2.0;
            let rst_high = pin("PinRESET") > 2.0;
            pcf.step(cs_high, clk_high, data_high, rst_high);
            Drive::QUIET
        }
        _ => Drive::QUIET,
    }
}

pub(super) fn on_event(
    kind: &mut Kind,
    id: &str,
    v: impl Fn(&str) -> f64,
    t: u64,
) -> Option<Drive> {
    match kind {
        Kind::Ws2812 {
            rst_time_ns, fb, ..
        } => {
            fb.on_reset(t, *rst_time_ns);
            Some(Drive::QUIET)
        }
        Kind::Sr04 {
            echo_high, echo, ..
        } => Some(echo.on_event(echo_high, t)),
        Kind::Dht22 {
            out_state,
            pin_driven,
            link,
            ..
        } => Some(link.on_event(out_state, pin_driven, t)),
        Kind::Ds18b20 { dq_low, wire, .. } => Some(wire.on_event(dq_low, t)),
        Kind::Ds1307 {
            sqw_freq,
            sqw_enabled,
            sqw_state,
            bus,
            ..
        } => Some(bus.on_sqw(*sqw_freq, *sqw_enabled, sqw_state, t)),
        Kind::Esp01 {
            baud_rate, uart, ..
        } => {
            let rx_high = v(&format!("{id}-pin1")) > 2.0;
            Some(uart.on_event(*baud_rate, rx_high, t))
        }
        Kind::SerialPort {
            baud_rate, uart, ..
        } => {
            let rx_high = v(&format!("{id}-rx")) > 2.0;
            Some(uart.on_event(id, *baud_rate, rx_high, t))
        }
        Kind::SerialTerm {
            baud_rate, uart, ..
        } => {
            let rx_high = v(&format!("{id}-rx")) > 2.0;
            Some(uart.on_event(id, *baud_rate, rx_high, t))
        }
        _ => None,
    }
}
