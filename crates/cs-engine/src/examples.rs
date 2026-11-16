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

macro_rules! circuit {
    ($body:literal) => {
        concat!(
            r#"<circuit version="1.0.0" rev="0" stepSize="1000000" stepsPS="1000000" NLsteps="100000" reaStep="1000000" animate="0" anicurr="0" ansi="0" width="4000" height="4000" >"#,
            $body
        )
    };
}

const DIVIDER: &str = circuit!(
    r#"
<item itemtype="FixedVolt" CircId="V1" Voltage="5 V" Pos="48,64" />
<item itemtype="Resistor" CircId="R1" Resistance="1 kΩ" Pos="112,64" />
<item itemtype="Node" CircId="MID" Pos="152,64" />
<item itemtype="Resistor" CircId="R2" Resistance="1 kΩ" Pos="200,64" />
<item itemtype="Ground" CircId="GND" Pos="240,80" />
<item itemtype="Voltmeter" CircId="VM" Pos="160,96" />
<item itemtype="Ground" CircId="GNDM" Pos="168,144" />
<item itemtype="Connector" CircId="c1" startpinid="V1-outnod" endpinid="R1-lPin" pointList="64,64,96,64" />
<item itemtype="Connector" CircId="c2" startpinid="R1-rPin" endpinid="MID-0" pointList="128,64,152,64" />
<item itemtype="Connector" CircId="c3" startpinid="MID-1" endpinid="R2-lPin" pointList="152,64,184,64" />
<item itemtype="Connector" CircId="c4" startpinid="R2-rPin" endpinid="GND-Gnd" pointList="216,64,240,64" />
<item itemtype="Connector" CircId="c5" startpinid="MID-2" endpinid="VM-lPin" pointList="152,64,152,112" />
<item itemtype="Connector" CircId="c6" startpinid="VM-rPin" endpinid="GNDM-Gnd" pointList="168,112,168,128" />
</circuit>
"#
);

const RC_LOWPASS: &str = circuit!(
    r#"
<item itemtype="WaveGen" CircId="WG" WaveType="Sine" Frequency="1 kHz" Amplitude="2 V" Offset="0 V" Pos="48,48" />
<item itemtype="Node" CircId="NIN" Pos="104,48" />
<item itemtype="Resistor" CircId="R1" Resistance="1 kΩ" Pos="152,48" />
<item itemtype="Node" CircId="NOUT" Pos="192,48" />
<item itemtype="Capacitor" CircId="C1" Capacitance="1 µF" Pos="240,48" />
<item itemtype="Ground" CircId="GND" Pos="280,64" />
<item itemtype="Oscope" CircId="SC" Pos="480,160" />
<item itemtype="Ground" CircId="GNDS" Pos="392,256" />
<item itemtype="Connector" CircId="c1" startpinid="WG-outnod" endpinid="NIN-0" pointList="64,48,104,48" />
<item itemtype="Connector" CircId="c2" startpinid="NIN-1" endpinid="R1-lPin" pointList="104,48,136,48" />
<item itemtype="Connector" CircId="c3" startpinid="R1-rPin" endpinid="NOUT-0" pointList="168,48,192,48" />
<item itemtype="Connector" CircId="c4" startpinid="NOUT-1" endpinid="C1-lPin" pointList="192,48,224,48" />
<item itemtype="Connector" CircId="c5" startpinid="C1-rPin" endpinid="GND-Gnd" pointList="256,48,280,48" />
<item itemtype="Connector" CircId="c6" startpinid="NIN-2" endpinid="SC-Pin0" pointList="104,48,104,112,392,112" />
<item itemtype="Connector" CircId="c7" startpinid="NOUT-2" endpinid="SC-Pin1" pointList="192,48,192,144,392,144" />
<item itemtype="Connector" CircId="c8" startpinid="SC-PinG" endpinid="GNDS-Gnd" pointList="392,224,392,240" />
</circuit>
"#
);

