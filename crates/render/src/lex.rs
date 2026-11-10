//! WGSL as tokens: the one lexer of the stain's assembler ([`crate::assemble`]) and the Print graph's scanners
//! ([`crate::print`]), so a comment is skipped as WGSL skips it, nested block comments included, wherever they look
//! past white space. Render's build script compiles it too, for the Print graph's registry check.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tok {
    Ident,
    Number,
    Comment,
    Space,
    Punct,
}

/// The length in bytes of the block comment `b` opens with, through the `*/` that closes it: WGSL's block comments
/// nest, so each `/*` inside it needs its own `*/` first. `None` where `b` does not open with `/*`, or the comment is
/// never closed. It compares bytes, so it is safe in UTF-8: a `/` or `*` byte is never part of a multi-byte character.
fn block_comment_len(b: &[u8]) -> Option<usize> {
    if !b.starts_with(b"/*") {
        return None;
    }
    let mut depth = 0usize;
    let mut rest = b;
    while !rest.is_empty() {
        if rest.starts_with(b"/*") {
            depth += 1;
            rest = &rest[2..];
        } else if rest.starts_with(b"*/") {
            depth -= 1;
            rest = &rest[2..];
            if depth == 0 {
                return Some(b.len() - rest.len());
            }
        } else {
            rest = &rest[1..];
        }
    }
    None
}

/// Whether `c` is WGSL's white space, Unicode's Pattern_White_Space (WGSL, "Blankspace and line breaks"): the
/// space, the tab, the line feed, vertical tab, form feed and carriage return, U+0085 (next line), U+200E and U+200F
/// (the left-to-right and right-to-left marks), U+2028 (line separator) and U+2029 (paragraph separator), the set
/// naga 30.0.1 lexes as blankspace. Unicode's White_Space differs ([`char::is_whitespace`]): it holds the no-break
/// space U+00A0 and others WGSL refuses, and not the marks U+200E and U+200F.
pub(crate) fn is_space(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'..='\r' | '\u{85}' | '\u{200E}' | '\u{200F}' | '\u{2028}' | '\u{2029}'
    )
}

/// `s` as tokens, each its kind and byte range: comments (`//` to the line's end, `/* */` nested), identifiers,
/// numbers (a digit and the letters and digits after it), white space ([`is_space`]) and single punctuation
/// characters; and whether every block comment is closed: one never closed is a comment to the end.
pub(crate) fn tokens(s: &str) -> (Vec<(Tok, usize, usize)>, bool) {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut closed = true;
    let mut i = 0;
    while i < b.len() {
        let start = i;
        let c = b[i];
        // The comment branches compare bytes, never slice `s`: inside a comment `i` steps a byte at a time, through
        // the middle of a multi-byte character, and a `/`, `*` or newline byte is never part of one (UTF-8).
        let tok = if b[i..].starts_with(b"//") {
            i = b[i..]
                .iter()
                .position(|&x| x == b'\n')
                .map_or(b.len(), |k| i + k);
            Tok::Comment
        } else if b[i..].starts_with(b"/*") {
            match block_comment_len(&b[i..]) {
                Some(len) => i += len,
                None => {
                    closed = false;
                    i = b.len();
                }
            }
            Tok::Comment
        } else if s[i..].starts_with(is_space) {
            i += s[i..].find(|c: char| !is_space(c)).unwrap_or(b.len() - i);
            Tok::Space
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            Tok::Ident
        } else if c.is_ascii_digit() {
            // A number's letters — its suffix, hex digits and exponent letter — are its own, never a name; a `.`, an
            // exponent's sign and the digits after them lex as punctuation and numbers, which nothing renames.
            while i < b.len() && b[i].is_ascii_alphanumeric() {
                i += 1;
            }
            Tok::Number
        } else {
            i += s[i..].chars().next().map_or(1, char::len_utf8);
            Tok::Punct
        };
        out.push((tok, start, i));
    }
    (out, closed)
}

/// One token of WGSL: its kind, its text, and whether it is trivia, a comment or white space.
pub(crate) struct Token<'a> {
    pub(crate) kind: Tok,
    pub(crate) text: &'a str,
    pub(crate) trivia: bool,
}

/// `wgsl`'s tokens ([`tokens`]), each trivia or not (a comment or white space, [`is_space`]: a Print graph pass's
/// scanners read it so), and whether every block comment is closed: one never closed is a comment to the end.
pub(crate) fn pass_tokens(wgsl: &str) -> (Vec<Token<'_>>, bool) {
    let (toks, closed) = tokens(wgsl);
    let toks = toks
        .into_iter()
        .map(|(kind, a, b)| {
            let text = &wgsl[a..b];
            let trivia = matches!(kind, Tok::Comment | Tok::Space);
            Token { kind, text, trivia }
        })
        .collect();
    (toks, closed)
}
