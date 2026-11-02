//! UART, I2C, and SPI decoders over sampled digital traces.
//!
//! Levels at or above `threshold` are high. UART is 8N1, LSB first, idle high.
//! I2C samples SDA on each rising SCL, MSB first, and treats a low 9th bit as
//! an ack. SPI mode 0 samples MOSI and MISO on rising SCK while CS is low,
//! MSB first.

/// One decoded UART byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UartByte {
    pub start_sample: usize,
    pub byte: u8,
    pub framing_ok: bool,
}

/// One I2C transaction, from start through the following data bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct I2cTransaction {
    pub start_sample: usize,
    /// 7-bit address.
    pub address: u8,
    pub read: bool,
    pub address_ack: bool,
    pub data: Vec<u8>,
    pub data_acks: Vec<bool>,
}

/// Eight bits shifted on one chip-select burst.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpiByte {
    pub start_sample: usize,
    pub mosi: u8,
    pub miso: u8,
}

/// 8N1 UART. `sample_hz / baud` is the samples per bit.
pub fn decode_uart(samples: &[f64], sample_hz: f64, baud: f64, threshold: f64) -> Vec<UartByte> {
    if samples.len() < 2 || sample_hz <= 0.0 || baud <= 0.0 {
        return Vec::new();
    }
    let spb = sample_hz / baud;
    if spb < 1.0 {
        return Vec::new();
    }
    let high = |i: usize| samples.get(i).copied().unwrap_or(threshold) >= threshold;
    let mut out = Vec::new();
    let mut i = 1usize;
    while i < samples.len() {
        if high(i.wrapping_sub(1)) && !high(i) {
            let start = i;
            let mut byte = 0u8;
            let mut ok = true;
            if high(center(start, 0.0, spb)) {
                ok = false;
            }
            for bit in 0..8 {
                if high(center(start, 1.0 + bit as f64, spb)) {
                    byte |= 1 << bit;
                }
            }
            if !high(center(start, 9.0, spb)) {
                ok = false;
            }
            let stop_at = center(start, 9.0, spb);
            if stop_at >= samples.len() {
                break;
            }
            out.push(UartByte {
                start_sample: start,
                byte,
                framing_ok: ok,
            });
            i = stop_at.saturating_add(1);
        } else {
            i += 1;
        }
    }
    out
}

fn center(edge: usize, bit: f64, spb: f64) -> usize {
    edge + ((bit + 0.5) * spb).round() as usize
}

/// `scl` and `sda` are the same length. A short slice is ignored.
pub fn decode_i2c(scl: &[f64], sda: &[f64], threshold: f64) -> Vec<I2cTransaction> {
    let n = scl.len().min(sda.len());
    if n < 2 {
        return Vec::new();
    }
    let hi = |v: f64| v >= threshold;
    let mut out = Vec::new();
    let mut active = false;
    let mut bits: Vec<bool> = Vec::new();
    let mut start_at = 0usize;
    let mut current: Option<I2cTransaction> = None;
    let mut got_addr = false;

    let flush_byte = |bits: &mut Vec<bool>,
                      current: &mut Option<I2cTransaction>,
                      got_addr: &mut bool,
                      start_at: usize| {
        if bits.len() < 9 {
            return;
        }
        let chunk: Vec<bool> = bits.drain(..9).collect();
        let mut byte = 0u8;
        for (i, bit) in chunk.iter().take(8).enumerate() {
            if *bit {
                byte |= 1 << (7 - i);
            }
        }
        let ack = !chunk[8];
        let txn = current.get_or_insert(I2cTransaction {
            start_sample: start_at,
            address: 0,
            read: false,
            address_ack: false,
            data: Vec::new(),
            data_acks: Vec::new(),
        });
        if !*got_addr {
            txn.address = byte >> 1;
            txn.read = byte & 1 == 1;
            txn.address_ack = ack;
            *got_addr = true;
        } else {
            txn.data.push(byte);
            txn.data_acks.push(ack);
        }
    };

    for i in 1..n {
        let scl_hi = hi(scl[i]);
        let sda_fall = hi(sda[i - 1]) && !hi(sda[i]);
        let sda_rise = !hi(sda[i - 1]) && hi(sda[i]);
        let scl_rise = !hi(scl[i - 1]) && scl_hi;
        if scl_hi && sda_fall {
            if let Some(txn) = current.take() {
                out.push(txn);
            }
            active = true;
            bits.clear();
            got_addr = false;
            start_at = i;
            current = Some(I2cTransaction {
                start_sample: start_at,
                address: 0,
                read: false,
                address_ack: false,
                data: Vec::new(),
                data_acks: Vec::new(),
            });
        } else if scl_hi && sda_rise && active {
            flush_byte(&mut bits, &mut current, &mut got_addr, start_at);
            if let Some(txn) = current.take() {
                out.push(txn);
            }
            active = false;
            bits.clear();
            got_addr = false;
        } else if active && scl_rise {
            bits.push(hi(sda[i]));
            if bits.len() == 9 {
                flush_byte(&mut bits, &mut current, &mut got_addr, start_at);
            }
        }
    }
    if let Some(txn) = current.take() {
        if txn.address_ack || !txn.data.is_empty() || txn.read || txn.address != 0 {
            out.push(txn);
        }
    }
    out
}

