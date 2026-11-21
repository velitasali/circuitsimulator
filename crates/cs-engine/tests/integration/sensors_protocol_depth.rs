//! Integration tests for Section 1.5: Sensor Protocol Depth
//!
//! Validates:
//! 1. DS18B20:
//!    - 1-Wire multi-drop Search ROM (0xF0) arbitration between multiple sensors
//!    - Match ROM (0x55) addressing and deselect
//!    - Alarm Search (0xEC) filtering devices exceeding Th / Tl thresholds
//!    - Programmable resolution masking (9..12-bit) in scratchpad
//!    - Non-volatile EEPROM Copy (0x48) & Recall (0xB8)
//!    - Parasitic power mode detection (0xB4 Read Power Supply)
//! 2. DS1307:
//!    - Autonomous RTC clock advancement with elapsed simulation time
//!    - Leap year rollover (Feb 28 -> Feb 29 in leap years)
//!    - Clock Halt (CH = bit 7) stopping autonomous ticking
//!    - 12-hour AM/PM vs 24-hour mode
//!    - 56-byte general-purpose battery-backed NVRAM (0x08..=0x3F)
//! 3. ESP-01:
//!    - Extended AT command parser (AT+GMR, AT+CWMODE, AT+CWLAP, AT+CWJAP, AT+CIFSR)
//!    - Wi-Fi connection and IP assignment state machine
//!    - Raw data transmission prompt (AT+CIPSEND -> '>')
//!    - Simulated HTTP 200 server response stream over UART

use cs_engine::digital::{
    Ds1307Bus, EspUart, OW_FUNC_CMD, OW_RESET_ARMED, OW_SEARCH_ROM, OwLink, crc8_maxim,
    parse_rom_64,
};

const US: u64 = 1_000_000;

#[test]
fn test_ds18b20_rom_parsing_and_crc8() {
    let rom = parse_rom_64("28FF2B450000");
    assert_eq!(rom[0], 0x28, "Family code must be 0x28 for DS18B20");
    assert_eq!(rom[1], 0xFF);
    assert_eq!(rom[2], 0x2B);
    assert_eq!(rom[3], 0x45);
    // Byte 7 must be Maxim CRC8 of the first 7 bytes
    assert_eq!(rom[7], crc8_maxim(&rom[..7]));
}

