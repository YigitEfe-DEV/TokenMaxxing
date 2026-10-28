//! Token counting engine.
//!
//! The engine keeps three very different things apart, because conflating them
//! is the most common source of wrong token numbers:
//!
//! * an **estimate** produced by a character ratio heuristic,
//! * an **approximation** produced by a structural emulation of byte-pair
//!   encoding, and
//! * an **exact** count produced by a real tokenizer vocabulary or by a
//!   provider API.
//!
//! Every [`TokenCount`] carries the [`CountKind`] that describes how it was
//! produced, so a caller can never present an estimate as a fact.

use crate::tokenizer::LocalTokenizer;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::io::{self, Read};
use std::iter::Peekable;
use std::path::Path;
use std::str::Chars;
use std::sync::Arc;

/// Average number of characters per token used by the ratio estimator.
const CHARS_PER_TOKEN: f64 = 3.5;

/// Number of characters that fit into a single sub-word piece.
const CHARS_PER_SUBWORD: usize = 4;

/// Input size (in bytes) above which [`EstimationMode::Hybrid`] switches from
/// the structural estimator to the ratio estimator.
const HYBRID_ACCURATE_LIMIT: usize = 100_000;

/// Size of the buffer used while reading from a stream.
const STREAM_BUFFER_BYTES: usize = 8192;

/// Describes where a token count came from and therefore how far it can be
/// trusted.
///
/// The variants are ordered from the weakest to the strongest guarantee. When
/// counts are combined the weakest guarantee wins, so a sum is never presented
/// as more trustworthy than its parts.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum CountKind {
    /// Produced by a character ratio heuristic. No tokenizer was involved.
    #[default]
    Estimated,
    /// Produced by a structural emulation of byte-pair encoding that has no
    /// access to a real vocabulary.
    Approximate,
    /// Produced by a local tokenizer that ships a real vocabulary.
    ExactLocal,
    /// Reported by a provider API that owns the authoritative tokenizer.
    ProviderApi,
}

impl CountKind {
    /// Human readable label used by the CLI and by the report formats.
    pub fn label(self) -> &'static str {
        match self {
            CountKind::Estimated => "estimated",
            CountKind::Approximate => "approximate",
            CountKind::ExactLocal => "exact (local tokenizer)",
            CountKind::ProviderApi => "exact (provider API)",
        }
    }

    /// Returns `true` when the count comes from a real tokenizer or from a
    /// provider API.
    pub fn is_exact(self) -> bool {
        matches!(self, CountKind::ExactLocal | CountKind::ProviderApi)
    }
}

/// The result of counting tokens in a piece of text.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TokenCount {
    /// Number of tokens.
    pub tokens: usize,
    /// Number of Unicode scalar values in the counted text.
    pub chars: usize,
    /// Number of UTF-8 bytes in the counted text.
    pub bytes: usize,
    /// How the token count was produced.
    pub kind: CountKind,
}

impl TokenCount {
    /// Builds a count from raw numbers and an explicit provenance.
    pub fn new(tokens: usize, chars: usize, bytes: usize, kind: CountKind) -> Self {
        Self {
            tokens,
            chars,
            bytes,
            kind,
        }
    }

    /// Returns `true` when nothing was counted.
    pub fn is_empty(&self) -> bool {
        self.tokens == 0 && self.chars == 0 && self.bytes == 0
    }
}

impl std::ops::Add for TokenCount {
    type Output = Self;

    /// Adds two counts.
    ///
    /// The resulting provenance is the weaker of both, except that an empty
    /// count carries no information and therefore never weakens the other
    /// side.
    fn add(self, other: Self) -> Self {
        let kind = if self.is_empty() {
            other.kind
        } else if other.is_empty() {
            self.kind
        } else {
            self.kind.min(other.kind)
        };

        Self {
            tokens: self.tokens + other.tokens,
            chars: self.chars + other.chars,
            bytes: self.bytes + other.bytes,
            kind,
        }
    }
}

