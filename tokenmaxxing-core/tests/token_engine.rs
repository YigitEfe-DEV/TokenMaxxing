//! Regression tests for the token engine.
//!
//! The important invariant is that streaming a text produces exactly the same
//! count as counting the whole text at once, no matter how the bytes are
//! split up by the reader.

use std::io::{self, Read};
use tokenmaxxing_core::engine::{CountKind, EstimationMode, TokenEngine};

/// Texts that exercise the tricky parts of the tokenizer emulation.
const CORPUS: &[&str] = &[
    "",
    " ",
    "   ",
    "a",
    "Hello world! This is a test.",
    "  leading and trailing whitespace   ",
    "multi\nline\ntext\nwith\nnewlines\n",
    "punctuation!?.,;:",
    "hyphenated-words and snake_case_words",
    "tabs\tand\tmixed   spaces",
    "Merhaba dünya! Bu bir testtir.",
    "日本語のテキストです。",
    "emoji 🚀🚀 and math ∑∫ symbols",
    "trailing punctuation...",
    "...leading punctuation",
    "supercalifragilisticexpialidocious",
    "CamelCaseIdentifier snake_case_ident SCREAMING_CASE",
    "https://example.com/path?query=value&other=1",
    "{\"json\":true,\"array\":[1,2,3]}",
    "one two three four five six seven eight nine ten",
];

/// A reader that hands out at most `chunk` bytes per read.
struct ChunkedReader<'a> {
    data: &'a [u8],
    chunk: usize,
    position: usize,
}

impl<'a> ChunkedReader<'a> {
    fn new(data: &'a [u8], chunk: usize) -> Self {
        Self {
            data,
            chunk: chunk.max(1),
            position: 0,
        }
    }
}

impl Read for ChunkedReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let remaining = &self.data[self.position..];
        let take = remaining.len().min(self.chunk).min(buffer.len());
        buffer[..take].copy_from_slice(&remaining[..take]);
        self.position += take;
        Ok(take)
    }
}

fn modes() -> [EstimationMode; 3] {
    [
        EstimationMode::Fast,
        EstimationMode::Accurate,
        EstimationMode::Hybrid,
    ]
}

#[test]
fn streaming_does_not_depend_on_the_chunk_size() {
    for text in CORPUS {
        for mode in modes() {
            let engine = TokenEngine::new(mode);
            let expected = engine.count_text(text);

            for chunk in [1usize, 2, 3, 5, 7, 13, 64, 4096] {
                let reader = ChunkedReader::new(text.as_bytes(), chunk);
                let streamed = engine
                    .count_stream(reader)
                    .unwrap_or_else(|error| panic!("mode {mode:?}, chunk {chunk}: {error}"));

                assert_eq!(
                    streamed, expected,
                    "mode {mode:?}, chunk size {chunk}, text {text:?}"
                );
            }
        }
    }
}

#[test]
fn streaming_keeps_multibyte_characters_intact() {
    // Every character here needs more than one byte, so the reader splits
    // several of them across reads.
    let text = "日本語のテキスト、 Türkçe karakterler, and emoji 🚀🚀🚀.";

    for mode in modes() {
        let engine = TokenEngine::new(mode);
        let expected = engine.count_text(text);

        for chunk in [1usize, 2, 3, 5, 11] {
            let reader = ChunkedReader::new(text.as_bytes(), chunk);
            let streamed = engine.count_stream(reader).unwrap();

            assert_eq!(streamed, expected, "mode {mode:?}, chunk size {chunk}");
            assert_eq!(streamed.chars, text.chars().count());
            assert_eq!(streamed.bytes, text.len());
        }
    }
}

#[test]
fn streaming_rejects_invalid_utf8_instead_of_replacing_it() {
    let engine = TokenEngine::new(EstimationMode::Accurate);
    let broken = [b'o', b'k', 0xff, b'!'];

    let error = engine
        .count_stream(ChunkedReader::new(&broken, 1))
        .expect_err("invalid UTF-8 has to be rejected");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn streaming_rejects_a_truncated_utf8_sequence() {
    let engine = TokenEngine::new(EstimationMode::Accurate);
    // The first two bytes of a three byte character.
    let truncated = [0xe6, 0x97];

    let error = engine
        .count_stream(ChunkedReader::new(&truncated, 1))
        .expect_err("a truncated UTF-8 sequence has to be rejected");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn streaming_a_large_text_matches_counting_it_at_once() {
    let text = "The quick brown fox jumps over the lazy dog. 日本語のテキスト。\n".repeat(2000);
    assert!(
        text.len() > 100_000,
        "the text has to exceed the hybrid limit"
    );

    for mode in modes() {
        let engine = TokenEngine::new(mode);
        let expected = engine.count_text(&text);
        let streamed = engine.count_stream(text.as_bytes()).unwrap();

        assert_eq!(streamed, expected, "mode {mode:?}");
    }
}

#[test]
fn the_built_in_modes_never_claim_to_be_exact() {
    for mode in modes() {
        let engine = TokenEngine::new(mode);
        let count = engine.count_text("Hello world!");

        assert!(!count.kind.is_exact(), "mode {mode:?} claimed to be exact");
    }
}

#[test]
fn the_count_method_is_reported_for_every_mode() {
    let fast = TokenEngine::new(EstimationMode::Fast)
        .count_text("Hello world!")
        .kind;
    let accurate = TokenEngine::new(EstimationMode::Accurate)
        .count_text("Hello world!")
        .kind;

    assert_eq!(fast, CountKind::Estimated);
    assert_eq!(accurate, CountKind::Approximate);
}

#[test]
fn characters_are_counted_as_characters_and_not_as_bytes() {
    let engine = TokenEngine::new(EstimationMode::Fast);
    let count = engine.count_text("日本語");

    assert_eq!(count.chars, 3);
    assert_eq!(count.bytes, 9);
}

#[test]
fn the_estimate_is_stable_for_the_same_input() {
    let engine = TokenEngine::new(EstimationMode::Accurate);
    let text = "TokenMaxxing counts the tokens of a repository.";

    let first = engine.count_text(text);
    let second = engine.count_text(text);

    assert_eq!(first, second);
    assert!(first.tokens > 0);
}
