//! How each game is played, shown from the (i) button on the home screen.
//! Plain language, short sections, most important first.

use dart_core::ModeKind;

pub struct Section {
    pub heading: &'static str,
    pub body: &'static str,
}

pub fn rules(kind: ModeKind) -> &'static [Section] {
    match kind {
        ModeKind::X01 => &[
            Section {
                heading: "Goal",
                body: "Start on 301 or 501 and be the first to bring your score down to exactly zero.",
            },
            Section {
                heading: "Your turn",
                body: "Throw three darts. Each one is taken off your score: a double counts twice the number, \
                       a treble three times. The outer bull is 25 and the bullseye 50.",
            },
            Section {
                heading: "Bust",
                body: "If a dart would take you below zero, the turn is a bust. Your score goes back to what it \
                       was at the start of the turn, and the next player throws.",
            },
            Section {
                heading: "Double in",
                body: "Optional. Your score only starts going down from your first double.",
            },
            Section {
                heading: "Double out",
                body: "Optional. You have to finish on a double or the bullseye, so being left on 1 is also a \
                       bust. When a finish is possible, the app suggests a route.",
            },
        ],
        ModeKind::Cricket => &[
            Section {
                heading: "Goal",
                body: "Close 15–20 and the Bull before everyone else. Other numbers don't count.",
            },
            Section {
                heading: "Marks",
                body: "A single is 1 mark, a double 2, a treble 3 and the bullseye 2. Three marks close a \
                       number, or 1, 5 or 7 if you change it in setup.",
            },
            Section {
                heading: "Points",
                body: "On by default. Hits on a number you've closed score its value (the Bull is 25) while an \
                       opponent still has it open. Once everyone has closed it, it's dead.",
            },
            Section {
                heading: "Winning",
                body: "Close everything with at least as many points as everyone else. If you're behind, keep \
                       scoring until you catch up. With points off, the first to close everything wins.",
            },
        ],
    }
}
