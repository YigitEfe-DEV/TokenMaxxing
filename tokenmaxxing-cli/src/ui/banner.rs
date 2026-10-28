//! The banner of the dashboard.
//!
//! The banner is a minimal wordmark: the name of the project in a crisp front
//! face with a restrained extrusion behind it. The depth comes from the
//! typography, not from a picture.
//!
//! The wordmark uses two rows, the subtitle below it makes three in total.

use ratatui::text::{Line, Span};

use super::theme::Theme;

/// The name of the project.
const WORDMARK: &str = "TokenMaxxing";

/// The subtitle below the wordmark.
const SUBTITLE: &str = "TOKEN ANALYSIS · COST OPTIMIZATION · BETTER CONTEXT";

/// The subtitle for terminals without unicode support.
const SUBTITLE_ASCII: &str = "TOKEN ANALYSIS | COST OPTIMIZATION | BETTER CONTEXT";

/// The rows of the wordmark: the extrusion and the front face.
const WORDMARK_HEIGHT: u16 = 2;

/// The smallest terminal that shows the wordmark and the subtitle.
const MIN_WIDTH: u16 = 24;

/// The number of rows the banner needs.
pub fn height(width: u16) -> u16 {
    if width < MIN_WIDTH {
        1
    } else {
        WORDMARK_HEIGHT + 1
    }
}

/// The lines of the banner.
///
/// The extrusion sits one column to the right of the front face, so the word
/// reads as a solid shape that recedes into the background.
pub fn lines(width: u16, theme: &Theme) -> Vec<Line<'static>> {
    if width < MIN_WIDTH {
        return vec![fallback(theme)];
    }

    // The wordmark is placed in the middle, the extrusion sits one column to
    // the right of the front face.
    let offset = " ".repeat(padding(width));
    let front = format!("{offset}{WORDMARK}");
    let depth = format!("{offset} {WORDMARK}");
    let subtitle = centered(&subtitle(supports_unicode()), width);

    vec![
        Line::from(Span::styled(depth, theme.wordmark_depth())),
        Line::from(Span::styled(front, theme.wordmark())),
        Line::from(Span::styled(subtitle, theme.dim())),
    ]
}

/// The wordmark without the depth, for a terminal that is too small for it.
pub fn fallback(theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(WORDMARK.to_string(), theme.wordmark()))
}

/// The number of spaces that put the wordmark in the middle of the width.
fn padding(width: u16) -> usize {
    let word = WORDMARK.chars().count() as u16;
    if word >= width {
        return 0;
    }

    usize::from((width - word) / 2)
}

/// Puts the text in the middle of the given width.
fn centered(text: &str, width: u16) -> String {
    let text_width = text.chars().count() as u16;
    if text_width >= width {
        return text.to_string();
    }

    let padding = (width - text_width) / 2;
    format!("{}{text}", " ".repeat(padding as usize))
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

    /// The text of a line.
    fn text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    #[test]
    fn test_the_wordmark_spells_the_name() {
        let theme = Theme::plain();
        let lines = lines(120, &theme);

        assert_eq!(text(&lines[1]).trim(), "TokenMaxxing");
        assert_eq!(text(&lines[0]).trim(), "TokenMaxxing");
    }

    #[test]
    fn test_the_banner_uses_three_rows() {
        let theme = Theme::plain();

        assert_eq!(lines(120, &theme).len(), 3);
        assert_eq!(lines(80, &theme).len(), 3);
        assert_eq!(height(120), 3);
        assert_eq!(height(80), 3);
    }

    #[test]
    fn test_the_extrusion_is_offset_by_one_column() {
        let theme = Theme::plain();
        let lines = lines(120, &theme);

        let front = text(&lines[1]);
        let depth = text(&lines[0]);

        assert_eq!(front.trim_start(), WORDMARK);
        assert_eq!(depth.trim_start(), WORDMARK);
        assert_eq!(
            depth.find(WORDMARK),
            front.find(WORDMARK).map(|column| column + 1),
            "the extrusion sits one column to the right"
        );
    }

    #[test]
    fn test_the_wordmark_is_centered() {
        let theme = Theme::plain();
        let lines = lines(120, &theme);
        let front = text(&lines[1]);
        let padding = front.len() - front.trim_start().len();

        // The word is in the middle, the room on the left and on the right is
        // the same.
        let word = "TokenMaxxing".len();
        assert_eq!(padding, (120 - word) / 2);
    }

    #[test]
    fn test_a_narrow_terminal_gets_the_fallback() {
        let theme = Theme::plain();
        let lines = lines(20, &theme);

        assert_eq!(lines.len(), 1);
        assert_eq!(text(&lines[0]).trim(), "TokenMaxxing");
        assert_eq!(height(20), 1);
    }

    #[test]
    fn test_the_fallback_never_clips_the_word() {
        let theme = Theme::plain();

        for width in [1u16, 5, 12, 20, 23] {
            let lines = lines(width, &theme);
            assert_eq!(text(&lines[0]).trim(), "TokenMaxxing", "width {width}");
        }
    }

    #[test]
    fn test_the_subtitle_has_a_plain_fallback() {
        assert!(subtitle(true).contains('·'));
        assert!(!subtitle(false).contains('·'));
        assert!(subtitle(false).contains("TOKEN ANALYSIS"));
    }

    #[test]
    fn test_the_banner_holds_the_subtitle() {
        let theme = Theme::plain();
        let lines = lines(120, &theme);

        assert!(text(&lines[2]).contains("TOKEN ANALYSIS"));
    }
}