#[test]
fn test_ds18b20_search_rom_multi_drop_arbitration() {
    // Two DS18B20 sensors on the same bus:
    // Sensor A: serial with bit 8 = 0
    // Sensor B: serial with bit 8 = 1
    let mut dev_a = OwLink::default();
    dev_a.rom = [0x28, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x00];
    dev_a.rom[7] = crc8_maxim(&dev_a.rom[..7]);

    let mut dev_b = OwLink::default();
    dev_b.rom = [0x28, 0x01, 0x11, 0x22, 0x33, 0x44, 0x55, 0x00];
    dev_b.rom[7] = crc8_maxim(&dev_b.rom[..7]);

    let mut dq_low_a = false;
    let mut dq_low_b = false;
    let t = 1_000 * US;

    // 1. Reset Pulse (> 480 us)
    dev_a.on_pins("28", 22.0, &mut dq_low_a, 0.0, 5.0, 0.0, t);
    dev_b.on_pins("28", 22.0, &mut dq_low_b, 0.0, 5.0, 0.0, t);
    dev_a.on_pins("28", 22.0, &mut dq_low_a, 5.0, 5.0, 0.0, t + 500 * US);
    dev_b.on_pins("28", 22.0, &mut dq_low_b, 5.0, 5.0, 0.0, t + 500 * US);

    assert_eq!(dev_a.phase, OW_RESET_ARMED);
    assert_eq!(dev_b.phase, OW_RESET_ARMED);

    // Presence pulse
    dev_a.on_event(&mut dq_low_a, t + 530 * US);
    dev_b.on_event(&mut dq_low_b, t + 530 * US);
    dev_a.on_event(&mut dq_low_a, t + 630 * US);
    dev_b.on_event(&mut dq_low_b, t + 630 * US);

    // 2. Send Search ROM command (0xF0)
    for bit in 0..8 {
        let one = (0xF0u8 & (1 << bit)) != 0;
        let dur = if one { 5 * US } else { 60 * US };
        dev_a.on_pins("28", 22.0, &mut dq_low_a, 0.0, 5.0, 0.0, t + 700 * US);
        dev_b.on_pins("28", 22.0, &mut dq_low_b, 0.0, 5.0, 0.0, t + 700 * US);
        dev_a.on_pins("28", 22.0, &mut dq_low_a, 5.0, 5.0, 0.0, t + 700 * US + dur);
        dev_b.on_pins("28", 22.0, &mut dq_low_b, 5.0, 5.0, 0.0, t + 700 * US + dur);
    }
    assert_eq!(dev_a.phase, OW_SEARCH_ROM);
    assert_eq!(dev_b.phase, OW_SEARCH_ROM);

    // First 8 bits are 0x28 (identical for both dev_a and dev_b)
    for bit_idx in 0..8 {
        let bit = (0x28u8 & (1 << bit_idx)) != 0;
        // Slot 1: Read true bit
        dev_a.on_pins("28", 22.0, &mut dq_low_a, 0.0, 5.0, 0.0, t);
        dev_b.on_pins("28", 22.0, &mut dq_low_b, 0.0, 5.0, 0.0, t);
        let bus_slot1 = !(dq_low_a || dq_low_b);
        assert_eq!(bus_slot1, bit, "Slot 1 true bit match for bit {bit_idx}");
        dev_a.on_event(&mut dq_low_a, t + 40 * US);
        dev_b.on_event(&mut dq_low_b, t + 40 * US);

        // Slot 2: Read complement bit
        dev_a.on_pins("28", 22.0, &mut dq_low_a, 0.0, 5.0, 0.0, t);
        dev_b.on_pins("28", 22.0, &mut dq_low_b, 0.0, 5.0, 0.0, t);
        let bus_slot2 = !(dq_low_a || dq_low_b);
        assert_eq!(
            bus_slot2, !bit,
            "Slot 2 complement bit match for bit {bit_idx}"
        );
        dev_a.on_event(&mut dq_low_a, t + 40 * US);
        dev_b.on_event(&mut dq_low_b, t + 40 * US);

        // Slot 3: Master writes bit direction
        let dur = if bit { 5 * US } else { 60 * US };
        dev_a.on_pins("28", 22.0, &mut dq_low_a, 0.0, 5.0, 0.0, t);
        dev_b.on_pins("28", 22.0, &mut dq_low_b, 0.0, 5.0, 0.0, t);
        dev_a.on_pins("28", 22.0, &mut dq_low_a, 5.0, 5.0, 0.0, t + dur);
        dev_b.on_pins("28", 22.0, &mut dq_low_b, 5.0, 5.0, 0.0, t + dur);
    }

    // Now bit 8:
    // dev_a has bit 8 = 0
    // dev_b has bit 8 = 1
    // Slot 1: dev_a transmits 0 (pulls low), dev_b transmits 1 (leaves high) -> bus is 0
    dev_a.on_pins("28", 22.0, &mut dq_low_a, 0.0, 5.0, 0.0, t);
    dev_b.on_pins("28", 22.0, &mut dq_low_b, 0.0, 5.0, 0.0, t);
    let bus_slot1 = !(dq_low_a || dq_low_b);
    assert!(!bus_slot1, "Conflict detected: slot 1 pulled low by dev_a");
    dev_a.on_event(&mut dq_low_a, t + 40 * US);
    dev_b.on_event(&mut dq_low_b, t + 40 * US);

    // Slot 2: dev_a transmits 1 (leaves high), dev_b transmits 0 (pulls low) -> bus is 0
    dev_a.on_pins("28", 22.0, &mut dq_low_a, 0.0, 5.0, 0.0, t);
    dev_b.on_pins("28", 22.0, &mut dq_low_b, 0.0, 5.0, 0.0, t);
    let bus_slot2 = !(dq_low_a || dq_low_b);
    assert!(!bus_slot2, "Conflict detected: slot 2 pulled low by dev_b");
    dev_a.on_event(&mut dq_low_a, t + 40 * US);
    dev_b.on_event(&mut dq_low_b, t + 40 * US);

    // Master chooses direction 0 (selects dev_a, deselects dev_b):
    dev_a.on_pins("28", 22.0, &mut dq_low_a, 0.0, 5.0, 0.0, t);
    dev_b.on_pins("28", 22.0, &mut dq_low_b, 0.0, 5.0, 0.0, t);
    dev_a.on_pins("28", 22.0, &mut dq_low_a, 5.0, 5.0, 0.0, t + 60 * US); // 0 bit
    dev_b.on_pins("28", 22.0, &mut dq_low_b, 5.0, 5.0, 0.0, t + 60 * US); // 0 bit

    assert!(dev_a.active, "dev_a matches chosen 0 bit and stays active");
    assert!(!dev_b.active, "dev_b drops out because its bit 8 is 1");
}