const OPAMP_GAIN: &str = circuit!(
    r#"
<item itemtype="WaveGen" CircId="WG" WaveType="Sine" Frequency="1 kHz" Amplitude="1 V" Offset="0 V" Phase="270 °" Pos="48,72" />
<item itemtype="Node" CircId="VIN" Pos="140,72" />
<item itemtype="opAmp" CircId="U1" VoltPos="12 V" VoltNeg="-12 V" Pos="240,80" />
<item itemtype="Node" CircId="OUT" Pos="288,80" />
<item itemtype="Resistor" CircId="RF" Resistance="1 kΩ" Pos="328,80" />
<item itemtype="Node" CircId="FB" Pos="368,80" />
<item itemtype="Resistor" CircId="RG" Resistance="1 kΩ" Pos="416,80" />
<item itemtype="Ground" CircId="GND" Pos="456,96" />
<item itemtype="Oscope" CircId="SC" Pos="568,56" />
<item itemtype="Ground" CircId="GNDS" Pos="480,156" />
<item itemtype="Connector" CircId="c1" startpinid="WG-outnod" endpinid="VIN-0" pointList="64,72,140,72" />
<item itemtype="Connector" CircId="c2" startpinid="VIN-1" endpinid="U1-inputNinv" pointList="140,72,216,72" />
<item itemtype="Connector" CircId="c3" startpinid="U1-output" endpinid="OUT-0" pointList="264,80,288,80" />
<item itemtype="Connector" CircId="c4" startpinid="OUT-1" endpinid="RF-lPin" pointList="288,80,312,80" />
<item itemtype="Connector" CircId="c5" startpinid="RF-rPin" endpinid="FB-0" pointList="344,80,368,80" />
<item itemtype="Connector" CircId="c6" startpinid="FB-1" endpinid="RG-lPin" pointList="368,80,400,80" />
<item itemtype="Connector" CircId="c7" startpinid="RG-rPin" endpinid="GND-Gnd" pointList="432,80,456,80" />
<item itemtype="Connector" CircId="c8" startpinid="FB-2" endpinid="U1-inputInv" pointList="368,80,368,120,200,120,200,88,216,88" />
<item itemtype="Connector" CircId="c9" startpinid="VIN-2" endpinid="SC-Pin0" pointList="140,72,140,8,480,8" />
<item itemtype="Connector" CircId="c10" startpinid="OUT-2" endpinid="SC-Pin1" pointList="288,80,288,40,480,40" />
<item itemtype="Connector" CircId="c11" startpinid="SC-PinG" endpinid="GNDS-Gnd" pointList="480,120,480,140" />
</circuit>
"#
);

const LED_SWITCH: &str = circuit!(
    r#"
<item itemtype="FixedVolt" CircId="V1" Voltage="5 V" Pos="48,64" />
<item itemtype="Switch" CircId="SW" Checked="true" Pos="112,64" />
<item itemtype="Resistor" CircId="R1" Resistance="330 Ω" Pos="176,64" />
<item itemtype="Led" CircId="LED" Pos="240,64" />
<item itemtype="Ground" CircId="GND" Pos="280,80" />
<item itemtype="Connector" CircId="c1" startpinid="V1-outnod" endpinid="SW-pinP0" pointList="64,64,96,64" />
<item itemtype="Connector" CircId="c2" startpinid="SW-switch0pinN" endpinid="R1-lPin" pointList="128,64,160,64" />
<item itemtype="Connector" CircId="c3" startpinid="R1-rPin" endpinid="LED-lPin" pointList="192,64,224,64" />
<item itemtype="Connector" CircId="c4" startpinid="LED-rPin" endpinid="GND-Gnd" pointList="256,64,280,64" />
</circuit>
"#
);

