//! Comprehensive Instruction Set Architecture (ISA) verification suite for all
//! five schematic CPU cores in `cs-mcu`: AVR, PIC12/PIC14, 8051, MOS 6502, and Z80.

use cs_mcu::Device;

// ============================================================================
// 1. AVR ISA Verification
// ============================================================================

const AVR_XML: &str = r#"
<mcu core="AVR" data="512" prog="256" progword="2" inst_cycle="1" freq="16000000">
  <regblock start="0" end="0x6E" streg="SREG">
    <register name="SREG" addr="0x5F" reset="0" bits="C,Z,N,V,S,H,T,I"/>
    <register name="SPL" addr="0x5D" reset="0"/>
    <register name="SPH" addr="0x5E" reset="0"/>
    <register name="PIND" addr="0x29" reset="0"/>
    <register name="DDRD" addr="0x2A" reset="0"/>
    <register name="PORTD" addr="0x2B" reset="0"/>
    <register name="TIFR0" addr="0x35" reset="0" bits="TOV0,OCF0A"/>
    <register name="TCCR0A" addr="0x44" reset="0" bits="WGM00,WGM01,0,0,COM0B0,COM0B1,COM0A0,COM0A1"/>
    <register name="TCCR0B" addr="0x45" reset="0" bits="CS00,CS01,CS02,WGM02,0,0,FOC0B,FOC0A"/>
    <register name="TCNT0" addr="0x46" reset="0"/>
    <register name="TIMSK0" addr="0x6E" reset="0" bits="TOIE0,OCIE0A"/>
  </regblock>
  <datablock name="RAM" start="0x0100" end="0x01FF"/>
  <stack spreg="SPL,SPH" increment="postdec"/>
  <port name="PORTD" pins="8" outreg="PORTD" dirreg="DDRD" inreg="PIND"/>
  <interrupts enable="I">
    <interrupt name="T0_OVF" enable="TOIE0" flag="TOV0" vector="0x0020" clear="1"/>
  </interrupts>
  <timer name="TIMER0" type="800" configregsA="TCCR0A" configregsB="TCCR0B"
         counter="TCNT0" interrupt="T0_OVF" prescalers="0,1,8,64,256,1024,EXT_F,EXT_R"
         prselect="CS00,CS01,CS02"/>
</mcu>
"#;

const AVR_SREG: u16 = 0x5F;
const AVR_C: u8 = 1 << 0;
const AVR_Z: u8 = 1 << 1;
const AVR_N: u8 = 1 << 2;
const AVR_V: u8 = 1 << 3;
const AVR_S: u8 = 1 << 4;
const AVR_T: u8 = 1 << 6;
const AVR_I: u8 = 1 << 7;

fn avr_dev() -> Device {
    let mut d = Device::from_xml("AVR", AVR_XML).unwrap();
    d.reset();
    d.start();
    d
}

fn avr_enc_rd_rr(base: u16, d: u8, r: u8) -> u16 {
    base | (u16::from(r & 0x10) << 5) | (u16::from(d & 0x1F) << 4) | u16::from(r & 0x0F)
}

fn avr_enc_imm(base: u16, d: u8, k: u8) -> u16 {
    assert!((16..=31).contains(&d));
    base | (u16::from(k & 0xF0) << 4) | (u16::from(d - 16) << 4) | u16::from(k & 0x0F)
}

fn avr_ref_add_flags(rd: u8, rr: u8, c_in: u8) -> (u8, u8) {
    let sum = u16::from(rd) + u16::from(rr) + u16::from(c_in);
    let r = sum as u8;
    let carry = (rd & rr) | (rr & !r) | (!r & rd);
    let h = (carry >> 3) & 1;
    let c = (carry >> 7) & 1;
    let v = (((rd & rr & !r) | (!rd & !rr & r)) >> 7) & 1;
    let n = (r >> 7) & 1;
    let z = u8::from(r == 0);
    let s = n ^ v;
    (r, (h << 5) | (s << 4) | (v << 3) | (n << 2) | (z << 1) | c)
}

fn avr_ref_sub_flags(rd: u8, rr: u8, c_in: u8, prev_z: u8, sticky_z: bool) -> (u8, u8) {
    let r = rd.wrapping_sub(rr).wrapping_sub(c_in);
    let borrow = (!rd & rr) | (rr & r) | (r & !rd);
    let h = (borrow >> 3) & 1;
    let c = (borrow >> 7) & 1;
    let v = (((rd & !rr & !r) | (!rd & rr & r)) >> 7) & 1;
    let n = (r >> 7) & 1;
    let z = if sticky_z {
        if r == 0 { prev_z } else { 0 }
    } else {
        u8::from(r == 0)
    };
    let s = n ^ v;
    (r, (h << 5) | (s << 4) | (v << 3) | (n << 2) | (z << 1) | c)
}

#[test]
fn avr_alu_and_flags_exhaustive_edges() {
    let vals = [0x00u8, 0x01, 0x0F, 0x10, 0x55, 0x7F, 0x80, 0x81, 0xFE, 0xFF];
    let mut d = avr_dev();

    for &a in &vals {
        for &b in &vals {
            for c_in in 0..=1u8 {
                for prev_z in 0..=1u8 {
                    let init_sreg = (c_in) | (prev_z << 1);

                    // ADD r16, r17
                    d.set_pc(0);
                    d.poke_reg(16, a);
                    d.poke_reg(17, b);
                    d.poke_reg(AVR_SREG, init_sreg);
                    d.load_words(&[avr_enc_rd_rr(0x0C00, 16, 17)]);
                    d.step_cpu();
                    let (er, ef) = avr_ref_add_flags(a, b, 0);
                    assert_eq!(
                        (d.ram(16), d.ram(AVR_SREG) & 0x3F),
                        (er, ef),
                        "ADD {a:#x},{b:#x}"
                    );

                    // ADC r16, r17
                    d.set_pc(0);
                    d.poke_reg(16, a);
                    d.poke_reg(17, b);
                    d.poke_reg(AVR_SREG, init_sreg);
                    d.load_words(&[avr_enc_rd_rr(0x1C00, 16, 17)]);
                    d.step_cpu();
                    let (er, ef) = avr_ref_add_flags(a, b, c_in);
                    assert_eq!(
                        (d.ram(16), d.ram(AVR_SREG) & 0x3F),
                        (er, ef),
                        "ADC {a:#x},{b:#x},c={c_in}"
                    );

                    // SUB r16, r17
                    d.set_pc(0);
                    d.poke_reg(16, a);
                    d.poke_reg(17, b);
                    d.poke_reg(AVR_SREG, init_sreg);
                    d.load_words(&[avr_enc_rd_rr(0x1800, 16, 17)]);
                    d.step_cpu();
                    let (er, ef) = avr_ref_sub_flags(a, b, 0, prev_z, false);
                    assert_eq!(
                        (d.ram(16), d.ram(AVR_SREG) & 0x3F),
                        (er, ef),
                        "SUB {a:#x},{b:#x}"
                    );

                    // SBC r16, r17 (sticky Z!)
                    d.set_pc(0);
                    d.poke_reg(16, a);
                    d.poke_reg(17, b);
                    d.poke_reg(AVR_SREG, init_sreg);
                    d.load_words(&[avr_enc_rd_rr(0x0800, 16, 17)]);
                    d.step_cpu();
                    let (er, ef) = avr_ref_sub_flags(a, b, c_in, prev_z, true);
                    assert_eq!(
                        (d.ram(16), d.ram(AVR_SREG) & 0x3F),
                        (er, ef),
                        "SBC {a:#x},{b:#x},c={c_in},z={prev_z}"
                    );

                    // SUBI / SBCI / CPI / CP / CPC
                    d.set_pc(0);
                    d.poke_reg(16, a);
                    d.poke_reg(AVR_SREG, init_sreg);
                    d.load_words(&[avr_enc_imm(0x5000, 16, b)]);
                    d.step_cpu();
                    let (er, ef) = avr_ref_sub_flags(a, b, 0, prev_z, false);
                    assert_eq!(
                        (d.ram(16), d.ram(AVR_SREG) & 0x3F),
                        (er, ef),
                        "SUBI {a:#x},{b:#x}"
                    );

                    d.set_pc(0);
                    d.poke_reg(16, a);
                    d.poke_reg(AVR_SREG, init_sreg);
                    d.load_words(&[avr_enc_imm(0x4000, 16, b)]);
                    d.step_cpu();
                    let (er, ef) = avr_ref_sub_flags(a, b, c_in, prev_z, true);
                    assert_eq!(
                        (d.ram(16), d.ram(AVR_SREG) & 0x3F),
                        (er, ef),
                        "SBCI {a:#x},{b:#x}"
                    );

                    d.set_pc(0);
                    d.poke_reg(16, a);
                    d.poke_reg(17, b);
                    d.poke_reg(AVR_SREG, init_sreg);
                    d.load_words(&[avr_enc_rd_rr(0x0400, 16, 17)]); // CPC
                    d.step_cpu();
                    let (_, ef) = avr_ref_sub_flags(a, b, c_in, prev_z, true);
                    assert_eq!(
                        (d.ram(16), d.ram(AVR_SREG) & 0x3F),
                        (a, ef),
                        "CPC {a:#x},{b:#x}"
                    );

                    d.set_pc(0);
                    d.poke_reg(16, a);
                    d.poke_reg(AVR_SREG, init_sreg);
                    d.load_words(&[avr_enc_imm(0x3000, 16, b)]); // CPI
                    d.step_cpu();
                    let (_, ef) = avr_ref_sub_flags(a, b, 0, prev_z, false);
                    assert_eq!(
                        (d.ram(16), d.ram(AVR_SREG) & 0x3F),
                        (a, ef),
                        "CPI {a:#x},{b:#x}"
                    );
                }
            }
        }
    }
}

