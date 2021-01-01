//! Stand-in for patched `qemu-system-*` in tests: attach the cosim arena,
//! set `running = 1`, then sleep until killed.

fn main() {
    let key = match shm_key_from_args(std::env::args().skip(1)) {
        Some(k) => k,
        None => {
            eprintln!("qemu-cosim-stub: no cosim-shm key in argv");
            std::process::exit(2);
        }
    };
    let mut tries = 0;
    let mut shm = loop {
        match cs_qemu::SharedArena::open(&key) {
            Ok(s) => break s,
            Err(_) if tries < 50 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => {
                eprintln!("qemu-cosim-stub: {e}");
                std::process::exit(3);
            }
        }
    };
    shm.as_mut().running = 1;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

fn shm_key_from_args<I: IntoIterator<Item = String>>(args: I) -> Option<String> {
    let mut first_slash: Option<String> = None;
    for a in args {
        if let Some(rest) = a.split("cosim-shm=").nth(1) {
            let key = rest.split(',').next().unwrap_or(rest).to_string();
            if !key.is_empty() {
                return Some(key);
            }
        }
        if first_slash.is_none() && a.starts_with('/') {
            first_slash = Some(a);
        }
    }
    first_slash
}
