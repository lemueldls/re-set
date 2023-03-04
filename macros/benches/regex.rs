use criterion::{criterion_group, criterion_main, Criterion};

pub fn benchmark(c: &mut Criterion) {
    let re = regex::Regex::new(r"~?[-+]?[[:digit:]]").unwrap();
    re_set_macros::find!(fn rs r"~?[-+]?[[:digit:]]");
    proc_macro_regex::regex!(pmr r"~?[-+]?[[:digit:]]");

    let input = "~-3";

    c.bench_function("regex", |b| b.iter(|| re.find(input)));
    c.bench_function("re-set", |b| b.iter(|| rs(input)));
    c.bench_function("proc-macro-regex", |b| b.iter(|| pmr(input)));
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
