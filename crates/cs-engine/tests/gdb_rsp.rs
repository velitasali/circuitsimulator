use cs_engine::debug::gdb::GdbServer;

#[test]
fn test_gdb_packet_checksum() {
    let packet = GdbServer::format_packet("OK");
    assert_eq!(packet, "$OK#9a");

    let status = GdbServer::format_packet("S05");
    assert_eq!(status, "$S05#b8");
}

#[test]
fn test_gdb_query_packets() {
    let supported =
        GdbServer::handle_command("qSupported:multiprocess+;swbreak+;hwbreak+;qRelocOffsets+");
    assert!(supported.contains("PacketSize=1000"));

    let attached = GdbServer::handle_command("qAttached");
    assert_eq!(attached, "1");

    let query_c = GdbServer::handle_command("qC");
    assert_eq!(query_c, "QC1");
}

#[test]
fn test_gdb_halt_reason() {
    let halt = GdbServer::handle_command("?");
    assert_eq!(halt, "S05");
}

#[test]
fn test_gdb_breakpoint_commands() {
    // z0,100,2 (remove breakpoint) -> OK
    let z0 = GdbServer::handle_command("z0,100,2");
    assert_eq!(z0, "OK");

    // Z0,100,2 (set breakpoint) -> OK
    let z0_set = GdbServer::handle_command("Z0,100,2");
    assert_eq!(z0_set, "OK");
}