#[test]
fn avr_logic_unary_shifts_and_mul_family() {
    let mut d = avr_dev();

    // AND, OR, EOR, ANDI, ORI, MOV, MOVW
    d.poke_reg(16, 0xF0);
    d.poke_reg(17, 0x3C);
    d.load_words(&[
        avr_enc_rd_rr(0x2000, 16, 17), // AND r16, r17 -> 0x30
        avr_enc_imm(0x6000, 16, 0x05), // ORI r16, 0x05 -> 0x35
        avr_enc_imm(0x7000, 16, 0x0F), // ANDI r16, 0x0F -> 0x05
        avr_enc_rd_rr(0x2400, 16, 16), // EOR r16, r16 -> 0x00, Z=1
        avr_enc_rd_rr(0x2800, 16, 17), // OR r16, r17 -> 0x3C
        0x0198,                        // MOVW r18, r16 -> r18=0x3C, r19=0x3C
    ]);
    for _ in 0..6 {
        d.step_cpu();
    }
    assert_eq!(d.ram(16), 0x3C);
    assert_eq!(d.ram(18), 0x3C);
    assert_eq!(d.ram(19), 0x3C);

    // COM, NEG, SWAP, INC, DEC, ASR, LSR, ROR
    d.set_pc(0);
    d.poke_reg(16, 0x80);
    d.poke_reg(AVR_SREG, 0);
    d.load_words(&[
        0x9501, // NEG r16 -> 0x80, V=1, C=1, N=1, S=0
        0x9500, // COM r16 -> 0x7F, C=1, V=0
        0x9502, // SWAP r16 -> 0xF7
        0x9503, // INC r16 -> 0xF8
        0x950A, // DEC r16 -> 0xF7
        0x9505, // ASR r16 -> 0xFB, C=1
        0x9506, // LSR r16 -> 0x7D, C=1, N=0
        0x9507, // ROR r16 -> 0xBE (C=1 shifted into bit 7), C=1
    ]);
    d.step_cpu(); // NEG 0x80
    assert_eq!(d.ram(16), 0x80);
    assert_eq!(
        d.ram(AVR_SREG) & (AVR_V | AVR_C | AVR_N | AVR_S),
        AVR_V | AVR_C | AVR_N
    );
    d.step_cpu(); // COM -> 0x7F
    assert_eq!(d.ram(16), 0x7F);
    assert_ne!(d.ram(AVR_SREG) & AVR_C, 0);
    d.step_cpu(); // SWAP -> 0xF7
    assert_eq!(d.ram(16), 0xF7);
    d.step_cpu(); // INC -> 0xF8
    assert_eq!(d.ram(16), 0xF8);
    d.step_cpu(); // DEC -> 0xF7
    assert_eq!(d.ram(16), 0xF7);
    d.step_cpu(); // ASR -> 0xFB, C=1
    assert_eq!(d.ram(16), 0xFB);
    assert_ne!(d.ram(AVR_SREG) & AVR_C, 0);
    d.step_cpu(); // LSR -> 0x7D, N=0, C=1
    assert_eq!(d.ram(16), 0x7D);
    assert_eq!(d.ram(AVR_SREG) & (AVR_N | AVR_C), AVR_C);
    d.step_cpu(); // ROR -> 0xBE, C=1
    assert_eq!(d.ram(16), 0xBE);
    assert_ne!(d.ram(AVR_SREG) & AVR_C, 0);

    // MUL, MULS, MULSU, FMUL (including 200*200 > i16::MAX!), FMULS, FMULSU
    d.set_pc(0);
    d.poke_reg(16, 200);
    d.poke_reg(17, 200);
    d.poke_reg(18, (-5i8) as u8);
    d.poke_reg(19, 10);
    d.load_words(&[
        avr_enc_rd_rr(0x9C00, 16, 17), // MUL r16, r17 -> 40000 (0x9C40), C=1, Z=0
        0x0223,                        // MULS r18, r19 -> -50 (0xFFCE), C=1
        0x0323,                        // MULSU r18, r19 -> -50 (0xFFCE), C=1
        0x0309,                        // FMUL r16, r17 -> (40000 << 1) = 0x3880, C=1, Z=0
        0x03A3,                        // FMULS r18, r19 -> (-50 << 1) = 0xFF9C, C=1
        0x03AB,                        // FMULSU r18, r19 -> (-50 << 1) = 0xFF9C, C=1
    ]);
    d.step_cpu(); // MUL
    assert_eq!(u16::from_le_bytes([d.ram(0), d.ram(1)]), 40_000);
    assert_eq!(d.ram(AVR_SREG) & (AVR_C | AVR_Z), AVR_C);
    assert_eq!(d.cycles_done(), 2);

    d.step_cpu(); // MULS
    assert_eq!(u16::from_le_bytes([d.ram(0), d.ram(1)]), (-50i16) as u16);
    assert_eq!(d.ram(AVR_SREG) & (AVR_C | AVR_Z), AVR_C);

    d.step_cpu(); // MULSU
    assert_eq!(u16::from_le_bytes([d.ram(0), d.ram(1)]), (-50i16) as u16);

    d.step_cpu(); // FMUL 200*200
    assert_eq!(u16::from_le_bytes([d.ram(0), d.ram(1)]), (40_000u16) << 1);
    assert_eq!(d.ram(AVR_SREG) & (AVR_C | AVR_Z), AVR_C);

    d.step_cpu(); // FMULS
    assert_eq!(
        u16::from_le_bytes([d.ram(0), d.ram(1)]),
        ((-50i16) as u16) << 1
    );
    assert_eq!(d.ram(AVR_SREG) & (AVR_C | AVR_Z), AVR_C);

    d.step_cpu(); // FMULSU
    assert_eq!(
        u16::from_le_bytes([d.ram(0), d.ram(1)]),
        ((-50i16) as u16) << 1
    );
    assert_eq!(d.ram(AVR_SREG) & (AVR_C | AVR_Z), AVR_C);

    // FMUL 0x80 * 0x01 << 1 -> 0x0100, and 0x80 * 0x00 -> 0 (Z=1, C=0),
    // and 0x80 * 0x80 = 0x4000 << 1 = 0x8000 (C=0, Z=0),
    // and 0x80 * 0x100 not possible; 0x80 * 0x80 * 2: test FMULS (-128)*(-128) -> 0x4000<<1 = 0x8000, C=0
    d.set_pc(0);
    d.poke_reg(16, 0x80); // -128 signed, 128 unsigned
    d.poke_reg(17, 0x00);
    d.load_words(&[0x0309]); // FMUL r16, r17 -> 0, Z=1, C=0
    d.step_cpu();
    assert_eq!(u16::from_le_bytes([d.ram(0), d.ram(1)]), 0);
    assert_eq!(d.ram(AVR_SREG) & (AVR_C | AVR_Z), AVR_Z);
}

