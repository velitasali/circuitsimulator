//! Unit and regression test suite for MCU device emulation.

use super::*;

const PIC14: &str = r#"
<mcu core="Pic14" data="256" prog="64" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x4F" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="00011000" bits="C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP"/>
    <register name="FSR" addr="0x04" reset="0"/>
    <register name="PORTA" addr="0x05" reset="0"/>
    <register name="PCLATH" addr="0x0A" reset="0" mask="00011111"/>
  </regblock>
  <regblock start="0x80" end="0x8F">
    <mapped addr="0x80" mapto="0x00"/>
    <mapped addr="0x82" mapto="0x02"/>
    <mapped addr="0x83" mapto="0x03"/>
    <mapped addr="0x84" mapto="0x04"/>
    <register name="OPTION" addr="0x81" reset="11111111"/>
    <register name="TRISA" addr="0x85" reset="11111111"/>
    <mapped addr="0x8A" mapto="0x0A"/>
  </regblock>
  <datablock start="0x0C" end="0x4F"/>
  <port name="PORTA" pins="5" outreg="PORTA" dirreg="!TRISA"/>
</mcu>
"#;

fn running() -> Device {
    let mut d = Device::from_xml("PIC14-1", PIC14).unwrap();
    d.reset();
    d.start();
    d
}

#[test]
fn movlw_movwf_gpr() {
    let mut d = running();
    // MOVLW 0x55; MOVWF 0x20; GOTO 2
    d.load_words(&[0x3055, 0x00A0, 0x2802]);
    d.step_cpu();
    assert_eq!(d.w(), 0x55);
    d.step_cpu();
    assert_eq!(d.ram(0x20), 0x55);
    assert_eq!(d.pc(), 2);
    assert_eq!(d.monitor_ram(0x20), 0x55);
    assert_eq!(d.monitor_ram(0x83), d.status()); // mapped STATUS
    d.set_monitor_ram(0x20, 0xAA);
    assert_eq!(d.ram(0x20), 0xAA);
    assert!(d.has_status());
    assert_eq!(d.status_bits().len(), 8);
}

#[test]
fn addlw_sets_status() {
    let mut d = running();
    d.load_words(&[0x300F, 0x3E01, 0x2802]); // MOVLW 0x0F; ADDLW 0x01
    d.step_cpu();
    d.step_cpu();
    assert_eq!(d.w(), 0x10);
    assert_eq!(d.status() & 1, 0); // C clear
    assert_ne!(d.status() & (1 << 1), 0); // DC set (0xF+1)
}

#[test]
fn call_return() {
    let mut d = running();
    // 0: CALL 2; 1: GOTO 1; 2: MOVLW 0xAA; 3: RETURN
    d.load_words(&[0x2002, 0x2801, 0x30AA, 0x0008]);
    d.step_cpu();
    assert_eq!(d.pc(), 2);
    d.step_cpu();
    assert_eq!(d.w(), 0xAA);
    d.step_cpu();
    assert_eq!(d.pc(), 1);
}

#[test]
fn porta_output_after_tris() {
    let mut d = running();
    // BSF STATUS,RP0; CLRF TRISA; BCF STATUS,RP0; MOVLW 0x01; MOVWF PORTA; GOTO 5
    d.load_words(&[0x1683, 0x0185, 0x1283, 0x3001, 0x0085, 0x2805]);
    for _ in 0..5 {
        d.step_cpu();
    }
    let pin = d.gpio_pins().find(|p| p.name.ends_with("PORTA0")).unwrap();
    assert!(pin.is_out, "TRISA should make PORTA outputs");
    assert!(pin.out_state, "PORTA0 should be high");
    let pin4 = d.gpio_pins().find(|p| p.name.ends_with("PORTA4")).unwrap();
    assert!(pin4.is_out);
    assert!(!pin4.out_state);
}

#[test]
fn hex_load_then_execute() {
    let mut d = running();
    let src = crate::hex::encode_hex(0, &[0x3055, 0x2801], 16);
    d.load_hex(&src).unwrap();
    d.step_cpu();
    assert_eq!(d.w(), 0x55);
}

#[test]
fn freq_ps_tick() {
    let d = running();
    // inst_cycle=4, freq=4e6 → psTick = 1e12 * 4 / 4e6 = 1e6
    assert_eq!(d.ps_tick, 1_000_000);
    assert_eq!(d.ps_inst, 1_000_000);
}

const PIC12: &str = r#"
<mcu core="Pic12" data="32" prog="32" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x1F" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="TMR0" addr="0x01" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="00011000" bits="C,DC,Z,PD,TO"/>
    <register name="FSR" addr="0x04" reset="0"/>
    <register name="GPIO" addr="0x05" reset="0"/>
    <register name="OPTION" addr="0x06" reset="11111111"/>
    <register name="TRISGPIO" addr="0x07" reset="11111111"/>
  </regblock>
  <port name="GPIO" pins="6" outreg="GPIO" dirreg="!TRISGPIO"/>
</mcu>
"#;

#[test]
fn pic12_movlw_and_tris() {
    let mut d = Device::from_xml("PIC12-1", PIC12).unwrap();
    d.reset();
    d.start();
    // MOVLW 0x00; TRIS; MOVLW 0x01; MOVWF GPIO; GOTO 4
    d.load_words(&[0xC00, 0x006, 0xC01, 0x025, 0xA04]);
    d.step_cpu();
    assert_eq!(d.w(), 0);
    d.step_cpu();
    let gp0 = d.gpio_pins().find(|p| p.name.ends_with("GPIO0")).unwrap();
    assert!(gp0.is_out, "TRIS 0 makes GPIO outputs");
    d.step_cpu();
    assert_eq!(d.w(), 1);
    d.step_cpu();
    let gp0 = d.gpio_pins().find(|p| p.name.ends_with("GPIO0")).unwrap();
    assert!(gp0.out_state);
    assert_eq!(d.ram(0x05), 1);
}

const AVR: &str = r#"
<mcu core="AVR" data="256" prog="64" progword="2" inst_cycle="1" freq="1000000">
  <datablock start="0" end="0x1F"/>
  <regblock start="0x20" end="0x5F" streg="SREG">
    <register name="PINB" addr="0x36" reset="0"/>
    <register name="DDRB" addr="0x37" reset="0"/>
    <register name="PORTB" addr="0x38" reset="0"/>
    <register name="SPL" addr="0x5D" reset="0"/>
    <register name="SPH" addr="0x5E" reset="0"/>
    <register name="SREG" addr="0x5F" reset="0" bits="C,Z,N,V,S,H,T,I"/>
  </regblock>
  <datablock start="0x60" end="0xFF"/>
  <stack spreg="SPL,SPH" increment="post-dec"/>
  <port name="PORTB" pins="8" outreg="PORTB" dirreg="DDRB" inreg="PINB"/>
</mcu>
"#;

fn avr_running() -> Device {
    let mut d = Device::from_xml("AVR-1", AVR).unwrap();
    d.reset();
    d.start();
    d
}

fn avr_ldi(r: u8, k: u8) -> u16 {
    let h = r - 16;
    0xE000 | (u16::from(k & 0xF0) << 4) | (u16::from(h) << 4) | u16::from(k & 0x0F)
}