/// Selects the estimator used by a [`TokenEngine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimationMode {
    /// Character ratio estimate of roughly 3.5 characters per token.
    ///
    /// The fastest mode. The result is always an estimate.
    Fast,
    /// Structural approximation that emulates byte-pair encoding.
    ///
    /// Closer to a real tokenizer than [`EstimationMode::Fast`] but still an
    /// approximation, because the tokenizer vocabulary is not available.
    Accurate,
    /// [`EstimationMode::Accurate`] for inputs up to 100 kB and
    /// [`EstimationMode::Fast`] above that.
    Hybrid,
}

/// Counts tokens in text, files and repositories.
pub struct TokenEngine {
    mode: EstimationMode,
    tokenizer: Option<Arc<dyn LocalTokenizer>>,
}

impl TokenEngine {
    /// Builds an engine that estimates token counts.
    pub fn new(mode: EstimationMode) -> Self {
        Self {
            mode,
            tokenizer: None,
        }
    }

    /// Builds an engine that counts with a real tokenizer vocabulary.
    ///
    /// Counts produced by such an engine are reported as
    /// [`CountKind::ExactLocal`]; the [`EstimationMode`] is irrelevant because
    /// the tokenizer decides the count.
    pub fn with_tokenizer(tokenizer: Arc<dyn LocalTokenizer>) -> Self {
        Self {
            mode: EstimationMode::Accurate,
            tokenizer: Some(tokenizer),
        }
    }

    /// Counts the tokens of a complete piece of text.
    pub fn count_text(&self, text: &str) -> TokenCount {
        let chars = text.chars().count();
        let bytes = text.len();

        if let Some(tokenizer) = &self.tokenizer {
            return TokenCount::new(
                tokenizer.count_tokens(text),
                chars,
                bytes,
                CountKind::ExactLocal,
            );
        }

        match self.mode {
            EstimationMode::Fast => {
                TokenCount::new(ratio_tokens(chars), chars, bytes, CountKind::Estimated)
            }
            EstimationMode::Accurate => {
                TokenCount::new(count_pretokens(text), chars, bytes, CountKind::Approximate)
            }
            EstimationMode::Hybrid => {
                if bytes > HYBRID_ACCURATE_LIMIT {
                    TokenCount::new(ratio_tokens(chars), chars, bytes, CountKind::Estimated)
                } else {
                    TokenCount::new(count_pretokens(text), chars, bytes, CountKind::Approximate)
                }
            }
        }
    }

    /// Counts the tokens of a stream.
    ///
    /// The reader is decoded as UTF-8 and multi-byte sequences that straddle a
    /// read boundary are carried over to the next read, so the result is the
    /// same as counting the whole text at once. Tokens are never split across
    /// a chunk boundary either.
    ///
    /// When the engine was built with [`TokenEngine::with_tokenizer`] the whole
    /// text has to be buffered, because an exact count requires the complete
    /// input.
    pub fn count_stream<R: Read>(&self, mut reader: R) -> io::Result<TokenCount> {
        let mut buffer = [0u8; STREAM_BUFFER_BYTES];
        let mut decoder = Utf8Decoder::new();
        let mut chars = 0usize;
        let mut bytes = 0usize;
        let mut pending = String::new();
        let mut pretokens = 0usize;
        let mut exact_text = String::new();
        let mut exceeds_hybrid_limit = false;

        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            bytes += read;

            let text = decoder.decode(&buffer[..read])?;
            if text.is_empty() {
                continue;
            }
            chars += text.chars().count();

            if self.tokenizer.is_some() {
                exact_text.push_str(&text);
                continue;
            }

            if !exceeds_hybrid_limit
                && bytes > HYBRID_ACCURATE_LIMIT
                && self.mode == EstimationMode::Hybrid
            {
                exceeds_hybrid_limit = true;
            }

            // The ratio estimator only needs the character total, every other
            // mode has to keep the text pieces aligned.
            if self.mode != EstimationMode::Fast {
                pending.push_str(&text);
                if let Some(cut) = safe_split_index(&pending) {
                    if cut > 0 {
                        pretokens += count_pretokens(&pending[..cut]);
                        pending.drain(..cut);
                    }
                }
            }
        }

