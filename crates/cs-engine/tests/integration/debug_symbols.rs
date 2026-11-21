use cs_engine::debug::symbols::{ElfParser, LstKind, LstParser, SourceLocation};
use std::path::Path;

#[test]
fn test_gpasm_lst_parser() {
    let lst_content = r#"
                      00001 ; PIC16F84 test
                      00002 list p=16f84
0000 3005             00003     movlw 0x05
0001 0086             00004     movwf PORTB
0002 2800             00005     goto 0
"#;
    let syms = LstParser::parse(lst_content, Path::new("main.asm"), LstKind::Gpasm);
    assert_eq!(
        syms.lookup_address(0),
        Some(&SourceLocation::new("main.asm", 3))
    );
    assert_eq!(
        syms.lookup_address(1),
        Some(&SourceLocation::new("main.asm", 4))
    );
    assert_eq!(
        syms.lookup_address(2),
        Some(&SourceLocation::new("main.asm", 5))
    );

    let lines = syms.lookup_lines(Path::new("main.asm"), 3);
    assert_eq!(lines, vec![0]);
}

#[test]
fn test_avra_lst_parser() {
    let lst_content = r#"; AVR Test
000000 e005              ldi r16, 0x05
000001 b908              out 0x18, r16
000002 9508              ret
"#;
    let syms = LstParser::parse(lst_content, Path::new("test.asm"), LstKind::Avra);
    assert_eq!(
        syms.lookup_address(0),
        Some(&SourceLocation::new("test.asm", 2))
    );
    assert_eq!(
        syms.lookup_address(1),
        Some(&SourceLocation::new("test.asm", 3))
    );
    assert_eq!(
        syms.lookup_address(2),
        Some(&SourceLocation::new("test.asm", 4))
    );
}

#[test]
fn test_sdcc_lst_parser() {
    let lst_content = r#"; SDCC 8051 listing
      000000                      _main:
      000000 75 90 55      [24]     mov _P1,#0x55
      000003 22            [12]     ret
"#;
    let syms = LstParser::parse(lst_content, Path::new("main.c"), LstKind::Sdcc);
    assert_eq!(
        syms.lookup_address(0),
        Some(&SourceLocation::new("main.c", 3))
    );
    assert_eq!(
        syms.lookup_address(3),
        Some(&SourceLocation::new("main.c", 4))
    );
}

#[test]
fn test_ca65_lst_parser() {
    let lst_content = r#"000000r 1  A9 42               lda #$42
000002r 1  8D 00 20            sta $2000
000005r 1  60                  rts
"#;
    let syms = LstParser::parse(lst_content, Path::new("main.s"), LstKind::Ca65);
    assert_eq!(
        syms.lookup_address(0),
        Some(&SourceLocation::new("main.s", 1))
    );
    assert_eq!(
        syms.lookup_address(2),
        Some(&SourceLocation::new("main.s", 2))
    );
    assert_eq!(
        syms.lookup_address(5),
        Some(&SourceLocation::new("main.s", 3))
    );
}

#[test]
fn test_auto_detect_lst_kind() {
    let lst_pic = "0000 3005             00003     movlw 0x05\n";
    let syms = LstParser::parse(lst_pic, Path::new("test.asm"), LstKind::Auto);
    assert_eq!(
        syms.lookup_address(0),
        Some(&SourceLocation::new("test.asm", 1))
    );
}

#[test]
fn test_elf_parser_empty_or_invalid() {
    let invalid_bytes = vec![0x7f, b'E', b'L', b'F', 0, 0, 0];
    let syms = ElfParser::parse(&invalid_bytes);
    assert!(syms.is_none());
}
