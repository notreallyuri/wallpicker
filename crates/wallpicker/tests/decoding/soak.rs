use super::*;

fn rss(pid: u32) -> u64 {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse::<u64>()
        .unwrap()
        * 1024
}

#[test]
#[ignore = "explicit decoder RSS qualification; runs 1,000 loops per codec"]
fn long_loops_keep_child_rss_stable() {
    let tmp = tempfile::tempdir().unwrap();
    for source in [
        fixtures::gif(&tmp.path().join("soak.gif"), 320, 180),
        fixtures::webp(&tmp.path().join("soak.webp"), 320, 180),
    ] {
        let format = source.info().format;
        let budget = Budget::new(256 * MIB, 1);
        let worker = start(source, &budget, 1);
        for _ in 0..100 {
            drop(next(&worker));
        }
        let pid = worker.process_id().unwrap();
        let baseline = rss(pid);
        let mut peak = baseline;
        let parent_baseline = rss(std::process::id());
        let mut parent_peak = parent_baseline;
        let now = Instant::now();
        for i in 0..2000 {
            drop(next(&worker));
            if i % 50 == 0 {
                peak = peak.max(rss(pid));
                parent_peak = parent_peak.max(rss(std::process::id()));
            }
        }
        let final_rss = rss(pid);
        println!(
            "{format:?}: 1000 loops at 320x180, baseline={baseline}, peak={peak}, final={final_rss}, elapsed={:?}",
            now.elapsed()
        );
        println!("parent: warmed={parent_baseline}, peak={parent_peak}");
        assert!(peak <= baseline + 8 * MIB, "sustained decoder growth");
        assert!(parent_peak <= parent_baseline + 8 * MIB, "sustained parent growth");
        assert!(peak <= options().address_space_bytes);
        drop(worker);
        assert_eq!(budget.usage().bytes, 0);
    }
}
