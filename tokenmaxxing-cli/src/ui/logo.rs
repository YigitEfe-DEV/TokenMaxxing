//! The custom TokenMaxxing logo.
//!
//! The artwork is the hand drawn logo of the project. It is stored the way it
//! was written down: a file of Python `print()` statements with ANSI colour
//! sequences inside them. This module turns that file into cells, scales it
//! down and hands it to ratatui.
//!
//! The scaling is a uniform downscale by an integer factor: every output cell
//! takes the character that appears most often in the block of source cells it
//! covers, and the colour of the most common colour of that character. A
//! uniform factor keeps the proportions of the logo, because a terminal cell
//! is about twice as tall as it is wide.

use std::collections::BTreeMap;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// The artwork as it was written down.
const SOURCE: &str = include_str!("../../assets/tokenmaxxing_logo.txt");

/// The scale factors the dashboard offers, from the sharpest to the smallest.
pub const SCALE_FACTORS: [usize; 3] = [2, 3, 4];

/// A colour of the artwork.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogoColor {
    /// The foreground of the terminal.
    Default,
    /// White.
    White,
    /// Bright white.
    BrightWhite,
    /// Red.
    Red,
    /// Bright red.
    BrightRed,
    /// Grey.
    Gray,
}

impl LogoColor {
    /// The colour of the terminal.
    pub fn terminal_color(self) -> Color {
        match self {
            LogoColor::Default => Color::Reset,
            LogoColor::White => Color::White,
            LogoColor::BrightWhite => Color::Gray,
            LogoColor::Red => Color::Red,
            LogoColor::BrightRed => Color::LightRed,
            LogoColor::Gray => Color::DarkGray,
        }
    }
}

/// One cell of the artwork.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogoCell {
    /// The character of the cell.
    pub character: char,
    /// The colour of the character.
    pub foreground: LogoColor,
    /// The colour behind the character, when the artwork has one.
    pub background: Option<LogoColor>,
}

impl LogoCell {
    /// An empty cell.
    pub fn empty() -> Self {
        Self {
            character: ' ',
            foreground: LogoColor::Default,
            background: None,
        }
    }
}

/// How much of a cell a character covers.
///
/// The value decides which character wins when two of them appear equally
/// often in the block that becomes one cell.
fn ink(character: char) -> u16 {
    match character {
        '█' => 255,
        '▓' => 191,
        '▒' => 127,
        '░' | '▀' | '▄' => 96,
        '·' => 40,
        '│' | '┼' | '├' | '┤' | '┬' | '┴' | '╪' | '╬' => 204,
        '─' | '═' => 76,
        '▌' | '▐' => 153,
        ' ' => 0,
        _ => 128,
    }
}

/// The colour of an ANSI foreground code.
fn foreground_of(code: &str) -> LogoColor {
    match code {
        "37" => LogoColor::White,
        "97" => LogoColor::BrightWhite,
        "31" => LogoColor::Red,
        "91" => LogoColor::BrightRed,
        "90" => LogoColor::Gray,
        _ => LogoColor::Default,
    }
}

/// The colour of an ANSI background code.
fn background_of(code: &str) -> Option<LogoColor> {
    match code {
        "41" => Some(LogoColor::Red),
        "47" => Some(LogoColor::White),
        _ => None,
    }
}

/// The artwork of the logo.
pub fn source() -> &'static str {
    SOURCE
}

/// Parses the source file into rows of cells.
///
/// The file holds Python `print()` statements, so the quotes around the
/// content and the escaped escape sequences have to be taken apart first.
pub fn parse(source: &str) -> Vec<Vec<LogoCell>> {
    source
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_row)
        .collect()
}

/// Parses a single row of the source file.
fn parse_row(line: &str) -> Vec<LogoCell> {
    let trimmed = line.trim();
    let content = match trimmed.strip_prefix("print(") {
        Some(rest) => rest.strip_suffix(')').unwrap_or(rest),
        None => trimmed,
    };
    // The quotes are part of the statement, the content is everything between.
    let content = content
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(content);

    let mut cells = Vec::new();
    let mut foreground = LogoColor::Default;
    let mut background: Option<LogoColor> = None;
    let mut rest = content;

    while !rest.is_empty() {
        if let Some(stripped) = rest.strip_prefix("\\033[") {
            if let Some(end) = stripped.find('m') {
                for code in stripped[..end].split(';') {
                    if code == "0" {
                        foreground = LogoColor::Default;
                        background = None;
                    } else if let Some(color) = background_of(code) {
                        background = Some(color);
                    } else {
                        foreground = foreground_of(code);
                    }
                }
                rest = &stripped[end + 1..];
                continue;
            }
        }

        let mut characters = rest.chars();
        let character = characters.next().unwrap_or(' ');
        cells.push(LogoCell {
            character,
            foreground,
            background,
        });
        rest = characters.as_str();
    }

    cells
}

