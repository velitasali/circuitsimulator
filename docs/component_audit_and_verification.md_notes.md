# Notes on the audit invariants

Working notes from the pass that closed the checks in §4 of
`component_audit_and_verification.md`. The audit file itself was not edited.
Phases 1–5 in that file were already in the tree. This pass added the missing
invariant coverage and the two behaviors those checks depend on.

Library tests that were run and passed: `live_update` (6), `bcd_to_dec` (3,
including the component geometry tests), `pin_parity`, `passive_pins_rest_near_ground_and_obey_kcl`,
`isolated_parts_register_a_stamp`, `reactive_and_time_sources_are_classified`,
`clock_toggles_over_a_half_period`. Integration: `simulation_smoke` (4).
`cargo test --workspace` had not been run yet. See the later pass below.

A later pass closed the list that used to sit at the bottom of this file.
`UnionFind::groups` now sorts pins and net roots. Node numbers no longer
follow `HashMap` iteration, which had been flipping the comparator's idle
line between processes. Extra tests that passed:
`i2c_to_parallel_write_drives_the_port_pins`,
`i2c_slave_releases_sda_and_a_write_updates_the_port_latch`,
`relay_inductance_delays_pull_in_until_the_coil_current_rises`,
`and_gate_open_collector_reaches_the_output_pin`, plus the library set
above and `simulation_smoke` again. Passivity was repeated eight times
after the sort. The first `cargo test --workspace` failed two catalog
checks. The MAX7219 geometry test now counts DOUT (6 pins). A potentiometer
wired on the old `PinA` / `PinB` names was its own island, because the
connection graph only walks `pin_ids`; those names are unioned onto
`lPin` / `rPin` / `wPin`, and an open wiper is the `wPin` node. One later
parallel run aborted with `STATUS_HEAP_CORRUPTION` inside the AngelScript
tests. `script_sets_pin_high` passed alone, and the following full run
passed. The audit file was still not edited.

## Invariant 2 — live kind replacement

`Circuit::update_component_kind` still updates capacitor, elcap, varcap,
inductor, varinductor, and transformer fields in place, then returns. Those
arms keep `volt`, `ieq`, `i1`, and `i2`.

Every other kind goes through `std::mem::replace`, then:

1. If both kinds are `Tunnel` and the new pin id is empty, write the old pin id back.
2. `Kind::adopt_runtime` (`crates/cs-engine/src/elements/runtime.rs`).
3. `rebind_id`.

`adopt_runtime` copies solver-accumulated state only when the variant matches.
The fresh kind already holds the new property values. Whole `Mcu`, `QemuDevice`,
and `ScriptCpu` values are not transplanted. `Push.closed`, `SwitchDip.state`,
`KeyPad.pressed`, and the KY-023 / KY-040 dials stay with the new kind.

Same-variant fields that are copied:

- Lamp `heat`. `resistance` is copied only when `voltage`, `power`, and `r_cold` compare equal. `tau` stays with the new kind.
- Relay `active` and `coil_i`. Csource `ctrl` (`sense_ohms` stays new). Clock `state`. WaveGen `v_out` and `wav_index` (index clamped to the new sample buffer).
- Stepper `angle`. DcMotor `angle` and `speed`. Servo `pos`, `target_pos`, `pulse_start_ps`, `sig_high`.
- Scr, Diac, Triac `conducting`. Voltmeter and Ammeter `last_out`. AudioOut `last_v`.
- AnalogMux `selected`, when the channel count matches.
- Memory `data` prefix. DynamicMemory `data` prefix plus `row_addr`, `col_addr`, `last_ras`, `last_cas`.
- I2CRam `data` prefix, `mem_ptr` (clamped), `mode`, `bit_count`, `rx_byte`, `last_scl`, `last_sda`. Pin objects stay new.
- Counter `count`, `last_clk`. BinCounter `count_a`, `count_b`, both last-clock flags. ShiftReg `shift_reg`, `latch_reg`, both last-clock flags.
- FlipFlop `q0` when `kind` matches, plus the Q and /Q logical levels.
- Latch output logical levels, up to the shorter channel count. There is no separate data register. `get_out_state` / `set_out_state` are the memory.
- Lm555 `out_state` and `discharge_on`. `out_high_v` / `out_low_v` stay new because `step` overwrites them from the rails.
- PCD8544 RAM and SPI cursors (`addr_x`, `addr_y`, `in_buf`, `in_bit`, `last_scl`, `pd`, `v_mode`, `h_mode`, `d_bit`, `e_bit`). Contrast, bias, and pins stay new.
- KS0108 both RAMs, page/column/start line, display-on flags, `last_en`. `cs_act_low` stays new.
- HD44780 DDRAM, CGRAM, address, shift, display, nibble, and clock cursors. Rows, columns, and pins stay new. The same helper covers AIP31068's inner HD44780, plus AIP `phase`, `control_byte`, and the TWI module.
- SSD1306 / SH1107 overlapping DDRAM always. When width and height match, address cursors, scroll (SSD1306 only), display flags, protocol flags, and the TWI module. `line_mask` / `row_mask` follow geometry and stay new. SH1107 `x_offset` stays new.
- TWI copy keeps the transaction and then restores the new kind's `address` and `mode`, so an address edit is not undone.
- I2CToParallel `out_latch`, `in_latch`, then `write_byte` of the kept latch, and that same TWI copy. The replacement SDA and INT pins are the new ones, released.
- Max7219 framebuffer: full clone when `modules` match (that includes the private shift chain). Otherwise the overlapping digit rows and `dout_high`.
- WS2812 framebuffer: full clone when `count` matches. Otherwise overlapping pixels, `bit_index`, `fall_ps`, `dout_high`.
- SR04 trigger/echo flags and `Sr04Echo`. DHT22 `out_state`, `pin_driven`, `DhtLink`. DS18B20 `dq_low` and `OwLink` (temperature and ROM stay new). DS1621 `tout_high`, `sda_low`, `scl_low`, `Ds1621Bus` (thresholds and temperature stay new). DS1307 `sqw_state`, `sda_low`, `scl_low`, `Ds1307Bus`. `time_updated`, `sqw_freq`, and `sqw_enabled` stay new. `seeded` lives on the bus, so a copied bus is not reseeded. ESP-01 `uart`.

