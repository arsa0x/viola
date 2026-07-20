/*
 * # this is comment
 *
 * greeting = "Hi there"
 *
 * :send .text $greeting .quoted
 *
 * headers = {
 *  "User-Agent": "viola-script"
 * }
 *
 * response = :http .get "http://127.0.0.1:3000/user" .header $headers
 */

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Ident(&'a str),
    Keyword(&'a str),

    Str(&'a str),
    Int(i32),
    Uint(u32),
    Float(f32),

    At,
    Meta(Meta<'a>),

    Colon,
    SemiColon,
    Comma,

    OpenBrace,
    CloseBrace,

    OpenParenthesis,
    CloseParenthesis,

    Assignment,

    EOF,
}

#[derive(Debug, PartialEq)]
pub struct Meta<'a> {
    pub name: &'a str,
    pub triggers: Vec<&'a str>,
    pub owner_only: bool,
    pub group_only: bool,
    pub description: Option<&'a str>,
    pub help: Option<&'a str>,
}