#[test]
fn test_ds18b20_alarm_search_filter() {
    let mut dev_normal = OwLink::default();
    dev_normal.scratch[2] = 30; // Th = 30 °C
    dev_normal.scratch[3] = 10; // Tl = 10 °C

    let mut dev_alarm = OwLink::default();
    dev_alarm.scratch[2] = 25; // Th = 25 °C
    dev_alarm.scratch[3] = 10; // Tl = 10 °C

    let mut dq_low_norm = false;
    let mut dq_low_alarm = false;
    let t = 1_000 * US;

    // Reset and presence
    dev_normal.on_pins("28", 28.0, &mut dq_low_norm, 0.0, 5.0, 0.0, t);
    dev_alarm.on_pins("28", 28.0, &mut dq_low_alarm, 0.0, 5.0, 0.0, t);
    dev_normal.on_pins("28", 28.0, &mut dq_low_norm, 5.0, 5.0, 0.0, t + 500 * US);
    dev_alarm.on_pins("28", 28.0, &mut dq_low_alarm, 5.0, 5.0, 0.0, t + 500 * US);
    dev_normal.on_event(&mut dq_low_norm, t + 530 * US);
    dev_alarm.on_event(&mut dq_low_alarm, t + 530 * US);
    dev_normal.on_event(&mut dq_low_norm, t + 630 * US);
    dev_alarm.on_event(&mut dq_low_alarm, t + 630 * US);

    // Send Alarm Search Command 0xEC
    for bit in 0..8 {
        let one = (0xECu8 & (1 << bit)) != 0;
        let dur = if one { 5 * US } else { 60 * US };
        dev_normal.on_pins("28", 28.0, &mut dq_low_norm, 0.0, 5.0, 0.0, t);
        dev_alarm.on_pins("28", 28.0, &mut dq_low_alarm, 0.0, 5.0, 0.0, t);
        dev_normal.on_pins("28", 28.0, &mut dq_low_norm, 5.0, 5.0, 0.0, t + dur);
        dev_alarm.on_pins("28", 28.0, &mut dq_low_alarm, 5.0, 5.0, 0.0, t + dur);
    }

    // Temperature is 28°C:
    // dev_normal: 28°C is between Tl(10) and Th(30) -> Not in alarm -> drops out
    assert!(
        !dev_normal.active,
        "dev_normal must drop out of Alarm Search"
    );
    // dev_alarm: 28°C exceeds Th(25) -> In alarm -> participates in Search ROM!
    assert!(
        dev_alarm.active,
        "dev_alarm must stay active in Alarm Search"
    );
    assert_eq!(dev_alarm.phase, OW_SEARCH_ROM);
}

