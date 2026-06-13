use crate::token::{SpannedToken, Token, TokenStream};

/// Lex the given source code and return a peekable token stream.
pub fn lex(source: &str) -> TokenStream<'_> {
    TokenStream::new(source)
}

/// Debug helper: lex and collect all tokens, printing errors.
pub fn lex_all(source: &str) -> Vec<SpannedToken> {
    let stream = TokenStream::new(source);
    let tokens: Vec<SpannedToken> = stream.collect();
    tokens
}

/// Check if a token should be ignored during certain parsing contexts
/// (used to filter out newlines, etc. if needed)
pub fn is_skippable(token: &Token) -> bool {
    matches!(token, Token::Null)
}