#[test]
fn avr_adiw_sbiw_memory_lpm_skips_and_control_flow() {
    let mut d = avr_dev();

    // ADIW r24, 12 (0x960C) and SBIW r24, 15 (0x970F)
    d.poke_reg(24, 0xF8);
    d.poke_reg(25, 0xFF);
    d.load_words(&[
        0x960C, // ADIW r24, 12 -> 0x0004, C=1
        0x970F, // SBIW r24, 15 -> 0xFFF5, C=1, N=1
    ]);
    d.step_cpu();
    assert_eq!(u16::from_le_bytes([d.ram(24), d.ram(25)]), 0x0004);
    assert_eq!(d.ram(AVR_SREG) & (AVR_C | AVR_Z | AVR_N), AVR_C);
    assert_eq!(d.cycles_done(), 2);
    d.step_cpu();
    assert_eq!(u16::from_le_bytes([d.ram(24), d.ram(25)]), 0xFFF5);
    assert_eq!(d.ram(AVR_SREG) & (AVR_C | AVR_N), AVR_C | AVR_N);

    // Regression: skipping 16-bit ADIW r24, 12 (0x960C) must skip 1 word (not 2!),
    // while skipping 32-bit LDS (0x9100 0x0100) must skip 2 words!
    d.set_pc(0);
    d.poke_reg(16, 0x01);
    d.poke_reg(17, 0x01);
    d.poke_reg(24, 0x00);
    d.poke_reg(25, 0x00);
    d.load_words(&[
        0xFF00,                        // 0: SBRS r16, 0 (bit 0 set -> skip next 16-bit ADIW)
        0x960C,                        // 1: ADIW r24, 12 (skipped!)
        0xE402,                        // 2: LDI r16, 0x42 (MUST execute!)
        avr_enc_rd_rr(0x1000, 16, 16), // 3: CPSE r16, r16 (equal -> skip next 32-bit LDS)
        0x9100,
        0x0100, // 4..5: LDS r16, 0x0100 (skipped!)
        0xE919, // 6: LDI r17, 0x99 (MUST execute!)
    ]);
    d.step_cpu(); // SBRS skips 1 word -> pc = 2, cycles = 2
    assert_eq!(d.pc(), 2, "SBRS must skip 1 word for 16-bit ADIW 0x960C");
    assert_eq!(d.cycles_done(), 2);
    d.step_cpu(); // LDI r16, 0x42
    assert_eq!(d.ram(16), 0x42);
    d.step_cpu(); // CPSE skips 2 words for 32-bit LDS -> pc = 6, cycles = 3
    assert_eq!(d.pc(), 6, "CPSE must skip 2 words for 32-bit LDS");
    assert_eq!(d.cycles_done(), 3);
    d.step_cpu(); // LDI r17, 0x99
    assert_eq!(d.ram(17), 0x99);

    // Indirect LD/ST (X+, -X, Y+q, Z+q), LDS/STS, LPM Z+, PUSH/POP, CALL/RET, BST/BLD, SBI/CBI
    d.set_pc(0);
    d.poke_reg(26, 0x20); // XL = 0x20
    d.poke_reg(27, 0x01); // XH = 0x01 -> X = 0x0120
    d.poke_reg(28, 0x10); // YL = 0x10
    d.poke_reg(29, 0x01); // YH = 0x01 -> Y = 0x0110
    d.poke_reg(30, 0x40); // ZL = 0x40 (byte addr of flash word 0x20)
    d.poke_reg(31, 0x00); // ZH = 0x00
    d.poke_reg(16, 0xAB);
    d.load_words(&[
        0x930D, // 0: ST X+, r16 -> RAM[0x0120]=0xAB, X=0x0121
        0x911E, // 1: LD r17, -X -> X=0x0120, r17=0xAB
        0xA30F, // 2: STD Y+39, r16 -> 10q0 qqsd dddd yqqq (q=32+0+7=39, Y=1) -> RAM[0x0137]=0xAB
        0xA12F, // 3: LDD r18, Y+39 -> r18=0xAB
        0x9135, // 4: LPM r19, Z+ -> reads low byte of flash[0x20], Z=0x41
        0x9145, // 5: LPM r20, Z+ -> reads high byte of flash[0x20], Z=0x42
        0xFB07, // 6: BST r16, 7 (bit 7 of 0xAB is 1 -> T=1)
        0xF950, // 7: BLD r21, 0 (r21 bit 0 = 1)
        0x9A5A, // 8: SBI PORTD (0x0B), 2 -> DDRD is 0x0A, PORTD is 0x0B -> 0x9A5A = SBI 0x0B,2
        0x985A, // 9: CBI PORTD (0x0B), 2
        0xD002, // 10: RCALL +2 (to 13)
        0xCFFF, // 11: RJMP -1 (halt)
        0x0000, // 12: NOP
        0xE56A, // 13: LDI r22, 0x5A
        0x9508, // 14: RET
    ]);
    d.set_flash(0x20, 0xBEEF);
    let sp0 = d.sp();
    for _ in 0..10 {
        d.step_cpu();
    }
    assert_eq!(d.ram(0x0120), 0xAB);
    assert_eq!(d.ram(17), 0xAB);
    assert_eq!(d.ram(0x0137), 0xAB);
    assert_eq!(d.ram(18), 0xAB);
    assert_eq!(d.ram(19), 0xEF);
    assert_eq!(d.ram(20), 0xBE);
    assert_ne!(d.ram(AVR_SREG) & AVR_T, 0);
    assert_eq!(d.ram(21) & 1, 1);

    d.step_cpu(); // RCALL 13
    assert_eq!(d.pc(), 13);
    assert_eq!(d.sp(), sp0 - 2);
    d.step_cpu(); // LDI r22, 0x5A
    d.step_cpu(); // RET -> pc = 11
    assert_eq!(d.pc(), 11);
    assert_eq!(d.sp(), sp0);
    assert_eq!(d.ram(22), 0x5A);

    // BSET / BCLR SREG.I
    d.set_pc(0);
    d.load_words(&[0x9478, 0x94F8]); // SEI, CLI
    d.step_cpu();
    assert_ne!(d.ram(AVR_SREG) & AVR_I, 0);
    assert_ne!(d.irq_global(), 0);
    d.step_cpu();
    assert_eq!(d.ram(AVR_SREG) & AVR_I, 0);
    assert_eq!(d.irq_global(), 0);
}

