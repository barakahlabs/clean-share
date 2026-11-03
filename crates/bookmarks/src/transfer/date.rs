//! Dates as exports write them, to ms since the epoch.

/// A Unix time in seconds, milliseconds or microseconds (browsers differ), or an ISO 8601 date
/// (`2026-03-01`, `2026-03-01T08:00:00Z`, `…T08:00:00.123+02:00`). `None` if it's neither.
pub fn parse(value: &str) -> Option<i64> {
  let value = value.trim();
  if let Ok(n) = value.parse::<i64>() {
    return Some(match n {
      n if n > 100_000_000_000_000 => n / 1000, // microseconds
      n if n > 100_000_000_000 => n,            // milliseconds
      n => n * 1000,                            // seconds
    })
    .filter(|&ms| ms > 0);
  }
  iso(value)
}

fn iso(value: &str) -> Option<i64> {
  let num = |s: &str| s.parse::<i64>().ok();
  let (date, time) = match value.split_once(['T', ' ']) {
    Some((d, t)) => (d, Some(t)),
    None => (value, None),
  };
  let mut ymd = date.splitn(3, '-');
  let (y, m, d) = (num(ymd.next()?)?, num(ymd.next()?)?, num(ymd.next()?)?);
  if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
    return None;
  }
  let mut ms = days_from_civil(y, m, d) * 86_400_000;
  if let Some(time) = time {
    // Split off the zone: "Z", "+02:00", "-0500".
    let (clock, offset_min) = match time.find(['Z', 'z', '+', '-']) {
      Some(at) => {
        let zone = &time[at..];
        let offset = if zone.eq_ignore_ascii_case("z") {
          0
        } else {
          let sign = if zone.starts_with('-') { -1 } else { 1 };
          let digits: String = zone[1..].chars().filter(char::is_ascii_digit).collect();
          let (h, m) = digits.split_at(digits.len().min(2));
          sign * (num(h).unwrap_or(0) * 60 + num(m).unwrap_or(0))
        };
        (&time[..at], offset)
      }
      None => (time, 0),
    };
    let mut hms = clock.split(':');
    let h = num(hms.next()?)?;
    let min = hms.next().map_or(Some(0), num)?;
    let sec: f64 = hms.next().map_or(Some(0.0), |s| s.parse().ok())?;
    ms += h * 3_600_000 + min * 60_000 + (sec * 1000.0) as i64 - offset_min * 60_000;
  }
  Some(ms)
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
  let y = if m <= 2 { y - 1 } else { y };
  let era = y.div_euclid(400);
  let yoe = y - era * 400;
  let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
  let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
  era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
  use super::parse;

  #[test]
  fn unix_times_in_any_unit() {
    assert_eq!(parse("1700000000"), Some(1_700_000_000_000));
    assert_eq!(parse("1700000000123"), Some(1_700_000_000_123));
    assert_eq!(parse("1700000000123456"), Some(1_700_000_000_123));
    assert_eq!(parse("0"), None);
  }

  #[test]
  fn iso_dates() {
    assert_eq!(parse("1970-01-02"), Some(86_400_000));
    assert_eq!(parse("2023-11-14T22:13:20Z"), Some(1_700_000_000_000));
    assert_eq!(parse("2023-11-14T22:13:20.5Z"), Some(1_700_000_000_500));
    assert_eq!(parse("2023-11-15T00:13:20+02:00"), Some(1_700_000_000_000));
    assert_eq!(parse("2023-11-14 17:13:20-0500"), Some(1_700_000_000_000));
    assert_eq!(parse("yesterday"), None);
    assert_eq!(parse("2023-13-01"), None);
  }
}
