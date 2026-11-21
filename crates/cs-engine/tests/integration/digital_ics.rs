use cs_engine::canvas::scene::Scene;
use cs_engine::digital::*;

#[test]
fn test_mux_demux_logic() {
    // 4-to-1 MUX (2 address bits)
    let mut mux = MuxState::new("mux0", 2);
    // Set inputs: in0=0, in1=1, in2=0, in3=1
    mux.inputs[0].get_inp_state(0.0);
    mux.inputs[1].get_inp_state(5.0);
    mux.inputs[2].get_inp_state(0.0);
    mux.inputs[3].get_inp_state(5.0);

    // Select input 1 (addr = 0b01)
    mux.addr_pins[0].get_inp_state(5.0);
    mux.addr_pins[1].get_inp_state(0.0);
    assert_eq!(mux.eval(), true);

    // Select input 2 (addr = 0b10)
    mux.addr_pins[0].get_inp_state(0.0);
    mux.addr_pins[1].get_inp_state(5.0);
    assert_eq!(mux.eval(), false);

    // Enable pin active-low disabled
    if let Some(en) = &mut mux.enable {
        en.get_inp_state(5.0); // disable
    }
    assert_eq!(mux.eval(), false);

    // 1-to-4 DEMUX (2 address bits)
    let mut demux = DemuxState::new("demux0", 2, false);
    demux.input.get_inp_state(5.0);
    // Select channel 3 (addr = 0b11)
    demux.addr_pins[0].get_inp_state(5.0);
    demux.addr_pins[1].get_inp_state(5.0);
    demux.eval();
    assert_eq!(demux.outputs[0].get_out_state(), false);
    assert_eq!(demux.outputs[1].get_out_state(), false);
    assert_eq!(demux.outputs[2].get_out_state(), false);
    assert_eq!(demux.outputs[3].get_out_state(), true);
}

#[test]
fn test_bcd_converters() {
    // BcdToDec
    let mut bcd2dec = BcdToDecState::new("bcd0", false, false);
    // Input = 6 (0b0110: in0=0, in1=1, in2=1, in3=0)
    bcd2dec.inputs[0].get_inp_state(0.0);
    bcd2dec.inputs[1].get_inp_state(5.0);
    bcd2dec.inputs[2].get_inp_state(5.0);
    bcd2dec.inputs[3].get_inp_state(0.0);
    bcd2dec.eval();
    for (i, p) in bcd2dec.outputs.iter().enumerate() {
        assert_eq!(p.get_out_state(), i == 6, "Pin out{i} mismatch for input 6");
    }

    // DecToBcd (Priority encoder)
    let mut dec2bcd = DecToBcdState::new("dec0", false, false);
    // Assert input 7
    dec2bcd.inputs[7].get_inp_state(5.0);
    dec2bcd.eval();
    // 7 in BCD is 0b0111 -> out0=1, out1=1, out2=1, out3=0
    assert_eq!(dec2bcd.outputs[0].get_out_state(), true);
    assert_eq!(dec2bcd.outputs[1].get_out_state(), true);
    assert_eq!(dec2bcd.outputs[2].get_out_state(), true);
    assert_eq!(dec2bcd.outputs[3].get_out_state(), false);
    assert_eq!(dec2bcd.gs.get_out_state(), true);

    // BcdTo7S
    let mut bcd7s = BcdTo7SState::new("b7s", false);
    // Digit 3: 0b0011 -> segments a,b,c,d,g should be ON (mask 0x4F: bit 0,1,2,3,6)
    bcd7s.inputs[0].get_inp_state(5.0);
    bcd7s.inputs[1].get_inp_state(5.0);
    bcd7s.inputs[2].get_inp_state(0.0);
    bcd7s.inputs[3].get_inp_state(0.0);
    // Normal operation (LT and BI/RBO high)
    bcd7s.lt.get_inp_state(5.0);
    bcd7s.bi_rbo.get_inp_state(5.0);
    bcd7s.eval();
    assert_eq!(bcd7s.segments[0].get_out_state(), true); // a
    assert_eq!(bcd7s.segments[1].get_out_state(), true); // b
    assert_eq!(bcd7s.segments[2].get_out_state(), true); // c
    assert_eq!(bcd7s.segments[3].get_out_state(), true); // d
    assert_eq!(bcd7s.segments[4].get_out_state(), false); // e
    assert_eq!(bcd7s.segments[5].get_out_state(), false); // f
    assert_eq!(bcd7s.segments[6].get_out_state(), true); // g
}

