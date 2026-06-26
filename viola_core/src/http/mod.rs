pub mod get;
pub mod post;
pub mod request;

use crate::context::Context;
use get::HttpGetBuilder;
use post::HttpPostBuilder;
use request::HttpRequestBuilder;

pub struct Http<'a> {
    pub ctx: &'a Context,
}

impl<'a> Http<'a> {
    pub fn get(self, url: impl Into<String>) -> HttpGetBuilder<'a> {
        HttpGetBuilder::new(self.ctx, url.into())
    }

    pub fn post(self, url: impl Into<String>) -> HttpPostBuilder<'a> {
        HttpPostBuilder::new(self.ctx, url.into())
    }

    pub fn raw(self, method: impl Into<String>, url: impl Into<String>) -> HttpRequestBuilder<'a> {
        HttpRequestBuilder::new(self.ctx, method.into(), url.into())
    }
}