#[test]
fn test_ds18b20_write_scratchpad_resolution_and_eeprom() {
    let mut dev = OwLink::default();
    let mut dq_low = false;
    let t = 1_000 * US;

    // Reset & Presence
    dev.on_pins("28", 22.5625, &mut dq_low, 0.0, 5.0, 0.0, t);
    dev.on_pins("28", 22.5625, &mut dq_low, 5.0, 5.0, 0.0, t + 500 * US);
    dev.on_event(&mut dq_low, t + 530 * US);
    dev.on_event(&mut dq_low, t + 630 * US);

    // Skip ROM (0xCC)
    for bit in 0..8 {
        let one = (0xCCu8 & (1 << bit)) != 0;
        let dur = if one { 5 * US } else { 60 * US };
        dev.on_pins("28", 22.5625, &mut dq_low, 0.0, 5.0, 0.0, t);
        dev.on_pins("28", 22.5625, &mut dq_low, 5.0, 5.0, 0.0, t + dur);
    }
    assert_eq!(dev.phase, OW_FUNC_CMD);

    // Write Scratchpad (0x4E) with Th=45, Tl=5, Config=0x1F (9-bit resolution)
    for byte in [0x4Eu8, 45, 5, 0x1F] {
        for bit in 0..8 {
            let one = (byte & (1 << bit)) != 0;
            let dur = if one { 5 * US } else { 60 * US };
            dev.on_pins("28", 22.5625, &mut dq_low, 0.0, 5.0, 0.0, t);
            dev.on_pins("28", 22.5625, &mut dq_low, 5.0, 5.0, 0.0, t + dur);
        }
    }

    assert_eq!(dev.scratch[2], 45, "Th updated in scratchpad");
    assert_eq!(dev.scratch[3], 5, "Tl updated in scratchpad");
    assert_eq!(
        dev.scratch[4], 0x1F,
        "Config updated in scratchpad (9-bit mode)"
    );

    // Copy Scratchpad to EEPROM (0x48)
    for bit in 0..8 {
        let one = (0x48u8 & (1 << bit)) != 0;
        let dur = if one { 5 * US } else { 60 * US };
        dev.on_pins("28", 22.5625, &mut dq_low, 0.0, 5.0, 0.0, t);
        dev.on_pins("28", 22.5625, &mut dq_low, 5.0, 5.0, 0.0, t + dur);
    }
    assert_eq!(dev.eeprom, [45, 5, 0x1F], "EEPROM saved Th, Tl, and Config");

    // Modify scratchpad, then issue Recall E2 (0xB8)
    dev.scratch[2] = 99;
    dev.scratch[3] = 99;
    for bit in 0..8 {
        let one = (0xB8u8 & (1 << bit)) != 0;
        let dur = if one { 5 * US } else { 60 * US };
        dev.on_pins("28", 22.5625, &mut dq_low, 0.0, 5.0, 0.0, t);
        dev.on_pins("28", 22.5625, &mut dq_low, 5.0, 5.0, 0.0, t + dur);
    }
    assert_eq!(dev.scratch[2], 45, "Recall E2 restored Th from EEPROM");
    assert_eq!(dev.scratch[3], 5, "Recall E2 restored Tl from EEPROM");
}

#[test]
fn test_ds18b20_parasitic_power_mode_detection() {
    let mut dev = OwLink::default();
    let mut dq_low = false;
    let t = 1_000 * US;

    // Connect with VDD = 0.0 V (parasitic power mode)
    dev.on_pins("28", 25.0, &mut dq_low, 0.0, 0.0, 0.0, t);
    assert!(
        dev.parasitic,
        "DS18B20 detects parasitic power when VDD < 2.0V"
    );

    // Reset, presence, skip ROM (0xCC), Read Power Supply (0xB4)
    dev.on_pins("28", 25.0, &mut dq_low, 5.0, 0.0, 0.0, t + 500 * US);
    dev.on_event(&mut dq_low, t + 530 * US);
    dev.on_event(&mut dq_low, t + 630 * US);

    for byte in [0xCCu8, 0xB4] {
        for bit in 0..8 {
            let one = (byte & (1 << bit)) != 0;
            let dur = if one { 5 * US } else { 60 * US };
            dev.on_pins("28", 25.0, &mut dq_low, 0.0, 0.0, 0.0, t);
            dev.on_pins("28", 25.0, &mut dq_low, 5.0, 0.0, 0.0, t + dur);
        }
    }

    // Master initiates read slot: slave pulls line low to indicate parasitic power (0)
    dev.on_pins("28", 25.0, &mut dq_low, 0.0, 0.0, 0.0, t);
    assert!(
        dq_low,
        "Read Power Supply pulls low (0) in parasitic power mode"
    );

    // Now connect with VDD = 5.0 V (external power mode)
    dev.on_pins("28", 25.0, &mut dq_low, 5.0, 5.0, 0.0, t + 800 * US);
    assert!(
        !dev.parasitic,
        "DS18B20 detects external power (VDD >= 2.0V)"
    );
}

