//! Built-in circuits the File menu can open without a path on disk.
//!
//! Loading one of these leaves the canvas unsaved. The strings are ordinary
//! `.sim1` documents.

/// One catalog circuit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Example {
    pub id: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub sim1: &'static str,
}

const DIVIDER: &str = r#"<circuit version="1.0.0" rev="0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" animate="0" anicurr="0" ansi="0" width="4000" height="4000" >
<item itemtype="FixedVolt" CircId="Fixed Voltage-1" Voltage="5 V" Pos="100,-100" />
<item itemtype="Resistor" CircId="Resistor-2" Resistance="1 kΩ" Pos="160,-100" />
<item itemtype="Resistor" CircId="Resistor-3" Resistance="1 kΩ" Pos="240,-100" />
<item itemtype="Ground" CircId="Ground-4" Pos="320,-100" />
<item itemtype="Node" CircId="Node-5" Pos="200,-100" />
<item itemtype="Connector" CircId="connector-1" startpinid="Fixed Voltage-1-outnod" endpinid="Resistor-2-lPin" pointList="100,-100,148,-100" />
<item itemtype="Connector" CircId="connector-2" startpinid="Resistor-2-rPin" endpinid="Node-5-0" pointList="180,-100,200,-100" />
<item itemtype="Connector" CircId="connector-3" startpinid="Resistor-3-lPin" endpinid="Node-5-1" pointList="232,-100,200,-100" />
<item itemtype="Connector" CircId="connector-4" startpinid="Resistor-3-rPin" endpinid="Ground-4-Gnd" pointList="264,-100,300,-100" />
</circuit>
"#;

const RC_LOWPASS: &str = r#"<circuit version="1.0.0" rev="0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" animate="0" anicurr="0" ansi="0" width="4000" height="4000" >
<item itemtype="FixedVolt" CircId="V1" Voltage="5 V" Pos="80,-80" />
<item itemtype="Resistor" CircId="R1" Resistance="1 kΩ" Pos="180,-80" />
<item itemtype="Capacitor" CircId="C1" Capacitance="1 µF" Pos="300,-80" />
<item itemtype="Ground" CircId="GND" Pos="400,-80" />
<item itemtype="Connector" CircId="connector-1" startpinid="V1-outnod" endpinid="R1-lPin" pointList="80,-80,160,-80" />
<item itemtype="Connector" CircId="connector-2" startpinid="R1-rPin" endpinid="C1-lPin" pointList="220,-80,280,-80" />
<item itemtype="Connector" CircId="connector-3" startpinid="C1-rPin" endpinid="GND-Gnd" pointList="340,-80,400,-80" />
</circuit>
"#;

const OPAMP_BUFFER: &str = r#"<circuit version="1.0.0" rev="0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" animate="0" anicurr="0" ansi="0" width="4000" height="4000" >
<item itemtype="FixedVolt" CircId="V1" Voltage="2.5 V" Pos="80,-80" />
<item itemtype="opAmp" CircId="U1" Pos="200,-80" />
<item itemtype="Connector" CircId="c1" startpinid="V1-outnod" endpinid="U1-inputNinv" pointList="80,-80,180,-70" />
<item itemtype="Connector" CircId="c2" startpinid="U1-output" endpinid="U1-inputInv" pointList="220,-80,220,-110,180,-110,180,-90" />
</circuit>
"#;

const DIGITAL_AND: &str = r#"<circuit version="1.0.0" rev="0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" animate="0" anicurr="0" ansi="0" width="4000" height="4000" >
<item itemtype="FixedVolt" CircId="V1" Voltage="5 V" Pos="60,-100" />
<item itemtype="Clock" CircId="CLK1" Voltage="5 V" Frequency="100 Hz" Pos="60,-40" />
<item itemtype="And Gate" CircId="AND1" Num_Inputs="2" Pos="160,-70" />
<item itemtype="Resistor" CircId="R1" Resistance="330 Ω" Pos="240,-70" />
<item itemtype="Led" CircId="LED1" Pos="320,-70" />
<item itemtype="Ground" CircId="GND" Pos="380,-70" />
<item itemtype="Connector" CircId="c1" startpinid="V1-outnod" endpinid="AND1-in0" pointList="60,-100,160,-80" />
<item itemtype="Connector" CircId="c2" startpinid="CLK1-outnod" endpinid="AND1-in1" pointList="60,-40,160,-60" />
<item itemtype="Connector" CircId="c3" startpinid="AND1-out" endpinid="R1-lPin" pointList="180,-70,220,-70" />
<item itemtype="Connector" CircId="c4" startpinid="R1-rPin" endpinid="LED1-lPin" pointList="260,-70,300,-70" />
<item itemtype="Connector" CircId="c5" startpinid="LED1-rPin" endpinid="GND-Gnd" pointList="340,-70,380,-70" />
</circuit>
"#;

const EXAMPLES: &[Example] = &[
    Example {
        id: "voltage-divider",
        title: "Voltage divider",
        summary: "Two equal resistors split a 5 V source. The middle node is 2.5 V.",
        sim1: DIVIDER,
    },
    Example {
        id: "rc-lowpass",
        title: "RC low-pass",
        summary: "1 kΩ in series with 1 µF to ground. The corner is about 159 Hz.",
        sim1: RC_LOWPASS,
    },
    Example {
        id: "opamp-buffer",
        title: "Op-Amp buffer",
        summary: "Unity-gain buffer (voltage follower) isolating a 2.5 V source.",
        sim1: OPAMP_BUFFER,
    },
    Example {
        id: "digital-and",
        title: "Digital AND gate & LED",
        summary: "Clock and DC source driving a 2-input AND gate to blink an LED through 330 Ω.",
        sim1: DIGITAL_AND,
    },
];

/// Every built-in circuit, in menu order.
pub fn all() -> &'static [Example] {
    EXAMPLES
}

/// Look up a circuit by its stable id.
pub fn get(id: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|e| e.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Circuit;
    use crate::canvas::Scene;

    #[test]
    fn each_example_loads_and_solves() {
        assert!(get("missing").is_none());
        assert_eq!(all().len(), 4);
        for ex in all() {
            let mut circuit = Circuit::from_sim1(ex.sim1).expect(ex.id);
            circuit.solve().expect(ex.id);
            let scene = Scene::from_sim1(ex.sim1).expect(ex.id);
            assert!(!scene.items().is_empty(), "{}", ex.id);
        }
        let div = get("voltage-divider").unwrap();
        let mut circuit = Circuit::from_sim1(div.sim1).unwrap();
        circuit.solve().unwrap();
        let mid = circuit.pin_voltage("Node-5-0").unwrap();
        assert!((mid - 2.5).abs() < 1e-6, "{mid}");

        let buf = get("opamp-buffer").unwrap();
        let mut circuit = Circuit::from_sim1(buf.sim1).unwrap();
        circuit.solve().unwrap();
        let vo = circuit.pin_voltage("U1-output").unwrap();
        assert!((vo - 2.5).abs() < 0.05, "{vo}");
    }
}
