use criterion::{criterion_group, criterion_main, Criterion};
use std::{fs, path::PathBuf};
use wdl::{
    model::DocumentSource,
    parsers::{PestParser, TreeSitterParser, WdlParser},
};

pub fn benchmark_pest(c: &mut Criterion) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let comprehensive_path = root
        .join("resources")
        .join("test")
        .join("comprehensive.wdl");
    let comprehensive_text = fs::read_to_string(comprehensive_path.clone()).unwrap();
    let mut parser = PestParser::new();
    let mut group = c.benchmark_group("pest");
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
}

pub fn benchmark_tree_sitter(c: &mut Criterion) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let comprehensive_path = root
        .join("resources")
        .join("test")
        .join("comprehensive.wdl");
    let comprehensive_text = fs::read_to_string(comprehensive_path.clone()).unwrap();
    let mut parser = TreeSitterParser::new().unwrap();
    let mut group = c.benchmark_group("tree-sitter");
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
}

criterion_group!(benches, benchmark_pest, benchmark_tree_sitter);
criterion_main!(benches);