fn avr_out(io: u8, r: u8) -> u16 {
    0xB800 | (u16::from((io >> 4) & 3) << 9) | (u16::from(r) << 4) | u16::from(io & 0x0F)
}

fn avr_add(d: u8, r: u8) -> u16 {
    0x0C00 | (u16::from(r & 0x10) << 5) | (u16::from(d) << 4) | u16::from(r & 0x0F)
}

fn avr_rjmp(k: i16) -> u16 {
    0xC000 | (k as u16) & 0x0FFF
}

fn avr_rcall(k: i16) -> u16 {
    0xD000 | (k as u16) & 0x0FFF
}

#[test]
fn avr_ldi_add_sets_half_carry() {
    let mut d = avr_running();
    // LDI r16, 0x0F; LDI r17, 0x01; ADD r16, r17
    d.load_words(&[avr_ldi(16, 0x0F), avr_ldi(17, 0x01), avr_add(16, 17)]);
    d.step_cpu();
    d.step_cpu();
    d.step_cpu();
    assert_eq!(d.ram(16), 0x10);
    assert_eq!(d.status() & (1 << 5), 1 << 5, "H should be set"); // S_H
    assert_eq!(d.status() & 1, 0, "C clear");
    assert_eq!(d.status() & (1 << 1), 0, "Z clear");
}

#[test]
fn avr_out_ddrb_portb() {
    let mut d = avr_running();
    // LDI r16, 1; OUT DDRB, r16; LDI r16, 1; OUT PORTB, r16
    d.load_words(&[
        avr_ldi(16, 1),
        avr_out(0x17, 16),
        avr_ldi(16, 1),
        avr_out(0x18, 16),
    ]);
    for _ in 0..4 {
        d.step_cpu();
    }
    let pin = d.gpio_pins().find(|p| p.name.ends_with("PORTB0")).unwrap();
    assert!(pin.is_out, "DDRB0 should make PORTB0 an output");
    assert!(pin.out_state, "PORTB0 should be high");
    let pin1 = d.gpio_pins().find(|p| p.name.ends_with("PORTB1")).unwrap();
    assert!(!pin1.is_out);
    assert!(!pin1.out_state);
}

#[test]
fn avr_rcall_ret() {
    let mut d = avr_running();
    // 0: RCALL +1 → 2; 1: RJMP -1; 2: LDI r16, 0xAA; 3: RET
    d.load_words(&[avr_rcall(1), avr_rjmp(-1), avr_ldi(16, 0xAA), 0x9508]);
    d.step_cpu();
    assert_eq!(d.pc(), 2);
    d.step_cpu();
    assert_eq!(d.ram(16), 0xAA);
    d.step_cpu();
    assert_eq!(d.pc(), 1);
}

#[test]
fn avr_push_pop() {
    let mut d = avr_running();
    // LDI r16, 0x5A; PUSH r16; LDI r16, 0; POP r16
    d.load_words(&[
        avr_ldi(16, 0x5A),
        0x920F | (16 << 4),
        avr_ldi(16, 0),
        0x900F | (16 << 4),
    ]);
    d.step_cpu();
    d.step_cpu();
    d.step_cpu();
    assert_eq!(d.ram(16), 0);
    d.step_cpu();
    assert_eq!(d.ram(16), 0x5A);
}

#[test]
fn avr_reset_sp_is_ramend_minus_one() {
    let d = avr_running();
    // ramSize 256 → ramEnd = 254
    assert_eq!(d.ram(0x5D), 254);
    assert_eq!(d.ram(0x5E), 0);
}

const I51: &str = r#"
<mcu core="8051" data="256" prog="256" progword="1" inst_cycle="12" cpu_cycle="6" freq="12000000">
  <datablock start="0" end="0x7F"/>
  <regblock start="0x80" end="0xFF" streg="PSW">
    <register name="P0" addr="0x80" reset="11111111"/>
    <register name="SP" addr="0x81" reset="00000111"/>
    <register name="DPL" addr="0x82" reset="0"/>
    <register name="DPH" addr="0x83" reset="0"/>
    <register name="P1" addr="0x90" reset="11111111"/>
    <register name="PSW" addr="0xD0" reset="0" bits="P,0,OV,RS0,RS1,F0,AC,Cy"/>
    <register name="ACC" addr="0xE0" reset="0"/>
    <register name="B" addr="0xF0" reset="0"/>
  </regblock>
  <stack spreg="SP" increment="pre-inc"/>
  <port name="PORT1" pins="8" outreg="P1"/>
</mcu>
"#;

fn i51_running() -> Device {
    let mut d = Device::from_xml("I51-1", I51).unwrap();
    d.reset();
    d.start();
    d
}

fn i51_pump(d: &mut Device, n: usize) {
    for _ in 0..n {
        d.step_cpu();
    }
}

#[test]
fn i51_mov_a_imm() {
    let mut d = i51_running();
    d.load_words(&[0x74, 0x55]); // MOV A, #0x55
    i51_pump(&mut d, 8);
    assert_eq!(d.w(), 0x55);
    assert_eq!(d.ram(0xE0), 0x55);
}

#[test]
fn i51_add_a_imm_sets_ac() {
    let mut d = i51_running();
    // MOV A, #0x0F; ADD A, #0x01
    d.load_words(&[0x74, 0x0F, 0x24, 0x01]);
    i51_pump(&mut d, 16);
    assert_eq!(d.w(), 0x10);
    assert_ne!(d.status() & (1 << 6), 0, "AC should be set");
    assert_eq!(d.status() & (1 << 7), 0, "Cy clear");
}

#[test]
fn i51_lcall_ret() {
    let mut d = i51_running();
    // 0: LCALL 0x0005; 3: SJMP $; 5: MOV A, #0xAA; 7: RET
    d.load_words(&[0x12, 0x00, 0x05, 0x80, 0xFE, 0x74, 0xAA, 0x22]);
    i51_pump(&mut d, 24);
    assert_eq!(d.w(), 0xAA);
    assert_eq!(d.pc(), 3);
}

#[test]
fn i51_mov_p1_gpio() {
    let mut d = i51_running();
    // MOV P1, #0x01; SJMP $
    d.load_words(&[0x75, 0x90, 0x01, 0x80, 0xFE]);
    i51_pump(&mut d, 24);
    let p10 = d.gpio_pins().find(|p| p.name.ends_with("PORT10")).unwrap();
    assert!(p10.is_out, "8051 port latch drives the pin");
    assert!(p10.out_state, "P1.0 should be high");
    let p11 = d.gpio_pins().find(|p| p.name.ends_with("PORT11")).unwrap();
    assert!(p11.is_out);
    assert!(!p11.out_state, "P1.1 should be low");
    assert_eq!(d.ram(0x90), 0x01);
}

#[test]
fn i51_movx_dptr_xram() {
    let mut d = i51_running();
    // MOV A,#0x55; MOV DPTR,#0x1234; MOVX @DPTR,A; MOV A,#0; MOVX A,@DPTR; SJMP $
    d.load_words(&[
        0x74, 0x55, 0x90, 0x12, 0x34, 0xF0, 0x74, 0x00, 0xE0, 0x80, 0xFE,
    ]);
    i51_pump(&mut d, 80);
    assert_eq!(d.xram()[0x1234], 0x55);
    assert_eq!(d.w(), 0x55);
}

