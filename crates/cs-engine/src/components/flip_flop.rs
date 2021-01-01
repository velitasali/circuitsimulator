//! FlipFlop: D, JK, RS, or T flip-flop.

use super::component::PropGroup;
use super::drawable::Drawable;
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_string};
use super::{CompPin, Component, Stampable};
use crate::canvas::Pin;
use crate::canvas::PinDirection;
use crate::canvas::Point;
use crate::canvas::Rect;
use crate::canvas::draw::{Align, Draw, PaintCtx};
use crate::digital::{FlipFlopKind, FlipFlopState, Trigger};
use crate::elements::Kind;
use crate::matrix::CircMatrix;
use crate::theme::{COMPONENT_BORDER_WIDTH, COMPONENT_FILL_ALPHA};

#[derive(Clone, Debug, PartialEq)]
pub struct FlipFlop {
    pub ff_kind: FlipFlopKind,
    pub use_rs: bool,
    pub trigger: Trigger,
    pub reset_inverted: bool,
    pub clock_inverted: bool,
}

impl crate::canvas::Item {
    pub fn flipflop(
        id: impl Into<String>,
        x: f64,
        y: f64,
        kind: &str,
        use_rs: bool,
        trigger: &str,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            FlipFlop {
                ff_kind: FlipFlopKind::from_str_name(kind),
                use_rs,
                trigger: Trigger::from_str_name(trigger),
                reset_inverted: true,
                clock_inverted: false,
            },
        )
    }
}

impl Default for FlipFlop {
    fn default() -> Self {
        Self {
            ff_kind: FlipFlopKind::D,
            use_rs: false,
            trigger: Trigger::Clock,
            reset_inverted: false,
            clock_inverted: false,
        }
    }
}

impl FlipFlop {
    pub const TYPE_ID: &'static str = "FlipFlop";
    pub fn to_element_kind(&self) -> Kind {
        let mut f = match self.ff_kind {
            FlipFlopKind::Jk => FlipFlopState::jk(""),
            FlipFlopKind::Rs => FlipFlopState::rs(""),
            FlipFlopKind::T => FlipFlopState::t(""),
            FlipFlopKind::D => FlipFlopState::d(""),
        };
        f.use_rs = self.use_rs;
        f.clocked.trigger = self.trigger;
        f.rst.set_inverted(self.reset_inverted);
        f.clk.set_inverted(self.clock_inverted);
        Kind::FlipFlop(f)
    }

    fn get_ff_kind(&self) -> PropValue {
        PropValue::String(self.ff_kind.as_str().to_string())
    }
    fn set_ff_kind(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Kind", v)?;
        self.ff_kind = FlipFlopKind::from_str_name(&s);
        Ok(())
    }
    fn get_use_rs(&self) -> PropValue {
        PropValue::Bool(self.use_rs)
    }
    fn set_use_rs(&mut self, v: PropValue) -> Result<(), PropError> {
        self.use_rs = expect_bool("UseRS", v)?;
        Ok(())
    }
    fn get_trigger(&self) -> PropValue {
        PropValue::String(self.trigger.as_str().to_string())
    }
    fn set_trigger(&mut self, v: PropValue) -> Result<(), PropError> {
        let s = expect_string("Trigger", v)?;
        self.trigger = Trigger::from_str_name(&s);
        Ok(())
    }
    fn get_reset_inverted(&self) -> PropValue {
        PropValue::Bool(self.reset_inverted)
    }
    fn set_reset_inverted(&mut self, v: PropValue) -> Result<(), PropError> {
        self.reset_inverted = expect_bool("ResetInverted", v)?;
        Ok(())
    }
    fn get_clock_inverted(&self) -> PropValue {
        PropValue::Bool(self.clock_inverted)
    }
    fn set_clock_inverted(&mut self, v: PropValue) -> Result<(), PropError> {
        self.clock_inverted = expect_bool("ClockInverted", v)?;
        Ok(())
    }
}

