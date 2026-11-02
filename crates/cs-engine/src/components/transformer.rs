//! Coupled inductor transformer component.

use super::component::{stamp_conductance_between, stamp_directed};
use super::props::{PropDef, PropError, PropValue, expect_bool, expect_float};
use super::{CompPin, Component, Stampable};
use crate::canvas::Rect;
use crate::elements::{Kind, TransformerState};
use crate::matrix::CircMatrix;

const MIN_L: f64 = 1e-12;
const MAX_L: f64 = 1e6;
const MIN_R: f64 = 1e-6;
const MAX_R: f64 = 1e12;

impl crate::canvas::Item {
    pub fn transformer(
        id: impl Into<String>,
        x: f64,
        y: f64,
        ind1: f64,
        ind2: f64,
        coupling: f64,
    ) -> Self {
        Self::transformer_with(id, x, y, ind1, ind2, coupling, 0.1, 0.1)
    }

    pub fn transformer_with(
        id: impl Into<String>,
        x: f64,
        y: f64,
        inductance1: f64,
        inductance2: f64,
        coupling: f64,
        r_coil1: f64,
        r_coil2: f64,
    ) -> Self {
        Self::new(
            id,
            x,
            y,
            Transformer {
                inductance1,
                inductance2,
                coupling,
                r_coil1,
                r_coil2,
                r_core: 1e12,
                i_sat: 0.0,
                hysteresis: 0.0,
                center_tap: false,
                aux_secondary: false,
                inductance3: 1.0,
                r_coil3: 0.1,
            },
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Transformer {
    pub inductance1: f64,
    pub inductance2: f64,
    pub coupling: f64,
    pub r_coil1: f64,
    pub r_coil2: f64,
    pub r_core: f64,
    pub i_sat: f64,
    pub hysteresis: f64,
    pub center_tap: bool,
    pub aux_secondary: bool,
    pub inductance3: f64,
    pub r_coil3: f64,
}

impl Default for Transformer {
    fn default() -> Self {
        Self {
            inductance1: 1.0,
            inductance2: 1.0,
            coupling: 0.99,
            r_coil1: 0.1,
            r_coil2: 0.1,
            r_core: 1e12,
            i_sat: 0.0,
            hysteresis: 0.0,
            center_tap: false,
            aux_secondary: false,
            inductance3: 1.0,
            r_coil3: 0.1,
        }
    }
}

impl Transformer {
    pub const TYPE_ID: &'static str = "Transformer";

    pub fn to_state(&self) -> TransformerState {
        TransformerState {
            inductance1: self.inductance1,
            inductance2: self.inductance2,
            coupling: self.coupling,
            r_coil1: self.r_coil1,
            r_coil2: self.r_coil2,
            r_core: self.r_core,
            i_sat: self.i_sat,
            hysteresis: self.hysteresis,
            center_tap: self.center_tap,
            aux_secondary: self.aux_secondary,
            inductance3: self.inductance3,
            r_coil3: self.r_coil3,
            ..Default::default()
        }
    }

    pub fn to_element_kind(&self) -> Kind {
        Kind::Transformer {
            state: self.to_state(),
        }
    }

    fn get_inductance1(&self) -> PropValue {
        PropValue::Float(self.inductance1)
    }
    fn set_inductance1(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inductance1 = expect_float("Inductance1", v)?.clamp(MIN_L, MAX_L);
        Ok(())
    }

    fn get_inductance2(&self) -> PropValue {
        PropValue::Float(self.inductance2)
    }
    fn set_inductance2(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inductance2 = expect_float("Inductance2", v)?.clamp(MIN_L, MAX_L);
        Ok(())
    }

    fn get_coupling(&self) -> PropValue {
        PropValue::Float(self.coupling)
    }
    fn set_coupling(&mut self, v: PropValue) -> Result<(), PropError> {
        self.coupling = expect_float("Coupling", v)?.clamp(0.0, 1.0);
        Ok(())
    }

    fn get_r_coil1(&self) -> PropValue {
        PropValue::Float(self.r_coil1)
    }
    fn set_r_coil1(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_coil1 = expect_float("Rcoil1", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }

    fn get_r_coil2(&self) -> PropValue {
        PropValue::Float(self.r_coil2)
    }
    fn set_r_coil2(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_coil2 = expect_float("Rcoil2", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }

    fn get_r_core(&self) -> PropValue {
        PropValue::Float(self.r_core)
    }
    fn set_r_core(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_core = expect_float("Rcore", v)?.clamp(0.0, 1e12);
        Ok(())
    }

    fn get_i_sat(&self) -> PropValue {
        PropValue::Float(self.i_sat)
    }
    fn set_i_sat(&mut self, v: PropValue) -> Result<(), PropError> {
        self.i_sat = expect_float("Isat", v)?.clamp(0.0, 1e4);
        Ok(())
    }

    fn get_hysteresis(&self) -> PropValue {
        PropValue::Float(self.hysteresis)
    }
    fn set_hysteresis(&mut self, v: PropValue) -> Result<(), PropError> {
        self.hysteresis = expect_float("Hysteresis", v)?.clamp(0.0, 1e3);
        Ok(())
    }

    fn get_center_tap(&self) -> PropValue {
        PropValue::Bool(self.center_tap)
    }
    fn set_center_tap(&mut self, v: PropValue) -> Result<(), PropError> {
        self.center_tap = expect_bool("CenterTap", v)?;
        Ok(())
    }

    fn get_aux_secondary(&self) -> PropValue {
        PropValue::Bool(self.aux_secondary)
    }
    fn set_aux_secondary(&mut self, v: PropValue) -> Result<(), PropError> {
        self.aux_secondary = expect_bool("AuxSecondary", v)?;
        Ok(())
    }

    fn get_inductance3(&self) -> PropValue {
        PropValue::Float(self.inductance3)
    }
    fn set_inductance3(&mut self, v: PropValue) -> Result<(), PropError> {
        self.inductance3 = expect_float("Inductance3", v)?.clamp(MIN_L, MAX_L);
        Ok(())
    }

    fn get_r_coil3(&self) -> PropValue {
        PropValue::Float(self.r_coil3)
    }
    fn set_r_coil3(&mut self, v: PropValue) -> Result<(), PropError> {
        self.r_coil3 = expect_float("Rcoil3", v)?.clamp(MIN_R, MAX_R);
        Ok(())
    }
}

impl Component for Transformer {
    fn type_id(&self) -> &'static str {
        Self::TYPE_ID
    }

    fn description(&self) -> &'static str {
        "Coupled inductor transformer."
    }

    fn props() -> &'static [PropDef<Self>] {
        static PROPS: &[PropDef<Transformer>] = &[
            PropDef::float(
                "Inductance1",
                "Primary Inductance",
                "H",
                MIN_L,
                MAX_L,
                Transformer::get_inductance1,
                Transformer::set_inductance1,
            )
            .with_info("Primary winding inductance in Henrys."),
            PropDef::float(
                "Inductance2",
                "Secondary Inductance",
                "H",
                MIN_L,
                MAX_L,
                Transformer::get_inductance2,
                Transformer::set_inductance2,
            )
            .with_info("Secondary winding inductance in Henrys."),
            PropDef::float(
                "Coupling",
                "Coupling Factor",
                "",
                0.0,
                1.0,
                Transformer::get_coupling,
                Transformer::set_coupling,
            )
            .with_info("Magnetic coupling coefficient (k) between windings (0.0 to 1.0)."),
            PropDef::float(
                "Rcoil1",
                "Primary Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                Transformer::get_r_coil1,
                Transformer::set_r_coil1,
            )
            .with_info("Primary winding series winding resistance in Ohms."),
            PropDef::float(
                "Rcoil2",
                "Secondary Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                Transformer::get_r_coil2,
                Transformer::set_r_coil2,
            )
            .with_info("Secondary winding series winding resistance in Ohms."),
            PropDef::float(
                "Rcore",
                "Core Resistance",
                "Ω",
                0.0,
                1e12,
                Transformer::get_r_core,
                Transformer::set_r_core,
            )
            .with_info("Core eddy current loss resistance in Ohms (0 or high = lossless)."),
            PropDef::float(
                "Isat",
                "Saturation Current",
                "A",
                0.0,
                1e4,
                Transformer::get_i_sat,
                Transformer::set_i_sat,
            )
            .with_info(
                "Ferromagnetic core saturation threshold current in Amps (0.0 = linear core).",
            ),
            PropDef::float(
                "Hysteresis",
                "Hysteresis Width",
                "A",
                0.0,
                1e3,
                Transformer::get_hysteresis,
                Transformer::set_hysteresis,
            )
            .with_info("Magnetic hysteresis coercive current width in Amps (0.0 = no hysteresis)."),
            PropDef::bool(
                "CenterTap",
                "Center Tap Secondary",
                Transformer::get_center_tap,
                Transformer::set_center_tap,
            )
            .with_info("Enables center-tapped secondary winding (adds pin ct)."),
            PropDef::bool(
                "AuxSecondary",
                "Auxiliary Secondary",
                Transformer::get_aux_secondary,
                Transformer::set_aux_secondary,
            )
            .with_info("Enables third isolated auxiliary secondary winding (adds pins a1, a2)."),
            PropDef::float(
                "Inductance3",
                "Auxiliary Inductance",
                "H",
                MIN_L,
                MAX_L,
                Transformer::get_inductance3,
                Transformer::set_inductance3,
            )
            .with_info("Auxiliary secondary winding inductance in Henrys."),
            PropDef::float(
                "Rcoil3",
                "Auxiliary Resistance",
                "Ω",
                MIN_R,
                MAX_R,
                Transformer::get_r_coil3,
                Transformer::set_r_coil3,
            )
            .with_info("Auxiliary secondary winding series resistance in Ohms."),
        ];
        PROPS
    }

    fn pin_geoms(&self) -> Vec<CompPin> {
        let mut pins = Vec::with_capacity(7);
        let p2_y = if self.aux_secondary { 56.0 } else { 24.0 };
        pins.push(CompPin::new("-p1", -16.0, 0.0, 180, 6.0));
        pins.push(CompPin::new("-p2", -16.0, p2_y, 180, 6.0));
        pins.push(CompPin::new("-s1", 16.0, 0.0, 0, 6.0));
        if self.center_tap {
            pins.push(CompPin::new("-ct", 16.0, 12.0, 0, 2.0));
        }
        pins.push(CompPin::new("-s2", 16.0, 24.0, 0, 6.0));
        if self.aux_secondary {
            pins.push(CompPin::new("-a1", 16.0, 32.0, 0, 6.0));
            pins.push(CompPin::new("-a2", 16.0, 56.0, 0, 6.0));
        }
        pins
    }

    fn body(&self) -> Rect {
        if self.aux_secondary {
            Rect::new(-12.0, 0.0, 24.0, 56.0)
        } else {
            Rect::new(-12.0, 0.0, 24.0, 24.0)
        }
    }
}

impl Stampable for Transformer {
    fn stamp(&self, matrix: &mut CircMatrix, pin_nodes: &[usize], dt: f64) {
        let state = self.to_state();
        let comp = state.companion(dt);
        let dim = comp.dim;

        let pairs: [(usize, usize); 4] = match dim {
            2 => [(0, 1), (2, 3), (0, 0), (0, 0)],
            3 if self.center_tap => [(0, 1), (2, 3), (3, 4), (0, 0)],
            3 => [(0, 1), (2, 3), (4, 5), (0, 0)],
            4 => [(0, 1), (2, 3), (3, 4), (5, 6)],
            _ => return,
        };

        for w in 0..dim {
            let (p_a, p_b) = pairs[w];
            if p_a < pin_nodes.len() && p_b < pin_nodes.len() {
                stamp_conductance_between(matrix, pin_nodes, p_a, p_b, comp.g[w][w]);
            }
        }

        for j in 0..dim {
            for col in (j + 1)..dim {
                let (a1, a2) = pairs[j];
                let (b1, b2) = pairs[col];
                let g = comp.g[j][col];
                if g != 0.0
                    && a1 < pin_nodes.len()
                    && a2 < pin_nodes.len()
                    && b1 < pin_nodes.len()
                    && b2 < pin_nodes.len()
                {
                    stamp_directed(matrix, pin_nodes, a1, b1, -g);
                    stamp_directed(matrix, pin_nodes, a1, b2, g);
                    stamp_directed(matrix, pin_nodes, a2, b1, g);
                    stamp_directed(matrix, pin_nodes, a2, b2, -g);

                    stamp_directed(matrix, pin_nodes, b1, a1, -g);
                    stamp_directed(matrix, pin_nodes, b1, a2, g);
                    stamp_directed(matrix, pin_nodes, b2, a1, g);
                    stamp_directed(matrix, pin_nodes, b2, a2, -g);
                }
            }
        }
    }
}

use super::Drawable;
use crate::canvas::draw::{Draw, PaintCtx};
use crate::theme::COMPONENT_BORDER_WIDTH;

impl Drawable for Transformer {
    fn paint(&self, d: &mut dyn Draw, ctx: &PaintCtx) -> bool {
        let bottom_y = if self.aux_secondary { 56.0 } else { 24.0 };
        // Ferromagnetic core lines
        d.line(
            -2.0,
            0.0,
            -2.0,
            bottom_y,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );
        d.line(
            2.0,
            0.0,
            2.0,
            bottom_y,
            ctx.pal.border,
            COMPONENT_BORDER_WIDTH,
        );

        // Primary coil arcs
        let primary_arcs: &[f64] = if self.aux_secondary {
            &[4.0, 12.0, 20.0, 28.0, 36.0, 44.0, 52.0]
        } else {
            &[4.0, 12.0, 20.0]
        };
        for &y in primary_arcs {
            d.arc(
                -10.0,
                y,
                4.0,
                -std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
        }

        // Secondary coil arcs
        for y in [4.0, 12.0, 20.0] {
            d.arc(
                10.0,
                y,
                4.0,
                3.0 * std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
        }

        // Polarity dots
        d.fill_circle(-12.0, 4.0, 1.4, ctx.pal.border);
        d.fill_circle(12.0, 4.0, 1.4, ctx.pal.border);

        // Center tap connector
        if self.center_tap {
            d.line(
                10.0,
                12.0,
                14.0,
                12.0,
                ctx.pal.border,
                COMPONENT_BORDER_WIDTH,
            );
            d.fill_circle(14.0, 12.0, 1.2, ctx.pal.border);
        }

        // Auxiliary secondary coil arcs
        if self.aux_secondary {
            for y in [36.0, 44.0, 52.0] {
                d.arc(
                    10.0,
                    y,
                    4.0,
                    3.0 * std::f64::consts::FRAC_PI_2,
                    std::f64::consts::FRAC_PI_2,
                    ctx.pal.border,
                    COMPONENT_BORDER_WIDTH,
                );
            }
            d.fill_circle(12.0, 36.0, 1.4, ctx.pal.border);
        }

        true
    }
}

impl crate::canvas::Scene {
    pub fn add_transformer(&mut self, x: f64, y: f64) -> String {
        let id = format!("Transformer-{}", self.items.len() + 1);
        self.items
            .push(crate::canvas::Item::new(&id, x, y, Transformer::default()));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Circuit;
    use crate::elements::Comp;

    #[test]
    fn default_transformer() {
        let t = Transformer::default();
        assert_eq!(t.type_id(), "Transformer");
        assert_eq!(t.inductance1, 1.0);
        assert_eq!(t.inductance2, 1.0);
        assert_eq!(t.coupling, 0.99);
        assert_eq!(t.r_coil1, 0.1);
        assert_eq!(t.r_coil2, 0.1);
        let pins = t.pin_geoms();
        assert_eq!(pins.len(), 4);
        assert_eq!(t.body(), Rect::new(-12.0, 0.0, 24.0, 24.0));
        assert_eq!(pins[0].local.x, -16.0);
        assert_eq!(pins[0].local.y, 0.0);
        assert_eq!(pins[1].local.x, -16.0);
        assert_eq!(pins[1].local.y, 24.0);
        assert_eq!(pins[2].local.x, 16.0);
        assert_eq!(pins[2].local.y, 0.0);
        assert_eq!(pins[3].local.x, 16.0);
        assert_eq!(pins[3].local.y, 24.0);
    }

    #[test]
    fn transformer_ac_coupling_and_turns_ratio() {
        // 1:2 step-up turns ratio (L1 = 1 H, L2 = 4 H, k = 0.999)
        let mut circ = Circuit::default();
        circ.dt = 1e-6;
        circ.add_comp(Comp::wave_gen(
            "wg", "Sine", 1000.0, 2.5, 0.0, 0.5, 0.0, 100, false, false,
        ));
        circ.add_comp(Comp {
            id: "tr".into(),
            kind: Kind::Transformer {
                state: TransformerState::new(1.0, 4.0, 0.999, 0.01, 0.01),
            },
        });
        circ.add_comp(Comp::resistor("rload", 10_000.0));
        circ.add_comp(Comp::ground("gnd1"));
        circ.add_comp(Comp::ground("gnd2"));

        circ.connect("wg-outnod", "tr-p1");
        circ.connect("tr-p2", "gnd1-Gnd");
        circ.connect("tr-s1", "rload-lPin");
        circ.connect("tr-s2", "rload-rPin");
        circ.connect("tr-s2", "gnd2-Gnd");

        let mut max_vp = 0.0_f64;
        let mut max_vs = 0.0_f64;
        for _ in 0..2000 {
            circ.step().unwrap();
            let vp = circ.pin_voltage("tr-p1").unwrap();
            let vs = circ.pin_voltage("tr-s1").unwrap();
            max_vp = max_vp.max(vp);
            max_vs = max_vs.max(vs);
        }
        assert!(
            (max_vp - 2.5).abs() < 0.05,
            "Primary peak should be ~2.5V, got {max_vp}"
        );
        assert!(
            (max_vs - 5.0).abs() < 0.15,
            "Secondary peak for 1:2 turns ratio should be ~5.0V, got {max_vs}"
        );
    }

    #[test]
    fn transformer_full_wave_bridge_rectifier() {
        let xml = r#"<circuit version="1.0.0" rev="0" stepSize="1000000" stepsPS="10000" NLsteps="200" reaStep="1000000" animate="1" anicurr="1" ansi="0" width="1200" height="800" >
<item itemtype="Oscope" CircId="Oscope-48" Basic_X="135" Basic_Y="135" BufferSize="600000" connectGnd="true" InputImped="10 MΩ" TestTime="0 s" DoTest="false" Tunnel1="i1" Tunnel2="i2" Tunnel3="" Tunnel4="" Pos="-32,-76" />
<item itemtype="Tunnel" CircId="Tunnel-238" Name="i2" IsBus="false" Pos="-224,80" rotation="-90" vflip="-1" />
<item itemtype="Tunnel" CircId="Tunnel-196" Name="i1" IsBus="false" Pos="-4,40" rotation="-90" />
<item itemtype="Capacitor" CircId="Capacitor-116" Capacitance="1 mF" Resistance="1 µΩ" InitVolt="0 V" Pos="16,80" rotation="90" />
<item itemtype="Diode" CircId="Diode-93" Zener="false" MaxCurrent="15 A" Resistance="50 mΩ" BrkDownV="0 V" SatCurrent="171.435 nA" EmCoef="2" Pos="-112,60" />
<item itemtype="Diode" CircId="Diode-99" Zener="false" MaxCurrent="15 A" Resistance="50 mΩ" BrkDownV="0 V" SatCurrent="171.435 nA" EmCoef="2" Pos="-48,60" />
<item itemtype="Diode" CircId="Diode-101" Zener="false" MaxCurrent="15 A" Resistance="50 mΩ" BrkDownV="0 V" SatCurrent="171.435 nA" EmCoef="2" Pos="-112,108" />
<item itemtype="Diode" CircId="Diode-102" Zener="false" MaxCurrent="15 A" Resistance="50 mΩ" BrkDownV="0 V" SatCurrent="171.435 nA" EmCoef="2" Pos="-48,108" />
<item itemtype="Voltmeter" CircId="Voltimeter-85" Rms="false" Pos="148,-8" />
<item itemtype="Ground" CircId="Ground-81" Pos="-208,128" />
<item itemtype="Ground" CircId="Ground-287" Pos="28,168" />
<item itemtype="WaveGen" CircId="WaveGen-80" WaveType="Sine" Frequency="1 kHz" Amplitude="2.5 V" Offset="0 V" Duty="50 %" Phase="0 °" Steps="100" Bipolar="false" Floating="false" File="" Pos="-264,72" />
<item itemtype="Transformer" CircId="Transformer-79" Inductance1="1 H" Inductance2="1 H" Coupling="0.99" Rcoil1="100 mΩ" Rcoil2="100 mΩ" Pos="-184,72" />
<item itemtype="Resistor" CircId="Resistor-240" Resistance="1 MΩ" ShowBands="true" Pos="44,76" rotation="90" />
<item itemtype="Ammeter" CircId="Amperimeter-280" Rms="false" Pos="-48,148" />
<item itemtype="Node" CircId="Node-197" Pos="16,40" />
<item itemtype="Node" CircId="Node-191" Pos="-24,84" />
<item itemtype="Node" CircId="Node-205" Pos="16,16" />
<item itemtype="Node" CircId="Node-271" Pos="44,16" />
<item itemtype="Node" CircId="Node-242" Pos="44,144" />
<item itemtype="Node" CircId="Node-281" Pos="16,144" />
<item itemtype="Node" CircId="Node-282" Pos="-140,84" />
<item itemtype="Node" CircId="Node-283" Pos="-84,108" />
<item itemtype="Node" CircId="Node-284" Pos="-84,60" />
<item itemtype="Node" CircId="Node-285" Pos="-224,72" />
<item itemtype="Node" CircId="Node-286" Pos="28,144" />
<item itemtype="Connector" CircId="Connector-590" startpinid="Capacitor-116-lPin" endpinid="Node-197-0" pointList="16,64,16,40" />
<item itemtype="Connector" CircId="Connector-589" startpinid="Tunnel-196-pin" endpinid="Node-197-1" pointList="-4,40,0,40,16,40" />
<item itemtype="Connector" CircId="Connector-580" startpinid="Node-191-2" endpinid="Diode-102-rPin" pointList="-24,84,-24,108,-32,108" />
<item itemtype="Connector" CircId="Connector-579" startpinid="Diode-99-rPin" endpinid="Node-191-0" pointList="-32,60,-24,60,-24,84" />
<item itemtype="Connector" CircId="Connector-632" startpinid="Node-205-2" endpinid="Node-191-1" pointList="16,16,-12,16,-12,84,-24,84" />
<item itemtype="Connector" CircId="Connector-631" startpinid="Node-197-2" endpinid="Node-205-0" pointList="16,40,16,16" />
<item itemtype="Connector" CircId="Connector-800" startpinid="Node-271-1" endpinid="Resistor-240-lPin" pointList="44,16,44,60" />
<item itemtype="Connector" CircId="Connector-802" startpinid="Node-271-2" endpinid="Node-205-1" pointList="44,16,16,16" />
<item itemtype="Connector" CircId="Connector-819" startpinid="Voltimeter-85-lPin" endpinid="Node-271-0" pointList="140,8,140,16,44,16" />
<item itemtype="Connector" CircId="Connector-820" startpinid="Node-242-2" endpinid="Voltimeter-85-rPin" pointList="44,144,156,144,156,8" />
<item itemtype="Connector" CircId="Connector-798" startpinid="Resistor-240-rPin" endpinid="Node-242-1" pointList="44,92,44,144" />
<item itemtype="Connector" CircId="Connector-824" startpinid="Amperimeter-280-rPin" endpinid="Node-281-1" pointList="-40,164,-40,168,8,168,8,144,16,144" />
<item itemtype="Connector" CircId="Connector-825" startpinid="Node-242-0" endpinid="Node-286-1" pointList="44,144,28,144" />
<item itemtype="Connector" CircId="Connector-839" startpinid="Node-286-0" endpinid="Node-281-0" pointList="28,144,16,144" />
<item itemtype="Connector" CircId="Connector-840" startpinid="Ground-287-Gnd" endpinid="Node-286-2" pointList="28,152,28,144" />
<item itemtype="Connector" CircId="Connector-827" startpinid="Amperimeter-280-lPin" endpinid="Node-282-1" pointList="-56,164,-56,168,-152,168,-152,84,-140,84" />
<item itemtype="Connector" CircId="Connector-828" startpinid="Diode-93-lPin" endpinid="Node-282-0" pointList="-128,60,-140,60,-140,84" />
<item itemtype="Connector" CircId="Connector-829" startpinid="Node-282-2" endpinid="Diode-101-lPin" pointList="-140,84,-140,108,-128,108" />
<item itemtype="Connector" CircId="Connector-830" startpinid="Capacitor-116-rPin" endpinid="Node-281-2" pointList="16,96,16,144" />
<item itemtype="Connector" CircId="Connector-298" startpinid="Diode-101-rPin" endpinid="Node-283-0" pointList="-96,108,-84,108" />
<item itemtype="Connector" CircId="Connector-831" startpinid="Transformer-79-s2" endpinid="Node-283-1" pointList="-168,96,-168,124,-84,124,-84,108" />
<item itemtype="Connector" CircId="Connector-832" startpinid="Node-283-2" endpinid="Diode-102-lPin" pointList="-84,108,-64,108" />
<item itemtype="Connector" CircId="Connector-375" startpinid="Diode-99-lPin" endpinid="Node-284-0" pointList="-64,60,-84,60" />
<item itemtype="Connector" CircId="Connector-833" startpinid="Transformer-79-s1" endpinid="Node-284-1" pointList="-168,72,-168,44,-84,44,-84,60" />
<item itemtype="Connector" CircId="Connector-834" startpinid="Node-284-2" endpinid="Diode-93-rPin" pointList="-84,60,-96,60" />
<item itemtype="Connector" CircId="Connector-738" startpinid="WaveGen-80-outnod" endpinid="Node-285-0" pointList="-248,72,-224,72" />
<item itemtype="Connector" CircId="Connector-836" startpinid="Transformer-79-p1" endpinid="Node-285-1" pointList="-200,72,-224,72" />
<item itemtype="Connector" CircId="Connector-837" startpinid="Node-285-2" endpinid="Tunnel-238-pin" pointList="-224,72,-224,80" />
<item itemtype="Connector" CircId="Connector-838" startpinid="Transformer-79-p2" endpinid="Ground-81-Gnd" pointList="-200,96,-208,96,-208,112" />
</circuit>"#;
        let mut circ = Circuit::from_sim1(xml).unwrap();
        circ.dt = 10e-6; // 10 us steps
        for _ in 0..500 {
            circ.step().unwrap();
        }
        let v_dc = circ.pin_voltage("Capacitor-116-lPin").unwrap()
            - circ.pin_voltage("Capacitor-116-rPin").unwrap();
        let v_i1 = circ.pin_voltage("Tunnel-196-pin").unwrap();
        assert!(
            v_dc > 0.9 && v_dc < 1.3,
            "Expected rectified DC voltage ~0.96V across Capacitor-116, got {v_dc}"
        );
        assert!(
            (v_dc - v_i1).abs() < 1e-3,
            "Expected Tunnel i1 voltage ({v_i1}) to match capacitor voltage ({v_dc})"
        );
    }
}