const HALF_WAVE: &str = circuit!(
    r#"
<item itemtype="WaveGen" CircId="WG" WaveType="Sine" Frequency="60 Hz" Amplitude="5 V" Offset="0 V" Phase="270 °" Pos="48,64" />
<item itemtype="Node" CircId="SRC" Pos="104,64" />
<item itemtype="Diode" CircId="D1" Pos="152,64" />
<item itemtype="Node" CircId="LOAD" Pos="192,64" />
<item itemtype="Resistor" CircId="R1" Resistance="1 kΩ" Pos="240,64" />
<item itemtype="Ground" CircId="GND" Pos="280,80" />
<item itemtype="Oscope" CircId="SC" Pos="480,176" />
<item itemtype="Ground" CircId="GNDS" Pos="392,272" />
<item itemtype="Connector" CircId="c1" startpinid="WG-outnod" endpinid="SRC-0" pointList="64,64,104,64" />
<item itemtype="Connector" CircId="c2" startpinid="SRC-1" endpinid="D1-lPin" pointList="104,64,136,64" />
<item itemtype="Connector" CircId="c3" startpinid="D1-rPin" endpinid="LOAD-0" pointList="168,64,192,64" />
<item itemtype="Connector" CircId="c4" startpinid="LOAD-1" endpinid="R1-lPin" pointList="192,64,224,64" />
<item itemtype="Connector" CircId="c5" startpinid="R1-rPin" endpinid="GND-Gnd" pointList="256,64,280,64" />
<item itemtype="Connector" CircId="c6" startpinid="SRC-2" endpinid="SC-Pin0" pointList="104,64,104,128,392,128" />
<item itemtype="Connector" CircId="c7" startpinid="LOAD-2" endpinid="SC-Pin1" pointList="192,64,192,160,392,160" />
<item itemtype="Connector" CircId="c8" startpinid="SC-PinG" endpinid="GNDS-Gnd" pointList="392,240,392,256" />
</circuit>
"#
);

const VOLT_REG: &str = circuit!(
    r#"
<item itemtype="FixedVolt" CircId="V1" Voltage="9 V" Pos="48,64" />
<item itemtype="Node" CircId="NIN" Pos="104,64" />
<item itemtype="VoltReg" CircId="VR" Voltage="5 V" Pos="160,64" />
<item itemtype="Node" CircId="NOUT" Pos="200,64" />
<item itemtype="Resistor" CircId="R1" Resistance="1 kΩ" Pos="240,64" />
<item itemtype="Ground" CircId="GND" Pos="280,80" />
<item itemtype="Ground" CircId="GNDR" Pos="160,128" />
<item itemtype="Voltmeter" CircId="VMIN" Pos="112,96" />
<item itemtype="Ground" CircId="GNDIN" Pos="120,144" />
<item itemtype="Voltmeter" CircId="VMOUT" Pos="208,96" />
<item itemtype="Ground" CircId="GNDOUT" Pos="216,144" />
<item itemtype="Connector" CircId="c1" startpinid="V1-outnod" endpinid="NIN-0" pointList="64,64,104,64" />
<item itemtype="Connector" CircId="c2" startpinid="NIN-1" endpinid="VR-input" pointList="104,64,144,64" />
<item itemtype="Connector" CircId="c3" startpinid="VR-output" endpinid="NOUT-0" pointList="176,64,200,64" />
<item itemtype="Connector" CircId="c4" startpinid="NOUT-1" endpinid="R1-lPin" pointList="200,64,224,64" />
<item itemtype="Connector" CircId="c5" startpinid="R1-rPin" endpinid="GND-Gnd" pointList="256,64,280,64" />
<item itemtype="Connector" CircId="c6" startpinid="VR-ref" endpinid="GNDR-Gnd" pointList="160,80,160,112" />
<item itemtype="Connector" CircId="c7" startpinid="NIN-2" endpinid="VMIN-lPin" pointList="104,64,104,112" />
<item itemtype="Connector" CircId="c8" startpinid="VMIN-rPin" endpinid="GNDIN-Gnd" pointList="120,112,120,128" />
<item itemtype="Connector" CircId="c9" startpinid="NOUT-2" endpinid="VMOUT-lPin" pointList="200,64,200,112" />
<item itemtype="Connector" CircId="c10" startpinid="VMOUT-rPin" endpinid="GNDOUT-Gnd" pointList="216,112,216,128" />
</circuit>
"#
);