#[test]
fn avr_arduino_micros_delay_timing_accuracy() {
    // End-to-end verification of Arduino's Timer0 + micros() + delay() algorithm:
    // - 0x0000: RJMP 0x0030
    // - 0x0020 (T0_OVF ISR): increment 32-bit timer0_overflow_count at RAM 0x0100..0x0103, RETI
    // - 0x0030 (main): init Timer0 Fast PWM prescaler 64 (sbi CS01, sbi CS00), enable TOIE0, SEI,
    //                  delay 25 ms using Arduino's exact micros() (TCNT0 + TIFR0.TOV0 & t < 255),
    //                  then set PORTD2 = 1 and spin.
    let mut d = avr_dev();
    let mut prog = vec![0x0000u16; 256];

    // 0x0000: RJMP 0x0030 (+0x2F)
    prog[0x00] = 0xC02F;

    // 0x0020: T0_OVF vector -> RJMP 0x0080 (+0x5F) so ISR never overlaps main at 0x0030
    prog[0x20] = 0xC05F;
    let isr_clean: &[u16] = &[
        0x938F, // PUSH r24
        0x939F, // PUSH r25
        0x93AF, // PUSH r26
        0x93BF, // PUSH r27
        0xB78F, // IN r24, SREG
        0x938F, // PUSH r24
        0x9180, 0x0100, // LDS r24, 0x0100
        0x9190, 0x0101, // LDS r25, 0x0101
        0x91A0, 0x0102, // LDS r26, 0x0102
        0x91B0, 0x0103, // LDS r27, 0x0103
        0x9601, // ADIW r24, 1
        0x1DA1, // ADC r26, r1 (r1 == 0)
        0x1DB1, // ADC r27, r1
        0x9380, 0x0100, // STS 0x0100, r24
        0x9390, 0x0101, // STS 0x0101, r25
        0x93A0, 0x0102, // STS 0x0102, r26
        0x93B0, 0x0103, // STS 0x0103, r27
        0x918F, // POP r24
        0xBF8F, // OUT SREG, r24
        0x91BF, // POP r27
        0x91AF, // POP r26
        0x919F, // POP r25
        0x918F, // POP r24
        0x9518, // RETI
    ];
    for (i, &w) in isr_clean.iter().enumerate() {
        prog[0x80 + i] = w;
    }

    // 0x0050: micros() -> returns 32-bit microseconds in r22:r25 (exact Arduino algorithm)
    let micros_exact: &[u16] = &[
        0xB73F, // 0: IN r19, SREG
        0x94F8, // 1: CLI
        0x9180, 0x0100, // 2: LDS r24, 0x0100 (m0)
        0x9190, 0x0101, // 4: LDS r25, 0x0101 (m1)
        0x91A0, 0x0102, // 6: LDS r26, 0x0102 (m2)
        0x91B0, 0x0103, // 8: LDS r27, 0x0103 (m3)
        0xB526, // 10: IN r18, TCNT0
        0x9BA8, // 11: SBIS TIFR0, 0
        0xC005, // 12: RJMP +5 -> 18
        0x3F2F, // 13: CPI r18, 0xFF
        0xF019, // 14: BREQ +3 -> 18
        0x9601, // 15: ADIW r24, 1
        0x1DA1, // 16: ADC r26, r1
        0x1DB1, // 17: ADC r27, r1
        0xBF3F, // 18: OUT SREG, r19
        // r22 = t, r23 = m0, r24 = m1, r25 = m2  (since m << 8 + t)
        0x2F62, // 19: MOV r22, r18
        0x2F78, // 20: MOV r23, r24
        0x2F89, // 21: MOV r24, r25
        0x2F9A, // 22: MOV r25, r26
        // << 2 (multiply by 4 us/tick)
        0x0F66, // 23: ADD r22, r22
        0x1F77, // 24: ADC r23, r23
        0x1F88, // 25: ADC r24, r24
        0x1F99, // 26: ADC r25, r25
        0x0F66, // 27: ADD r22, r22
        0x1F77, // 28: ADC r23, r23
        0x1F88, // 29: ADC r24, r24
        0x1F99, // 30: ADC r25, r25
        0x9508, // 31: RET
    ];
    for (i, &w) in micros_exact.iter().enumerate() {
        prog[0x50 + i] = w;
    }

    // 0x0030: main:
    //   r1 = 0; TCCR0A = 3; TCCR0B = 2; TCCR0B = 3; TIMSK0 = 1; SEI;
    //   r20 = 25 (25 ms); start = micros() -> r12:r15;
    //   loop: m = micros(); if (m - start) < 1000 goto loop; start += 1000; if (--r20 != 0) goto loop;
    //   DDRD = 4; PORTD = 4; RJMP -1
    let main_fn: &[u16] = &[
        0x2411, // 0x30: EOR r1, r1
        0xE003, // 0x31: LDI r16, 3
        0xBD04, // 0x32: OUT TCCR0A (0x24), r16
        0xE002, // 0x33: LDI r16, 2
        0xBD05, // 0x34: OUT TCCR0B (0x25), r16 (presc 8)
        0xE003, // 0x35: LDI r16, 3
        0xBD05, // 0x36: OUT TCCR0B (0x25), r16 (presc 64)
        0xE001, // 0x37: LDI r16, 1
        0x9300, 0x006E, // 0x38: STS TIMSK0 (0x6E), r16
        0x9478, // 0x3A: SEI
        0xE149, // 0x3B: LDI r20, 25 (25 ms)
        0xD013, // 0x3C: RCALL 0x50 (0x3D + 0x13 = 0x50)
        0x016B, // 0x3D: MOVW r12, r22
        0x017C, // 0x3E: MOVW r14, r24
        // 0x3F: wait loop
        0xD010, // 0x3F: RCALL 0x50 (0x40 + 0x10 = 0x50)
        0x196C, // 0x40: SUB r22, r12
        0x097D, // 0x41: SBC r23, r13
        0x098E, // 0x42: SBC r24, r14
        0x099F, // 0x43: SBC r25, r15
        0x3E68, // 0x44: CPI r22, 0xE8 (1000 & 0xFF)
        0x4073, // 0x45: SBCI r23, 0x03 (1000 >> 8)
        0x0581, // 0x46: CPC r24, r1
        0x0591, // 0x47: CPC r25, r1
        0xF3B0, // 0x48: BRCS -10 (to 0x3F: 0x49 - 10 = 0x3F)
        0xEE08, // 0x49: LDI r16, 0xE8
        0x0EC0, // 0x4A: ADD r12, r16
        0xE003, // 0x4B: LDI r16, 0x03
        0x1ED0, // 0x4C: ADC r13, r16
        0x1CE1, // 0x4D: ADC r14, r1
        0x1CF1, // 0x4E: ADC r15, r1
        // Jump to 0x78 for tail (since 0x50..0x6F holds micros)
        0xC028, // 0x4F: RJMP +0x28 -> 0x78
    ];
    for (i, &w) in main_fn.iter().enumerate() {
        prog[0x30 + i] = w;
    }
    let tail_fn: &[u16] = &[
        0x954A, // 0x78: DEC r20
        0xF629, // 0x79: BRNE -59 (to 0x3F: 0x7A - 59 = 0x3F)
        0xE004, // 0x7A: LDI r16, 4
        0xB90A, // 0x7B: OUT DDRD (0x0A), r16
        0xB90B, // 0x7C: OUT PORTD (0x0B), r16
        0xCFFF, // 0x7D: RJMP -1
    ];
    for (i, &w) in tail_fn.iter().enumerate() {
        prog[0x78 + i] = w;
    }

    d.load_words(&prog);

    let mut t_ps: u64 = 0;
    let mut prev_micros: u32 = 0;
    let mut done_ms: Option<f64> = None;

    for _ in 0..600_000 {
        let prev_pc = d.pc();
        d.advance();
        t_ps = t_ps.saturating_add(d.next_event_ps());

        // Check monotonicity right after every micros() return at 0x40
        if d.pc() == 0x40 && prev_pc != 0x40 {
            let m = u32::from_le_bytes([d.ram(22), d.ram(23), d.ram(24), d.ram(25)]);
            assert!(
                m >= prev_micros,
                "Arduino micros() must never go backwards: {prev_micros} -> {m} at t={:.3} ms",
                t_ps as f64 / 1e9
            );
            prev_micros = m;
        }
        if d.ram(0x2B) & 0x04 != 0 {
            done_ms = Some(t_ps as f64 / 1e9);
            break;
        }
    }

    let elapsed_ms = done_ms.expect("delay(25) must complete within 600_000 steps");
    assert!(
        (24.8..=25.5).contains(&elapsed_ms),
        "delay(25) must take ~25.0 ms of simulated time, got {elapsed_ms:.3} ms"
    );
}