`reset_device_stamps`, which `re_solve` calls, does not clear RAM, relay `active`, lamp `heat`, or `Csource::ctrl`. It does call `stamp_init` on gates, flip-flops, latches, and MCUs. Flip-flop `stamp_init` drives Q from `q0`, so a preserved `q0` comes back after the re-solve. Latch `stamp_init` forces every output low, so a preserved latch level does not survive the re-solve. The live-update tests assert before any solve.

Tests: `crates/cs-engine/src/components/tests/live_update.rs`.

- Memory `data[0] = 0xA5` survives a same-property refresh.
- Counter count 7, stepper angle 123, lamp heat 0.5 and a custom resistance survive when the part properties are unchanged.
- A lamp voltage edit keeps heat and replaces resistance with `r_cold`.
- Relay `active` stays set when `Rcoil` changes to 50 Ω.
- Capacitor `volt = 2.5` survives a capacitance edit (the in-place arm, not `adopt_runtime`).
- `Ldr` property `Lux` = 1000 changes `Kind::Ldr.resistance` away from the lux-1 default. `ResistorDip` property `Bussed` = true sets `bussed` and publishes `U1-com`. `AndGate` property `InvertInputs` = true sets `inputs[0].inverted`. There is no `GateState` field named `invert_inputs`. NAND / NOR / XNOR / Inverter invert the output pin instead. Property writes go through `Part::set_prop_text`.

Pin-id stability for every harness case is still `pin_parity_and_live_prefix`. That test stayed green after the BCD enable pin became real, because the id string is still `U1-in4`.

## BCD-to-decimal output enable

`BcdToDecState.oe` is `IoPin::input("{id}-in4")`. The pin is not inverted. `eval` treats `oe.inp_state() == true` as disable and calls `set_out_state(false)` on every output. `set_out_state` applies the output's own `inverted` flag, so an active-low output rests high when it is logically deasserted. An unconnected enable reads low and the decoder stays enabled. `active_low` on the part still only inverts the outputs. The default part is active-high.

Wired through `apply_family`, `pin_ids` (the old `format!("{}-in4")` phantom is gone), `rebind_id`, `stamp_iopin`, and the digital-queue sample before `eval`.

Solver test: `bcd_to_dec_output_enable_blanks_the_selected_line` in `simulation/tests.rs`. Floating inputs select output 0. With OE open that pin is a logic high into 10 kΩ. OE at 5 V blanks it. The active-low twin rests low while enabled and rests high while blanked.

## Invariant 3 — passivity and KCL

`crates/cs-engine/src/components/tests/passivity.rs`, inventory `passivity_expected.txt`.

Every harness default is inserted as `U1`. Each distinct `pin_ids` entry, including companion nodes such as `U1-mid`, is tied to ground through 1 kΩ. `solve()`. A pin above 0.1 V is a `driver` (the voltage is recorded to three decimals). The 0.1 V floor is above a 100 kΩ pull-up into the 1 kΩ shunt, which sits near 50 mV. Then the first pin is also driven by 5 V through another 1 kΩ, `solve()`, `re_solve()` once, and the pin currents are summed.