const LDR_DIVIDER: &str = circuit!(
    r#"
<item itemtype="FixedVolt" CircId="V1" Voltage="5 V" Pos="48,48" />
<item itemtype="Resistor" CircId="R1" Resistance="1 kΩ" Pos="112,48" />
<item itemtype="Node" CircId="MID" Pos="152,48" />
<item itemtype="LDR" CircId="LDR1" Lux="1000" Pos="200,48" />
<item itemtype="Ground" CircId="GND" Pos="240,64" />
<item itemtype="Voltmeter" CircId="VM" Pos="160,80" />
<item itemtype="Ground" CircId="GNDM" Pos="168,128" />
<item itemtype="Connector" CircId="c1" startpinid="V1-outnod" endpinid="R1-lPin" pointList="64,48,96,48" />
<item itemtype="Connector" CircId="c2" startpinid="R1-rPin" endpinid="MID-0" pointList="128,48,152,48" />
<item itemtype="Connector" CircId="c3" startpinid="MID-1" endpinid="LDR1-lPin" pointList="152,48,184,48" />
<item itemtype="Connector" CircId="c4" startpinid="LDR1-rPin" endpinid="GND-Gnd" pointList="216,48,240,48" />
<item itemtype="Connector" CircId="c5" startpinid="MID-2" endpinid="VM-lPin" pointList="152,48,152,96" />
<item itemtype="Connector" CircId="c6" startpinid="VM-rPin" endpinid="GNDM-Gnd" pointList="168,96,168,112" />
</circuit>
"#
);

const NPN_SWITCH: &str = circuit!(
    r#"
<item itemtype="FixedVolt" CircId="V1" Voltage="5 V" Pos="48,48" />
<item itemtype="Led" CircId="LED" Pos="112,48" />
<item itemtype="Resistor" CircId="RC" Resistance="330 Ω" Pos="176,48" />
<item itemtype="BJT" CircId="Q1" Pos="200,96" />
<item itemtype="Clock" CircId="CLK" Voltage="5 V" Frequency="2 Hz" Pos="48,96" />
<item itemtype="Resistor" CircId="RB" Resistance="10 kΩ" Pos="128,96" />
<item itemtype="Ground" CircId="GND" Pos="208,152" />
<item itemtype="Connector" CircId="c1" startpinid="V1-outnod" endpinid="LED-lPin" pointList="64,48,96,48" />
<item itemtype="Connector" CircId="c2" startpinid="LED-rPin" endpinid="RC-lPin" pointList="128,48,160,48" />
<item itemtype="Connector" CircId="c3" startpinid="RC-rPin" endpinid="Q1-collector" pointList="192,48,208,48,208,80" />
<item itemtype="Connector" CircId="c4" startpinid="CLK-outnod" endpinid="RB-lPin" pointList="64,96,112,96" />
<item itemtype="Connector" CircId="c5" startpinid="RB-rPin" endpinid="Q1-base" pointList="144,96,184,96" />
<item itemtype="Connector" CircId="c6" startpinid="Q1-emiter" endpinid="GND-Gnd" pointList="208,112,208,136" />
</circuit>
"#
);