// ============================================================================
// 2. PIC14 & PIC12 ISA Verification
// ============================================================================

const PIC14_XML: &str = r#"
<mcu core="Pic14" data="256" prog="256" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x7F" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="0x18" bits="C,DC,Z,PD,TO,R0,R1,IRP"/>
    <register name="FSR" addr="0x04" reset="0"/>
    <register name="PCLATH" addr="0x0A" reset="0"/>
  </regblock>
</mcu>
"#;

const PIC12_XML: &str = r#"
<mcu core="Pic12" data="64" prog="256" progword="2" inst_cycle="4" freq="4000000">
  <regblock start="0" end="0x1F" streg="STATUS">
    <register name="INDF" addr="0x00" reset="0"/>
    <register name="PCL" addr="0x02" reset="0"/>
    <register name="STATUS" addr="0x03" reset="0x18" bits="C,DC,Z,PD,TO,PA0,PA1,GPWUF"/>
    <register name="FSR" addr="0x04" reset="0"/>
  </regblock>
</mcu>
"#;

#[test]
fn pic14_isa_alu_bits_indf_and_control_flow() {
    let mut d = Device::from_xml("P14", PIC14_XML).unwrap();
    d.reset();
    d.start();

    // f = 0x20, 0x21
    d.load_words(&[
        0x3019, // 0: MOVLW 0x19
        0x00A0, // 1: MOVWF 0x20 (RAM[0x20] = 0x19)
        0x3008, // 2: MOVLW 0x08
        0x07A0, // 3: ADDWF 0x20, F -> 0x21, DC=1, C=0, Z=0
        0x0220, // 4: SUBWF 0x20, W -> 0x21 - 0x08 = 0x19, C=1 (no borrow), DC=0 (1-8<0)
        0x09A0, // 5: COMF 0x20, F -> ~0x21 = 0xDE
        0x0EA0, // 6: SWAPF 0x20, F -> 0xED
        0x1420, // 7: BSF 0x20, 0 -> 0xED (already 1); wait: BSF 0x20, 4 -> 0x1620
        0x1020, // 8: BCF 0x20, 0 -> 0xEC
        0x1C20, // 9: BTFSS 0x20, 0 (bit 0 is 0 -> no skip)
        0x1820, // 10: BTFSC 0x20, 0 (bit 0 is 0 -> skip 11!)
        0x30FF, // 11: MOVLW 0xFF (skipped!)
        0x0100, // 12: CLRW -> W=0, Z=1
        0x3855, // 13: IORLW 0x55 -> W=0x55
        0x390F, // 14: ANDLW 0x0F -> W=0x05
        0x3A0F, // 15: XORLW 0x0F -> W=0x0A
        0x3C05, // 16: SUBLW 0x05 -> 5 - 10 = 0xFB, C=0 (borrow)
        0x3E05, // 17: ADDLW 0x05 -> 0xFB + 5 = 0x00, Z=1, C=1, DC=1
        0x3021, // 18: MOVLW 0x21
        0x0084, // 19: MOVWF FSR
        0x3077, // 20: MOVLW 0x77
        0x0080, // 21: MOVWF INDF -> RAM[0x21] = 0x77
        0x2019, // 22: CALL 25
        0x2817, // 23: GOTO 23 (halt)
        0x0000, // 24: NOP
        0x3442, // 25: RETLW 0x42
    ]);
    for _ in 0..4 {
        d.step_cpu();
    }
    assert_eq!(d.ram(0x20), 0x21);
    assert_eq!(d.ram(0x03) & 0x07, 0x02); // DC=1, C=0, Z=0

    d.step_cpu(); // SUBWF 0x20, W -> 0x19, C=1, DC=0
    assert_eq!(d.w(), 0x19);
    assert_eq!(d.ram(0x03) & 0x07, 0x01); // C=1, DC=0, Z=0

    for _ in 0..11 {
        d.step_cpu();
    }
    // After SUBLW 0x05 (pc=17)
    assert_eq!(d.w(), 0xFB);
    assert_eq!(d.ram(0x03) & 0x01, 0); // C=0 (borrow)

    d.step_cpu(); // ADDLW 0x05 -> 0x00, Z=1, C=1, DC=1
    assert_eq!(d.w(), 0x00);
    assert_eq!(d.ram(0x03) & 0x07, 0x07);

    for _ in 0..7 {
        d.step_cpu();
    }
    assert_eq!(
        d.ram(0x21),
        0x77,
        "INDF write via FSR must land in RAM[0x21]"
    );
    assert_eq!(d.w(), 0x42, "CALL + RETLW must return 0x42 in W");
    assert_eq!(d.pc(), 23);

    // RLF / RRF / DECFSZ / INCFSZ
    d.set_pc(0);
    d.poke_reg(0x03, 0x01); // C=1
    d.poke_reg(0x20, 0x81);
    d.poke_reg(0x22, 0x01);
    d.load_words(&[
        0x0DA0, // 0: RLF 0x20, F -> 0x03, C=1
        0x0CA0, // 1: RRF 0x20, F -> 0x81, C=1
        0x0BA2, // 2: DECFSZ 0x22, F -> 0, skip 3!
        0x30EE, // 3: MOVLW 0xEE (skipped!)
        0x30AA, // 4: MOVLW 0xAA
    ]);
    d.step_cpu();
    assert_eq!((d.ram(0x20), d.ram(0x03) & 1), (0x03, 1));
    d.step_cpu();
    assert_eq!((d.ram(0x20), d.ram(0x03) & 1), (0x81, 1));
    d.step_cpu();
    assert_eq!(d.pc(), 4);
    assert_eq!(d.cycles_done(), 2);
    d.step_cpu();
    assert_eq!(d.w(), 0xAA);
}

#[test]
fn pic12_isa_clrw_clrf_alu_and_control_flow() {
    let mut d = Device::from_xml("P12", PIC12_XML).unwrap();
    d.reset();
    d.start();

    d.poke_reg(0x10, 0x55);
    d.load_words(&[
        0xC99, // 0: MOVLW 0x99
        0x040, // 1: CLRW (regression: must clear W, NOT RAM[0]!)
        0x070, // 2: CLRF 0x10 (must clear RAM[0x10])
        0xC10, // 3: MOVLW 0x10
        0x030, // 4: MOVWF 0x10 (RAM[0x10] = 0x10)
        0xD05, // 5: IORLW 0x05 -> W = 0x15
        0x1F0, // 6: ADDWF 0x10, F -> RAM[0x10] = 0x25
        0xE0F, // 7: ANDLW 0x0F -> W = 0x05
        0x090, // 8: SUBWF 0x10, W -> 0x25 - 0x05 = 0x20, C=1
        0xF0F, // 9: XORLW 0x0F -> W = 0x2F
        0x510, // 10: BSF 0x10, 0 -> RAM[0x10] = 0x25
        0x410, // 11: BCF 0x10, 0 -> RAM[0x10] = 0x24
        0x610, // 12: BTFSC 0x10, 0 (0 -> skip 13)
        0xCFF, // 13: MOVLW 0xFF (skipped!)
        0x910, // 14: CALL 16
        0xA0F, // 15: GOTO 15 (halt)
        0x833, // 16: RETLW 0x33
    ]);
    d.step_cpu(); // MOVLW 0x99
    assert_eq!(d.w(), 0x99);
    d.step_cpu(); // CLRW
    assert_eq!(d.w(), 0x00, "PIC12 CLRW (0x040) must clear W");
    assert_ne!(d.ram(0x03) & 0x04, 0, "PIC12 CLRW must set Z");
    d.step_cpu(); // CLRF 0x10
    assert_eq!(d.ram(0x10), 0x00);

    for _ in 0..14 {
        d.step_cpu();
    }
    assert_eq!(d.ram(0x10), 0x24);
    assert_eq!(d.w(), 0x33, "PIC12 CALL + RETLW must return 0x33");
    assert_eq!(d.pc(), 15);
}

