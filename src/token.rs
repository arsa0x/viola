use std::borrow::Cow;

/// Represents a lexical token produced by the lexer.
///
/// A `Token` is the smallest meaningful unit of the source code.
/// Tokens are produced by the lexer and later consumed by the parser.
///
/// The lifetime `'a` is used for token data that can borrow directly
/// from the original source string, avoiding unnecessary allocations.
/// String-like tokens use [`Cow`] so they can either borrow from the
/// source or own an allocated [`String`] when transformation is required.
///
/// # Examples
///
/// ```
/// use std::borrow::Cow;
/// use viola_script::token::Token;
///
/// let identifier = Token::Ident("hello");
/// let string = Token::Str(Cow::Borrowed("world"));
///
/// assert_eq!(identifier, Token::Ident("hello"));
/// assert_eq!(string, Token::Str(Cow::Borrowed("world")));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    /// An identifier.
    ///
    /// Identifiers are names used for variables, functions, types,
    /// or other user-defined entities.
    ///
    /// The string slice borrows directly from the source.
    Ident(&'a str),

    /// A string literal.
    ///
    /// Uses [`Cow`] because the string can either be borrowed from the
    /// source or owned when the lexer needs to process escape sequences
    /// or otherwise transform the string.
    Str(Cow<'a, str>),

    /// The beginning of a string template.
    ///
    /// Contains the literal string content appearing before the first
    /// template expression.
    StrTemplateStart(Cow<'a, str>),

    /// The middle portion of a string template.
    ///
    /// Represents literal string content between template expressions.
    StrTemplateMiddle(Cow<'a, str>),

    /// The ending portion of a string template.
    ///
    /// Represents the final literal string content after the last
    /// template expression.
    StrTemplateEnd(Cow<'a, str>),

    /// An integer literal.
    ///
    /// Stores the parsed integer value as a signed 64-bit integer.
    Int(i64),

    /// A floating-point literal.
    ///
    /// Stores the parsed value as a 64-bit IEEE-754 floating-point number.
    Float(f64),

    /// The boolean literal `true`.
    True,

    /// The boolean literal `false`.
    False,

    /// Equality operator `==`.
    Eq,

    /// Inequality operator `!=`.
    NotEq,

    /// Less-than operator `<`.
    Lt,

    /// Less-than-or-equal operator `<=`.
    Le,

    /// Greater-than operator `>`.
    Gt,

    /// Greater-than-or-equal operator `>=`.
    Ge,

    /// Addition operator `+`.
    Plus,

    /// Subtraction operator `-`.
    Minus,

    /// Multiplication operator `*`.
    Star,

    /// Division operator `/`.
    Slash,

    /// Logical negation operator `!`.
    Bang,

    /// Assignment operator `=`.
    Assign,

    /// Pipe operator `|`.
    Pipe,

    /// Left curly brace `{`.
    LBrace,

    /// Right curly brace `}`.
    RBrace,

    /// Left square bracket `[`.
    LBracket,

    /// Right square bracket `]`.
    RBracket,

    /// Left parenthesis `(`.
    LParen,

    /// Right parenthesis `)`.
    RParen,

    /// Colon `:`.
    Colon,

    /// Comma `,`.
    Comma,

    /// At sign `@`.
    At,

    /// Dot `.`.
    Dot,

    /// A newline character or sequence of newline characters.
    ///
    /// Depending on the lexer configuration, newlines may be significant
    /// to the parser or may be used only as separators between statements.
    Newline,

    /// Marks the end of the input.
    ///
    /// `EOF` is emitted by the lexer when there are no more tokens
    /// available from the source.
    EOF,
}