const ASTABLE_555: &str = circuit!(
    r#"
<item itemtype="FixedVolt" CircId="V1" Voltage="5 V" rotation="90" Pos="320,16" />
<item itemtype="Node" CircId="N5" Pos="320,48" />
<item itemtype="Node" CircId="VCCN" Pos="224,48" />
<item itemtype="Node" CircId="RAIL" Pos="360,48" />
<item itemtype="Lm555" CircId="U1" Pos="200,96" />
<item itemtype="Resistor" CircId="RA" Resistance="10 kΩ" rotation="90" Pos="360,80" />
<item itemtype="Node" CircId="MID" Pos="360,96" />
<item itemtype="Resistor" CircId="RB" Resistance="10 kΩ" rotation="90" Pos="360,112" />
<item itemtype="Node" CircId="LOW" Pos="360,128" />
<item itemtype="Node" CircId="TIM" Pos="320,128" />
<item itemtype="Capacitor" CircId="C1" Capacitance="10 µF" rotation="90" Pos="360,144" />
<item itemtype="Ground" CircId="GNDT" Pos="360,176" />
<item itemtype="Ground" CircId="GND" Pos="150,136" />
<item itemtype="Resistor" CircId="RL" Resistance="330 Ω" Pos="140,100" />
<item itemtype="Led" CircId="LED" rotation="180" Pos="80,100" />
<item itemtype="Ground" CircId="GNDL" Pos="40,116" />
<item itemtype="Connector" CircId="c1" startpinid="V1-outnod" endpinid="N5-0" pointList="320,32,320,48" />
<item itemtype="Connector" CircId="c2" startpinid="N5-1" endpinid="VCCN-0" pointList="320,48,224,48" />
<item itemtype="Connector" CircId="c3" startpinid="N5-2" endpinid="RAIL-0" pointList="320,48,360,48" />
<item itemtype="Connector" CircId="c4" startpinid="VCCN-1" endpinid="U1-ePin7" pointList="224,48,224,84" />
<item itemtype="Connector" CircId="c5" startpinid="VCCN-2" endpinid="U1-ePin3" pointList="224,48,100,48,100,108,176,108" />
<item itemtype="Connector" CircId="c6" startpinid="RAIL-1" endpinid="RA-lPin" pointList="360,48,360,64" />
<item itemtype="Connector" CircId="c7" startpinid="RA-rPin" endpinid="MID-0" pointList="360,96,360,96" />
<item itemtype="Connector" CircId="c8" startpinid="MID-1" endpinid="RB-lPin" pointList="360,96,360,96" />
<item itemtype="Connector" CircId="c9" startpinid="U1-ePin6" endpinid="MID-2" pointList="224,92,300,92,300,96,360,96" />
<item itemtype="Connector" CircId="c10" startpinid="RB-rPin" endpinid="LOW-0" pointList="360,128,360,128" />
<item itemtype="Connector" CircId="c11" startpinid="LOW-1" endpinid="C1-lPin" pointList="360,128,360,128" />
<item itemtype="Connector" CircId="c12" startpinid="C1-rPin" endpinid="GNDT-Gnd" pointList="360,160,360,160" />
<item itemtype="Connector" CircId="c13" startpinid="LOW-2" endpinid="TIM-0" pointList="360,128,320,128" />
<item itemtype="Connector" CircId="c14" startpinid="TIM-1" endpinid="U1-ePin5" pointList="320,128,280,128,280,100,224,100" />
<item itemtype="Connector" CircId="c15" startpinid="TIM-2" endpinid="U1-ePin1" pointList="320,128,320,220,120,220,120,92,176,92" />
<item itemtype="Connector" CircId="c16" startpinid="U1-ePin0" endpinid="GND-Gnd" pointList="176,84,150,84,150,120" />
<item itemtype="Connector" CircId="c17" startpinid="U1-ePin2" endpinid="RL-rPin" pointList="176,100,156,100" />
<item itemtype="Connector" CircId="c18" startpinid="RL-lPin" endpinid="LED-lPin" pointList="124,100,96,100" />
<item itemtype="Connector" CircId="c19" startpinid="LED-rPin" endpinid="GNDL-Gnd" pointList="64,100,40,100" />
</circuit>
"#
);

const DIGITAL_AND: &str = circuit!(
    r#"
<item itemtype="FixedVolt" CircId="V1" Voltage="5 V" Pos="48,48" />
<item itemtype="Clock" CircId="CLK1" Voltage="5 V" Frequency="2 Hz" Pos="48,56" />
<item itemtype="And Gate" CircId="AND1" Num_Inputs="2" Pos="160,52" />
<item itemtype="Resistor" CircId="R1" Resistance="330 Ω" Pos="224,52" />
<item itemtype="Led" CircId="LED1" Pos="288,52" />
<item itemtype="Ground" CircId="GND" Pos="328,68" />
<item itemtype="Connector" CircId="c1" startpinid="V1-outnod" endpinid="AND1-in0" pointList="64,48,144,48" />
<item itemtype="Connector" CircId="c2" startpinid="CLK1-outnod" endpinid="AND1-in1" pointList="64,56,144,56" />
<item itemtype="Connector" CircId="c3" startpinid="AND1-out" endpinid="R1-lPin" pointList="176,52,208,52" />
<item itemtype="Connector" CircId="c4" startpinid="R1-rPin" endpinid="LED1-lPin" pointList="240,52,272,52" />
<item itemtype="Connector" CircId="c5" startpinid="LED1-rPin" endpinid="GND-Gnd" pointList="304,52,328,52" />
</circuit>
"#
);

