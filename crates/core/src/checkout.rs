//! Suggested finishes for X01, found by searching every combination of up to
//! three darts and picking the cheapest by a "how easy is this to hit" cost.

use crate::dart::{Dart, Segment};

/// Lower is easier/more conventional.
fn cost(d: &Dart, finishing: bool) -> u32 {
    use crate::dart::Mult::*;
    match (d.segment, d.mult) {
        (Segment::Num(n), Single) => if finishing { 4 } else { 1 + (n < 10) as u32 },
        (Segment::Num(20 | 16), Double) if finishing => 0,
        (Segment::Num(18 | 12 | 10 | 8), Double) if finishing => 1,
        (Segment::Num(_), Double) => if finishing { 2 } else { 6 },
        (Segment::Num(20 | 19), Triple) => 2,
        (Segment::Num(_), Triple) => 3,
        (Segment::Bull, Single) => 5,
        (Segment::Bull, _) => if finishing { 3 } else { 6 },
        (Segment::Miss, _) => u32::MAX / 8,
    }
}

fn all_darts() -> Vec<Dart> {
    let mut v: Vec<Dart> = (1..=20)
        .rev()
        .flat_map(|n| [Dart::triple(n), Dart::single(n), Dart::double(n)])
        .collect();
    v.push(Dart::outer_bull());
    v.push(Dart::bullseye());
    v
}

/// Best way to score exactly `score` with at most `darts` darts, ending on a
/// double (or bullseye) when `double_out` is set.
pub fn checkout(score: u16, darts: usize, double_out: bool) -> Option<Vec<Dart>> {
    if score == 0 || darts == 0 || score as usize > 60 * darts {
        return None;
    }
    let all = all_darts();
    let finishers: Vec<Dart> = all.iter().copied().filter(|d| !double_out || d.is_double()).collect();

    let mut best: Option<(u32, Vec<Dart>)> = None;
    let mut consider = |route: Vec<Dart>| {
        let n = route.len();
        let c = 100 * n as u32
            + route.iter().enumerate().map(|(i, d)| cost(d, i + 1 == n)).sum::<u32>();
        if best.as_ref().is_none_or(|(bc, _)| c < *bc) {
            best = Some((c, route));
        }
    };

    for f in &finishers {
        let Some(rest) = score.checked_sub(f.points()) else { continue };
        if rest == 0 {
            consider(vec![*f]);
            continue;
        }
        for a in all.iter().take_while(|_| darts >= 2) {
            match rest.checked_sub(a.points()) {
                Some(0) => consider(vec![*a, *f]),
                Some(r) if darts >= 3 => {
                    for b in &all {
                        if b.points() == r {
                            consider(vec![*a, *b, *f]);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    best.map(|(_, r)| r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_finishes() {
        assert_eq!(checkout(170, 3, true).unwrap(), vec![Dart::triple(20), Dart::triple(20), Dart::bullseye()]);
        assert_eq!(checkout(40, 3, true).unwrap(), vec![Dart::double(20)]);
        assert_eq!(checkout(100, 3, true).unwrap(), vec![Dart::triple(20), Dart::double(20)]);
        assert_eq!(checkout(50, 1, true).unwrap(), vec![Dart::bullseye()]);
        assert!(checkout(169, 3, true).is_none());
        assert!(checkout(1, 3, true).is_none());
        assert!(checkout(100, 1, true).is_none());
        assert_eq!(checkout(60, 1, false).unwrap(), vec![Dart::triple(20)]);
    }

    #[test]
    fn every_hint_is_a_real_double_out() {
        for s in 2..=170u16 {
            if let Some(route) = checkout(s, 3, true) {
                assert_eq!(route.iter().map(|d| d.points()).sum::<u16>(), s, "{s}");
                assert!(route.last().unwrap().is_double(), "{s}");
            } else {
                assert!(matches!(s, 159 | 162 | 163 | 165 | 166 | 168 | 169), "no route for {s}");
            }
        }
    }
}