The extra solve matters for meters. `last_out` is updated after the stamp. The second solve makes the output Norton agree with the node.

`current_out_of_pin` returning `None` is `current-gap`. It is not treated as 0 A. A sum whose absolute value exceeds `1e-6` or `1e-4` times the sum of absolute currents is `implicit-return`. The file stores the category, not the ampere digits, so a floating-point twitch does not fail the test. A new line, or a line that disappears, fails it. A panic fails it outright. A solve error is an inventory line.

What the sweep actually showed:

- Default `VarResistor` resistance is 0. `add_to_circuit` stores that 0. `to_element_kind` would clamp to `1e-12` Ω. The stamp floors at `1e-12` Ω either way, which is `1e12` S. In the pull-down circuit the matrix is singular (`could not solve matrix`). The 1 mS shunt disappears beside that conductance, and the ground node's tiny `CERO_DOUB` current leaves a residual on a zero pivot.
- One-terminal parts and Norton-to-ground parts do not obey pin KCL. The return is the matrix datum. That covers Ground, FixedVolt, Rail, Clock, VoltSource, CurrSource, WaveGen, OpAmp, Comparator, Voltmeter, Ammeter, NotGate, FlipFlop, and the default QEMU device. Battery, not-gate, flip-flop, mux, BCD decoder, the joysticks, ESP-01, I2C-to-parallel, the parallel memories, and QEMU also sit above 0.1 V with every pin pulled down. Comparator does not. `reset_stamp` drives the output low, and the two inputs pulled to ground compare equal, so the line is `implicit-return` with no driver voltage. ESP-01 idles TX at 3.3 V. QEMU's default pins sit near 3.17 V. Before nets were sorted, that compare flipped: some processes recorded `driver 4.808 V implicit-return`. After the sort the no-driver line held for eight runs.
- Current arms exist where the stamp is a real branch: FreqMeter, SevenSegment (every segment and common), SevenSegmentBCD, SdCard, TftDisplay, Pcf8833, and LedMatrix (`rPin` / `cPin` sums). Those parts close KCL while idle near 0 V, so they dropped out of the inventory. FreqMeter was already absent. `None` is still `current-gap`, not 0 A. Still `current-gap`: Node, Push, SwitchDip, Relay (the contacts have no arm), KeyPad, Diode and Zener (`mid`), Mux, Demux, AnalogMux, the BCD converters, both adders, TestUnit, TouchPad, the sensors, Stepper, Servo, the protocol displays (including Max7219 and WS2812), the counters, shift register, magnitude comparator, Adc, Dac, I2CToParallel (address pins `in2`..`in4` are in `pin_ids` and are not stamped), Lm555, the memories, Function, Scr, Diac, Triac, Csource, ResistorDip, Tunnel, Bus, Header, and Socket. A new arm that starts returning `Some` has to update the file.
- SerialPort, SerialTerm, Subcircuit, and SubPackage are still not inserted. Bus, Header, and Socket are inserted. They publish pins and return no pin current, so the sweep records `current-gap`. Dial, Shape, and the default Mcu part publish no solver pins. The default MCU has no GPIO until a device file is loaded.

## Invariant 4 — stamp registration

`crates/cs-engine/src/components/tests/stamp_registry.rs`, inventory `stamp_expected.txt`.

Each default part is stamped alone: one node per distinct pin id, `build_pin_cache`, `stamp_with_cache` at `ANALOG_DT_DEFAULT` (1 µs). `ENode::has_stamp` is `#[cfg(test)]` and is true when `|total_admit|` or `|total_curr|` exceeds `1e-18`. The report lists only parts that contribute nothing:

- `no-pins` — Dial, Shape, default Mcu.
- `not-inserted` — SerialPort, SerialTerm, Subcircuit, SubPackage.
- `no-stamp` — Node (junction), Push, KeyPad, DHT22, DS18B20, Tunnel, Bus, Header, Socket.

DHT22 and DS18B20 have pins. At rest the open-drain line stamps nothing. A default Switch does stamp, so it is absent from this list. Push and KeyPad do not. FreqMeter, SevenSegmentBCD, SdCard, TftDisplay, Pcf8833, and LedMatrix now stamp, so they left this list. Bus, Header, and Socket stamp an empty arm. The continuity is a union in `build_nets`, not a conductance. SerialPort and SerialTerm stay `not-inserted`.

A new empty `{}` arm adds a `no-stamp` line and fails until the file is updated. A listed part that starts stamping drops its line and fails the same way.