/// Scales the artwork down by an integer factor.
///
/// Every output cell covers a block of `factor` by `factor` source cells and
/// takes the character that appears most often in that block. Spaces do not
/// take part in the count, so a block that holds a stroke keeps the stroke.
pub fn scale(rows: &[Vec<LogoCell>], factor: usize) -> Vec<Vec<LogoCell>> {
    if factor <= 1 {
        return rows.to_vec();
    }

    let width = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut scaled = Vec::new();

    for block_row in rows.chunks(factor) {
        let mut line = Vec::new();

        for column in 0..width.div_ceil(factor) {
            let mut block = Vec::new();
            for row in block_row {
                for cell in row.iter().skip(column * factor).take(factor) {
                    block.push(*cell);
                }
            }

            line.push(reduce(&block));
        }

        scaled.push(line);
    }

    scaled
}

/// Reduces a block of cells to the single cell that represents it.
fn reduce(block: &[LogoCell]) -> LogoCell {
    let mut characters: BTreeMap<char, usize> = BTreeMap::new();
    for cell in block {
        if cell.character != ' ' {
            *characters.entry(cell.character).or_default() += 1;
        }
    }

    if characters.is_empty() {
        return LogoCell::empty();
    }

    // The character that appears most often wins. When two of them appear
    // equally often the one that covers more of the cell wins, and when even
    // that is equal the smaller code point wins, so the result never depends
    // on the order of the map.
    let character = characters
        .iter()
        .max_by(|left, right| {
            left.1
                .cmp(right.1)
                .then_with(|| ink(*right.0).cmp(&ink(*left.0)))
                .then_with(|| right.0.cmp(left.0))
        })
        .map(|(character, _)| *character)
        .unwrap_or(' ');

    let mut colours: BTreeMap<(LogoColor, Option<LogoColor>), usize> = BTreeMap::new();
    for cell in block {
        if cell.character == character {
            *colours
                .entry((cell.foreground, cell.background))
                .or_default() += 1;
        }
    }

    let ((foreground, background), _) = colours
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .unwrap_or(((LogoColor::Default, None), 0));

    LogoCell {
        character,
        foreground,
        background,
    }
}

/// The width of the artwork when it is scaled by `factor`.
pub fn scaled_width(source_width: usize, factor: usize) -> usize {
    source_width.div_ceil(factor)
}

/// The lines of the scaled artwork, ready for ratatui.
///
/// Cells that share a style become one span, so a row of the logo is a handful
/// of spans instead of a hundred.
pub fn lines(rows: &[Vec<LogoCell>], colored: bool) -> Vec<Line<'static>> {
    rows.iter()
        .map(|row| {
            let mut spans: Vec<Span<'static>> = Vec::new();
            let mut buffer = String::new();
            let mut current: Option<Style> = None;

            for cell in row {
                let style = style_of(cell, colored);
                if current != Some(style) {
                    if let Some(style) = current.take() {
                        if !buffer.is_empty() {
                            spans.push(Span::styled(std::mem::take(&mut buffer), style));
                        }
                    }
                    current = Some(style);
                }
                buffer.push(cell.character);
            }

            if let Some(style) = current {
                if !buffer.is_empty() {
                    spans.push(Span::styled(buffer, style));
                }
            }

            Line::from(spans)
        })
        .collect()
}