#[test]
fn test_adc_dac() {
    // 8-bit ADC with 0..5V range
    let mut adc = AdcState::new("adc0", 8, 5.0, 0.0);
    adc.oe.get_inp_state(5.0);
    let code = adc.convert(2.5);
    assert_eq!(code, 128);
    // Bit 7 should be high, rest low
    assert_eq!(adc.outputs[7].get_out_state(), true);
    assert_eq!(adc.outputs[6].get_out_state(), false);

    // 8-bit DAC with 5.0V vref
    let mut dac = DacState::new("dac0", 8, 5.0);
    // Input 128 (0x80) -> ~2.51V
    dac.inputs[7].get_inp_state(5.0);
    let vout = dac.eval();
    assert!((vout - 2.5098).abs() < 0.02);
}

#[test]
fn test_arithmetic_counters_and_adder() {
    // 4-bit Counter
    let mut counter = CounterState::new("cnt0", 4, 10);
    counter.enable.get_inp_state(5.0);
    // Clock 5 times
    for _ in 0..5 {
        counter.clock.get_inp_state(0.0);
        counter.eval();
        counter.clock.get_inp_state(5.0);
        counter.eval();
    }
    assert_eq!(counter.count, 5);
    assert_eq!(counter.outputs[0].get_out_state(), true);
    assert_eq!(counter.outputs[1].get_out_state(), false);
    assert_eq!(counter.outputs[2].get_out_state(), true);
    assert_eq!(counter.outputs[3].get_out_state(), false);

    // 4-bit Full Adder: 5 + 9 = 14 (0b1110)
    let mut adder = FullAdderState::new("fa0", 4);
    // A = 5 (0b0101)
    adder.a_inputs[0].get_inp_state(5.0);
    adder.a_inputs[1].get_inp_state(0.0);
    adder.a_inputs[2].get_inp_state(5.0);
    adder.a_inputs[3].get_inp_state(0.0);
    // B = 9 (0b1001)
    adder.b_inputs[0].get_inp_state(5.0);
    adder.b_inputs[1].get_inp_state(0.0);
    adder.b_inputs[2].get_inp_state(0.0);
    adder.b_inputs[3].get_inp_state(5.0);
    adder.ci.get_inp_state(0.0);
    adder.eval();
    assert_eq!(adder.sum_outputs[0].get_out_state(), false); // 0
    assert_eq!(adder.sum_outputs[1].get_out_state(), true); // 1
    assert_eq!(adder.sum_outputs[2].get_out_state(), true); // 1
    assert_eq!(adder.sum_outputs[3].get_out_state(), true); // 1
    assert_eq!(adder.co.get_out_state(), false); // carry=0

    // Magnitude Comparator: A=10, B=7 -> A > B
    let mut comp = MagnitudeCompState::new("cmp0", 4);
    // A = 10 (0b1010)
    comp.a_inputs[0].get_inp_state(0.0);
    comp.a_inputs[1].get_inp_state(5.0);
    comp.a_inputs[2].get_inp_state(0.0);
    comp.a_inputs[3].get_inp_state(5.0);
    // B = 7 (0b0111)
    comp.b_inputs[0].get_inp_state(5.0);
    comp.b_inputs[1].get_inp_state(5.0);
    comp.b_inputs[2].get_inp_state(5.0);
    comp.b_inputs[3].get_inp_state(0.0);
    comp.eval();
    assert_eq!(comp.out_gt.get_out_state(), true);
    assert_eq!(comp.out_eq.get_out_state(), false);
    assert_eq!(comp.out_lt.get_out_state(), false);

    // Shift Register: shift in 0b101
    let mut sreg = ShiftRegState::new("sr0", 8);
    sreg.master_reset.get_inp_state(5.0); // inactive reset
    sreg.oe.get_inp_state(0.0); // active low OE enabled
    // Clock in 1 then 0 then 1
    for bit in [1, 0, 1] {
        sreg.ser_in.get_inp_state(if bit == 1 { 5.0 } else { 0.0 });
        sreg.clk_shift.get_inp_state(0.0);
        sreg.eval();
        sreg.clk_shift.get_inp_state(5.0);
        sreg.eval();
    }
    // Latch
    sreg.clk_latch.get_inp_state(0.0);
    sreg.eval();
    sreg.clk_latch.get_inp_state(5.0);
    sreg.eval();
    assert_eq!(sreg.shift_reg, 5);
    assert_eq!(sreg.outputs[0].get_out_state(), true);
    assert_eq!(sreg.outputs[1].get_out_state(), false);
    assert_eq!(sreg.outputs[2].get_out_state(), true);

    // Boolean function: (A & B) | !C
    let mut func = FunctionState::new("f0", 3, "(A & B) | !C");
    // A=1, B=1, C=1 -> (1&1)|0 = 1
    func.inputs[0].get_inp_state(5.0);
    func.inputs[1].get_inp_state(5.0);
    func.inputs[2].get_inp_state(5.0);
    assert_eq!(func.eval(), true);
    // A=0, B=1, C=1 -> (0&1)|0 = 0
    func.inputs[0].get_inp_state(0.0);
    assert_eq!(func.eval(), false);
    // A=0, B=0, C=0 -> (0&0)|1 = 1
    func.inputs[2].get_inp_state(0.0);
    assert_eq!(func.eval(), true);
}

