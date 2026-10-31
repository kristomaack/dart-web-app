use dart_core::Dart;

pub fn ordinal(n: u8) -> String {
    let suffix = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

pub fn route(darts: &[Dart]) -> String {
    darts.iter().map(|d| d.to_string()).collect::<Vec<_>>().join("  ")
}

/// Tally for a Close Out target: cricket-style slash/cross/circled cross for
/// three hits, otherwise a count like "2/5"; always ⊗ once closed.
pub fn marks(marks: u8, hits: u8) -> String {
    match (marks, hits) {
        (0, _) => String::new(),
        (m, h) if m >= h => "⊗".into(),
        (1, 3) => "╱".into(),
        (2, 3) => "✕".into(),
        (m, h) => format!("{m}/{h}"),
    }
}

pub fn date(ms: f64) -> String {
    let d = js_sys::Date::new(&ms.into());
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!(
        "{} {} · {:02}:{:02}",
        d.get_date(),
        MONTHS[d.get_month() as usize % 12],
        d.get_hours(),
        d.get_minutes()
    )
}
