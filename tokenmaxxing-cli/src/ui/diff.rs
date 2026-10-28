//! A small line diff for the preview of a proposed change.
//!
//! The diff only has to show what a change does, so it trims the common
//! beginning and the common end of the two texts and shows the rest.

/// One line of a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    /// A line that stays.
    Context(String),
    /// A line that goes away.
    Removed(String),
    /// A line that comes in.
    Added(String),
}

impl DiffLine {
    /// The text of the line.
    pub fn text(&self) -> &str {
        match self {
            DiffLine::Context(text) | DiffLine::Removed(text) | DiffLine::Added(text) => text,
        }
    }

    /// The marker of the line.
    pub fn marker(&self) -> &'static str {
        match self {
            DiffLine::Context(_) => " ",
            DiffLine::Removed(_) => "-",
            DiffLine::Added(_) => "+",
        }
    }
}

/// Computes the difference between two texts.
pub fn diff_lines(before: &str, after: &str) -> Vec<DiffLine> {
    let before: Vec<&str> = before.lines().collect();
    let after: Vec<&str> = after.lines().collect();

    let mut prefix = 0;
    while prefix < before.len() && prefix < after.len() && before[prefix] == after[prefix] {
        prefix += 1;
    }

    let mut suffix = 0;
    while suffix < before.len() - prefix
        && suffix < after.len() - prefix
        && before[before.len() - 1 - suffix] == after[after.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let mut lines = Vec::new();

    for line in &before[..prefix] {
        lines.push(DiffLine::Context((*line).to_string()));
    }
    for line in &before[prefix..before.len() - suffix] {
        lines.push(DiffLine::Removed((*line).to_string()));
    }
    for line in &after[prefix..after.len() - suffix] {
        lines.push(DiffLine::Added((*line).to_string()));
    }
    for line in &before[before.len() - suffix..] {
        lines.push(DiffLine::Context((*line).to_string()));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_texts_have_no_changes() {
        let diff = diff_lines("a\nb\n", "a\nb\n");

        assert!(diff.iter().all(|line| matches!(line, DiffLine::Context(_))));
        assert_eq!(diff.len(), 2);
    }

    #[test]
    fn test_a_removed_line_is_marked() {
        let diff = diff_lines("a\nb\nc\n", "a\nc\n");

        assert!(diff.contains(&DiffLine::Removed("b".to_string())));
        assert!(diff.contains(&DiffLine::Context("a".to_string())));
        assert!(diff.contains(&DiffLine::Context("c".to_string())));
    }

    #[test]
    fn test_an_added_line_is_marked() {
        let diff = diff_lines("a\n", "a\nb\n");

        assert!(diff.contains(&DiffLine::Added("b".to_string())));
    }

    #[test]
    fn test_the_markers_are_distinct() {
        assert_eq!(DiffLine::Context("x".into()).marker(), " ");
        assert_eq!(DiffLine::Removed("x".into()).marker(), "-");
        assert_eq!(DiffLine::Added("x".into()).marker(), "+");
    }
}
