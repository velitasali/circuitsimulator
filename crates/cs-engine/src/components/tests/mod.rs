//! Comprehensive component property, geometry, and circuit view test suite.

pub mod recorder;
mod test_actives;
mod test_displays;
mod test_generic_matrix;
mod test_logic;
mod test_memory_converters;
mod test_passives;
mod test_sources_meters;
mod test_subcircuits_mcu;
mod test_switches_sensors;

use super::*;

#[test]
fn component_construction() {
    let r = Part::Resistor(Resistor {
        resistance: 100.0,
        show_bands: true,
    });
    assert_eq!(r.type_id(), "Resistor");
    assert_eq!(r.human_name(), "Resistor");
    assert_eq!(r.default_show_prop(), "Resistance");
}
