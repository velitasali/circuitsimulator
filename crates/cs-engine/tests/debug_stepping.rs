use cs_engine::debug::{BreakReason, DebugSession, DebugState};

#[test]
fn test_debug_session_lifecycle() {
    let session = DebugSession::default();
    assert!(!session.is_active());
    assert_eq!(session.state(), DebugState::Idle);

    session.step_into();
    assert!(session.is_active());
    assert_eq!(session.state(), DebugState::SteppingInto);

    // Step from 0x00 to 0x01
    let reason = session.on_mcu_step(0x01, 0x00, 0x100, 0x00);
    assert_eq!(reason, Some(BreakReason::StepCompleted));
    assert!(session.is_paused());

    session.resume();
    assert_eq!(session.state(), DebugState::Running);

    session.stop();
    assert!(!session.is_active());
    assert_eq!(session.state(), DebugState::Idle);
}

#[test]
fn test_debug_breakpoints() {
    let session = DebugSession::default();
    session.set_active(true);
    session.resume();

    // Add address breakpoint at 0x0020
    session.add_addr_breakpoint(0x0020);
    assert!(session.has_addr_breakpoint(0x0020));

    // Execution hits 0x0010 (no break)
    assert_eq!(session.on_mcu_step(0x0010, 0x000F, 0x100, 0), None);

    // Execution hits 0x0020 (break hit)
    let reason = session.on_mcu_step(0x0020, 0x001F, 0x100, 0);
    assert_eq!(
        reason,
        Some(BreakReason::Breakpoint {
            address: 0x0020,
            location: None
        })
    );
    assert!(session.is_paused());

    // Remove breakpoint
    session.remove_addr_breakpoint(0x0020);
    assert!(!session.has_addr_breakpoint(0x0020));
}

#[test]
fn test_debug_step_over() {
    let session = DebugSession::default();
    session.set_active(true);

    // Simulate stepping over a CALL at PC=0x10, return addr=0x12, initial SP=0x01FF
    session.step_over_target(0x01FF, Some(0x12));

    // While in subroutine: PC=0x50, SP=0x01FD (nested call stack, depth > target_sp)
    // No break
    assert_eq!(session.on_mcu_step(0x50, 0x10, 0x01FD, 0x12), None);

    // When subroutine returns to exit_pc: PC=0x12
    let reason = session.on_mcu_step(0x12, 0x55, 0x01FF, 0x00);
    assert_eq!(reason, Some(BreakReason::StepCompleted));
    assert!(session.is_paused());
}

#[test]
fn test_debug_step_out() {
    let session = DebugSession::default();
    session.set_active(true);

    // Inside subroutine with SP=0x01FD, stepping out to target SP >= 0x01FF
    session.step_out_target(0x01FF);

    // Subroutine returns and SP pops back to 0x01FF
    let reason = session.on_mcu_step(0x12, 0x55, 0x01FF, 0x12);
    assert_eq!(reason, Some(BreakReason::StepCompleted));
    assert!(session.is_paused());
}
