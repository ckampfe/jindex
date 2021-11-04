use std::io::Read;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use jindex::jindex;
use jindex::path_value_sink::{GronWriter, JSONPointerWriter, JSONPointerWriterOptions};

fn gron_benchmark(c: &mut Criterion) {
    let mut larger_inputs_group = c.benchmark_group("larger inputs");

    larger_inputs_group.measurement_time(std::time::Duration::from_secs(20));

    let mut f = std::fs::File::open("fixtures/big.json").unwrap();
    let mut buf = String::new();
    f.read_to_string(&mut buf).unwrap();
    let json: serde_json::Value = serde_json::from_str(&buf).unwrap();

    larger_inputs_group.bench_function("jindex gron big.json", |b| {
        b.iter(|| {
            let mut writer = vec![];
            let mut sink = GronWriter::new(&mut writer);
            jindex(&mut sink, black_box(&json)).unwrap()
        })
    });

    larger_inputs_group.finish();

    /////////////////////////////////////////////////

    let mut smaller_inputs_group = c.benchmark_group("smaller inputs");

    let mut f = std::fs::File::open("fixtures/github.json").unwrap();
    let mut buf = String::new();
    f.read_to_string(&mut buf).unwrap();
    let json: serde_json::Value = serde_json::from_str(&buf).unwrap();

    smaller_inputs_group.bench_function("jindex gron github.json", |b| {
        b.iter(|| {
            let mut writer = vec![];
            let mut sink = GronWriter::new(&mut writer);
            jindex(&mut sink, black_box(&json)).unwrap()
        })
    });

    let mut f = std::fs::File::open("fixtures/three.json").unwrap();
    let mut buf = String::new();
    f.read_to_string(&mut buf).unwrap();
    let json: serde_json::Value = serde_json::from_str(&buf).unwrap();

    smaller_inputs_group.bench_function("jindex gron three.json", |b| {
        b.iter(|| {
            let mut writer = vec![];
            let mut sink = GronWriter::new(&mut writer);
            jindex(&mut sink, black_box(&json)).unwrap()
        })
    });

    smaller_inputs_group.finish();
}

fn json_pointer_benchmark(c: &mut Criterion) {
    let mut larger_inputs_group = c.benchmark_group("larger inputs");

    larger_inputs_group.measurement_time(std::time::Duration::from_secs(20));

    let mut f = std::fs::File::open("fixtures/big.json").unwrap();
    let mut buf = String::new();
    f.read_to_string(&mut buf).unwrap();
    let json: serde_json::Value = serde_json::from_str(&buf).unwrap();

    larger_inputs_group.bench_function("jindex jsonpointer big.json", |b| {
        b.iter(|| {
            let mut writer = vec![];
            let options = JSONPointerWriterOptions::default();
            let mut sink = JSONPointerWriter::new(&mut writer, options);
            jindex(&mut sink, black_box(&json)).unwrap()
        })
    });

    larger_inputs_group.finish();

    /////////////////////////////////////////////////

    let mut smaller_inputs_group = c.benchmark_group("smaller inputs");

    let mut f = std::fs::File::open("fixtures/github.json").unwrap();
    let mut buf = String::new();
    f.read_to_string(&mut buf).unwrap();
    let json: serde_json::Value = serde_json::from_str(&buf).unwrap();

    smaller_inputs_group.bench_function("jindex jsonpointer github.json", |b| {
        b.iter(|| {
            let mut writer = vec![];
            let options = JSONPointerWriterOptions::default();
            let mut sink = JSONPointerWriter::new(&mut writer, options);
            jindex(&mut sink, black_box(&json)).unwrap()
        })
    });

    let mut f = std::fs::File::open("fixtures/three.json").unwrap();
    let mut buf = String::new();
    f.read_to_string(&mut buf).unwrap();
    let json: serde_json::Value = serde_json::from_str(&buf).unwrap();

    smaller_inputs_group.bench_function("jindex jsonpointer three.json", |b| {
        b.iter(|| {
            let mut writer = vec![];
            let options = JSONPointerWriterOptions::default();
            let mut sink = JSONPointerWriter::new(&mut writer, options);
            jindex(&mut sink, black_box(&json)).unwrap()
        })
    });

    smaller_inputs_group.finish();
}

criterion_group!(benches, gron_benchmark, json_pointer_benchmark);
criterion_main!(benches);
