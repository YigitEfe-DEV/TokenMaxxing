//! Colors and styles of the dashboard.
//!
//! The palette is restrained: cyan for the structure, green for exact
//! measurements, purple for the provider and yellow for warnings. Terminals
//! that cannot show colors get the same layout without any color.

use ratatui::style::{Color, Modifier, Style};

/// The palette of the dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Whether the terminal can show colors.
    pub colored: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self::detect()
    }
}

impl Theme {
    /// Detects what the terminal supports.
    ///
    /// `NO_COLOR` and a `dumb` terminal switch the colors off, which is the
    /// convention that these environments follow.
    pub fn detect() -> Self {
        let colored = std::env::var_os("NO_COLOR").is_none()
            && std::env::var("TERM")
                .map(|term| term != "dumb" && !term.is_empty())
                .unwrap_or(true);
        Self { colored }
    }

    /// A theme without any color, used by the tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn plain() -> Self {
        Self { colored: false }
    }

    fn style(&self, color: Color) -> Style {
        if self.colored {
            Style::default().fg(color)
        } else {
            Style::default()
        }
    }

    /// The accent of the interface.
    pub fn accent(&self) -> Style {
        self.style(Color::Cyan)
    }

    /// The style of a title.
    pub fn title(&self) -> Style {
        self.style(Color::Cyan).add_modifier(Modifier::BOLD)
    }

    /// The style of a measurement that is exact.
    pub fn exact(&self) -> Style {
        self.style(Color::Green)
    }

    /// The style of a measurement that is an estimate.
    pub fn estimate(&self) -> Style {
        self.style(Color::Yellow)
    }

    /// The style of provider information.
    pub fn provider(&self) -> Style {
        self.style(Color::Magenta)
    }

    /// The style of secondary information.
    pub fn dim(&self) -> Style {
        if self.colored {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().add_modifier(Modifier::DIM)
        }
    }

    /// The style of the focused panel.
    pub fn focused(&self) -> Style {
        if self.colored {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        }
    }

    /// The style of a warning.
    pub fn warning(&self) -> Style {
        self.style(Color::Yellow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_the_plain_theme_has_no_colors() {
        let theme = Theme::plain();

        assert!(!theme.colored);
        assert_eq!(theme.accent().fg, None);
        assert_eq!(theme.dim().fg, None);
    }

    #[test]
    fn test_the_colored_theme_uses_the_palette() {
        let theme = Theme { colored: true };

        assert_eq!(theme.accent().fg, Some(Color::Cyan));
        assert_eq!(theme.exact().fg, Some(Color::Green));
        assert_eq!(theme.estimate().fg, Some(Color::Yellow));
        assert_eq!(theme.provider().fg, Some(Color::Magenta));
    }
}