#[test]
fn i51_movx_ri_minimal_xml() {
    const MIN: &str = r#"
<mcu core="8051" data="256" prog="256" progword="1" inst_cycle="12" cpu_cycle="6" freq="12000000">
  <datablock start="0" end="0x7F"/>
  <regblock start="0x80" end="0xFF" streg="PSW">
    <register name="SP" addr="0x81" reset="00000111"/>
    <register name="P1" addr="0x90" reset="11111111"/>
    <register name="PSW" addr="0xD0" reset="0" bits="P,0,OV,RS0,RS1,F0,AC,Cy"/>
    <register name="ACC" addr="0xE0" reset="0"/>
  </regblock>
  <stack spreg="SP" increment="pre-inc"/>
  <port name="PORT1" pins="8" outreg="P1"/>
</mcu>
"#;
    let mut d = Device::from_xml("I51-1", MIN).unwrap();
    d.reset();
    d.start();
    d.load_words(&[0x74, 0x3C, 0x78, 0x20, 0xF2, 0x80, 0xFE]);
    i51_pump(&mut d, 80);
    assert_eq!(d.w(), 0x3C, "ACC after MOVX");
    assert_eq!(d.xram()[0x20], 0x3C);
    assert_eq!(d.ram(0), 0x20, "R0");
}

#[test]
fn i51_movx_ri_xram() {
    let mut d = i51_running();
    // MOV R0,#0x10; MOV A,#0xAB; MOVX @R0,A; MOV A,#0; MOVX A,@R0
    d.load_words(&[0x78, 0x10, 0x74, 0xAB, 0xF2, 0x74, 0x00, 0xE2, 0x80, 0xFE]);
    i51_pump(&mut d, 80);
    assert_eq!(d.xram()[0x10], 0xAB);
    assert_eq!(d.w(), 0xAB);
}

const I51_BUS: &str = r#"
<mcu core="8051" data="256" prog="256" progword="1" inst_cycle="12" cpu_cycle="6" freq="12000000">
  <datablock start="0" end="0x7F"/>
  <regblock start="0x80" end="0xFF" streg="PSW">
    <register name="P0" addr="0x80" reset="11111111"/>
    <register name="SP" addr="0x81" reset="00000111"/>
    <register name="DPL" addr="0x82" reset="0"/>
    <register name="DPH" addr="0x83" reset="0"/>
    <register name="P1" addr="0x90" reset="11111111"/>
    <register name="P2" addr="0xA0" reset="11111111"/>
    <register name="PSW" addr="0xD0" reset="0" bits="P,0,OV,RS0,RS1,F0,AC,Cy"/>
    <register name="ACC" addr="0xE0" reset="0"/>
    <register name="B" addr="0xF0" reset="0"/>
  </regblock>
  <stack spreg="SP" increment="pre-inc"/>
  <port name="PORT0" pins="8" outreg="P0" outmask="11111111" opencol="11111111"/>
  <port name="PORT1" pins="8" outreg="P1"/>
  <port name="PORT2" pins="8" outreg="P2" outmask="11111111"/>
  <ioport name="PORTE" pins="ALE,PSEN,EA"/>
</mcu>
"#;

fn i51_bus_running() -> Device {
    let mut d = Device::from_xml("I51-1", I51_BUS).unwrap();
    d.reset();
    d.start();
    d
}

#[test]
fn i51_ale_psen_idle_after_reset() {
    let d = i51_bus_running();
    let ale = d.gpio_pins().find(|p| p.label == "ALE").unwrap();
    assert!(ale.is_out, "ALE is an output");
    assert!(!ale.out_state, "ALE idles low");
    let psen = d.gpio_pins().find(|p| p.label == "PSEN").unwrap();
    assert!(psen.is_out, "PSEN is an output");
    assert!(psen.out_state, "PSEN idles high (inactive)");
    assert!(d.gpio_pins().any(|p| p.label == "EA"), "EA pin from ioport");
}

#[test]
fn i51_ea_low_takes_port0_port2() {
    let mut d = i51_bus_running();
    d.set_pin_input("I51-1-PORTEEA", false);
    d.step_cpu();
    let p00 = d.gpio_pins().find(|p| p.name.ends_with("PORT00")).unwrap();
    assert!(p00.out_ctrl, "EA low: core controls PORT0");
    assert!(p00.dir_ctrl);
    let p20 = d.gpio_pins().find(|p| p.name.ends_with("PORT20")).unwrap();
    assert!(p20.out_ctrl, "EA low: core controls PORT2");
    let ale = d.gpio_pins().find(|p| p.label == "ALE").unwrap();
    assert!(ale.out_state, "ALE rises at the start of a bus cycle");
    assert!(d.bus_remain.is_some(), "bus events are queued");
}

#[test]
fn i51_ea_high_keeps_internal_rom() {
    let mut d = i51_bus_running();
    d.set_pin_input("I51-1-PORTEEA", true);
    d.load_words(&[0x74, 0x55]);
    i51_pump(&mut d, 8);
    assert_eq!(d.w(), 0x55);
    let p00 = d.gpio_pins().find(|p| p.name.ends_with("PORT00")).unwrap();
    assert!(!p00.out_ctrl, "EA high: PORT0 stays GPIO");
}

const MCS65: &str = r#"
<mcu core="6502" data="1" prog="65536" progword="1" inst_cycle="1" freq="1000000">
  <progblock>
    <progval addr="0xFFFC" value="0"/>
    <progval addr="0xFFFD" value="0"/>
  </progblock>
</mcu>
"#;

fn mcs65_running() -> Device {
    let mut d = Device::from_xml("MCS65-1", MCS65).unwrap();
    d.reset();
    d.start();
    d
}

fn mcs65_pump(d: &mut Device, n: usize) {
    for _ in 0..n {
        d.step_cpu();
    }
}

#[test]
fn mcs65_lda_imm() {
    let mut d = mcs65_running();
    d.load_words(&[0xA9, 0x55]); // LDA #$55
    mcs65_pump(&mut d, 32);
    assert_eq!(d.w(), 0x55);
}

#[test]
fn mcs65_adc_imm() {
    let mut d = mcs65_running();
    // LDA #$0F; ADC #$01
    d.load_words(&[0xA9, 0x0F, 0x69, 0x01]);
    mcs65_pump(&mut d, 48);
    assert_eq!(d.w(), 0x10);
    assert_eq!(d.status() & 1, 0, "C clear");
    assert_eq!(d.status() & (1 << 1), 0, "Z clear");
    assert_eq!(d.status() & (1 << 7), 0, "N clear");
}

#[test]
fn mcs65_jsr_rts() {
    let mut d = mcs65_running();
    // 0: JSR $0006; 3: JMP $0003; 6: LDA #$AA; 8: RTS
    d.load_words(&[0x20, 0x06, 0x00, 0x4C, 0x03, 0x00, 0xA9, 0xAA, 0x60]);
    mcs65_pump(&mut d, 64);
    assert_eq!(d.w(), 0xAA);
    assert_eq!(d.pc(), 3);
}