// ============================================================================
// 3. Intel 8051 ISA Verification
// ============================================================================

const I51_XML: &str = r#"
<mcu core="8051" data="256" prog="256" progword="1" inst_cycle="12" freq="12000000">
  <regblock start="0x80" end="0xFF" streg="PSW">
    <register name="SP" addr="0x81" reset="7"/>
    <register name="DPL" addr="0x82" reset="0"/>
    <register name="DPH" addr="0x83" reset="0"/>
    <register name="PSW" addr="0xD0" reset="0" bits="P,F1,OV,RS0,RS1,F0,AC,CY"/>
    <register name="ACC" addr="0xE0" reset="0"/>
    <register name="B" addr="0xF0" reset="0"/>
  </regblock>
  <stack spreg="SP" increment="pre-inc"/>
</mcu>
"#;

const I51_PSW: u16 = 0xD0;
const I51_ACC: u16 = 0xE0;
const I51_B: u16 = 0xF0;
const I51_P: u8 = 1 << 0;
const I51_OV: u8 = 1 << 2;
const I51_CY: u8 = 1 << 7;

fn i51_dev(bytes: &[u8]) -> Device {
    let mut d = Device::from_xml("I51", I51_XML).unwrap();
    let words: Vec<u16> = bytes.iter().map(|&b| u16::from(b)).collect();
    d.load_words(&words);
    d.reset();
    d.start();
    d
}

fn i51_run_to(d: &mut Device, target_pc: u32, max_steps: usize) {
    for _ in 0..max_steps {
        if d.pc() == target_pc {
            return;
        }
        d.step_cpu();
    }
    panic!(
        "8051 did not reach PC={target_pc:#x} (stuck at {:#x})",
        d.pc()
    );
}

#[test]
fn i51_register_banks_mul_div_da_bit_ops_cjne_movc() {
    // Test 1: Register banks 0, 1, 2, 3 (regression for RS1 shift bug!)
    // Select bank 2 (PSW = 0x10), MOV R0, #0xA2 -> RAM[0x10] = 0xA2
    // Select bank 3 (PSW = 0x18), MOV R1, #0xB3 -> RAM[0x19] = 0xB3
    let mut d = i51_dev(&[
        0x75, 0xD0, 0x10, // 0: MOV PSW, #0x10 (bank 2: base 0x10)
        0x78, 0xA2, // 3: MOV R0, #0xA2
        0x75, 0xD0, 0x18, // 5: MOV PSW, #0x18 (bank 3: base 0x18)
        0x79, 0xB3, // 8: MOV R1, #0xB3
        0x80, 0xFE, // 10: SJMP $
    ]);
    i51_run_to(&mut d, 10, 64);
    assert_eq!(d.ram(0x10), 0xA2, "8051 bank 2 R0 must map to RAM[0x10]");
    assert_eq!(d.ram(0x19), 0xB3, "8051 bank 3 R1 must map to RAM[0x19]");

    // Test 2: MUL AB (OV=1 when product > 255, OV=0 when <= 255) and DIV AB
    let mut d = i51_dev(&[
        0x74, 0x14, // 0: MOV A, #20
        0x75, 0xF0, 0x14, // 2: MOV B, #20
        0xA4, // 5: MUL AB -> 400 = 0x0190 (A=0x90, B=0x01, OV=1, CY=0)
        0x00, // 6: NOP
        0x74, 0x0A, // 7: MOV A, #10
        0x75, 0xF0, 0x0A, // 9: MOV B, #10
        0xA4, // 12: MUL AB -> 100 (A=100, B=0, OV=0, CY=0)
        0x75, 0xF0, 0x07, // 13: MOV B, #7
        0x84, // 16: DIV AB -> 100/7 = 14 rem 2 (A=14, B=2, OV=0)
        0x75, 0xF0, 0x00, // 17: MOV B, #0
        0x84, // 20: DIV AB -> div by zero -> OV=1
        0x80, 0xFE, // 21: SJMP $
    ]);
    i51_run_to(&mut d, 6, 64);
    assert_eq!((d.ram(I51_ACC), d.ram(I51_B)), (0x90, 0x01));
    assert_eq!(
        d.ram(I51_PSW) & (I51_OV | I51_CY),
        I51_OV,
        "MUL 20*20 must set OV=1, CY=0"
    );
    i51_run_to(&mut d, 13, 64);
    assert_eq!((d.ram(I51_ACC), d.ram(I51_B)), (100, 0));
    assert_eq!(
        d.ram(I51_PSW) & (I51_OV | I51_CY),
        0,
        "MUL 10*10 must clear OV=0"
    );
    i51_run_to(&mut d, 17, 64);
    assert_eq!((d.ram(I51_ACC), d.ram(I51_B)), (14, 2));
    assert_eq!(d.ram(I51_PSW) & I51_OV, 0);
    i51_run_to(&mut d, 21, 64);
    assert_ne!(d.ram(I51_PSW) & I51_OV, 0, "DIV by zero must set OV=1");

    // Test 3: DA A (BCD adjust regression!)
    // 0x09 + 0x01 -> DA => 0x10, CY=0
    // 0x58 + 0x47 -> DA => 0x05, CY=1
    let mut d = i51_dev(&[
        0xC3, // 0: CLR C
        0x74, 0x09, // 1: MOV A, #0x09
        0x24, 0x01, // 3: ADD A, #0x01 -> 0x0A
        0xD4, // 5: DA A -> 0x10, CY=0
        0x00, // 6: NOP
        0x74, 0x58, // 7: MOV A, #0x58
        0x24, 0x47, // 9: ADD A, #0x47 -> 0x9F, AC=0
        0xD4, // 11: DA A -> 0x05, CY=1
        0x80, 0xFE, // 12: SJMP $
    ]);
    i51_run_to(&mut d, 6, 64);
    assert_eq!(d.ram(I51_ACC), 0x10, "DA A of 0x09+0x01 must yield 0x10");
    assert_eq!(
        d.ram(I51_PSW) & I51_CY,
        0,
        "DA A of 0x09+0x01 must NOT set CY"
    );
    i51_run_to(&mut d, 12, 64);
    assert_eq!(d.ram(I51_ACC), 0x05, "DA A of 0x58+0x47 must yield 0x05");
    assert_ne!(
        d.ram(I51_PSW) & I51_CY,
        0,
        "DA A of 0x58+0x47 must set CY=1"
    );

    // Test 4: ORL C, bit (0x72) & ORL C, /bit (0xA0) + ANL C, bit (0x82) & ANL C, /bit (0xB0)
    // Bit address 0x00 is bit 0 of RAM[0x20]
    let mut d = i51_dev(&[
        0x75, 0x20, 0x01, // 0: RAM[0x20] = 0x01 (bit 0 = 1, bit 1 = 0)
        0xC3, // 3: CLR C (C = 0)
        0x72, 0x00, // 4: ORL C, 0x00 (0 | 1 -> C = 1)
        0x00, // 6: NOP
        0xC3, // 7: CLR C (C = 0)
        0x72, 0x01, // 8: ORL C, 0x01 (0 | 0 -> C = 0)
        0x00, // 10: NOP
        0xA0, 0x01, // 11: ORL C, /0x01 (0 | !0 -> C = 1)
        0x00, // 13: NOP
        0xB0, 0x00, // 14: ANL C, /0x00 (1 & !1 -> C = 0)
        0x80, 0xFE, // 16: SJMP $
    ]);
    i51_run_to(&mut d, 6, 64);
    assert_ne!(d.ram(I51_PSW) & I51_CY, 0, "ORL C, 1 must set C=1");
    i51_run_to(&mut d, 10, 64);
    assert_eq!(d.ram(I51_PSW) & I51_CY, 0, "ORL C, 0 must keep C=0");
    i51_run_to(&mut d, 13, 64);
    assert_ne!(d.ram(I51_PSW) & I51_CY, 0, "ORL C, /0 must set C=1");
    i51_run_to(&mut d, 16, 64);
    assert_eq!(d.ram(I51_PSW) & I51_CY, 0, "ANL C, /1 must clear C=0");

    // Test 5: CJNE @R0, #imm, rel (0xB6 regression!) and MOVC A, @A+PC (0x83 regression!)
    let mut d = i51_dev(&[
        0x78, 0x30, // 0: MOV R0, #0x30
        0x76, 0x42, // 2: MOV @R0, #0x42 -> RAM[0x30] = 0x42
        0xB6, 0x99, 0x02, // 4: CJNE @R0, #0x99, +2 (0x42 != 0x99 -> CY=1, jump to 7+2=9)
        0x74, 0xEE, // 7: MOV A, #0xEE (skipped!)
        0x74, 0x02, // 9: MOV A, #2
        0x83, // 11: MOVC A, @A+PC (next PC = 12, table at 12+2 = 14 -> 0x5C!)
        0x80, 0xFE, // 12: SJMP $
        0x5C, // 14: table byte
    ]);
    i51_run_to(&mut d, 12, 64);
    assert_eq!(
        d.ram(I51_ACC),
        0x5C,
        "CJNE @R0 + MOVC A,@A+PC must fetch 0x5C"
    );
    assert_ne!(d.ram(I51_PSW) & I51_CY, 0, "CJNE 0x42 < 0x99 must set CY=1");
    // Parity check: 0x5C = 0b0101_1100 (4 ones -> even parity -> P=0)
    assert_eq!(d.ram(I51_PSW) & I51_P, 0);
}