/// SPI mode 0. `cs` low selects the device. Bits are MSB first.
pub fn decode_spi(
    sck: &[f64],
    mosi: &[f64],
    miso: &[f64],
    cs: &[f64],
    threshold: f64,
) -> Vec<SpiByte> {
    let n = sck.len().min(mosi.len()).min(miso.len()).min(cs.len());
    if n < 2 {
        return Vec::new();
    }
    let hi = |v: f64| v >= threshold;
    let mut out = Vec::new();
    let mut mosi_acc = 0u8;
    let mut miso_acc = 0u8;
    let mut count = 0u8;
    let mut start = 0usize;
    for i in 1..n {
        let selected = !hi(cs[i]);
        let rise = !hi(sck[i - 1]) && hi(sck[i]);
        if !selected {
            mosi_acc = 0;
            miso_acc = 0;
            count = 0;
            continue;
        }
        if rise {
            if count == 0 {
                start = i;
            }
            mosi_acc = (mosi_acc << 1) | u8::from(hi(mosi[i]));
            miso_acc = (miso_acc << 1) | u8::from(hi(miso[i]));
            count += 1;
            if count == 8 {
                out.push(SpiByte {
                    start_sample: start,
                    mosi: mosi_acc,
                    miso: miso_acc,
                });
                mosi_acc = 0;
                miso_acc = 0;
                count = 0;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(bits: &[(bool, usize)]) -> Vec<f64> {
        let mut out = Vec::new();
        for &(high, n) in bits {
            out.extend(std::iter::repeat(if high { 1.0 } else { 0.0 }).take(n));
        }
        out
    }

    #[test]
    fn uart_reads_0x55() {
        let spb = 10usize;
        // idle, start, then 0x55 LSB first (1,0,1,0,1,0,1,0), stop.
        let mut pattern = vec![(true, spb), (false, spb)];
        for bit in 0..8 {
            pattern.push(((0x55u8 >> bit) & 1 == 1, spb));
        }
        pattern.push((true, spb));
        pattern.push((true, spb));
        let wave = level(&pattern);
        let got = decode_uart(&wave, 10_000.0, 1_000.0, 0.5);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].byte, 0x55);
        assert!(got[0].framing_ok);
    }

    #[test]
    fn i2c_address_write_with_ack() {
        // Start, address 0x50 write (0xA0), ack, data 0x01, ack, stop.
        // SCL idles high. Each bit: SCL low, then high.
        let bit_slots = |byte: u8, ack_low: bool| -> (Vec<f64>, Vec<f64>) {
            let mut scl = Vec::new();
            let mut sda = Vec::new();
            for i in 0..8 {
                let high = (byte >> (7 - i)) & 1 == 1;
                scl.extend_from_slice(&[0.0, 1.0]);
                sda.extend_from_slice(&[if high { 1.0 } else { 0.0 }; 2]);
            }
            scl.extend_from_slice(&[0.0, 1.0]);
            let ack = if ack_low { 0.0 } else { 1.0 };
            sda.extend_from_slice(&[ack, ack]);
            (scl, sda)
        };
        let (a_scl, a_sda) = bit_slots(0xA0, true);
        let (d_scl, d_sda) = bit_slots(0x01, true);
        // idle high, start (SDA falls while SCL high), payload, stop (SDA rises while SCL high).
        let mut scl = vec![1.0, 1.0, 1.0];
        let mut sda = vec![1.0, 1.0, 0.0];
        scl.extend(a_scl);
        sda.extend(a_sda);
        scl.extend(d_scl);
        sda.extend(d_sda);
        scl.extend_from_slice(&[1.0, 1.0]);
        sda.extend_from_slice(&[0.0, 1.0]);
        let txns = decode_i2c(&scl, &sda, 0.5);
        assert_eq!(txns.len(), 1, "{txns:?}");
        assert_eq!(txns[0].address, 0x50);
        assert!(!txns[0].read);
        assert!(txns[0].address_ack);
        assert_eq!(txns[0].data, vec![0x01]);
        assert_eq!(txns[0].data_acks, vec![true]);
    }

    #[test]
    fn spi_mode0_shifts_msb_first() {
        // CS low, eight rising clocks, MOSI 0xA5, MISO 0x3C.
        let mosi_byte = 0xA5u8;
        let miso_byte = 0x3Cu8;
        let mut sck = vec![0.0];
        let mut mosi = vec![0.0];
        let mut miso = vec![0.0];
        let mut cs = vec![0.0];
        for bit in 0..8 {
            let mo = if (mosi_byte >> (7 - bit)) & 1 == 1 {
                1.0
            } else {
                0.0
            };
            let mi = if (miso_byte >> (7 - bit)) & 1 == 1 {
                1.0
            } else {
                0.0
            };
            sck.extend_from_slice(&[0.0, 1.0]);
            mosi.extend_from_slice(&[mo, mo]);
            miso.extend_from_slice(&[mi, mi]);
            cs.extend_from_slice(&[0.0, 0.0]);
        }
        let got = decode_spi(&sck, &mosi, &miso, &cs, 0.5);
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(got[0].mosi, 0xA5);
        assert_eq!(got[0].miso, 0x3C);
    }
}
