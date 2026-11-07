// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Boot-impact normalization (Phase 5).
//!
//! Every platform signal is reduced to a per-item duration in milliseconds,
//! then mapped to the shared High/Medium/Low scale by [`impact_from_millis`].

use crate::model::BootImpact;
use std::collections::HashMap;

/// Items at or above this startup cost are High impact.
pub const HIGH_THRESHOLD_MS: u64 = 3_000;
/// Items at or above this startup cost (and below High) are Medium impact.
pub const MEDIUM_THRESHOLD_MS: u64 = 500;

pub fn impact_from_millis(ms: u64) -> BootImpact {
    if ms >= HIGH_THRESHOLD_MS {
        BootImpact::High
    } else if ms >= MEDIUM_THRESHOLD_MS {
        BootImpact::Medium
    } else {
        BootImpact::Low
    }
}

/// Parse the output of `systemd-analyze [--user] blame`.
///
/// Lines look like `  2.345s NetworkManager.service`, `  850ms foo.service`,
/// `  1min 2.100s some.timer`. Lines that do not parse are skipped. Returns a
/// map from unit name to milliseconds.
pub fn parse_systemd_blame(output: &str) -> HashMap<String, u64> {
    output
        .lines()
        .filter_map(|line| {
            // A duration may span several tokens ("1min 2.100s"); the unit
            // name is always the last token.
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let (unit, durations) = tokens.split_last()?;
            if durations.is_empty() {
                return None;
            }
            let mut total_ms: u64 = 0;
            for tok in durations {
                total_ms = total_ms.checked_add(parse_duration_token(tok)?)?;
            }
            Some((unit.to_string(), total_ms))
        })
        .collect()
}

/// Parse one systemd duration token: `2.345s`, `850ms`, `1min`, `2h`, `1d`,
/// `300us`. Returns milliseconds.
fn parse_duration_token(tok: &str) -> Option<u64> {
    let split = tok.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
    let (num, unit) = tok.split_at(split);
    let value: f64 = num.parse().ok()?;
    let factor_ms = match unit {
        "us" => 0.001,
        "ms" => 1.0,
        "s" => 1_000.0,
        "min" => 60_000.0,
        "h" => 3_600_000.0,
        "d" => 86_400_000.0,
        _ => return None,
    };
    Some((value * factor_ms).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_map_to_tiers() {
        assert_eq!(impact_from_millis(0), BootImpact::Low);
        assert_eq!(impact_from_millis(499), BootImpact::Low);
        assert_eq!(impact_from_millis(500), BootImpact::Medium);
        assert_eq!(impact_from_millis(2_999), BootImpact::Medium);
        assert_eq!(impact_from_millis(3_000), BootImpact::High);
    }

    #[test]
    fn parses_blame_output() {
        let out = "\
          2.345s NetworkManager.service
           850ms foo.service
          1min 2.100s slow-boot.service
          bogus line here
                 300us tiny.timer
";
        let map = parse_systemd_blame(out);
        assert_eq!(map.get("NetworkManager.service"), Some(&2_345));
        assert_eq!(map.get("foo.service"), Some(&850));
        assert_eq!(map.get("slow-boot.service"), Some(&62_100));
        assert_eq!(map.get("tiny.timer"), Some(&0));
        assert!(!map.contains_key("bogus"));
        assert_eq!(map.len(), 4);
    }

    #[test]
    fn empty_and_garbage_input_yields_nothing() {
        assert!(parse_systemd_blame("").is_empty());
        assert!(parse_systemd_blame("unit-only.service").is_empty());
        assert!(parse_systemd_blame("12xyz foo.service").is_empty());
    }
}