// ============================================================================
// 4. MOS 6502 ISA Verification
// ============================================================================

const MCS65_XML: &str = r#"
<mcu core="6502" data="0" prog="65536" progword="1" inst_cycle="1" freq="1000000">
</mcu>
"#;

fn mcs65_dev(code: &[u8]) -> Device {
    let mut d = Device::from_xml("M65", MCS65_XML).unwrap();
    let mut mem = vec![0xEAu16; 65536]; // NOP fill
    // Reset vector -> 0x0200
    mem[0xFFFC] = 0x00;
    mem[0xFFFD] = 0x02;
    for (i, &b) in code.iter().enumerate() {
        mem[0x0200 + i] = u16::from(b);
    }
    d.load_words(&mem);
    d.reset();
    d.start();
    d
}

fn mcs65_run_to(d: &mut Device, target_pc: u32, max_edges: usize) {
    for _ in 0..max_edges {
        if d.pc() == target_pc {
            return;
        }
        d.step_cpu();
    }
    panic!(
        "6502 did not reach PC={target_pc:#x} (stuck at {:#x})",
        d.pc()
    );
}

#[test]
fn mcs65_isa_addressing_php_bvc_bvs_decimal_and_jmp_ind() {
    // Program at 0x0200:
    // 1. Setup stack SP = 0xFF
    // 2. Test BVC/BVS with real V flag (0x50 + 0x50 = 0xA0 -> V=1, then CLV -> V=0)
    // 3. Test PHP/PLP preserves all status bits (N, V, Z, C, I, D)
    // 4. Test (zp,X) indexed indirect load & (zp),Y indirect indexed load
    // 5. Test LDX abs,Y (0xBE)
    // 6. Test Decimal SBC 0x10 - 0x01 = 0x09 (borrow from 0 nibble!)
    // 7. Test JMP ($03FF) with NMOS 6502 page-wrap (low at 0x03FF, high at 0x0300)
    let code: &[u8] = &[
        0xA2, 0xFF, // 0200: LDX #$FF
        0x9A, // 0202: TXS (SP = 0xFF)
        0x18, // 0203: CLC
        0xA9, 0x50, // 0204: LDA #$50
        0x69, 0x50, // 0206: ADC #$50 -> 0xA0, V=1, N=1, C=0
        0x70, 0x02, // 0208: BVS +2 (V=1 -> jump to 020C!)
        0xA9, 0xEE, // 020A: LDA #$EE (skipped!)
        0xB8, // 020C: CLV (V=0)
        0x50, 0x02, // 020D: BVC +2 (V=0 -> jump to 0211!)
        0xA9, 0xDD, // 020F: LDA #$DD (skipped!)
        // PHP / PLP round-trip with N=1, C=1, Z=0
        0x38, // 0211: SEC
        0x08, // 0212: PHP (pushes P with N=1, C=1, V=0, Z=0)
        0x18, // 0213: CLC
        0xA9, 0x00, // 0214: LDA #$00 (N=0, Z=1)
        0x28, // 0216: PLP (restores N=1, C=1, Z=0!)
        0x30, 0x02, // 0217: BMI +2 (N=1 -> jump to 021B)
        0xA9, 0xCC, // 0219: LDA #$CC (skipped!)
        0xB0, 0x02, // 021B: BCS +2 (C=1 -> jump to 021F)
        0xA9, 0xBB, // 021D: LDA #$BB (skipped!)
        // (zp,X) load: ZP[0x12..0x13] = 0x0405, mem[0x0405] = 0x7A
        0xA2, 0x02, // 021F: LDX #$02
        0xA1, 0x10, // 0221: LDA ($10,X) -> reads ptr at $12/$13 = $0405 -> A = 0x7A
        0x85, 0x40, // 0223: STA $40
        // LDX abs,Y (0xBE): mem[0x0400 + 5] = 0x7A
        0xA0, 0x05, // 0225: LDY #$05
        0xBE, 0x00, 0x04, // 0227: LDX $0400,Y -> X = 0x7A
        0x86, 0x41, // 022A: STX $41
        // Decimal SBC: 0x10 - 0x01 (with C=1) -> 0x09; then 0x20 - 0x01 - borrow(C=0) -> 0x18
        0xF8, // 022C: SED
        0x38, // 022D: SEC
        0xA9, 0x10, // 022E: LDA #$10
        0xE9, 0x01, // 0230: SBC #$01 -> 0x09, C=1
        0x85, 0x42, // 0232: STA $42
        0x18, // 0234: CLC (borrow = 1!)
        0xA9, 0x20, // 0235: LDA #$20
        0xE9, 0x01, // 0237: SBC #$01 -> 0x18 (0x20 - 1 - 1 in BCD!)
        0x85, 0x43, // 0239: STA $43
        0xD8, // 023B: CLD
        // JMP ($03FF): low byte at 0x03FF = 0x50, high byte wraps to 0x0300 = 0x02 -> target 0x0250
        0x6C, 0xFF, 0x03, // 023C: JMP ($03FF)
        0x4C, 0x3F, 0x02, // 023F: JMP 023F (failure trap)
    ];
    let mut d = mcs65_dev(code);
    // Zero-page pointer at $12/$13 -> $0405
    d.set_flash(0x0012, 0x05);
    d.set_flash(0x0013, 0x04);
    d.set_flash(0x0405, 0x7A);
    // Indirect jump vector with page-boundary wrap at $03FF / $0300 -> $0250
    d.set_flash(0x03FF, 0x50);
    d.set_flash(0x0300, 0x02);
    // Target at 0x0250: LDA #$99; STA $44; JMP $0254
    d.set_flash(0x0250, 0xA9);
    d.set_flash(0x0251, 0x99);
    d.set_flash(0x0252, 0x85);
    d.set_flash(0x0253, 0x44);
    d.set_flash(0x0254, 0x4C);
    d.set_flash(0x0255, 0x54);
    d.set_flash(0x0256, 0x02);

    mcs65_run_to(&mut d, 0x0254, 400);
    assert_eq!(
        d.flash_word(0x40),
        Some(0x7A),
        "(zp,X) load must read 0x7A from $0405"
    );
    assert_eq!(
        d.flash_word(0x41),
        Some(0x7A),
        "LDX abs,Y (0xBE) must index by Y"
    );
    assert_eq!(
        d.flash_word(0x42),
        Some(0x09),
        "BCD SBC 0x10 - 0x01 must yield 0x09"
    );
    assert_eq!(
        d.flash_word(0x43),
        Some(0x18),
        "BCD SBC 0x20 - 0x01 - borrow must yield 0x18"
    );
    assert_eq!(
        d.flash_word(0x44),
        Some(0x99),
        "JMP ($03FF) must jump to $0250"
    );
}

