use viola_script::lexer::Lexer;

#[derive(Clone, Copy, Debug)]
pub enum Fixture {
    Small,
    Medium,
    Large,
}

impl Fixture {
    fn source(self) -> &'static str {
        match self {
            Fixture::Small => include_str!("fixtures/small.vi"),
            Fixture::Medium => include_str!("fixtures/medium.vi"),
            Fixture::Large => include_str!("fixtures/large.vi"),
        }
    }
}

#[divan::bench(args = [
    Fixture::Small,
    Fixture::Medium,
    Fixture::Large,
])]
fn lexer(bencher: divan::Bencher, fixture: Fixture) {
    bencher.bench(|| {
        Lexer::new(fixture.source()).tokenize().unwrap();
    });
}

fn main() {
    divan::main();
}