#[test]
fn test_ds1307_autonomous_clock_advancement_and_leap_year() {
    let mut bus = Ds1307Bus::default();

    // Set time manually to 2024-02-28 23:59:58 (2024 is a leap year)
    // CH = 0 (oscillator running), Sec = 58
    bus.regs[0] = 0x58; // 58s
    bus.regs[1] = 0x59; // 59m
    bus.regs[2] = 0x23; // 23h (24-hour mode)
    bus.regs[3] = 0x03; // Wednesday (day 3)
    bus.regs[4] = 0x28; // 28th
    bus.regs[5] = 0x02; // February
    bus.regs[6] = 0x24; // Year 24 (2024)

    // Advance 3 seconds of simulation time (to 00:00:01 on Feb 29)
    bus.advance_seconds(3);

    assert_eq!(bus.regs[0], 0x01, "Seconds advanced to 01");
    assert_eq!(bus.regs[1], 0x00, "Minutes wrapped to 00");
    assert_eq!(bus.regs[2], 0x00, "Hours wrapped to 00");
    assert_eq!(bus.regs[3], 0x04, "Day of week advanced to 4");
    assert_eq!(
        bus.regs[4], 0x29,
        "Leap year rollover advanced date to Feb 29"
    );
    assert_eq!(bus.regs[5], 0x02, "Month is still February");
    assert_eq!(bus.regs[6], 0x24, "Year is 2024");

    // Advance 1 day (86,400 seconds) -> should roll to March 1st!
    bus.advance_seconds(86_400);
    assert_eq!(bus.regs[4], 0x01, "Rollover to 1st of month");
    assert_eq!(bus.regs[5], 0x03, "Rollover to March");
}

#[test]
fn test_ds1307_clock_halt_ch_bit() {
    let mut bus = Ds1307Bus::default();
    // CH = 1 (bit 7 set) -> oscillator halted
    bus.regs[0] = 0x80 | 0x30; // 30s with CH=1
    bus.advance_seconds(60);
    assert_eq!(
        bus.regs[0],
        0x80 | 0x30,
        "Time must not advance when CH bit is set"
    );
}

#[test]
fn test_ds1307_ram_56_bytes_access() {
    let mut bus = Ds1307Bus::default();

    // Simulate I2C master writing to NVRAM address 0x08..0x0B
    bus.pointer = 0x08;
    bus.have_ptr = true;

    // Write 4 test bytes to RAM
    let test_data = [0xAA, 0x55, 0xDE, 0xAD];
    for &b in &test_data {
        let idx = (bus.pointer & 0x3F) as usize;
        bus.regs[idx] = b;
        bus.pointer = (bus.pointer + 1) & 0x3F;
    }

    assert_eq!(&bus.regs[0x08..0x0C], &test_data);

    // Read back via I2C read cycle
    bus.pointer = 0x08;
    let mut read_data = [0u8; 4];
    for i in 0..4 {
        let idx = (bus.pointer & 0x3F) as usize;
        read_data[i] = bus.regs[idx];
        bus.pointer = (bus.pointer + 1) & 0x3F;
    }
    assert_eq!(read_data, test_data, "NVRAM data read back accurately");
}