/// The style of a cell.
fn style_of(cell: &LogoCell, colored: bool) -> Style {
    if !colored {
        return Style::default();
    }

    let mut style = Style::default().fg(cell.foreground.terminal_color());
    if let Some(background) = cell.background {
        style = style.bg(background.terminal_color());
    }
    style
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The artwork of the logo as it is stored in the repository.
    fn parsed() -> Vec<Vec<LogoCell>> {
        parse(SOURCE)
    }

    #[test]
    fn test_the_source_holds_twelve_rows() {
        let rows = parsed();

        assert_eq!(rows.len(), 12, "the artwork has twelve rows");
        for row in &rows {
            assert_eq!(row.len(), 222, "every row is 222 cells wide");
        }
    }

    #[test]
    fn test_the_parser_takes_the_print_statements_apart() {
        let rows = parse("print(\"\\033[0;31mA\\033[0m\")");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), 1);
        assert_eq!(rows[0][0].character, 'A');
        assert_eq!(rows[0][0].foreground, LogoColor::Red);
    }

    #[test]
    fn test_the_parser_keeps_the_colours() {
        let rows = parsed();
        let colours: Vec<LogoColor> = rows
            .iter()
            .flat_map(|row| row.iter())
            .map(|cell| cell.foreground)
            .collect();

        assert!(colours.contains(&LogoColor::Red));
        assert!(colours.contains(&LogoColor::BrightRed));
        assert!(colours.contains(&LogoColor::White));
        assert!(colours.contains(&LogoColor::BrightWhite));
        assert!(colours.contains(&LogoColor::Gray));
    }

    #[test]
    fn test_the_parser_keeps_the_backgrounds() {
        let rows = parsed();
        let backgrounds: Vec<Option<LogoColor>> = rows
            .iter()
            .flat_map(|row| row.iter())
            .map(|cell| cell.background)
            .collect();

        assert!(backgrounds.contains(&Some(LogoColor::Red)));
        assert!(backgrounds.contains(&Some(LogoColor::White)));
    }

    #[test]
    fn test_the_scaling_halves_the_size() {
        let rows = parsed();
        let scaled = scale(&rows, 2);

        assert_eq!(scaled.len(), 6, "twelve rows become six");
        assert_eq!(scaled[0].len(), 111, "222 cells become 111");
    }

    #[test]
    fn test_the_scaling_keeps_the_line_structure() {
        let rows = parsed();

        for factor in SCALE_FACTORS {
            let scaled = scale(&rows, factor);
            assert_eq!(scaled.len(), 12 / factor);
            assert_eq!(scaled[0].len(), 222usize.div_ceil(factor));
        }
    }

    #[test]
    fn test_the_scaling_keeps_the_colours() {
        let rows = parsed();
        let scaled = scale(&rows, 2);
        let colours: Vec<LogoColor> = scaled
            .iter()
            .flat_map(|row| row.iter())
            .map(|cell| cell.foreground)
            .collect();

        assert!(colours.contains(&LogoColor::Red));
        assert!(colours.contains(&LogoColor::BrightRed));
        assert!(colours.contains(&LogoColor::White));
        assert!(colours.contains(&LogoColor::Gray));
    }

    #[test]
    fn test_the_scaling_keeps_the_shape_of_the_strokes() {
        let rows = parsed();
        let scaled = scale(&rows, 2);

        // The top of the logo is a band that spans the whole width, so the
        // first row of the scaled logo has to be filled over its whole width.
        let filled = scaled[0]
            .iter()
            .filter(|cell| cell.character != ' ')
            .count();
        assert!(filled > 90, "the top edge survives the scaling");

        // The strokes of the letters are made of block characters, they have
        // to survive as well.
        let strokes = scaled
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| matches!(cell.character, '█' | '│' | '┼' | '▀' | '▄'))
            .count();
        assert!(strokes > 100, "the strokes of the letters survive");
    }

    #[test]
    fn test_the_scaling_is_deterministic() {
        let rows = parsed();

        assert_eq!(scale(&rows, 2), scale(&rows, 2));
    }

    #[test]
    fn test_the_lines_are_ready_for_ratatui() {
        let rows = parsed();
        let scaled = scale(&rows, 2);
        let lines = lines(&scaled, true);

        assert_eq!(lines.len(), 6);
        for line in &lines {
            assert!(!line.spans.is_empty());
        }
    }

    #[test]
    fn test_without_colours_the_characters_stay_the_same() {
        let rows = parsed();
        let scaled = scale(&rows, 2);
        let colored = lines(&scaled, true);
        let plain = lines(&scaled, false);

        let text = |lines: &[Line<'_>]| {
            lines
                .iter()
                .map(|line| {
                    line.spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(text(&colored), text(&plain));
    }
}
