use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use viola_script::lexer::Lexer;

fn lexer_small(c: &mut Criterion) {
    c.bench_function("lexer_small", |b| {
        b.iter(|| {
            black_box(
                Lexer::new(black_box(include_str!("fixtures/small.vi")))
                    .tokenize()
                    .unwrap(),
            );
        });
    });
}

fn lexer_medium(c: &mut Criterion) {
    c.bench_function("lexer_medium", |b| {
        b.iter(|| {
            black_box(
                Lexer::new(black_box(include_str!("fixtures/medium.vi")))
                    .tokenize()
                    .unwrap(),
            );
        });
    });
}

fn lexer_large(c: &mut Criterion) {
    c.bench_function("lexer_large", |b| {
        b.iter(|| {
            black_box(
                Lexer::new(black_box(include_str!("fixtures/large.vi")))
                    .tokenize()
                    .unwrap(),
            );
        });
    });
}

criterion_group!(benches, lexer_small, lexer_medium, lexer_large);
criterion_main!(benches);