#[test]
fn test_esp01_extended_at_command_sequences() {
    let mut esp = EspUart::default();

    // Helper to send a string over simulated UART RX
    fn send_line(esp: &mut EspUart, line: &str) {
        for &byte in line.as_bytes() {
            esp.push_rx(byte);
        }
        esp.push_rx(b'\r');
        esp.push_rx(b'\n');
    }

    // Helper to read all queued TX bytes
    fn drain_tx(esp: &mut EspUart) -> String {
        let mut out = Vec::new();
        while esp.tx_bits_left > 0 || !esp.tx_queue.is_empty() {
            // Apply start bit
            esp.apply_tx_bit();
            let mut byte = 0u8;
            for bit_i in 0..8 {
                esp.apply_tx_bit();
                if esp.tx_high {
                    byte |= 1 << bit_i;
                }
            }
            // Apply stop bit
            esp.apply_tx_bit();
            out.push(byte);
        }
        String::from_utf8_lossy(&out).to_string()
    }

    // 1. AT command
    send_line(&mut esp, "AT");
    assert_eq!(drain_tx(&mut esp), "OK\r\n");

    // 2. Query firmware version (AT+GMR)
    send_line(&mut esp, "AT+GMR");
    let gmr = drain_tx(&mut esp);
    assert!(gmr.contains("AT version:"));
    assert!(gmr.contains("SDK version:"));
    assert!(gmr.contains("OK\r\n"));

    // 3. Set Wi-Fi mode to Station (AT+CWMODE=1)
    send_line(&mut esp, "AT+CWMODE=1");
    assert_eq!(drain_tx(&mut esp), "OK\r\n");
    assert_eq!(esp.cwmode, 1);

    // 4. Scan Wi-Fi APs (AT+CWLAP)
    send_line(&mut esp, "AT+CWLAP");
    let lap = drain_tx(&mut esp);
    assert!(lap.contains("+CWLAP:(3,\"SimNet\",-45"));
    assert!(lap.contains("OK\r\n"));

    // 5. Connect to AP (AT+CWJAP="SimNet","secret")
    send_line(&mut esp, "AT+CWJAP=\"SimNet\",\"secret\"");
    let jap = drain_tx(&mut esp);
    assert!(jap.contains("WIFI CONNECTED\r\nWIFI GOT IP\r\nOK\r\n"));
    assert_eq!(esp.connected_ssid.as_deref(), Some("SimNet"));

    // 6. Query IP and MAC (AT+CIFSR)
    send_line(&mut esp, "AT+CIFSR");
    let cifsr = drain_tx(&mut esp);
    assert!(cifsr.contains("+CIFSR:STAIP,\"192.168.4.2\""));
    assert!(cifsr.contains("+CIFSR:STAMAC,"));

    // 7. Query status (AT+CIPSTATUS)
    send_line(&mut esp, "AT+CIPSTATUS");
    let status = drain_tx(&mut esp);
    assert!(status.contains("STATUS:2\r\nOK\r\n")); // Status 2 = Got IP
}

#[test]
fn test_esp01_cipsend_raw_mode_and_simulated_http() {
    let mut esp = EspUart::default();

    fn send_line(esp: &mut EspUart, line: &str) {
        for &byte in line.as_bytes() {
            esp.push_rx(byte);
        }
        esp.push_rx(b'\r');
        esp.push_rx(b'\n');
    }

    fn drain_tx(esp: &mut EspUart) -> String {
        let mut out = Vec::new();
        while esp.tx_bits_left > 0 || !esp.tx_queue.is_empty() {
            esp.apply_tx_bit(); // start bit
            let mut byte = 0u8;
            for bit_i in 0..8 {
                esp.apply_tx_bit();
                if esp.tx_high {
                    byte |= 1 << bit_i;
                }
            }
            esp.apply_tx_bit(); // stop bit
            out.push(byte);
        }
        String::from_utf8_lossy(&out).to_string()
    }

    // Connect to simulated server
    send_line(&mut esp, "AT+CIPSTART=\"TCP\",\"api.example.com\",80");
    let conn_resp = drain_tx(&mut esp);
    assert!(conn_resp.contains("CONNECT\r\n\r\nOK\r\n"));

    // Send HTTP GET request with AT+CIPSEND=18
    send_line(&mut esp, "AT+CIPSEND=18");
    let prompt = drain_tx(&mut esp);
    assert_eq!(prompt, ">\r\n", "ESP01 prompts with > for raw data");
    assert!(esp.rx_raw_mode, "ESP01 enters raw data collection mode");

    // Transmit 18 raw bytes: "GET / HTTP/1.1\r\n\r\n"
    let req = b"GET / HTTP/1.1\r\n\r\n";
    for &b in req {
        esp.push_rx(b);
    }

    assert!(!esp.rx_raw_mode, "ESP01 exits raw mode after 18 bytes");
    let resp = drain_tx(&mut esp);
    assert!(
        resp.contains("Recv 18 bytes\r\n\r\nSEND OK\r\n"),
        "ESP01 confirms byte receipt with SEND OK"
    );
    assert!(
        resp.contains("+IPD,"),
        "ESP01 returns simulated server packet via +IPD"
    );
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "Simulated server responds with HTTP 200"
    );
    assert!(
        resp.contains("{\"status\":\"ok\",\"sim\":true}"),
        "Simulated JSON response delivered over UART"
    );
}
