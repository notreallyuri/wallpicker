/// The date of `secs` since the Unix epoch (UTC): (year, month, day).
pub fn civil_date(secs: u64) -> (i64, u32, u32) {
    // Howard Hinnant's days-to-civil.
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// "2026-10-06" and "2026-10-06 14:03" for `secs` (UTC).
pub fn date_and_time(secs: u64) -> (String, String) {
    let (y, m, d) = civil_date(secs);
    let date = format!("{y:04}-{m:02}-{d:02}");
    let time = format!("{date} {:02}:{:02}", (secs / 3600) % 24, (secs / 60) % 60);
    (date, time)
}