impl Component for FlipFlop {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Flip-Flop (D, JK, RS, or T)."
    }

    fn props() -> &'static [PropDef<Self>] {
        const KIND_OPTIONS: &[&str] = &["D", "JK", "RS", "T"];
        const TRIGGER_OPTIONS: &[&str] = &["None", "Clock", "Enable"];
        const KIND: PropDef<FlipFlop> = {
            let mut p = PropDef::enumeration(
                "Kind",
                "Type",
                KIND_OPTIONS,
                FlipFlop::get_ff_kind,
                FlipFlop::set_ff_kind,
            )
            .with_info("Architecture type (e.g. D, JK, RS, or T).");
            p.structural = true;
            p
        };
        const USE_RS: PropDef<FlipFlop> = {
            let mut p = PropDef::bool(
                "UseRS",
                "Use Set/Reset",
                FlipFlop::get_use_rs,
                FlipFlop::set_use_rs,
            )
            .with_info("Shows/hides the Set and Reset pins.");
            p.structural = true;
            p
        };
        static PROPS: &[PropDef<FlipFlop>] = &[
            KIND,
            USE_RS,
            PropDef::enumeration(
                "Trigger",
                "Trigger",
                TRIGGER_OPTIONS,
                FlipFlop::get_trigger,
                FlipFlop::set_trigger,
            )
            .with_info(
                "\"Clock\" triggers every active edge.\n\"Enable\" any change during active state.\n\"None\" hides Clock pin.",
            ),
            PropDef::bool(
                "ResetInverted",
                "Reset Inverted",
                FlipFlop::get_reset_inverted,
                FlipFlop::set_reset_inverted,
            )
            .with_info("Invert the asynchronous Reset pin input."),
            PropDef::bool(
                "ClockInverted",
                "Clock Inverted",
                FlipFlop::get_clock_inverted,
                FlipFlop::set_clock_inverted,
            )
            .with_info("Inverted clock trigger (trigger on falling edge instead of rising edge)."),
        ];
        PROPS
    }

    fn prop_groups(&self) -> Vec<PropGroup> {
        let mut rows = self.prop_rows();
        for r in &mut rows {
            match r.name {
                "ResetInverted" => r.visible = self.use_rs,
                "ClockInverted" => r.visible = self.trigger != Trigger::None,
                _ => {}
            }
        }
        super::group_rows_by(
            rows,
            &[
                ("Main", &["Kind", "UseRS", "Trigger"]),
                ("Inputs", &["ResetInverted", "ClockInverted"]),
            ],
        )
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        match self.ff_kind {
            FlipFlopKind::Jk => flipflop_jk_pins("", self.use_rs)
                .into_iter()
                .map(CompPin::from)
                .collect(),
            FlipFlopKind::Rs => flipflop_rs_pins("")
                .into_iter()
                .map(CompPin::from)
                .collect(),
            FlipFlopKind::T => flipflop_t_pins("", self.use_rs)
                .into_iter()
                .map(CompPin::from)
                .collect(),
            FlipFlopKind::D => flipflop_d_pins("", self.use_rs)
                .into_iter()
                .map(CompPin::from)
                .collect(),
        }
    }

    fn body(&self) -> Rect {
        Rect::new(-16.0, -16.0, 32.0, 32.0)
    }
}

impl Stampable for FlipFlop {
    fn stamp(&self, _matrix: &mut CircMatrix, _pin_nodes: &[usize], _dt: f64) {}
}

