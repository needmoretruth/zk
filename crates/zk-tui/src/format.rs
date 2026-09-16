//! Numbers the way a reader wants them: durations in µs/ms/s, sizes in B/KiB/MiB, counts grouped.

use std::time::Duration;

/// A duration measured in microseconds, with three significant figures.
pub(crate) fn duration(micros: u64) -> String {
    if micros < 1_000 {
        return format!("{micros} µs");
    }
    if micros < 1_000_000 {
        return format!("{} ms", significant(micros as f64 / 1_000.0));
    }
    format!("{} s", significant(micros as f64 / 1_000_000.0))
}

/// Time a task has been running, as the working line shows it: `1.4s`.
pub(crate) fn elapsed(elapsed: Duration) -> String {
    format!("{:.1}s", elapsed.as_secs_f64())
}

/// A size in bytes, with binary units.
pub(crate) fn size(bytes: u64) -> String {
    const KIB: u64 = 1 << 10;
    const MIB: u64 = 1 << 20;
    if bytes < KIB {
        return format!("{bytes} B");
    }
    if bytes < MIB {
        return format!("{} KiB", significant(bytes as f64 / KIB as f64));
    }
    format!("{} MiB", significant(bytes as f64 / MIB as f64))
}

/// A count with thousands separated: `12,345`.
pub(crate) fn count(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// Bytes as lowercase hex in groups of four bytes, so a reader can count them.
pub(crate) fn hex_groups(bytes: &[u8]) -> String {
    bytes
        .chunks(4)
        .map(|group| group.iter().map(|byte| format!("{byte:02x}")).collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

fn significant(value: f64) -> String {
    if value < 10.0 {
        format!("{value:.2}")
    } else if value < 100.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_pick_their_unit() {
        assert_eq!(duration(812), "812 µs");
        assert_eq!(duration(12_345), "12.3 ms");
        assert_eq!(duration(1_240_000), "1.24 s");
        assert_eq!(elapsed(Duration::from_millis(1_420)), "1.4s");
    }

    #[test]
    fn sizes_use_binary_units() {
        assert_eq!(size(192), "192 B");
        assert_eq!(size(1_536), "1.50 KiB");
        assert_eq!(size(3 * 1024 * 1024), "3.00 MiB");
    }

    #[test]
    fn counts_and_hex_are_grouped() {
        assert_eq!(count(1_234_567), "1,234,567");
        assert_eq!(count(999), "999");
        assert_eq!(hex_groups(&[0xde, 0xad, 0xbe, 0xef, 0x01]), "deadbeef 01");
    }
}