#[test]
fn test_memory_components() {
    // 256-byte Static RAM (8 address bits, 8 data bits)
    let mut mem = MemoryState::new("ram0", 8, 8, false);
    // Write 0x42 to address 0x10
    mem.cs.get_inp_state(0.0); // CS active low
    mem.we.get_inp_state(0.0); // WE active low
    mem.oe.get_inp_state(5.0); // OE inactive
    // Addr = 16 (bit 4)
    mem.addr_pins[4].get_inp_state(5.0);
    // Data = 0x42 (bits 1, 6)
    mem.data_pins[1].get_inp_state(5.0);
    mem.data_pins[6].get_inp_state(5.0);
    mem.eval();
    assert_eq!(mem.data[16], 0x42);

    // Read back from address 16
    mem.we.get_inp_state(5.0); // WE inactive
    mem.oe.get_inp_state(0.0); // OE active
    for p in &mut mem.data_pins {
        p.get_inp_state(0.0);
    }
    mem.eval();
    assert_eq!(mem.data_pins[1].get_out_state(), true);
    assert_eq!(mem.data_pins[6].get_out_state(), true);
    assert_eq!(mem.data_pins[0].get_out_state(), false);

    // Dynamic RAM
    let mut dram = DynamicMemoryState::new("dram0", 4);
    // Latch row 3
    dram.addr_pins[0].get_inp_state(5.0);
    dram.addr_pins[1].get_inp_state(5.0);
    dram.ras.get_inp_state(5.0);
    dram.eval();
    dram.ras.get_inp_state(0.0); // falling edge latches row 3
    dram.eval();
    assert_eq!(dram.row_addr, 3);

    // Set col 2, write byte 0x42 (bits 1 and 6 high), WE active (low)
    dram.addr_pins[0].get_inp_state(0.0);
    dram.addr_pins[1].get_inp_state(5.0);
    dram.data_pins[1].get_inp_state(5.0);
    dram.data_pins[6].get_inp_state(5.0);
    dram.we.get_inp_state(0.0);
    dram.cas.get_inp_state(5.0);
    dram.eval();
    dram.cas.get_inp_state(0.0); // falling edge on CAS writes data
    dram.eval();
    let full_addr = (3 << 4) | 2;
    assert_eq!(dram.data[full_addr], 0x42);

    // Read back: WE high (read), OE low (enabled), CAS falling edge
    dram.we.get_inp_state(5.0);
    dram.oe.get_inp_state(0.0);
    for p in &mut dram.data_pins {
        p.get_inp_state(0.0);
    }
    dram.cas.get_inp_state(5.0);
    dram.eval();
    dram.cas.get_inp_state(0.0);
    dram.eval();
    assert_eq!(dram.data_pins[1].get_out_state(), true);
    assert_eq!(dram.data_pins[6].get_out_state(), true);
    assert_eq!(dram.data_pins[0].get_out_state(), false);
}