Hard checks beside the file:

- Capacitor, ElCapacitor, Inductor, VarCapacitor, VarInductor, Transformer, and Lamp set `has_reactive()` and do not set `has_time_sources()`.
- Clock and WaveGen set `has_time_sources()`. The harness clock is 1 kHz. `solve()` leaves `U1-outnod` low. `run_ps(500_000_000)` takes it high.
- VoltReg does not set `has_time_sources()`. It is a DC regulator (`volt_changed(vin, vout, vref)`, dropout 0.7 V, `VOLTREG_DEFAULT_VREF` 1.2, admit `1e6`). `scan_has_nonlinear()` is true. The cached `has_nonlinear()` flag is the wrong query: a fresh `Circuit` starts with that cache set.

The §4 sentence that groups VoltReg with Clock and WaveGen as a time source does not match the model. The test asserts the DC classification.

## Invariant 5 — smoke

`crates/cs-engine/tests/simulation_smoke.rs`. It does not repeat the phase 3–5 protocol tests in `simulation/tests.rs`.

- Scene path: ground, two 5 V sources, an AND gate, and 10 kΩ. `power_on()`. The gate output is above 4 V.
- Relay, circuit-level: 5 V across a 100 Ω coil (50 mA, `i_on` is 20 mA). The SPST contact passes 5 V into 1 kΩ.
- Controlled source, circuit-level: 1.5 V times gain 2 produces 3 V.
- PCD8544 readings: RST is tied to 5 V before `power_on()`. `step` calls `reset()` while RST is low, and `reset()` sets `pd`. `hex_bitmap` returns an empty string while `pd` is set or while both display bits are clear. After power-on the running kind is set to `pd = false`, `d_bit = true`, `ram[0][0] = 0xA5`, then `tick()`. `publish_solution` is `pub(crate)`, so the integration test goes through `tick()`. The reading text contains `A5`.

## Later pass — what closed, and what stayed open on purpose

`pin_ids` lists the schematic suffix. `build_nets` calls `bind_alias` only when that older wire is already present, and unions it onto the canonical pin. A potentiometer does the same for `PinA` / `PinB` / `PinM` onto `lPin` / `rPin` / `wPin`. When either end is wired and the wiper is not, it adds `wPin`, because the connection graph only walks `pin_ids`. The parity residue is 5 lines in `pin_parity_expected.txt`: SerialPort, SerialTerm, two SubPackage lines, and QemuDevice `Gnn` / `Rst` (`only-solver`). Bus, Header, and Socket match. A header or socket lists `pin0`..`pin{2n-1}`. A bus lists `ePin0`, `ePin1`..`ePin{width}`, and `busPinI`. These older mismatches are gone because `pin_ids` now matches the schematic name: Pot `PinA` / `PinB` / `PinM`, a grounded LED's `rPin`, OpAmp power pins unless `power_pins`, Mux/Demux address bit 3 on the default 3-bit part, AnalogMux `com` / `en` / `ch{i}` (those stay aliases), seven-segment short names (same node as `pin_a`..`pin_dot` and `pin_commona`), Max7219 `dout`, WS2812 `vcc` (solver) vs `vdd` (alias), SCR `lPin` / `rPin` (canonical `aPin` / `kPin` / `gPin`), Triac `mt1Pin` / `mt2Pin` (canonical `lPin` / `rPin` / `gPin`). TouchPad `nodeX` / `nodeY` are still published and are excluded as internal nodes, the same way diode `mid` is.

Seven-segment paint reads `pin_a`..`pin_dot` and that digit's `pin_common{letter}`. The short names are the same node, not extra dice. Digit count clamps to 1..=8, including the die count (`n * 8`). Threshold is the property when it is above 0.1 V, otherwise 1.4 V. Counter, full adder, and half adder (the half adder is a `FullAdderState`) clamp bits to 1..=32. Magnitude comparator stays 1..=16. `bit_mask` uses `u32::MAX` at 32 bits or more.

Mux and Demux `AddrBits` max is 5 (`MAX_ADDR_BITS`). The solver clamp matches. The default part is still 3 bits. For 3 bits or more the address pins span `8*(n-1)`, with pin `i` at `x0 - 8*i`.

Bin-counter section B is clocked inside `BinCounterState::eval` from QA, not from a second sample of the schematic clock. On `clk_a`'s falling edge, if Q0 was high, section B increments (decade wraps at 4, binary `& 7`). `last_clk_b` tracks the new Q0. The handler copies the schematic reset onto `r0_2` and holds both set-9 pins low. A clock frequency edit, after `update_component_kind` releases the component borrow, calls `reschedule_clock`: cancel `EventTarget::Device(index)` (because `events.add` does nothing while an event is live), set `state` from `clock_is_high(now)`, then arm `clock_next_event_ps`. Frequency 0 only cancels.

