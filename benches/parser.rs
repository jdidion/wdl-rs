use criterion::{criterion_group, criterion_main, Criterion};
use std::{fs, path::PathBuf};
use wdl::{
    model::DocumentSource,
    parsers::{PestParser, TreeSitterParser, WdlParser},
};

pub fn benchmark_pest(c: &mut Criterion) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_dir = root.join("resources").join("test");
    let mut parser = PestParser::new();
    let mut group = c.benchmark_group("pest");

    let comprehensive_path = test_dir.join("comprehensive.wdl");
    let comprehensive_text = fs::read_to_string(comprehensive_path.clone()).unwrap();
    group.bench_function("comprehensive", |b| {
        b.iter(|| {
            parser
                .parse_text(
                    &comprehensive_text,
                    DocumentSource::File(comprehensive_path.clone()),
                )
                .unwrap()
        })
    });

    let expressions_path = test_dir.join("expressions.wdl");
    let expressions_text = fs::read_to_string(expressions_path.clone()).unwrap();
    group.bench_function("expressions", |b| {
        b.iter(|| {
            parser
                .parse_text(
                    &expressions_text,
                    DocumentSource::File(expressions_path.clone()),
                )
                .unwrap()
        })
    });
}

pub fn benchmark_tree_sitter(c: &mut Criterion) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_dir = root.join("resources").join("test");
    let mut parser = TreeSitterParser::new().unwrap();
    let mut group = c.benchmark_group("tree-sitter");

    let comprehensive_path = test_dir.join("comprehensive.wdl");
    let comprehensive_text = fs::read_to_string(comprehensive_path.clone()).unwrap();
    group.bench_function("comprehensive", |b| {
        b.iter(|| {
            parser
                .parse_text(
                    &comprehensive_text,
                    DocumentSource::File(comprehensive_path.clone()),
                )
                .unwrap();
        })
    });

    let expressions_path = test_dir.join("expressions.wdl");
    let expressions_text = fs::read_to_string(expressions_path.clone()).unwrap();
    group.bench_function("expressions", |b| {
        b.iter(|| {
            parser
                .parse_text(
                    &expressions_text,
                    DocumentSource::File(expressions_path.clone()),
                )
                .unwrap()
        })
    });
}

criterion_group!(benches, benchmark_pest, benchmark_tree_sitter);
criterion_main!(benches);
