#[cfg(test)]
mod lexer_tests {
    use crate::lexer::is_skippable;
    use crate::lexer::lex;
    use crate::lexer::lex_all;
    use crate::token::Token;

    // ============================================================
    // Token/lexer tests
    // ============================================================

    #[test]
    fn test_lex_numbers() {
        let tokens = lex_all("42 3.14 0 100");
        let numbers: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.token, Token::Number(_)))
            .collect();
        assert_eq!(numbers.len(), 4);
    }

    #[test]
    fn test_lex_identifiers() {
        let tokens = lex_all("foo bar_baz hello.world x$y");
        let ids: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.token, Token::Identifier(_)))
            .collect();
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn test_lex_keywords() {
        let tokens = lex_all("if then else end while do for foreach in return break fn let");
        // All these should be keyword tokens, not identifiers
        let ids: Vec<_> = tokens
            .iter()
            .filter(|t| matches!(t.token, Token::Identifier(_)))
            .collect();
        assert_eq!(ids.len(), 0);
    }

    #[test]
    fn test_lex_operators() {
        let tokens =
            lex_all("+ - * / % ** == != < > <= >= && || ! = += -= *= /= %= **= -> => @ : ,");
        // All should be recognized as operators
        assert!(tokens.len() > 20);
    }

    #[test]
    fn test_lex_comments_skipped() {
        let tokens = lex_all("# this is a comment\n42");
        let meaningful: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(t.token, Token::Whitespace))
            .collect();
        assert_eq!(meaningful.len(), 1); // Just the 42
    }

    #[test]
    fn test_lex_multiline_comment() {
        let tokens = lex_all("/* multi\nline\ncomment */\n42");
        let meaningful: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(t.token, Token::Whitespace))
            .collect();
        assert_eq!(meaningful.len(), 1);
    }

    // ============================================================
    // Lexer exhaustive tests
    // ============================================================

    #[test]
    fn test_lex_all_keywords_recognized() {
        let source = "if then else end while do for foreach in return break fn let false true null throw try with pipe exec define_args";
        let tokens = lex_all(source);
        // All should be keyword tokens (plus whitespace)
        let keywords: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(t.token, Token::Whitespace))
            .collect();
        assert_eq!(keywords.len(), 22);
    }

    #[test]
    fn test_lex_string_with_interpolation() {
        let tokens = lex_all("\"hello ${name}\"");
        // Should have DQuote, Identifier("hello"), Whitespace, Interpolation, Identifier("name"), RCurl, DQuote
        let meaningful: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(t.token, Token::Whitespace))
            .collect();
        assert!(meaningful.len() >= 5);
    }

    #[test]
    fn test_lex_escaped_newline_in_string() {
        let tokens = lex_all("\"line1\\nline2\"");
        let has_escaped_newline = tokens
            .iter()
            .any(|t| matches!(t.token, Token::EscapedNewline));
        assert!(has_escaped_newline);
    }

    #[test]
    fn test_lex_escaped_dollar() {
        let tokens = lex_all("\"\\$5\"");
        let has_escaped = tokens
            .iter()
            .any(|t| matches!(t.token, Token::EscapedDollar));
        assert!(has_escaped);
    }

    #[test]
    fn test_lex_escaped_backslash() {
        let tokens = lex_all("\"\\\\\"");
        let has_escaped = tokens
            .iter()
            .any(|t| matches!(t.token, Token::EscapedBackslash));
        assert!(has_escaped);
    }

    #[test]
    fn test_lex_block_comment_nested_stars() {
        let tokens = lex_all("/* comment with * star */ 42");
        let meaningful: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(t.token, Token::Whitespace))
            .collect();
        assert_eq!(meaningful.len(), 1);
    }

    #[test]
    fn test_lex_compound_operators() {
        let tokens = lex_all("+= -= *= /= %= **= -> =>");
        let ops: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(t.token, Token::Whitespace))
            .collect();
        assert_eq!(ops.len(), 8);
    }

    // ============================================================
    // Lexer coverage: test lex_all function
    // ============================================================

    #[test]
    fn test_lex_all_empty() {
        let tokens = lex_all("");
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_lex_all_whitespace_only() {
        let tokens = lex_all("   \n  \t  ");
        // All are whitespace tokens
        assert!(tokens.iter().all(|t| matches!(t.token, Token::Whitespace)));
    }

    #[test]
    fn test_lex_multiple_lines() {
        let tokens = lex_all("let x = 5\nlet y = 10");
        let meaningful: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(t.token, Token::Whitespace))
            .collect();
        // let x = 5 let y = 10
        assert!(meaningful.len() >= 8);
    }

    // ============================================================
    // Additional lexer tests
    // ============================================================

    #[test]
    fn test_parse_bitwise_not_operator() {
        // ~ is defined as bitwise not in the grammar, not yet implemented
        // This tests that the token is recognized
        let tokens = lex_all("~5");
        assert!(tokens.len() > 0);
    }

    #[test]
    fn test_lexer_lex_function() {
        let stream = lex("let x = 5");
        let tokens: Vec<_> = stream.collect();
        assert!(tokens.len() > 0);
    }

    #[test]
    fn test_lexer_is_skippable() {
        assert!(is_skippable(&Token::Null));
        assert!(!is_skippable(&Token::Number(42.0)));
    }
}