Scr and Triac stamp `1/r_gate` from the gate to the cathode or MT1. The default is 100 Ω. The trigger stays `v_gate_th` (0.7 V). `i_trig * r_gate` is not the trigger; the defaults would fire at 1 V. The on-state is still `SWITCH_CLOSED_ADMIT` plus the 0.7 V SCR Norton drop. Off conductance is `1/res_off`. Diac uses `res_on` 500 Ω, `res_off` 1e8 Ω, and `hold_curr` 0.01 A.

Relay with `inductance > 0` is a series RL companion: `g = 1/(R + L/dt)`, history current from `coil_i`, pull-in from `|coil_i|`. `L == 0` stays a pure `1/r_coil` stamp and a V/R pull-in, so the existing `solve()` tests and the smoke relay still close on the first Newton step. The default part inductance is 0.1 H. `has_reactive` includes that case. `relay_inductance_delays_pull_in_until_the_coil_current_rises` uses 0.1 H and `run_ps(2_000_000_000)`. Lamp `tau` defaults to 0.02 s (`LAMP_THERMAL_TAU`) and is not copied by `adopt_runtime`. Csource `sense_ohms` defaults to 1.0 (`CSOURCE_SENSE_OHMS`) and is not copied either. The current-control test still uses 1 Ω.

LM555 part defaults are OutHighV 5.0 and OutLowV 0.0. `follow_rails` is true when those are the loaded values. `step` overwrites the levels only then. `adopt_runtime` copies `out_state` and `discharge_on` only. And, Or, Xor, Not, and Buffer expose `OpenCollector`. `to_element_kind` calls `set_open_col` after `set_tristate`, and `apply_family` puts the mode back at the end, because `set_output_imp` only updates an `Output` pin. The property id is `OpenCollector`. It is not structural. The QML canvas JSON has no open-collector key.

I2C-to-parallel reuses `TwiModule::set_slave` and `volt_changed_pins`. There is no second bus stack. The address is the property. Pins `in2`..`in4` are not sampled. A received byte calls `write_byte` and updates `in_latch`. A slave read loads `tx_reg` from `read_byte`. INT is open-drain and goes low when `read_byte() != in_latch`. `new` calls `set_slave(address & 0x7F)`, `apply_family`, `write_byte(0xFF)`, then releases SDA and INT. An open-collector pin is born sinking; a slave that holds SDA low makes every host bit read as 0, and the address "ACK" is just the stuck bus. Ports stay push-pull. Solver test: `i2c_to_parallel_write_drives_the_port_pins` (address `0xA0`, data `0x0F`, P0 and P4 loaded by 10 kΩ). Direct test: `i2c_slave_releases_sda_and_a_write_updates_the_port_latch`.

High-Z inputs reuse `PLOT_INPUT_ADMIT` (1e-7 S), stamped to ground at 0 V. That covers FreqMeter (both `stamp` and `stamp_with_cache`), SevenSegmentBCD, SdCard, TftDisplay, and Pcf8833. LedMatrix builds `rows * cols` dice, anode `rPin{row}`, cathode `cPin{col}`. `vertical_pins` is geometry only. Rows and columns clamp with `.max(1)`. No protocol was added for SD, SPI, or the TFT parts. Dial, Shape, and SubPackage still stamp nothing. SerialPort, SerialTerm, Subcircuit, and SubPackage are still not inserted. Header and socket rows are one net each (`pin{i}` with `pin{i+n}`) when either end is already wired. A bus keeps its lines apart. `ePin0` and `busPinI` are one trunk. Buses whose trunks share a net share line `ePin{k}` for the overlapping width (`ePin1` is line 0). A line nobody wired is not added as its own node. The default Mcu still publishes no GPIO until a device file is loaded.

Preview stamps follow the off-state companion: 1e-9 S and no Norton current for RgbLed, SevenSegment, and both LedBar paths. Max7219's last pin (DOUT; the default part has 6 pins) and WS2812 pin index 2 (order `din`, `vcc`, `dout`, `gnd`) preview idle DOUT at 0 V with `SOURCE_ADMIT`. The ESP-01 preview still forces pin 0 to 3.3 V.

Servo's electrical stamp is a constant 1e-7 S on `PinSig`. Position and `target_pos` are not electrical, `stamp_changed` is not set, and the supply pins have no model. `update_motors_dt` still integrates `pos`. No motor-current model was added.
