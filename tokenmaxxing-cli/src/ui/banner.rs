//! The ASCII banner of the dashboard.

/// The letters of the banner, five rows each.
///
/// The font is deliberately narrow so that the banner fits into an eighty
/// column terminal.
const GLYPHS: [(char, [&str; 5]); 12] = [
    ('T', ["█████", " ███ ", " ███ ", " ███ ", " ███ "]),
    ('O', [" ███ ", "█   █", "█   █", "█   █", " ███ "]),
    ('K', ["█   █", "█  █ ", "███  ", "█  █ ", "█   █"]),
    ('E', ["█████", "█    ", "███  ", "█    ", "█████"]),
    ('N', ["█   █", "██  █", "█ █ █", "█  ██", "█   █"]),
    ('M', ["█   █", "██ ██", "█ █ █", "█   █", "█   █"]),
    ('A', [" ███ ", "█  █ ", "█████", "█  █ ", "█  █ "]),
    ('X', ["█   █", " █ █ ", "  █  ", " █ █ ", "█   █"]),
    ('X', ["█   █", " █ █ ", "  █  ", " █ █ ", "█   █"]),
    ('I', ["█████", "  █  ", "  █  ", "  █  ", "█████"]),
    ('N', ["█   █", "██  █", "█ █ █", "█  ██", "█   █"]),
    ('G', [" ███ ", "█   █", "█  ██", "█   █", " ███ "]),
];

/// The text the banner spells.
const WORD: &str = "TOKENMAXXING";

/// The subtitle below the banner.
const SUBTITLE: &str = "TOKEN ANALYSIS · COST OPTIMIZATION · BETTER CONTEXT";

/// The subtitle for terminals without unicode support.
const SUBTITLE_ASCII: &str = "TOKEN ANALYSIS | COST OPTIMIZATION | BETTER CONTEXT";

/// The number of rows of the banner.
pub const HEIGHT: usize = 5;

/// The width of the banner in columns.
pub fn width() -> usize {
    GLYPHS
        .iter()
        .map(|(_, glyph)| glyph[0].chars().count())
        .sum::<usize>()
        + GLYPHS.len().saturating_sub(1)
}

/// The rows of the banner.
pub fn lines() -> Vec<String> {
    (0..HEIGHT)
        .map(|row| {
            GLYPHS
                .iter()
                .map(|(_, glyph)| glyph[row])
                .collect::<Vec<&str>>()
                .join(" ")
        })
        .collect()
}

/// The subtitle, in unicode when the terminal can show it.
pub fn subtitle(unicode: bool) -> String {
    if unicode {
        SUBTITLE.to_string()
    } else {
        SUBTITLE_ASCII.to_string()
    }
}

/// Whether the terminal can show unicode.
pub fn supports_unicode() -> bool {
    for name in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(value) = std::env::var(name) {
            if value.to_ascii_uppercase().contains("UTF-8") {
                return true;
            }
            if !value.is_empty() {
                return false;
            }
        }
    }
    true
}

/// A compact banner for terminals that are too narrow for the full one.
pub fn compact() -> String {
    format!(" {WORD} ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_banner_spells_the_name() {
        let lines = lines();

        assert_eq!(lines.len(), HEIGHT);
        for line in &lines {
            assert_eq!(line.chars().count(), width());
        }
    }

    #[test]
    fn test_the_banner_fits_into_eighty_columns() {
        assert!(width() <= 80, "the banner is {} columns wide", width());
    }

    #[test]
    fn test_every_glyph_has_five_rows_of_the_same_width() {
        for (letter, glyph) in GLYPHS {
            assert_eq!(glyph.len(), HEIGHT, "the glyph of {letter}");
            for row in glyph {
                assert_eq!(row.chars().count(), 5, "the glyph of {letter}");
            }
        }
    }

    #[test]
    fn test_the_glyphs_are_the_letters_of_the_name() {
        let letters: String = GLYPHS.iter().map(|(letter, _)| *letter).collect();

        assert_eq!(letters, WORD);
    }

    #[test]
    fn test_the_subtitle_has_a_plain_fallback() {
        assert!(subtitle(true).contains('·'));
        assert!(!subtitle(false).contains('·'));
        assert!(subtitle(false).contains("TOKEN ANALYSIS"));
    }
}