#[test]
fn mcs65_sta_lda_zp() {
    let mut d = mcs65_running();
    // LDA #$55; STA $10; LDA #$00; LDA $10
    d.load_words(&[0xA9, 0x55, 0x85, 0x10, 0xA9, 0x00, 0xA5, 0x10]);
    mcs65_pump(&mut d, 64);
    assert_eq!(d.w(), 0x55);
}

#[test]
fn mcs65_inx_wraps_and_sets_z() {
    let mut d = mcs65_running();
    // LDX #$FF; INX
    d.load_words(&[0xA2, 0xFF, 0xE8]);
    mcs65_pump(&mut d, 40);
    assert_eq!(d.status() & (1 << 1), 1 << 1, "Z should be set");
    // A is still 0; X wrapped. Re-read X via TXA.
    let mut d = mcs65_running();
    d.load_words(&[0xA2, 0xFF, 0xE8, 0x8A]);
    mcs65_pump(&mut d, 48);
    assert_eq!(d.w(), 0);
}

const Z80: &str = r#"
<mcu core="Z80" data="1" prog="65536" progword="1" inst_cycle="1" freq="1000000">
</mcu>
"#;

fn z80_running() -> Device {
    let mut d = Device::from_xml("Z80-1", Z80).unwrap();
    d.reset();
    d.start();
    d
}

fn z80_pump(d: &mut Device, n: usize) {
    for _ in 0..n {
        d.step_cpu();
    }
}

#[test]
fn z80_ld_a_imm() {
    let mut d = z80_running();
    d.load_words(&[0x3E, 0x55]); // LD A,0x55
    z80_pump(&mut d, 32);
    assert_eq!(d.w(), 0x55);
}

#[test]
fn z80_add_a_imm() {
    let mut d = z80_running();
    // LD A,0x0F; ADD A,0x01
    d.load_words(&[0x3E, 0x0F, 0xC6, 0x01]);
    z80_pump(&mut d, 48);
    assert_eq!(d.w(), 0x10);
    assert_eq!(d.status() & 1, 0, "C clear");
    assert_ne!(d.status() & (1 << 4), 0, "H should be set");
    assert_eq!(d.status() & (1 << 6), 0, "Z clear");
    assert_eq!(d.status() & (1 << 7), 0, "S clear");
}

#[test]
fn z80_call_ret() {
    let mut d = z80_running();
    // 0: CALL 0x0006; 3: JR $; 5: NOP pad; 6: LD A,0xAA; 8: RET
    d.load_words(&[0xCD, 0x06, 0x00, 0x18, 0xFE, 0x00, 0x3E, 0xAA, 0xC9]);
    z80_pump(&mut d, 80);
    assert_eq!(d.w(), 0xAA);
    // C++ `getPC` is live `m_PC` (incremented at M1 T3). After RET to the
    // `JR $` at 3, PC sits in 3..=5 depending on the current T-state.
    assert!(d.pc() <= 5, "RET should return to JR $ at 3, pc={}", d.pc());
}

#[test]
fn z80_ld_hl_mem() {
    let mut d = z80_running();
    // LD HL,0x0100; LD (HL),0x55; LD A,0x00; LD A,(HL)
    d.load_words(&[0x21, 0x00, 0x01, 0x36, 0x55, 0x3E, 0x00, 0x7E]);
    z80_pump(&mut d, 80);
    assert_eq!(d.w(), 0x55);
}

#[test]
fn z80_inc_b_wraps_and_sets_z() {
    let mut d = z80_running();
    // LD B,0xFF; INC B; LD A,B
    d.load_words(&[0x06, 0xFF, 0x04, 0x78]);
    z80_pump(&mut d, 48);
    assert_eq!(d.w(), 0);
    assert_ne!(d.status() & (1 << 6), 0, "Z should be set");
}

const MCS65_BUS: &str = r#"
<mcu core="6502" data="1" prog="65536" progword="1" inst_cycle="1" freq="1000000">
  <progblock>
    <progval addr="0xFFFC" value="0"/>
    <progval addr="0xFFFD" value="0"/>
  </progblock>
  <ioport name="PORTA" pins="16"/>
  <ioport name="PORTD" pins="8"/>
  <ioport name="CPORT0" pins="RW,P0,P1,P2,SYNC,IRQ,NMI,RDY,SO"/>
</mcu>
"#;

fn mcs65_bus_running() -> Device {
    let mut d = Device::from_xml("MCS65-1", MCS65_BUS).unwrap();
    d.reset();
    d.start();
    d
}

fn gpio_out(d: &Device, port: &str) -> u32 {
    crate::port::port_out_val(&d.host.ports, port)
}

fn pin_by_label<'a>(d: &'a Device, label: &str) -> &'a crate::port::GpioPin {
    d.gpio_pins()
        .find(|p| p.label == label)
        .unwrap_or_else(|| panic!("missing pin {label}"))
}

#[test]
fn mcs65_bus_stamp() {
    let d = mcs65_bus_running();
    let rw = pin_by_label(&d, "RW");
    assert!(rw.is_out, "RW is an output");
    assert!(rw.out_state, "RW idles high (read)");
    let a0 = d.gpio_pins().find(|p| p.name.ends_with("PORTA0")).unwrap();
    assert!(a0.is_out, "PORTA is the address bus (output)");
    let d0 = d.gpio_pins().find(|p| p.name.ends_with("PORTD0")).unwrap();
    assert!(!d0.is_out, "PORTD idles as input");
    let rdy = pin_by_label(&d, "RDY");
    assert!(!rdy.is_out);
    assert!(rdy.inp_state, "RDY idles high");
}

#[test]
fn mcs65_bus_fetch_drives_address() {
    let mut d = mcs65_bus_running();
    d.load_words(&[0xA9, 0x55, 0x4C, 0x02, 0x00]); // LDA #$55; JMP $
    let mut saw_reset_vec = false;
    let mut saw_pc0 = false;
    for _ in 0..120 {
        d.advance();
        let addr = gpio_out(&d, "PORTA");
        if addr == 0xFFFC || addr == 0xFFFD {
            saw_reset_vec = true;
        }
        if addr == 0 || addr == 1 {
            saw_pc0 = true;
        }
    }
    assert_eq!(d.w(), 0x55);
    assert!(saw_reset_vec, "reset sequence reads $FFFC/$FFFD");
    assert!(saw_pc0, "fetch should put PC 0/1 on PORTA");
    let rw = pin_by_label(&d, "RW");
    assert!(rw.out_state, "LDA / JMP do not write");
}

#[test]
fn mcs65_bus_sta_drives_data() {
    let mut d = mcs65_bus_running();
    // LDA #$55; STA $10
    d.load_words(&[0xA9, 0x55, 0x85, 0x10]);
    let mut saw = false;
    for _ in 0..400 {
        d.advance();
        let rw = pin_by_label(&d, "RW");
        if rw.is_out && !rw.out_state {
            let data = gpio_out(&d, "PORTD");
            let addr = gpio_out(&d, "PORTA");
            if data == 0x55 && addr == 0x10 {
                saw = true;
                break;
            }
        }
    }
    assert!(saw, "STA $10 should drop RW and drive PORTD=0x55 at $10");
    assert_eq!(d.w(), 0x55);
}