#[test]
fn test_scene_digital_ic_roundtrip() {
    let mut scene = Scene::default();
    scene.add_mux(100.0, 100.0);
    scene.add_demux(200.0, 100.0);
    scene.add_bcd_to_dec(300.0, 100.0);
    scene.add_dec_to_bcd(400.0, 100.0);
    scene.add_bcd_to_7s(500.0, 100.0);
    scene.add_seven_segment_bcd(600.0, 100.0);
    scene.add_i2c_to_parallel(100.0, 300.0);
    scene.add_adc(200.0, 300.0);
    scene.add_dac(300.0, 300.0);
    scene.add_counter(400.0, 300.0);
    scene.add_bin_counter(500.0, 300.0);
    scene.add_full_adder(600.0, 300.0);
    scene.add_magnitude_comp(100.0, 500.0);
    scene.add_shift_reg(200.0, 500.0);
    scene.add_function(300.0, 500.0);
    scene.add_memory(400.0, 500.0);
    scene.add_dynamic_memory(500.0, 500.0);
    scene.add_i2c_ram(600.0, 500.0);
    scene.add_lm555(100.0, 700.0);

    assert_eq!(scene.items().len(), 19);

    // Serialize to XML (to_sim1)
    let xml = scene.to_sim1();
    assert!(xml.contains("itemtype=\"Mux\""));
    assert!(xml.contains("itemtype=\"Demux\""));
    assert!(xml.contains("itemtype=\"BcdToDec\""));
    assert!(xml.contains("itemtype=\"DecToBcd\""));
    assert!(xml.contains("itemtype=\"BcdTo7Segment\""));
    assert!(xml.contains("itemtype=\"SevenSegmentBCD\""));
    assert!(xml.contains("itemtype=\"I2CToParallel\""));
    assert!(xml.contains("itemtype=\"Adc\""));
    assert!(xml.contains("itemtype=\"Dac\""));
    assert!(xml.contains("itemtype=\"Counter\""));
    assert!(xml.contains("itemtype=\"BinCounter\""));
    assert!(xml.contains("itemtype=\"FullAdder\""));
    assert!(xml.contains("itemtype=\"MagnitudeComp\""));
    assert!(xml.contains("itemtype=\"ShiftReg\""));
    assert!(xml.contains("itemtype=\"Function\""));
    assert!(xml.contains("itemtype=\"Memory\""));
    assert!(xml.contains("itemtype=\"DynamicMemory\""));
    assert!(xml.contains("itemtype=\"I2CRam\""));
    assert!(xml.contains("itemtype=\"Lm555\""));

    // Convert to simulation circuit
    let circuit = scene.to_circuit();
    assert_eq!(circuit.components().len(), 19);

    // Deserialize from XML (from_sim1) and verify all 19 ICs load faithfully
    let loaded = Scene::from_sim1(&xml).expect("parse sim1 containing digital ICs");
    assert_eq!(loaded.items().len(), 19);
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::Mux(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::Demux(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::BcdToDec(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::DecToBcd(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::BcdTo7Segment(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::SevenSegmentBCD(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::I2CToParallel(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::Adc(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::Dac(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::Counter(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::BinCounter(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::FullAdder(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::MagnitudeComp(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::ShiftReg(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::Function(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::Memory(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::DynamicMemory(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::I2CRam(_)))
    );
    assert!(
        loaded
            .items()
            .iter()
            .any(|it| matches!(&it.kind, cs_engine::components::Part::Lm555(_)))
    );
}

#[test]
fn test_default_inverted_pins_and_tooltips_parity() {
    let mut scene = Scene::new();
    scene.add_saved_item(cs_engine::canvas::scene::Item::mux("Mux-1", 0.0, 0.0, 3));
    scene.add_saved_item(cs_engine::canvas::scene::Item::demux(
        "Demux-1", 100.0, 0.0, 3, false,
    ));
    scene.add_saved_item(cs_engine::canvas::scene::Item::dynamic_memory(
        "DRAM-1",
        200.0,
        0.0,
        8,
        Vec::new(),
    ));
    scene.add_saved_item(cs_engine::canvas::scene::Item::memory(
        "RAM-1",
        300.0,
        0.0,
        8,
        8,
        false,
        Vec::new(),
    ));
    scene.add_saved_item(cs_engine::canvas::scene::Item::latch(
        "Latch-1", 400.0, 0.0, 8, false, true, "Clock", false,
    ));
    scene.add_saved_item(cs_engine::canvas::scene::Item::shift_reg(
        "ShiftReg-1",
        500.0,
        0.0,
        8,
    ));
    scene.add_saved_item(cs_engine::canvas::scene::Item::bcd_to_7s(
        "B7S-1", 600.0, 0.0, false,
    ));
    scene.add_saved_item(cs_engine::canvas::scene::Item::bcd_to_dec(
        "B2D-1", 700.0, 0.0, false, false,
    ));
    scene.add_saved_item(cs_engine::canvas::scene::Item::dec_to_bcd(
        "D2B-1", 800.0, 0.0, false, false,
    ));
    scene.add_saved_item(cs_engine::canvas::scene::Item::flipflop(
        "FF-1", 900.0, 0.0, "D", true, "Clock",
    ));
    scene.add_saved_item(cs_engine::canvas::scene::Item::counter(
        "Cnt-1", 1000.0, 0.0, 4, 10,
    ));
    scene.add_saved_item(cs_engine::canvas::scene::Item::bin_counter(
        "BinCnt-1", 1100.0, 0.0, false,
    ));
    scene.add_saved_item(cs_engine::canvas::scene::Item::analog_mux(
        "AMux-1", 1200.0, 0.0, 8, 1000.0,
    ));

    // Verify default inverted pins for all components
    assert!(
        scene.is_pin_inverted("Mux-1-enable"),
        "MUX OE pin should be inverted by default"
    );
    assert!(
        !scene.is_pin_inverted("Mux-1-in0"),
        "MUX D0 pin should not be inverted"
    );
    assert!(
        !scene.is_pin_inverted("Mux-1-addr0"),
        "MUX S0 pin should not be inverted"
    );

    assert!(
        scene.is_pin_inverted("Demux-1-enable"),
        "DEMUX OE pin should be inverted by default"
    );
    assert!(
        !scene.is_pin_inverted("Demux-1-in"),
        "DEMUX D pin should not be inverted"
    );

    assert!(
        scene.is_pin_inverted("DRAM-1-ras"),
        "DRAM RAS pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("DRAM-1-cas"),
        "DRAM CAS pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("DRAM-1-we"),
        "DRAM WE pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("DRAM-1-oe"),
        "DRAM OE pin should be inverted by default"
    );
    assert!(
        !scene.is_pin_inverted("DRAM-1-a0"),
        "DRAM A0 pin should not be inverted"
    );

    assert!(
        scene.is_pin_inverted("RAM-1-cs"),
        "RAM CS pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("RAM-1-we"),
        "RAM WE pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("RAM-1-oe"),
        "RAM OE pin should be inverted by default"
    );

    assert!(
        scene.is_pin_inverted("Latch-1-oe"),
        "Latch OE pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("Latch-1-rst"),
        "Latch RST pin should be inverted by default"
    );

    assert!(
        scene.is_pin_inverted("ShiftReg-1-oe"),
        "ShiftReg OE pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("ShiftReg-1-rst"),
        "ShiftReg RST pin should be inverted by default"
    );

    assert!(
        scene.is_pin_inverted("B7S-1-enable"),
        "BcdTo7S OE pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("B7S-1-rst"),
        "BcdTo7S RST pin should be inverted by default"
    );

    assert!(
        scene.is_pin_inverted("B2D-1-enable"),
        "BcdToDec OE pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("D2B-1-enable"),
        "DecToBcd OE pin should be inverted by default"
    );

    assert!(
        scene.is_pin_inverted("FF-1-set"),
        "FlipFlop SET pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("FF-1-rst"),
        "FlipFlop RST pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("FF-1-out1"),
        "FlipFlop !Q pin should be inverted by default"
    );

    assert!(
        scene.is_pin_inverted("Cnt-1-rst"),
        "Counter RST pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("Cnt-1-set"),
        "Counter SET pin should be inverted by default"
    );

    assert!(
        scene.is_pin_inverted("BinCnt-1-rst"),
        "BinCounter RST pin should be inverted by default"
    );
    assert!(
        scene.is_pin_inverted("BinCnt-1-dir"),
        "BinCounter DIR pin should be inverted by default"
    );

    assert!(
        scene.is_pin_inverted("AMux-1-PinEnable"),
        "AnalogMux EN pin should be inverted by default"
    );

    // Test toggle inversion
    scene.toggle_pin_inverted("Mux-1-enable");
    assert!(
        !scene.is_pin_inverted("Mux-1-enable"),
        "MUX OE pin should now be non-inverted after toggle"
    );
    scene.toggle_pin_inverted("Mux-1-enable");
    assert!(
        scene.is_pin_inverted("Mux-1-enable"),
        "MUX OE pin should be inverted again after 2nd toggle"
    );

    // Test hover tooltip for MUX OE pin and other pins
    let mut canvas = cs_engine::canvas::Canvas::empty();
    *canvas.scene_mut() = scene;
    let mux_oe_pin = canvas
        .scene()
        .hit_pin(cs_engine::canvas::Point::new(0.0, -48.0))
        .expect("hit MUX OE pin");
    assert_eq!(mux_oe_pin.id, "Mux-1-enable");
    let mux_oe_tip = canvas
        .hover_tooltip(cs_engine::canvas::Point::new(0.0, -48.0))
        .expect("tooltip for MUX OE pin");
    assert!(
        mux_oe_tip.contains("Output enable (active low)"),
        "tooltip should contain 'Output enable (active low)': {mux_oe_tip}"
    );
}
