//! Live spawn of `qemu-cosim-stub` against a shared-memory arena.

use std::path::Path;
use std::time::{Duration, Instant};

use cs_qemu::{QemuProcess, SharedArena, shm_key};

#[test]
fn stub_sets_arena_running() {
    let key = shm_key(std::process::id(), "stub");
    let shm = SharedArena::create(&key).expect("shm");
    let stub = env!("CARGO_BIN_EXE_qemu-cosim-stub");
    let mut p =
        QemuProcess::spawn(Path::new(stub), &[format!("esp32-cs,cosim-shm={key}")]).expect("stub");
    let start = Instant::now();
    while shm.as_ref().running == 0 && start.elapsed() < Duration::from_secs(5) {
        assert!(p.is_running(), "stub exited before setting running");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(shm.as_ref().running, 1);
    #[cfg(unix)]
    {
        p.stop();
        assert!(p.stopped());
        p.resume();
    }
    p.kill();
    assert!(!p.is_running());
}