const Z80_BUS: &str = r#"
<mcu core="Z80" data="1" prog="65536" progword="1" inst_cycle="1" freq="1000000">
  <ioport name="PORTA" pins="16"/>
  <ioport name="PORTD" pins="8"/>
  <ioport name="CPORT0" pins="M1,MREQ,IORQ,RD,WR,RFSH,HALT,WAIT,INT,NMI,BUSRQ,BUSAK,RESET"/>
</mcu>
"#;

fn z80_bus_running() -> Device {
    let mut d = Device::from_xml("Z80-1", Z80_BUS).unwrap();
    d.reset();
    d.start();
    d
}

#[test]
fn z80_bus_stamp() {
    let d = z80_bus_running();
    for name in ["MREQ", "IORQ", "RD", "WR", "M1", "RFSH", "HALT", "BUSAK"] {
        let p = pin_by_label(&d, name);
        assert!(p.is_out, "{name} is an output");
        assert!(p.out_state, "{name} idles high (inactive)");
    }
    for name in ["WAIT", "INT", "NMI", "RESET", "BUSRQ"] {
        let p = pin_by_label(&d, name);
        assert!(!p.is_out, "{name} is an input");
        assert!(p.inp_state, "{name} idles high");
    }
    let a0 = d.gpio_pins().find(|p| p.name.ends_with("PORTA0")).unwrap();
    assert!(a0.is_out, "PORTA is the address bus");
    let d0 = d.gpio_pins().find(|p| p.name.ends_with("PORTD0")).unwrap();
    assert!(!d0.is_out, "PORTD idles as input");
}

#[test]
fn z80_bus_m1_asserted_on_fetch() {
    let mut d = z80_bus_running();
    d.load_words(&[0x00]); // NOP
    let mut saw_m1 = false;
    let mut saw_mreq = false;
    for _ in 0..80 {
        d.advance();
        let m1 = pin_by_label(&d, "M1");
        let mreq = pin_by_label(&d, "MREQ");
        if m1.is_out && !m1.out_state {
            saw_m1 = true;
        }
        if mreq.is_out && !mreq.out_state {
            saw_mreq = true;
        }
        if saw_m1 && saw_mreq {
            break;
        }
    }
    assert!(saw_m1, "M1 should go low during opcode fetch");
    assert!(saw_mreq, "MREQ should go low during opcode fetch");
}

#[test]
fn z80_bus_write_drives_data() {
    let mut d = z80_bus_running();
    // LD HL,0x0100; LD (HL),0x55
    d.load_words(&[0x21, 0x00, 0x01, 0x36, 0x55]);
    let mut saw = false;
    for _ in 0..400 {
        d.advance();
        let wr = pin_by_label(&d, "WR");
        if wr.is_out && !wr.out_state {
            let data = gpio_out(&d, "PORTD");
            let addr = gpio_out(&d, "PORTA");
            if data == 0x55 && addr == 0x0100 {
                saw = true;
                break;
            }
        }
    }
    assert!(
        saw,
        "LD (HL),0x55 should drop WR and drive PORTD=0x55 at $0100"
    );
}

const PIC14_IRQ: &str = r#"
<mcu core="Pic14" data="256" prog="64" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x0B" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="TMR0" addr="0x01" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="00011000" bits="C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP"/>
    <register name="FSR" addr="0x04" reset="0"/>
    <register name="PORTA" addr="0x05" reset="0"/>
    <register name="PCLATH" addr="0x0A" reset="0" mask="00011111"/>
    <register name="INTCON" addr="0x0B" reset="0" bits="RBIF,INTF,T0IF,RBIE,INTE,T0IE,PEIE,GIE"/>
    <register name="TXSTA" addr="0x18" reset="0" bits="TX9D,TRMT,BRGH,SYNC,TXEN,TX9"/>
    <register name="TXREG" addr="0x19" reset="0"/>
    <register name="RCREG" addr="0x1A" reset="0"/>
    <register name="RCSTA" addr="0x1C" reset="0" bits="RX9D,OERR,FERR,ADDEN,CREN,SREN,RX9,SPEN"/>
    <register name="SPBRG" addr="0x1B" reset="0"/>
  </regblock>
  <regblock start="0x80" end="0x8B">
    <mapped addr="0x80" mapto="0x00"/>
    <mapped addr="0x82" mapto="0x02"/>
    <mapped addr="0x83" mapto="0x03"/>
    <mapped addr="0x84" mapto="0x04"/>
    <register name="OPTION" addr="0x81" reset="11111111" bits="PS0,PS1,PS2,PSA,T0SE,T0CS,INTEDG,RBPU"/>
    <register name="TRISA" addr="0x85" reset="11111111"/>
    <mapped addr="0x8A" mapto="0x0A"/>
    <mapped addr="0x8B" mapto="0x0B"/>
  </regblock>
  <port name="PORTA" pins="5" outreg="PORTA" dirreg="!TRISA"/>
  <interrupts enable="GIE">
    <interrupt name="T0_OVF" enable="T0IE" flag="T0IF" priority="1" vector="0x0004"/>
    <interrupt name="USART_T" enable="TXIE" flag="TXIF" priority="1" vector="0x0004"/>
    <interrupt name="USART_R" enable="RCIE" flag="RCIF" priority="1" vector="0x0004"/>
  </interrupts>
  <timer name="TIMER0" type="800" configregsA="OPTION" counter="TMR0"
         clockpin="PORTA4" interrupt="T0_OVF" prescalers="2,4,8,16,32,64,128,256"/>
  <usart name="USART0" number="1" configregsA="TXSTA" configregsB="RCSTA" interrupt="USART_T">
    <trunit type="tx" pin="PORTA0" register="TXREG"/>
    <trunit type="rx" pin="PORTA1" register="RCREG" interrupt="USART_R"/>
  </usart>
</mcu>
"#;

fn pic_irq() -> Device {
    let mut d = Device::from_xml("PIC14-1", PIC14_IRQ).unwrap();
    d.reset();
    d.start();
    d
}

#[test]
fn pic_timer0_overflow_takes_interrupt() {
    let mut d = pic_irq();
    d.load_words(&[0x2800]); // GOTO 0
    d.poke_reg(0x81, 0x08); // OPTION: internal clock, PSA=1 (prescale 1)
    d.poke_reg(0x01, 0xFE); // TMR0
    d.poke_reg(0x0B, 0xA0); // GIE | T0IE
    assert_eq!(d.irq_global(), 0x80, "GIE watch should enable global IRQ");
    assert!(
        d.timer_remain_ps().is_some(),
        "TIMER0 should be scheduled (internal clock)"
    );
    let mut took = false;
    for _ in 0..16 {
        d.advance();
        if d.pc() == 4 {
            took = true;
            break;
        }
    }
    assert!(took, "TMR0 overflow should vector to 0x04, pc={}", d.pc());
    assert_ne!(d.ram(0x0B) & (1 << 2), 0, "T0IF should be set");
    assert_eq!(d.ram(0x0B) & (1 << 7), 0, "GIE cleared on entry");
}