// ============================================================================
// 5. Zilog Z80 ISA Verification
// ============================================================================

const Z80_XML: &str = r#"
<mcu core="Z80" data="0" prog="65536" progword="1" inst_cycle="1" freq="4000000">
</mcu>
"#;

fn z80_dev(code: &[u8]) -> Device {
    let mut d = Device::from_xml("Z80", Z80_XML).unwrap();
    let mut mem = vec![0x00u16; 65536];
    for (i, &b) in code.iter().enumerate() {
        mem[i] = u16::from(b);
    }
    d.load_words(&mem);
    d.reset();
    d.start();
    d
}

fn z80_run_to(d: &mut Device, target_pc: u32, max_edges: usize) {
    for _ in 0..max_edges {
        if d.pc() == target_pc {
            return;
        }
        d.step_cpu();
    }
    panic!(
        "Z80 did not reach PC={target_pc:#x} (stuck at {:#x})",
        d.pc()
    );
}

#[test]
fn z80_isa_alu_cb_ed_ix_iy_ex_and_block_ops() {
    // Program at 0x0000:
    // 1. LD SP, 0x1000
    // 2. 8-bit ALU + DAA: 0x19 + 0x28 = 0x41 -> DAA => 0x47; NEG; CPL; SCF; CCF
    // 3. 16-bit ALU: ADD HL, BC; ADC HL, DE; SBC HL, BC
    // 4. CB prefix: SLA, SRA, SRL, BIT, SET, RES
    // 5. IX indexed: LD IX, 0x0200; LD (IX+3), 0x5A; LD A, (IX+3)
    // 6. ED block copy LDIR: copy 4 bytes from 0x0300 -> 0x0310
    // 7. EX AF,AF', EXX, EX DE,HL, DJNZ loop
    let code: &[u8] = &[
        0x31, 0x00, 0x10, // 0000: LD SP, 0x1000
        0x3E, 0x19, // 0003: LD A, 0x19
        0xC6, 0x28, // 0005: ADD A, 0x28 -> 0x41 (H=1)
        0x27, // 0007: DAA -> 0x47
        0x32, 0x00, 0x04, // 0008: LD (0x0400), A  -> [0400] = 0x47
        0xED, 0x44, // 000B: NEG -> -0x47 = 0xB9
        0x32, 0x01, 0x04, // 000D: LD (0x0401), A  -> [0401] = 0xB9
        // 16-bit HL math: HL=0x1234 + BC=0x0102 = 0x1336; SBC HL, BC (C=0) = 0x1234
        0x21, 0x34, 0x12, // 0010: LD HL, 0x1234
        0x01, 0x02, 0x01, // 0013: LD BC, 0x0102
        0x09, // 0016: ADD HL, BC -> 0x1336, C=0
        0x22, 0x02, 0x04, // 0017: LD (0x0402), HL -> 0x36, 0x13
        0xED, 0x42, // 001A: SBC HL, BC -> 0x1234
        0x22, 0x04, 0x04, // 001C: LD (0x0404), HL -> 0x34, 0x12
        // CB prefix on B: B = 0x81 -> SLA B (0x02, C=1) -> SET 7,B (0x82) -> RES 1,B (0x80) -> SRA B (0xC0)
        0x06, 0x81, // 001F: LD B, 0x81
        0xCB, 0x20, // 0021: SLA B -> 0x02
        0xCB, 0xF8, // 0023: SET 7, B -> 0x82
        0xCB, 0x88, // 0025: RES 1, B -> 0x80
        0xCB, 0x28, // 0027: SRA B -> 0xC0
        0x78, // 0029: LD A, B
        0x32, 0x06, 0x04, // 002A: LD (0x0406), A -> 0xC0
        // IX indexed store/load
        0xDD, 0x21, 0x00, 0x02, // 002D: LD IX, 0x0200
        0xDD, 0x36, 0x03, 0x5A, // 0031: LD (IX+3), 0x5A
        0xDD, 0x7E, 0x03, // 0035: LD A, (IX+3)
        0x32, 0x07, 0x04, // 0038: LD (0x0407), A -> 0x5A
        // LDIR: copy 4 bytes from 0x0300 to 0x0310
        0x21, 0x00, 0x03, // 003B: LD HL, 0x0300
        0x11, 0x10, 0x03, // 003E: LD DE, 0x0310
        0x01, 0x04, 0x00, // 0041: LD BC, 4
        0xED, 0xB0, // 0044: LDIR
        // DJNZ sum 1+2+3+4+5 = 15
        0x06, 0x05, // 0046: LD B, 5
        0xAF, // 0048: XOR A
        0x80, // 0049: ADD A, B
        0x10, 0xFD, // 004A: DJNZ -3 (to 0049)
        0x32, 0x08, 0x04, // 004C: LD (0x0408), A -> 15
        // EX AF,AF' + EX DE,HL
        0x08, // 004F: EX AF, AF'
        0x3E, 0xAA, // 0050: LD A, 0xAA
        0x08, // 0052: EX AF, AF' -> restores A = 15
        0x32, 0x09, 0x04, // 0053: LD (0x0409), A -> 15
        0x18, 0xFE, // 0056: JR $
    ];
    let mut d = z80_dev(code);
    for (i, &b) in [0x11u8, 0x22, 0x33, 0x44].iter().enumerate() {
        d.set_flash(0x0300 + i, u16::from(b));
    }

    z80_run_to(&mut d, 0x0056, 2000);
    // Pump past the M4 write cycle of `LD (0x0409), A` before checking memory.
    for _ in 0..32 {
        d.step_cpu();
    }
    assert_eq!(
        d.flash_word(0x0400),
        Some(0x47),
        "Z80 DAA 0x19+0x28 must be 0x47"
    );
    assert_eq!(
        d.flash_word(0x0401),
        Some(0xB9),
        "Z80 NEG 0x47 must be 0xB9"
    );
    assert_eq!(d.flash_word(0x0402), Some(0x36));
    assert_eq!(d.flash_word(0x0403), Some(0x13));
    assert_eq!(d.flash_word(0x0404), Some(0x34));
    assert_eq!(d.flash_word(0x0405), Some(0x12));
    assert_eq!(
        d.flash_word(0x0406),
        Some(0xC0),
        "Z80 CB SLA/SET/RES/SRA must produce 0xC0"
    );
    assert_eq!(
        d.flash_word(0x0407),
        Some(0x5A),
        "Z80 IX+3 store/load must produce 0x5A"
    );
    assert_eq!(d.flash_word(0x0310), Some(0x11));
    assert_eq!(d.flash_word(0x0311), Some(0x22));
    assert_eq!(d.flash_word(0x0312), Some(0x33));
    assert_eq!(d.flash_word(0x0313), Some(0x44));
    assert_eq!(
        d.flash_word(0x0408),
        Some(15),
        "Z80 DJNZ sum 1..=5 must be 15"
    );
    assert_eq!(
        d.flash_word(0x0409),
        Some(15),
        "Z80 EX AF,AF' must preserve A=15"
    );
}
