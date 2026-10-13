use super::*;

#[test]
fn address_space_enforcement_child() {
    if std::env::var_os("WALLPICKER_LIMIT_PROBE").is_none() {
        return;
    }
    // SAFETY: anonymous mappings, valid lengths, no dereferences. A successful
    // small mapping is released with the same address and length.
    unsafe {
        let small = libc::mmap(
            std::ptr::null_mut(),
            4096,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_ne!(small, libc::MAP_FAILED);
        assert_eq!(libc::munmap(small, 4096), 0);
        let large = libc::mmap(
            std::ptr::null_mut(),
            256 * 1024 * 1024,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );
        assert_eq!(large, libc::MAP_FAILED);
        assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ENOMEM));
    }
    println!("kernel refused over-budget mmap");
}

#[test]
fn kernel_enforces_address_space_limit_in_exec_child() {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "decoding::tests::address_space_enforcement_child",
            "--nocapture",
        ])
        .env("WALLPICKER_LIMIT_PROBE", "1");
    process::constrain(&mut command, 128 * 1024 * 1024);
    let output = command.output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("kernel refused over-budget mmap"));
}

#[test]
fn invalid_worker_headers_are_rejected_before_frame_allocation() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("still.png");
    wallpicker_core::media::RgbaImage::new(1, 1).save(&path).unwrap();
    let source = Source::open(&path, Default::default()).unwrap();
    for bytes in [
        vec![99],
        [vec![1], vec![255; 16]].concat(),
        [vec![2], u32::MAX.to_le_bytes().to_vec()].concat(),
    ] {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        writer.write_all(&bytes).unwrap();
        assert!(protocol::read_frame(&mut reader, &source, Instant::now() + Duration::from_secs(1)).is_err());
    }
}

#[test]
fn reservation_is_atomic_across_competing_workers() {
    let budget = Budget::new(20, 2);
    let barrier = Arc::new(std::sync::Barrier::new(9));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let (budget, barrier) = (budget.clone(), barrier.clone());
            std::thread::spawn(move || {
                let reservation = budget.reserve(10);
                barrier.wait();
                barrier.wait();
                drop(reservation);
            })
        })
        .collect();
    barrier.wait();
    assert_eq!(budget.usage(), Usage { bytes: 20, workers: 2 });
    barrier.wait();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(budget.usage(), Usage::default());
}
