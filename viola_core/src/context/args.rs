use super::Context;

impl Context {
    pub fn arg(&self, index: usize) -> Option<&str> {
        self.args.get(index).map(|s| s.as_str())
    }

    pub fn rest(&self) -> String {
        self.args.join(" ")
    }

    pub fn rest_from(&self, index: usize) -> String {
        self.args
            .iter()
            .skip(index)
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}
