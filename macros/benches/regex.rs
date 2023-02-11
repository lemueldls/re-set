use criterion::{criterion_group, criterion_main, Criterion};
use re_set_macros::find;

pub fn benchmark(c: &mut Criterion) {
    let re = regex::Regex::new(r"[a-z]+").unwrap();

    find!(lex | r"[a-z]+");

    let input = "abcdefghijklmnopqrstuvwxyz";

    c.bench_function("regex", |b| b.iter(|| re.find(input)));
    c.bench_function("re-set", |b| b.iter(|| lex(input)));
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