impl Drawable for FlipFlop {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        d.fill_round_rect(
            -16.0,
            -16.0,
            32.0,
            32.0,
            2.0,
            ctx.pal.body.fade(COMPONENT_FILL_ALPHA),
        );
        d.stroke_round_rect(
            -16.0,
            -16.0,
            32.0,
            32.0,
            2.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.line(
            -16.0,
            -4.0,
            -12.0,
            0.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.line(
            -12.0,
            0.0,
            -16.0,
            4.0,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.text(
            0.0,
            0.0,
            self.ff_kind.as_str(),
            8.0,
            ctx.pal.border,
            Align::Center,
        );
        true
    }
}

impl crate::canvas::Scene {
    pub fn add_flipflop_d(&mut self, x: f64, y: f64) -> String {
        let id = format!("FlipFlopD-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::flipflop(&id, x, y, "D", false, "pos"));
        id
    }

    pub fn add_flipflop_jk(&mut self, x: f64, y: f64) -> String {
        let id = format!("FlipFlopJK-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::flipflop(&id, x, y, "JK", false, "pos"));
        id
    }

    pub fn add_flipflop_rs(&mut self, x: f64, y: f64) -> String {
        let id = format!("FlipFlopRS-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::flipflop(&id, x, y, "RS", false, "pos"));
        id
    }

    pub fn add_flipflop_t(&mut self, x: f64, y: f64) -> String {
        let id = format!("FlipFlopT-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::flipflop(&id, x, y, "T", false, "pos"));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_flip_flop() {
        let ff = FlipFlop::default();
        assert_eq!(ff.type_id(), "FlipFlop");
        assert_eq!(ff.ff_kind, FlipFlopKind::D);
        assert!(!ff.use_rs);
        assert_eq!(ff.trigger, Trigger::Clock);
        assert_eq!(ff.pin_geoms().len(), 4);
    }

    #[test]
    fn flip_flop_with_rs() {
        let mut ff = FlipFlop::default();
        ff.use_rs = true;
        assert_eq!(ff.pin_geoms().len(), 6);
    }
}

fn flipflop_d_pins(id: &str, use_rs: bool) -> Vec<Pin> {
    let mut pins = vec![
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in0"),
            item_id: id.to_string(),
            local: Point::new(-24.0, -8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "D".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in3"),
            item_id: id.to_string(),
            local: Point::new(-24.0, 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: ">".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::Out),
            id: format!("{id}-out0"),
            item_id: id.to_string(),
            local: Point::new(24.0, -8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "Q".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::Out),
            id: format!("{id}-out1"),
            item_id: id.to_string(),
            local: Point::new(24.0, 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "!Q".into(),
            unused: false,
        },
    ];
    if use_rs {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in1"),
            item_id: id.to_string(),
            local: Point::new(0.0, -24.0),
            angle: 90,
            length: 8.0,
            is_bus: false,
            label: "S".into(),
            unused: false,
        });
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in2"),
            item_id: id.to_string(),
            local: Point::new(0.0, 24.0),
            angle: 270,
            length: 8.0,
            is_bus: false,
            label: "R".into(),
            unused: false,
        });
    }
    pins
}

fn flipflop_jk_pins(id: &str, use_rs: bool) -> Vec<Pin> {
    let mut pins = vec![
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in0"),
            item_id: id.to_string(),
            local: Point::new(-24.0, -8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "J".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in4"),
            item_id: id.to_string(),
            local: Point::new(-24.0, 0.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: ">".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in1"),
            item_id: id.to_string(),
            local: Point::new(-24.0, 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "K".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::Out),
            id: format!("{id}-out0"),
            item_id: id.to_string(),
            local: Point::new(24.0, -8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "Q".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::Out),
            id: format!("{id}-out1"),
            item_id: id.to_string(),
            local: Point::new(24.0, 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "!Q".into(),
            unused: false,
        },
    ];
    if use_rs {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in2"),
            item_id: id.to_string(),
            local: Point::new(0.0, -24.0),
            angle: 90,
            length: 8.0,
            is_bus: false,
            label: "S".into(),
            unused: false,
        });
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in3"),
            item_id: id.to_string(),
            local: Point::new(0.0, 24.0),
            angle: 270,
            length: 8.0,
            is_bus: false,
            label: "R".into(),
            unused: false,
        });
    }
    pins
}

fn flipflop_rs_pins(id: &str) -> Vec<Pin> {
    vec![
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in0"),
            item_id: id.to_string(),
            local: Point::new(-24.0, -8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "S".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in2"),
            item_id: id.to_string(),
            local: Point::new(-24.0, 0.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: ">".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in1"),
            item_id: id.to_string(),
            local: Point::new(-24.0, 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "R".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::Out),
            id: format!("{id}-out0"),
            item_id: id.to_string(),
            local: Point::new(24.0, -8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "Q".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::Out),
            id: format!("{id}-out1"),
            item_id: id.to_string(),
            local: Point::new(24.0, 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "!Q".into(),
            unused: false,
        },
    ]
}

fn flipflop_t_pins(id: &str, use_rs: bool) -> Vec<Pin> {
    let mut pins = vec![
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in0"),
            item_id: id.to_string(),
            local: Point::new(-24.0, -8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: "T".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in3"),
            item_id: id.to_string(),
            local: Point::new(-24.0, 8.0),
            angle: 180,
            length: 8.0,
            is_bus: false,
            label: ">".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::Out),
            id: format!("{id}-out0"),
            item_id: id.to_string(),
            local: Point::new(24.0, -8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "Q".into(),
            unused: false,
        },
        Pin {
            direction: Some(PinDirection::Out),
            id: format!("{id}-out1"),
            item_id: id.to_string(),
            local: Point::new(24.0, 8.0),
            angle: 0,
            length: 8.0,
            is_bus: false,
            label: "!Q".into(),
            unused: false,
        },
    ];
    if use_rs {
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in1"),
            item_id: id.to_string(),
            local: Point::new(0.0, -24.0),
            angle: 90,
            length: 8.0,
            is_bus: false,
            label: "S".into(),
            unused: false,
        });
        pins.push(Pin {
            direction: Some(PinDirection::In),
            id: format!("{id}-in2"),
            item_id: id.to_string(),
            local: Point::new(0.0, 24.0),
            angle: 270,
            length: 8.0,
            is_bus: false,
            label: "R".into(),
            unused: false,
        });
    }
    pins
}
