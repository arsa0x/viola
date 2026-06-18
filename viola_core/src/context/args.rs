use crate::context::Context;

pub struct Args<'a> {
    pub ctx: &'a Context,
}

impl<'a> Args<'a> {
    pub fn arg(&self, index: usize) -> Option<&str> {
        self.ctx.args.get(index).map(|s| s.as_str())
    }

    pub fn rest(&self) -> String {
        self.ctx.args.join(" ")
    }

    pub fn rest_from(&self, index: usize) -> String {
        self.ctx
            .args
            .iter()
            .skip(index)
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}
