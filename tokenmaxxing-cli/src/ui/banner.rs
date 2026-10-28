//! The banner of the dashboard.
//!
//! The banner is the hand drawn logo of the project. The artwork is scaled
//! down to the size that fits into the terminal, and a terminal that is too
//! narrow for it gets a short text instead.

use ratatui::text::{Line, Span};

use super::logo::{self, LogoCell, SCALE_FACTORS};
use super::theme::Theme;

/// The subtitle below the logo.
const SUBTITLE: &str = "TOKEN ANALYSIS · COST OPTIMIZATION · BETTER CONTEXT";

/// The subtitle for terminals without unicode support.
const SUBTITLE_ASCII: &str = "TOKEN ANALYSIS | COST OPTIMIZATION | BETTER CONTEXT";

/// The text that is shown when the logo does not fit.
const FALLBACK: &str = "TokenMaxxing";

/// The number of rows of the artwork.
const ARTWORK_HEIGHT: usize = 12;

/// The artwork of the logo.
fn artwork() -> Vec<Vec<LogoCell>> {
    logo::parse(logo::source())
}

/// The width of the artwork.
fn artwork_width() -> usize {
    artwork().iter().map(Vec::len).max().unwrap_or(0)
}

/// The scale factor that fits into a terminal of the given width.
///
/// The sharpest variant wins. `None` means that even the smallest variant is
/// too wide, and the text fallback has to be used.
pub fn scale_factor(width: u16) -> Option<usize> {
    let source_width = artwork_width();

    SCALE_FACTORS
        .iter()
        .copied()
        .find(|factor| logo::scaled_width(source_width, *factor) <= usize::from(width))
}

/// The number of rows the banner needs, including the subtitle.
pub fn height(width: u16) -> u16 {
    match scale_factor(width) {
        Some(factor) => (ARTWORK_HEIGHT / factor) as u16 + 1,
        None => 2,
    }
}

/// The lines of the banner.
///
/// The logo comes first, the subtitle below it.
pub fn lines(width: u16, theme: &Theme) -> Vec<Line<'static>> {
    let Some(factor) = scale_factor(width) else {
        return fallback_lines(theme);
    };

    let scaled = logo::scale(&artwork(), factor);
    let mut lines = logo::lines(&scaled, theme.colored);

    lines.push(Line::from(Span::styled(
        subtitle(supports_unicode()),
        theme.dim(),
    )));

    lines
}

/// The lines that are shown when the logo does not fit.
fn fallback_lines(theme: &Theme) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(FALLBACK.to_string(), theme.title())),
        Line::from(Span::styled(subtitle(supports_unicode()), theme.dim())),
    ]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_sharpest_variant_fits_into_a_wide_terminal() {
        assert_eq!(scale_factor(120), Some(2));
        assert_eq!(scale_factor(111), Some(2));
    }

    #[test]
    fn test_a_narrow_terminal_gets_a_smaller_variant() {
        assert_eq!(scale_factor(100), Some(3));
        assert_eq!(scale_factor(74), Some(3));
        assert_eq!(scale_factor(60), Some(4));
        assert_eq!(scale_factor(56), Some(4));
    }

    #[test]
    fn test_a_terminal_that_is_too_narrow_gets_no_variant() {
        assert_eq!(scale_factor(55), None);
        assert_eq!(scale_factor(20), None);
    }

    #[test]
    fn test_the_height_grows_with_the_variant() {
        assert_eq!(height(120), 7, "six rows of logo and the subtitle");
        assert_eq!(height(100), 5, "four rows of logo and the subtitle");
        assert_eq!(height(60), 4, "three rows of logo and the subtitle");
        assert_eq!(height(40), 2, "the fallback and the subtitle");
    }

    #[test]
    fn test_the_banner_holds_the_logo_and_the_subtitle() {
        let theme = Theme::plain();
        let lines = lines(120, &theme);

        assert_eq!(lines.len(), 7);
        assert!(lines[6].spans[0].content.contains("TOKEN ANALYSIS"));
    }

    #[test]
    fn test_a_narrow_terminal_gets_the_fallback() {
        let theme = Theme::plain();
        let lines = lines(40, &theme);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].spans[0].content, FALLBACK);
    }

    #[test]
    fn test_the_logo_keeps_its_characters_without_colours() {
        let theme = Theme::plain();
        let lines = lines(120, &theme);

        let text: String = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();

        assert!(text.contains('█'), "the logo keeps its block characters");
        assert!(text.contains('▄') || text.contains('▀'));
    }

    #[test]
    fn test_the_subtitle_has_a_plain_fallback() {
        assert!(subtitle(true).contains('·'));
        assert!(!subtitle(false).contains('·'));
        assert!(subtitle(false).contains("TOKEN ANALYSIS"));
    }
}