#[test]
fn pic_retfie_restores_gie() {
    let mut d = pic_irq();
    // 0: GOTO 0; 4: RETFIE
    let mut words = vec![0x2800; 8];
    words[0] = 0x2800;
    words[4] = 0x0009;
    d.load_words(&words);
    d.poke_reg(0x81, 0x08);
    d.poke_reg(0x01, 0xFE);
    d.poke_reg(0x0B, 0xA0);
    for _ in 0..16 {
        d.advance();
        if d.pc() == 4 {
            break;
        }
    }
    assert_eq!(d.pc(), 4);
    d.step_cpu(); // RETFIE
    assert_ne!(d.ram(0x0B) & (1 << 7), 0, "RETFIE should set GIE");
    assert_eq!(d.pc(), 0, "RETFIE returns to the PC pushed at entry");
}

#[test]
fn pic_usart_tx_drives_pin() {
    let mut d = pic_irq();
    d.load_words(&[0x2800]);
    d.poke_reg(0x1B, 0); // SPBRG=0 → period = 16*ps_inst
    d.poke_reg(0x18, 0x10); // TXSTA TXEN (bit 4)
    d.poke_reg(0x19, 0x55); // TXREG
    let mut bits = Vec::new();
    let mut last = true;
    for _ in 0..64 {
        d.advance();
        let pin = d.gpio_pins().find(|p| p.name.ends_with("PORTA0")).unwrap();
        if pin.out_state != last || bits.is_empty() {
            last = pin.out_state;
            bits.push(last);
        }
        if bits.len() >= 10 {
            break;
        }
    }
    assert!(
        bits.iter().any(|b| !*b),
        "USART TX start bit should pull PORTA0 low, saw {bits:?}"
    );
    assert!(
        bits.len() >= 2,
        "USART TX should toggle PORTA0, saw {bits:?}"
    );
}

const AVR_OC: &str = r#"
<mcu core="AVR" data="256" prog="64" progword="2" inst_cycle="1" freq="1000000">
  <regblock start="0" end="0x6E" streg="SREG">
    <register name="SREG" addr="0x5F" reset="0" bits="C,Z,N,V,S,H,T,I"/>
    <register name="PORTB" addr="0x25" reset="0"/>
    <register name="DDRB" addr="0x24" reset="0"/>
    <register name="PINB" addr="0x23" reset="0"/>
    <register name="TCCR0A" addr="0x44" reset="0" bits="WGM00,WGM01,0,0,COM0B0,COM0B1,COM0A0,COM0A1"/>
    <register name="TCCR0B" addr="0x45" reset="0" bits="CS00,CS01,CS02,WGM02,0,0,FOC0B,FOC0A"/>
    <register name="TCNT0" addr="0x46" reset="0"/>
    <register name="OCR0A" addr="0x47" reset="0"/>
    <register name="PCMSK" addr="0x40" reset="0"/>
    <register name="PCICR" addr="0x16" reset="0" bits="PCIE"/>
    <register name="PCIFR" addr="0x17" reset="0" bits="PCIF"/>
    <register name="EICRA" addr="0x69" reset="0" bits="ISC00,ISC01"/>
    <register name="EIMSK" addr="0x3D" reset="0" bits="INT0"/>
    <register name="EIFR" addr="0x3C" reset="0" bits="INTF0"/>
    <register name="TIFR0" addr="0x35" reset="0" bits="TOV0,OCF0A"/>
    <register name="TIMSK0" addr="0x6E" reset="0" bits="TOIE0,OCIE0A"/>
  </regblock>
  <port name="PORTB" pins="8" outreg="PORTB" dirreg="DDRB" inreg="PINB">
    <interrupt name="PCINT" mask="PCMSK"/>
    <extint name="INT0" pin="PORTB2" configbits="ISC00,ISC01"/>
  </port>
  <interrupts enable="I">
    <interrupt name="TIM0_OVF" enable="TOIE0" flag="TOV0" vector="0x0020" clear="1"/>
    <interrupt name="TIM0_COMPA" enable="OCIE0A" flag="OCF0A" vector="0x0028" clear="1"/>
    <interrupt name="PCINT" enable="PCIE" flag="PCIF" vector="0x0006" clear="1"/>
    <interrupt name="INT0" enable="INT0" flag="INTF0" vector="0x0002" clear="1"/>
  </interrupts>
  <timer name="TIMER0" type="800" configregsA="TCCR0A" configregsB="TCCR0B"
         counter="TCNT0" interrupt="TIM0_OVF" prescalers="0,1,8,64,256,1024,EXT_F,EXT_R"
         prselect="CS00,CS01,CS02">
    <ocunit name="OC0A" pin="PORTB3" ocreg="OCR0A" bits="COM0A0,COM0A1" interrupt="TIM0_COMPA"/>
  </timer>
</mcu>
"#;

fn avr_oc() -> Device {
    let mut d = Device::from_xml("AVR-1", AVR_OC).unwrap();
    d.reset();
    d.start();
    d
}

fn pin_named<'a>(d: &'a Device, suffix: &str) -> &'a crate::port::GpioPin {
    d.gpio_pins()
        .find(|p| p.name.ends_with(suffix) || p.label == suffix)
        .unwrap_or_else(|| panic!("missing pin {suffix}"))
}

#[test]
fn avr_fast_pwm_oc_clears_pin_on_match() {
    let mut d = avr_oc();
    d.load_words(&[0xC000]); // RJMP 0
    d.poke_reg(0x47, 2); // OCR0A = 2
    d.poke_reg(0x44, 0x83); // COM0A1 | WGM01 | WGM00 Fast PWM, CLR on match
    d.poke_reg(0x45, 0x01); // CS00 prescale 1
    let mut saw_low = false;
    for _ in 0..32 {
        d.advance();
        let pin = pin_named(&d, "PORTB3");
        if pin.out_ctrl && !pin.out_state {
            saw_low = true;
            break;
        }
    }
    assert!(
        saw_low,
        "OC0A Fast PWM COM=CLR should drive PORTB3 low on match"
    );
}

#[test]
fn avr_oc_compare_match_raises_interrupt() {
    let mut d = avr_oc();
    d.load_words(&[0xC000]);
    d.poke_reg(0x47, 1);
    d.poke_reg(0x44, 0x02); // WGM01 CTC, COM disconnected
    d.poke_reg(0x6E, 0x02); // OCIE0A
    d.poke_reg(0x5F, 0x80); // I
    d.poke_reg(0x45, 0x01); // CS00
    let mut took = false;
    for _ in 0..32 {
        d.advance();
        if d.pc() == 0x28 {
            took = true;
            break;
        }
    }
    assert!(
        took,
        "OC0A compare match should vector to 0x28, pc={}",
        d.pc()
    );
}

#[test]
fn avr_port_pin_change_interrupt() {
    let mut d = avr_oc();
    d.load_words(&[0xC000]);
    d.poke_reg(0x40, 0x01); // PCMSK bit 0
    d.poke_reg(0x16, 0x01); // PCIE
    d.poke_reg(0x5F, 0x80); // I
    d.set_pin_input("AVR-1-PORTB0", false);
    d.set_pin_input("AVR-1-PORTB0", true);
    let mut took = false;
    for _ in 0..8 {
        d.advance();
        if d.pc() == 0x06 {
            took = true;
            break;
        }
    }
    assert!(took, "PCINT on PORTB0 should vector to 0x06, pc={}", d.pc());
}