const BCD_DISPLAY: &str = circuit!(
    r#"
<item itemtype="Clock" CircId="CLK" Voltage="5 V" Frequency="1 Hz" Pos="40,64" />
<item itemtype="BinCounter" CircId="CTR" IsDecade="true" Pos="160,80" />
<item itemtype="Ground" CircId="GNDR" Pos="136,120" />
<item itemtype="FixedVolt" CircId="V1" Voltage="5 V" Pos="200,32" />
<item itemtype="BcdTo7S" CircId="DEC" Pos="280,72" />
<item itemtype="SevenSegment" CircId="DISP" Pos="420,72" />
<item itemtype="Ground" CircId="GND" Pos="428,144" />
<item itemtype="Connector" CircId="c1" startpinid="CLK-outnod" endpinid="CTR-Pin_clk" pointList="56,64,136,64" />
<item itemtype="Connector" CircId="c2" startpinid="CTR-Pin_rst" endpinid="GNDR-Gnd" pointList="136,72,136,104" />
<item itemtype="Connector" CircId="c3" startpinid="CTR-out0" endpinid="DEC-in0" pointList="184,64,256,64" />
<item itemtype="Connector" CircId="c4" startpinid="CTR-out1" endpinid="DEC-in1" pointList="184,72,256,72" />
<item itemtype="Connector" CircId="c5" startpinid="CTR-out2" endpinid="DEC-in2" pointList="184,80,256,80" />
<item itemtype="Connector" CircId="c6" startpinid="CTR-out3" endpinid="DEC-in3" pointList="184,88,256,88" />
<item itemtype="Connector" CircId="c7" startpinid="V1-outnod" endpinid="DEC-in4" pointList="216,32,272,32" />
<item itemtype="Connector" CircId="c8" startpinid="DEC-out0" endpinid="DISP-pin_a" pointList="304,48,396,48" />
<item itemtype="Connector" CircId="c9" startpinid="DEC-out1" endpinid="DISP-pin_b" pointList="304,56,396,56" />
<item itemtype="Connector" CircId="c10" startpinid="DEC-out2" endpinid="DISP-pin_c" pointList="304,64,396,64" />
<item itemtype="Connector" CircId="c11" startpinid="DEC-out3" endpinid="DISP-pin_d" pointList="304,72,396,72" />
<item itemtype="Connector" CircId="c12" startpinid="DEC-out4" endpinid="DISP-pin_e" pointList="304,80,396,80" />
<item itemtype="Connector" CircId="c13" startpinid="DEC-out5" endpinid="DISP-pin_f" pointList="304,88,396,88" />
<item itemtype="Connector" CircId="c14" startpinid="DEC-out6" endpinid="DISP-pin_g" pointList="304,96,396,96" />
<item itemtype="Connector" CircId="c15" startpinid="DISP-pin_commona" endpinid="GND-Gnd" pointList="428,104,428,128" />
</circuit>
"#
);

const POT_MOTOR: &str = circuit!(
    r#"
<item itemtype="FixedVolt" CircId="V1" Voltage="5 V" Pos="48,64" />
<item itemtype="Potentiometer" CircId="POT" Resistance="100 Ω" Pos="128,64" />
<item itemtype="Ground" CircId="GNDP" Pos="168,80" />
<item itemtype="DcMotor" CircId="M1" Pos="128,160" />
<item itemtype="Ground" CircId="GND" Pos="200,176" />
<item itemtype="Connector" CircId="c1" startpinid="V1-outnod" endpinid="POT-lPin" pointList="64,64,112,64" />
<item itemtype="Connector" CircId="c2" startpinid="POT-rPin" endpinid="GNDP-Gnd" pointList="144,64,168,64" />
<item itemtype="Connector" CircId="c3" startpinid="POT-wPin" endpinid="M1-lPin" pointList="128,80,128,120,64,120,64,160,88,160" />
<item itemtype="Connector" CircId="c4" startpinid="M1-rPin" endpinid="GND-Gnd" pointList="168,160,200,160" />
</circuit>
"#
);

