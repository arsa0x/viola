use crate::native::Value;

pub struct SingleSelectSpec<'a> {
    pub sections: &'a [Value],
    pub title: Option<&'a str>,
    pub text_body: Option<&'a str>,
    pub footer: Option<&'a str>,
    pub select_label: Option<&'a str>,
    pub quoted: bool,
}