#[test]
fn avr_extint_rising_edge() {
    let mut d = avr_oc();
    d.load_words(&[0xC000]);
    d.poke_reg(0x69, 0x03); // ISC01:ISC00 = rising
    d.poke_reg(0x3D, 0x01); // INT0
    d.poke_reg(0x5F, 0x80);
    d.set_pin_input("AVR-1-PORTB2", false);
    d.set_pin_input("AVR-1-PORTB2", true);
    let mut took = false;
    for _ in 0..8 {
        d.advance();
        if d.pc() == 0x02 {
            took = true;
            break;
        }
    }
    assert!(took, "INT0 rising should vector to 0x02, pc={}", d.pc());
}

const PIC_CCP: &str = r#"
<mcu core="Pic14" data="256" prog="64" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x1F" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="TMR0" addr="0x01" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="00011000" bits="C,DC,Z,PD,TO,RP0|R0,RP1|R1,IRP"/>
    <register name="FSR" addr="0x04" reset="0"/>
    <register name="PORTB" addr="0x06" reset="0"/>
    <register name="PCLATH" addr="0x0A" reset="0" mask="00011111"/>
    <register name="INTCON" addr="0x0B" reset="0" bits="RBIF,INTF,T0IF,RBIE,INTE,T0IE,PEIE,GIE"/>
    <register name="TMR1L" addr="0x0E" reset="0"/>
    <register name="TMR1H" addr="0x0F" reset="0"/>
    <register name="T1CON" addr="0x10" reset="0" bits="TMR1ON,TMR1CS,T1SYNC,T1OSCEN,T1CKPS0,T1CKPS1"/>
    <register name="TMR2" addr="0x11" reset="0"/>
    <register name="T2CON" addr="0x12" reset="0" bits="T2CKPS0,T2CKPS1,TMR2ON,TOUTPS0,TOUTPS1,TOUTPS2,TOUTPS3"/>
    <register name="CCPR1L" addr="0x15" reset="0"/>
    <register name="CCPR1H" addr="0x16" reset="0"/>
    <register name="CCP1CON" addr="0x17" reset="0" bits="CCP1M0,CCP1M1,CCP1M2,CCP1M3,DC1B0,DC1B1"/>
    <register name="PR2" addr="0x92" reset="255"/>
    <register name="TRISB" addr="0x86" reset="11111111"/>
  </regblock>
  <port name="PORTB" pins="8" outreg="PORTB" dirreg="!TRISB"/>
  <interrupts enable="GIE">
    <interrupt name="T2_OVF" enable="TMR2IE" flag="TMR2IF" priority="1" vector="0x0004"/>
    <interrupt name="CCP1" enable="CCP1IE" flag="CCP1IF" priority="1" vector="0x0004"/>
  </interrupts>
  <timer name="TIMER1" type="160" configregsA="T1CON" counter="TMR1L,TMR1H"
         clockpin="PORTB0" interrupt="T1_OVF" prescalers="1,2,4,8"/>
  <timer name="TIMER2" type="820" configregsA="T2CON" configregsB="PR2" counter="TMR2"
         interrupt="T2_OVF" prescalers="1,4,16"/>
  <ccpunit name="CCP1" type="00" pin="PORTB3" ccpreg="CCPR1L,CCPR1H"
           interrupt="CCP1" configregsA="CCP1CON"/>
</mcu>
"#;

fn pic_ccp() -> Device {
    let mut d = Device::from_xml("PIC14-1", PIC_CCP).unwrap();
    d.reset();
    d.start();
    d
}

#[test]
fn pic_ccp_pwm_clears_pin_on_match() {
    let mut d = pic_ccp();
    d.load_words(&[0x2800]); // GOTO 0
    d.poke_reg(0x92, 15); // PR2
    d.poke_reg(0x15, 4); // CCPR1L
    d.poke_reg(0x17, 0x0C); // PWM mode CCPxM=1100
    d.poke_reg(0x12, 0x04); // TMR2ON, prescale 1
    let mut saw_low = false;
    let mut saw_high_after = false;
    for _ in 0..64 {
        d.advance();
        let pin = pin_named(&d, "PORTB3");
        if pin.out_ctrl && !pin.out_state {
            saw_low = true;
        }
        if saw_low && pin.out_ctrl && pin.out_state {
            saw_high_after = true;
            break;
        }
    }
    assert!(saw_low, "CCP PWM should drive PORTB3 low on match");
    assert!(
        saw_high_after,
        "CCP PWM should set PORTB3 high on TIMER2 overflow"
    );
}

#[test]
fn pic_ccp_pwm_high_byte_readonly() {
    let mut d = pic_ccp();
    d.load_words(&[0x2800]);
    d.poke_reg(0x16, 0xAB);
    d.poke_reg(0x17, 0x0C); // PWM
    d.poke_reg(0x16, 0x00);
    assert_eq!(d.ram(0x16), 0xAB, "CCPR1H is read-only in PWM mode");
}

#[test]
fn pic_ccp_capture_rising_stores_tmr1() {
    let mut d = pic_ccp();
    d.load_words(&[0x2800]);
    d.poke_reg(0x0E, 0x42); // TMR1L
    d.poke_reg(0x0F, 0x00); // TMR1H
    d.poke_reg(0x17, 0x05); // capture rising edge
    d.set_pin_input("PIC14-1-PORTB3", false);
    d.set_pin_input("PIC14-1-PORTB3", true);
    assert_eq!(d.ram(0x15), 0x42, "CCPR1L should capture TMR1L");
    assert_eq!(d.ram(0x16), 0x00, "CCPR1H should capture TMR1H");
}

const AVR_TINYX5: &str = r#"
<mcu core="AVR" data="256" prog="64" progword="2" inst_cycle="1" freq="1000000">
  <regblock start="0" end="0x6E" streg="SREG">
    <register name="SREG" addr="0x5F" reset="0" bits="C,Z,N,V,S,H,T,I"/>
    <register name="PORTB" addr="0x38" reset="0"/>
    <register name="DDRB" addr="0x37" reset="0"/>
    <register name="PINB" addr="0x36" reset="0"/>
    <register name="GTCCR" addr="0x4C" reset="0" bits="PSR0,PSR1,FOC1B,FOC1A,COM1B0,COM1B1,PWM1B,TSM"/>
    <register name="TCCR1" addr="0x4F" reset="0" bits="CS10,CS11,CS12,CS13,COM1A0,COM1A1,PWM1A,CTC1"/>
    <register name="TCNT1" addr="0x4E" reset="0"/>
    <register name="OCR1A" addr="0x4A" reset="0"/>
    <register name="OCR1B" addr="0x4B" reset="0"/>
    <register name="OCR1C" addr="0x4D" reset="0"/>
    <register name="TIFR" addr="0x58" reset="0" bits="TOV1,OCF1A"/>
    <register name="TIMSK" addr="0x59" reset="0" bits="TOIE1,OCIE1A"/>
  </regblock>
  <port name="PORTB" pins="8" outreg="PORTB" dirreg="DDRB" inreg="PINB"/>
  <interrupts enable="I">
    <interrupt name="TIM1_OVF" enable="TOIE1" flag="TOV1" vector="0x0008" clear="1"/>
    <interrupt name="TIM1_COMPA" enable="OCIE1A" flag="OCF1A" vector="0x0006" clear="1"/>
  </interrupts>
  <timer name="TIMER1" type="810" configregsA="TCCR1" configregsB="GTCCR"
         counter="TCNT1" topreg0="OCR1C" interrupt="TIM1_OVF"
         prescalers="0,1,2,4,8,16,32,64,128,256,512,1024,2048,4096,8192,16384"
         prselect="CS10,CS11,CS12,CS13">
    <ocunit name="OC1A" pin="PORTB1" ocreg="OCR1A" bits="COM1A0,COM1A1" interrupt="TIM1_COMPA"/>
    <ocunit name="OC1B" pin="PORTB4" ocreg="OCR1B" bits="COM1B0,COM1B1"/>
  </timer>