const EXAMPLES: &[Example] = &[
    Example {
        id: "led-switch",
        title: "LED and switch",
        summary: "A closed switch lights an LED through 330 Ω from 5 V. Open the switch to turn it off.",
        sim1: LED_SWITCH,
    },
    Example {
        id: "voltage-divider",
        title: "Voltage divider",
        summary: "Two equal resistors split a 5 V source. The meter on the middle node reads 2.5 V.",
        sim1: DIVIDER,
    },
    Example {
        id: "rc-lowpass",
        title: "RC low-pass",
        summary: "1 kΩ and 1 µF low-pass a 1 kHz sine. The corner is about 159 Hz, so the scope shows a smaller output.",
        sim1: RC_LOWPASS,
    },
    Example {
        id: "opamp-gain",
        title: "Op-amp gain of 2",
        summary: "Non-inverting amplifier. A 1 V sine becomes a 2 V sine on the scope.",
        sim1: OPAMP_GAIN,
    },
    Example {
        id: "half-wave",
        title: "Half-wave rectifier",
        summary: "A diode keeps the positive half of a 5 V sine. The scope shows the source and the load.",
        sim1: HALF_WAVE,
    },
    Example {
        id: "volt-reg",
        title: "5 V regulator",
        summary: "A linear regulator drops 9 V to 5 V. The meters read the input and the output.",
        sim1: VOLT_REG,
    },
    Example {
        id: "ldr-divider",
        title: "LDR divider",
        summary: "An LDR and 1 kΩ divide 5 V. Turn the LDR dial and the meter moves.",
        sim1: LDR_DIVIDER,
    },
    Example {
        id: "npn-switch",
        title: "NPN LED switch",
        summary: "A 2 Hz clock drives an NPN that switches an LED on the collector.",
        sim1: NPN_SWITCH,
    },
    Example {
        id: "astable-555",
        title: "555 LED blink",
        summary: "A 555 astable blinks an LED at about 5 Hz.",
        sim1: ASTABLE_555,
    },
    Example {
        id: "digital-and",
        title: "Digital AND gate and LED",
        summary: "A 2 Hz clock and a steady 5 V drive a 2-input AND gate. The LED blinks with the clock.",
        sim1: DIGITAL_AND,
    },
    Example {
        id: "bcd-display",
        title: "Counter and 7-segment",
        summary: "A 1 Hz clock steps a decade counter. The decoder shows the count on a 7-segment display.",
        sim1: BCD_DISPLAY,
    },
    Example {
        id: "pot-motor",
        title: "Potentiometer motor speed",
        summary: "A potentiometer sets the voltage on a DC motor. Turn the dial to change the speed.",
        sim1: POT_MOTOR,
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

    fn loaded(id: &str) -> Circuit {
        let ex = get(id).unwrap_or_else(|| panic!("missing {id}"));
        let scene = Scene::from_sim1(ex.sim1).unwrap_or_else(|e| panic!("{id}: {e}"));
        for w in scene.wires() {
            let end = w.end_pin.as_deref().unwrap_or("");
            let start = pin_world(&scene, &w.start_pin);
            let stop = pin_world(&scene, end);
            let a = w.points.first().copied().unwrap();
            let b = w.points.last().copied().unwrap();
            let ds = (a.x - start.x).hypot(a.y - start.y);
            let de = (b.x - stop.x).hypot(b.y - stop.y);
            assert!(
                ds < 1.0 && de < 1.0,
                "{id} wire {} ends ({}, {}) ({}, {}) pins ({}, {}) ({}, {})",
                w.id,
                a.x,
                a.y,
                b.x,
                b.y,
                start.x,
                start.y,
                stop.x,
                stop.y
            );
        }
        let mut circuit = scene.to_circuit();
        circuit.solve().unwrap_or_else(|e| panic!("{id}: {e}"));
        circuit
    }

    fn pin_world(scene: &Scene, id: &str) -> crate::canvas::Point {
        for it in scene.items() {
            for p in it.pins() {
                if p.id == id {
                    return it.map_local(p.local);
                }
            }
        }
        panic!("missing pin {id}");
    }

    fn volts(circuit: &Circuit, pin: &str) -> f64 {
        circuit
            .pin_voltage(pin)
            .unwrap_or_else(|| panic!("no voltage {pin}"))
    }

    #[test]
    fn each_example_loads_and_solves() {
        assert!(get("missing").is_none());
        assert_eq!(all().len(), 12);
        for ex in all() {
            let mut direct = Circuit::from_sim1(ex.sim1).expect(ex.id);
            direct.solve().expect(ex.id);
            let _ = loaded(ex.id);
        }

        let div = loaded("voltage-divider");
        let mid = volts(&div, "MID-0");
        assert!((mid - 2.5).abs() < 1e-3, "{mid}");

        let amp = loaded("opamp-gain");
        let vin = volts(&amp, "U1-inputNinv");
        let vo = volts(&amp, "U1-output");
        assert!((vin - 1.0).abs() < 0.05, "{vin}");
        assert!((vo - 2.0).abs() < 0.15, "{vo}");

        let led = loaded("led-switch");
        assert!(volts(&led, "SW-switch0pinN") > 4.5);
        assert!(volts(&led, "LED-rPin").abs() < 1e-3);

        let reg = loaded("volt-reg");
        let vin = volts(&reg, "VR-input");
        let vout = volts(&reg, "VR-output");
        assert!((vin - 9.0).abs() < 0.2, "{vin}");
        assert!((vout - 5.0).abs() < 0.2, "{vout}");

        let ldr = loaded("ldr-divider");
        let mid = volts(&ldr, "MID-0");
        assert!((mid - 2.5).abs() < 0.05, "{mid}");

        let rect = loaded("half-wave");
        let src = volts(&rect, "WG-outnod");
        let load = volts(&rect, "LOAD-0");
        assert!(src > 4.0, "{src}");
        assert!(load > 3.0, "{load}");

        let motor = loaded("pot-motor");
        let vm = volts(&motor, "M1-lPin");
        assert!((vm - 2.0).abs() < 0.5, "{vm}");

        let blink = loaded("astable-555");
        assert!((volts(&blink, "U1-ePin7") - 5.0).abs() < 0.2);
        // Trigger sits near ground at start, so the latch comes out of reset high.
        assert!(volts(&blink, "U1-ePin2") > 2.0);

        let count = loaded("bcd-display");
        // Yellow segments light at 2.4 V. Digit 0 is a–f on and g off.
        assert!(
            volts(&count, "DISP-pin_a") > 2.4,
            "{}",
            volts(&count, "DISP-pin_a")
        );
        assert!(
            volts(&count, "DISP-pin_g") < 1.0,
            "{}",
            volts(&count, "DISP-pin_g")
        );

        let mut canvas = crate::canvas::Canvas::empty();
        canvas
            .load_sim1(get("bcd-display").unwrap().sim1, None)
            .unwrap();
        canvas.power_on();
        let start = canvas
            .readings()
            .get("DISP")
            .map(|r| r.text.clone())
            .unwrap_or_default();
        assert_eq!(start, "11111100");
        let _ = canvas.take_dirty();
        let mut advanced = false;
        for _ in 0..200 {
            canvas.tick();
            let reading = canvas
                .readings()
                .get("DISP")
                .map(|r| r.text.clone())
                .unwrap_or_default();
            let dirty = canvas.take_dirty();
            if reading != start {
                // Digit 1 is segments b and c. The mask is a,b,c,d,e,f,g,dot.
                assert_eq!(reading, "01100000", "first step should be a 1");
                assert!(
                    dirty.items.contains("DISP"),
                    "digit {reading} did not repaint"
                );
                advanced = true;
                break;
            }
        }
        assert!(advanced, "display stayed on {start}");

        // 2 Hz clock: low for 0.25 s, high for 0.25 s. The LED must follow both halves.
        let mut and = Scene::from_sim1(get("digital-and").unwrap().sim1)
            .unwrap()
            .to_circuit();
        and.solve().unwrap();
        let led = |c: &Circuit| c.pin_voltage("LED1-lPin").unwrap();
        assert!(led(&and) < 1.0, "clock low {}", led(&and));
        and.run_ps(300_000_000_000).unwrap();
        assert!(led(&and) > 2.0, "clock high {}", led(&and));
        and.run_ps(300_000_000_000).unwrap();
        assert!(led(&and) < 1.0, "clock low again {}", led(&and));
    }
}
