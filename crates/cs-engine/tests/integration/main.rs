//! Consolidated integration test binary for `cs-engine`.
//!
//! Keeping all integration test modules in a single crate avoids linking
//! `cs-engine` and its graphics/simulation dependencies dozens of separate times.

mod board_subcircuits;
mod catalog_components;
mod component_defaults_and_props_parity;
mod component_prop_info;
mod component_properties;
mod component_shortcuts;
mod components_behavior;
mod dc_sweep;
mod debug_stepping;
mod debug_symbols;
mod digital_ics;
mod displays_parity;
mod electromechanical_physics;
mod gdb_rsp;
mod installer_parity;
mod linking_and_exposed;
mod load_sim1;
mod local_subcircuits;
mod mcu_qemu_pins;
mod multiple_prop_dialogs;
mod oscope_tunnel_parity;
mod overload_warnings;
mod package_designer;
mod palette_defaults;
mod periph_serial_monitors;
mod peripheral_remap;
mod power_semiconductors_physics;
mod rc_transient_verification;
mod sensors_and_actuators_parity;
mod sensors_protocol_depth;
mod simulation_smoke;
mod switch_props_coverage;
mod touchpad_parity;
mod transformer_physics;
mod varset_eval;
mod visual_delegates;
mod wavegen_parity;
