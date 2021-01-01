//! Intel HEX matching C++ `MemData::loadHex`.
//!
//! `bits` is `wordSize * 8` (PIC flash is 16). Byte addresses are divided by
//! `bits/8`. Type 1 is EOF; type 4 is Extended Linear Address. Other types
//! are skipped with a warning (treated as success for the record). Reaching
//! the image end returns Ok (C++ warns and still succeeds). Missing EOF is
//! an error.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HexError {
    StartCode { line: usize },
    LineSize { line: usize },
    Checksum { line: usize },
    NoEof,
    Parse { line: usize, msg: String },
}

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HexError::StartCode { line } => write!(f, "wrong start code at line {line}"),
            HexError::LineSize { line } => write!(f, "wrong line size at line {line}"),
            HexError::Checksum { line } => write!(f, "checksum error at line {line}"),
            HexError::NoEof => write!(f, "no end of file record"),
            HexError::Parse { line, msg } => write!(f, "line {line}: {msg}"),
        }
    }
}

impl std::error::Error for HexError {}

/// Load Intel HEX into `image`. `bits` is word width (8 or 16).
///
/// When `resize` is false, words past `image.len()` are ignored (Ok), matching
/// C++ "PGM End reached" (config words are not applied here).
pub fn load_hex(src: &str, image: &mut Vec<u16>, resize: bool, bits: u32) -> Result<(), HexError> {
    let word_size = (bits / 8).max(1) as usize;
    let mut addr_base: u32 = 0;
    let mut n_line = 0usize;

    for raw in src.lines() {
        let line: String = raw.chars().filter(|c| *c != ' ').collect();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with(':') {
            return Err(HexError::StartCode { line: n_line });
        }
        let body = &line[1..];
        if body.len() < 10 || body.len() % 2 != 0 {
            return Err(HexError::LineSize { line: n_line });
        }
        let n_bytes = parse_hex_u32(&body[0..2], n_line)? as usize;
        let line_size = 2 + 4 + 2 + n_bytes * 2 + 2;
        if body.len() != line_size {
            return Err(HexError::LineSize { line: n_line });
        }

        let mut checksum: u32 = n_bytes as u32;
        checksum += parse_hex_u32(&body[2..4], n_line)?;
        checksum += parse_hex_u32(&body[4..6], n_line)?;
        let rec_addr = parse_hex_u32(&body[2..6], n_line)?;
        let rec_type = parse_hex_u32(&body[6..8], n_line)?;
        checksum += rec_type;

        if rec_type == 1 {
            let cs = parse_hex_u32(&body[8..10], n_line)?;
            checksum += cs;
            if checksum & 0xFF != 0 {
                return Err(HexError::Checksum { line: n_line + 1 });
            }
            return Ok(());
        }
        if rec_type == 4 {
            // Extended Linear Address: two bytes payload.
            for i in 0..n_bytes {
                checksum += parse_hex_u32(&body[8 + i * 2..10 + i * 2], n_line)?;
            }
            addr_base = parse_hex_u32(&body[8..12], n_line)? << 16;
            let cs = parse_hex_u32(&body[8 + n_bytes * 2..10 + n_bytes * 2], n_line)?;
            checksum += cs;
            if checksum & 0xFF != 0 {
                return Err(HexError::Checksum { line: n_line + 1 });
            }
            n_line += 1;
            continue;
        }
        if rec_type != 0 {
            for i in 0..n_bytes {
                checksum += parse_hex_u32(&body[8 + i * 2..10 + i * 2], n_line)?;
            }
            let cs = parse_hex_u32(&body[8 + n_bytes * 2..10 + n_bytes * 2], n_line)?;
            checksum += cs;
            if checksum & 0xFF != 0 {
                return Err(HexError::Checksum { line: n_line + 1 });
            }
            n_line += 1;
            continue;
        }

        let mut addr = ((addr_base + rec_addr) / word_size as u32) as usize;
        let mut i = 8usize;
        while i < 8 + n_bytes * 2 {
            let mut data = parse_hex_u32(&body[i..i + 2], n_line)?;
            checksum += data;
            i += 2;
            if word_size == 2 {
                if i + 2 > 8 + n_bytes * 2 {
                    return Err(HexError::Parse {
                        line: n_line,
                        msg: "truncated word".into(),
                    });
                }
                let hi = parse_hex_u32(&body[i..i + 2], n_line)?;
                checksum += hi;
                data += hi << 8;
                i += 2;
            }
            if resize {
                if addr >= image.len() {
                    image.resize(addr + 1, 0xFFFF);
                }
                image[addr] = data as u16;
            } else if addr < image.len() {
                image[addr] = data as u16;
            }
            // C++: past end of PGM is a warning and success (config word).
            addr += 1;
        }
        let cs = parse_hex_u32(&body[i..i + 2], n_line)?;
        checksum += cs;
        if checksum & 0xFF != 0 {
            return Err(HexError::Checksum { line: n_line + 1 });
        }
        n_line += 1;
    }
    Err(HexError::NoEof)
}

fn parse_hex_u32(s: &str, line: usize) -> Result<u32, HexError> {
    u32::from_str_radix(s, 16).map_err(|_| HexError::Parse {
        line,
        msg: format!("bad hex `{s}`"),
    })
}

/// Encode `words` at word-index `start` as Intel HEX (little-endian when 16-bit).
pub fn encode_hex(start_word: u16, words: &[u16], bits: u32) -> String {
    let word_size = (bits / 8).max(1) as usize;
    let mut data = Vec::new();
    for w in words {
        data.push(*w as u8);
        if word_size == 2 {
            data.push((*w >> 8) as u8);
        }
    }
    let byte_addr = start_word as u32 * word_size as u32;
    let mut out = String::new();
    // Split into 16-byte records.
    let mut off = 0usize;
    while off < data.len() {
        let n = (data.len() - off).min(16);
        let addr = byte_addr + off as u32;
        let rec = &data[off..off + n];
        let mut bytes = vec![n as u8, (addr >> 8) as u8, addr as u8, 0];
        bytes.extend_from_slice(rec);
        let sum: u32 = bytes.iter().map(|&b| b as u32).sum();
        let cs = (!(sum as u8)).wrapping_add(1);
        out.push(':');
        for b in bytes {
            out.push_str(&format!("{b:02X}"));
        }
        out.push_str(&format!("{cs:02X}\n"));
        off += n;
    }
    out.push_str(":00000001FF\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_16bit_words() {
        let words = [0x3055u16, 0x00A0];
        let src = encode_hex(0, &words, 16);
        let mut image = vec![0x3FFF; 8];
        load_hex(&src, &mut image, false, 16).unwrap();
        assert_eq!(image[0], 0x3055);
        assert_eq!(image[1], 0x00A0);
        assert_eq!(image[2], 0x3FFF);
    }

    #[test]
    fn checksum_detects_corruption() {
        let src = ":020000003001CCxx\n:00000001FF\n";
        let mut image = vec![0; 4];
        assert!(load_hex(src, &mut image, false, 16).is_err());
    }

    #[test]
    fn eof_required() {
        let src = encode_hex(0, &[0x3000], 16);
        let trimmed = src.replace(":00000001FF\n", "");
        let mut image = vec![0; 4];
        assert_eq!(
            load_hex(&trimmed, &mut image, false, 16).unwrap_err(),
            HexError::NoEof
        );
    }
}