</mcu>
"#;

fn avr_tinyx5() -> Device {
    let mut d = Device::from_xml("AVR-1", AVR_TINYX5).unwrap();
    d.reset();
    d.start();
    d
}

#[test]
fn tinyx5_pwm1a_inverted_pin() {
    let mut d = avr_tinyx5();
    d.load_words(&[0xC000]); // RJMP 0
    d.poke_reg(0x4D, 7); // OCR1C top
    d.poke_reg(0x4A, 3); // OCR1A
    // PWM1A | COM1A0 (toggle→CLR+inv) | CS10
    d.poke_reg(0x4F, 0x40 | 0x10 | 0x01);
    let mut saw_main_low = false;
    let mut saw_inv_high = false;
    for _ in 0..64 {
        d.advance();
        let main = pin_named(&d, "PORTB1");
        let inv = pin_named(&d, "PORTB0");
        if main.out_ctrl && !main.out_state {
            saw_main_low = true;
            if inv.out_ctrl && inv.out_state {
                saw_inv_high = true;
                break;
            }
        }
    }
    assert!(saw_main_low, "PWM1A COM=TOG should CLR PORTB1 on match");
    assert!(
        saw_inv_high,
        "PWM1A complementary output should drive PORTB0 opposite of PORTB1"
    );
}

#[test]
fn tinyx5_pwm1b_inverted_pin() {
    let mut d = avr_tinyx5();
    d.load_words(&[0xC000]);
    d.poke_reg(0x4D, 7);
    d.poke_reg(0x4B, 3); // OCR1B
    d.poke_reg(0x4F, 0x01); // CS10 so TIMER1 runs
    // PWM1B | COM1B0
    d.poke_reg(0x4C, 0x40 | 0x10);
    let mut saw_main_low = false;
    let mut saw_inv_high = false;
    for _ in 0..64 {
        d.advance();
        let main = pin_named(&d, "PORTB4");
        let inv = pin_named(&d, "PORTB3");
        if main.out_ctrl && !main.out_state {
            saw_main_low = true;
            if inv.out_ctrl && inv.out_state {
                saw_inv_high = true;
                break;
            }
        }
    }
    assert!(saw_main_low, "PWM1B COM=TOG should CLR PORTB4 on match");
    assert!(
        saw_inv_high,
        "PWM1B complementary output should drive PORTB3 opposite of PORTB4"
    );
}

#[test]
fn tinyx5_psr1_reads_as_zero() {
    let mut d = avr_tinyx5();
    d.load_words(&[0xC000]);
    d.poke_reg(0x4C, 0x02); // PSR1
    assert_eq!(d.ram(0x4C) & 0x02, 0, "PSR1 always reads as 0");
}

#[test]
fn trans_module_names_usart_twi_spi() {
    let xml = r#"
<mcu core="AVR" data="256" prog="64" progword="2" eeprom="32">
  <regblock start="0" end="0x5F" streg="SREG">
    <register name="SREG" addr="0x5F" reset="0" bits="C,Z,N,V,S,H,T,I"/>
  </regblock>
  <usart name="USART0"/>
  <twi name="TWI0"/>
  <spi name="SPI0"/>
</mcu>
"#;
    let d = Device::from_xml("t", xml).unwrap();
    assert_eq!(
        d.trans_module_names(),
        vec!["USART0".to_string(), "TWI0".to_string(), "SPI0".to_string()]
    );
    assert_eq!(d.eeprom().len(), 32);
    let mut buf = vec![0x11u8; 32];
    buf[0] = 0xAB;
    let mut d = d;
    d.replace_eeprom(&buf);
    assert_eq!(d.eeprom()[0], 0xAB);
    assert_eq!(d.eeprom()[1], 0x11);
}

#[test]
fn avr_rjmp_self_loop_fast_forwards_to_timer_match() {
    let mut d = avr_oc();
    // 0: RJMP 0 (0xCFFF jumps back to 0)
    d.load_words(&[0xCFFF]);
    d.poke_reg(0x47, 10); // OCR0A = 10
    d.poke_reg(0x44, 0x02); // WGM01 CTC
    d.poke_reg(0x6E, 0x02); // OCIE0A
    d.poke_reg(0x5F, 0x80); // Global Interrupt Enable (I)
    d.poke_reg(0x45, 0x01); // CS00 (Prescaler = 1)

    // Step once into the spin loop
    d.step_cpu();
    assert_eq!(d.pc(), 0);
    assert!(d.is_spinning, "PC==prev_pc should detect spin loop");

    // Advance: should jump directly to the timer match in fast-forward!
    let mut took = false;
    let mut steps = 0;
    for _ in 0..5 {
        d.advance();
        steps += 1;
        if d.pc() == 0x28 {
            took = true;
            break;
        }
    }
    assert!(took, "Should vector to OC0A ISR at 0x28, pc={}", d.pc());
    assert!(
        steps <= 2,
        "Fast-forward should take at most 2 advance steps instead of 10+ cycles (took {steps})"
    );
    assert!(!d.is_spinning, "Spinning must clear on interrupt");
}

#[test]
fn pic_goto_self_detects_spin() {
    let mut d = running();
    // 0: GOTO 0 (0x2800)
    d.load_words(&[0x2800]);
    d.step_cpu();
    assert_eq!(d.pc(), 0);
    assert!(d.is_spinning, "PIC GOTO $ should mark is_spinning");
}

#[test]
fn external_pin_interrupt_wakes_spinning_mcu() {
    let mut d = avr_oc();
    // 0: RJMP 0
    d.load_words(&[0xCFFF]);
    d.poke_reg(0x69, 0x03); // ISC01:ISC00 = rising edge
    d.poke_reg(0x3D, 0x01); // INT0 enable
    d.poke_reg(0x5F, 0x80); // I bit
    d.set_pin_input("AVR-1-PORTB2", false);

    d.step_cpu();
    assert!(d.is_spinning);

    // External pin changes -> clears spin and queues interrupt
    d.set_pin_input("AVR-1-PORTB2", true);
    assert!(!d.is_spinning, "GPIO input change must clear spin state");

    d.advance();
    assert_eq!(d.pc(), 0x02, "Should vector to INT0 ISR at 0x02");
}