        decoder.finish()?;

        if let Some(tokenizer) = &self.tokenizer {
            return Ok(TokenCount::new(
                tokenizer.count_tokens(&exact_text),
                chars,
                bytes,
                CountKind::ExactLocal,
            ));
        }

        let (tokens, kind) = match self.mode {
            EstimationMode::Fast => (ratio_tokens(chars), CountKind::Estimated),
            EstimationMode::Accurate => (
                pretokens + count_pretokens(&pending),
                CountKind::Approximate,
            ),
            EstimationMode::Hybrid if exceeds_hybrid_limit => {
                (ratio_tokens(chars), CountKind::Estimated)
            }
            EstimationMode::Hybrid => (
                pretokens + count_pretokens(&pending),
                CountKind::Approximate,
            ),
        };

        Ok(TokenCount::new(tokens, chars, bytes, kind))
    }

    /// Counts the tokens of multiple pieces of text.
    pub fn count_batch(&self, texts: &[String]) -> TokenCount {
        texts
            .iter()
            .map(|text| self.count_text(text))
            .fold(TokenCount::default(), |a, b| a + b)
    }

    /// Counts the tokens of multiple pieces of text in parallel.
    pub fn count_batch_parallel(&self, texts: &[String]) -> TokenCount {
        texts
            .par_iter()
            .map(|text| self.count_text(text))
            .reduce(TokenCount::default, |a, b| a + b)
    }

    /// Counts the tokens of every text file in a directory tree.
    ///
    /// Files that cannot be read as UTF-8 are skipped.
    pub fn count_repository<P: AsRef<Path>>(&self, path: P) -> TokenCount {
        text_files(path)
            .par_iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .map(|content| self.count_text(&content))
            .reduce(TokenCount::default, |a, b| a + b)
    }
}

/// Lists the files of a directory tree.
///
/// The list is used for repository wide counting and for the provider
/// adapters, which have to send one message per file.
pub fn text_files<P: AsRef<Path>>(path: P) -> Vec<std::path::PathBuf> {
    let walker = WalkBuilder::new(path).build();

    walker
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Converts a character count into a token count with the ratio estimator.
fn ratio_tokens(chars: usize) -> usize {
    if chars == 0 {
        return 0;
    }
    (chars as f64 / CHARS_PER_TOKEN).ceil() as usize
}

/// Counts tokens the way a byte-pair-encoding tokenizer approximately would.
///
/// * whitespace is attached to the word that follows it, like the `" word"`
///   pieces most BPE vocabularies contain,
/// * punctuation is split into single character pieces, and
/// * words longer than [`CHARS_PER_SUBWORD`] characters are split into
///   sub-word pieces.
///
/// The result stays an approximation: without the real vocabulary there is no
/// way to know which merges a tokenizer would actually perform.
fn count_pretokens(text: &str) -> usize {
    let mut tokens = 0usize;
    let mut chars = text.chars().peekable();

    while let Some(current) = chars.next() {
        if current.is_whitespace() {
            let mut run = 1usize;
            while chars.peek().is_some_and(|c| c.is_whitespace()) {
                run += 1;
                chars.next();
            }
            if chars.peek().is_some_and(|c| is_word_char(*c)) {
                // The whitespace becomes part of the word that follows it.
                tokens += piece_tokens(take_word(&mut chars));
            } else {
                tokens += piece_tokens(run);
            }
        } else if is_word_char(current) {
            tokens += piece_tokens(1 + take_word(&mut chars));
        } else {
            // Punctuation is a piece of its own.
            tokens += 1;
        }
    }

    tokens
}

/// Number of pieces a run of `len` characters is split into.
///
/// The first [`CHARS_PER_SUBWORD`] characters form one piece and every further
/// group of that size adds another one.
fn piece_tokens(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    1 + len.saturating_sub(CHARS_PER_SUBWORD) / CHARS_PER_SUBWORD
}

/// Consumes the word characters at the front of `chars` and returns their
/// count.
fn take_word(chars: &mut Peekable<Chars<'_>>) -> usize {
    let mut len = 0usize;
    while chars.peek().is_some_and(|c| is_word_char(*c)) {
        len += 1;
        chars.next();
    }
    len
}

/// Returns `true` for characters that belong to a word.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Returns the largest index at which `text` may be cut without breaking a
/// token, or `None` when no such index exists.
///
/// Both sides of the cut have to be piece aligned: the prefix must end with a
/// complete piece and the suffix must start with a piece that begins there.
/// Two positions qualify:
///
/// * the first character of a whitespace run, because the whitespace belongs
///   to the piece that follows it, and
/// * the position right behind a punctuation character.
fn safe_split_index(text: &str) -> Option<usize> {
    let mut best: Option<usize> = None;
    let mut previous: Option<char> = None;

    for (index, current) in text.char_indices() {
        if !current.is_whitespace() && !is_word_char(current) {
            best = Some(index + current.len_utf8());
        } else if current.is_whitespace() && previous.is_some_and(|p| !p.is_whitespace()) {
            best = Some(index);
        }
        previous = Some(current);
    }

    best
}

/// Decodes a byte stream into UTF-8 text while carrying incomplete multi-byte
/// sequences over to the next read.
struct Utf8Decoder {
    carry: Vec<u8>,
}

impl Utf8Decoder {
    fn new() -> Self {
        Self { carry: Vec::new() }
    }

    /// Decodes as much of `bytes` as possible and keeps a trailing incomplete
    /// sequence for the next call.
    fn decode(&mut self, bytes: &[u8]) -> io::Result<String> {
        let mut buffer = Vec::with_capacity(self.carry.len() + bytes.len());
        buffer.extend_from_slice(&self.carry);
        buffer.extend_from_slice(bytes);
        self.carry.clear();

        match std::str::from_utf8(&buffer) {
            Ok(text) => Ok(text.to_string()),
            Err(error) => {
                let valid_up_to = error.valid_up_to();
                match error.error_len() {
                    // The buffer ends with an incomplete sequence: keep it for
                    // the next read instead of corrupting the text.
                    None => {
                        self.carry.extend_from_slice(&buffer[valid_up_to..]);
                        Ok(String::from_utf8_lossy(&buffer[..valid_up_to]).into_owned())
                    }
                    Some(_) => Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("input is not valid UTF-8 at byte offset {valid_up_to}"),
                    )),
                }
            }
        }
    }

    /// Fails when the stream ended with an incomplete UTF-8 sequence.
    fn finish(&self) -> io::Result<()> {
        if self.carry.is_empty() {
            return Ok(());
        }
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "input ended with an incomplete UTF-8 sequence",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Hello world! This is a test.";

    #[test]
    fn test_fast_estimation_is_a_ratio_of_characters() {
        let engine = TokenEngine::new(EstimationMode::Fast);
        let count = engine.count_text(SAMPLE);

        assert_eq!(count.tokens, 8);
        assert_eq!(count.chars, 28);
        assert_eq!(count.bytes, 28);
        assert_eq!(count.kind, CountKind::Estimated);
        assert!(!count.kind.is_exact());
    }

    #[test]
    fn test_accurate_estimation_is_still_an_approximation() {
        let engine = TokenEngine::new(EstimationMode::Accurate);
        let count = engine.count_text(SAMPLE);

        // "Hello", " world", "!", " This", " is", " a", " test", "."
        assert_eq!(count.tokens, 8);
        assert_eq!(count.kind, CountKind::Approximate);
        assert!(!count.kind.is_exact());
    }

    #[test]
    fn test_accurate_mode_merges_whitespace_into_the_following_word() {
        let engine = TokenEngine::new(EstimationMode::Accurate);

        // A space costs nothing on its own, it belongs to the next word.
        assert_eq!(engine.count_text("a b").tokens, 2);
        assert_eq!(engine.count_text("ab").tokens, 1);
        assert_eq!(engine.count_text("a  b").tokens, 2);
    }

    #[test]
    fn test_accurate_mode_splits_long_words_into_sub_words() {
        let engine = TokenEngine::new(EstimationMode::Accurate);

        let short = engine.count_text("cat").tokens;
        let long = engine.count_text("internationalization").tokens;

        assert_eq!(short, 1);
        assert!(long > short);
    }

    #[test]
    fn test_empty_text_counts_nothing() {
        let engine = TokenEngine::new(EstimationMode::Accurate);
        let count = engine.count_text("");

        assert_eq!(count.tokens, 0);
        assert_eq!(count.chars, 0);
        assert_eq!(count.bytes, 0);
    }

    #[test]
    fn test_characters_and_bytes_differ_for_multibyte_text() {
        let engine = TokenEngine::new(EstimationMode::Fast);
        let count = engine.count_text("naïve 日本語");

        assert_eq!(count.chars, 9);
        assert_eq!(count.bytes, 16);
    }

    #[test]
    fn test_adding_counts_keeps_the_weaker_provenance() {
        let estimated = TokenCount::new(10, 40, 40, CountKind::Estimated);
        let approximate = TokenCount::new(8, 40, 40, CountKind::Approximate);
        let exact = TokenCount::new(9, 40, 40, CountKind::ExactLocal);

        assert_eq!((estimated + approximate.clone()).kind, CountKind::Estimated);
        assert_eq!((approximate + exact).kind, CountKind::Approximate);
    }

    #[test]
    fn test_adding_an_empty_count_keeps_the_provenance() {
        let approximate = TokenCount::new(8, 40, 40, CountKind::Approximate);
        let sum = approximate + TokenCount::default();

        assert_eq!(sum.kind, CountKind::Approximate);
        assert_eq!(sum.tokens, 8);
    }

    #[test]
    fn test_batch_keeps_the_provenance_of_the_counts() {
        let engine = TokenEngine::new(EstimationMode::Accurate);
        let texts = vec![SAMPLE.to_string(), "second text".to_string()];
        let count = engine.count_batch(&texts);

        assert_eq!(count.kind, CountKind::Approximate);
        assert!(count.tokens > 0);
    }

    #[test]
    fn test_batch_parallel_matches_batch() {
        let engine = TokenEngine::new(EstimationMode::Accurate);
        let texts = vec![SAMPLE.to_string(), "second text".to_string()];

        assert_eq!(
            engine.count_batch(&texts),
            engine.count_batch_parallel(&texts)
        );
    }

    #[test]
    fn test_stream_matches_text_for_every_mode() {
        let text = "Merhaba dünya! Bu bir testtir — 日本語のテキストも入ります。";

        for mode in [
            EstimationMode::Fast,
            EstimationMode::Accurate,
            EstimationMode::Hybrid,
        ] {
            let engine = TokenEngine::new(mode);
            let streamed = engine.count_stream(text.as_bytes()).unwrap();

            assert_eq!(streamed, engine.count_text(text), "mode {mode:?}");
        }
    }

    #[test]
    fn test_stream_rejects_invalid_utf8() {
        let engine = TokenEngine::new(EstimationMode::Fast);
        let error = engine.count_stream([0x66, 0x6f, 0xff, 0x6f].as_slice());

        assert!(error.is_err());
        assert_eq!(error.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_stream_rejects_truncated_utf8() {
        let engine = TokenEngine::new(EstimationMode::Fast);
        // The first two bytes of a three byte character.
        let error = engine.count_stream([0xe6, 0x97].as_slice());

        assert!(error.is_err());
        assert_eq!(error.unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_hybrid_mode_switches_to_the_ratio_estimator_for_large_inputs() {
        let engine = TokenEngine::new(EstimationMode::Hybrid);
        let text = "a".repeat(HYBRID_ACCURATE_LIMIT + 1);
        let count = engine.count_text(&text);

        assert_eq!(count.kind, CountKind::Estimated);
    }
}
