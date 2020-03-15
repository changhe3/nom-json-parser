use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use nom_json_parser::{parse, IResult, Json};
use std::fs::File;
use std::io::Read;

fn run_benchmark(c: &mut Criterion) {
    let paths: &[&str] = &[
        "benches/data/canada.json",
        "benches/data/citm_catalog.json",
        "benches/data/twitter.json",
    ];
    let mut group = c.benchmark_group("Parsing ");
    group.sample_size(10);
    for path in paths {
        let data = {
            let mut s = String::new();
            File::open(path).unwrap().read_to_string(&mut s).unwrap();
            s
        };
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(path),
            &data.as_str(),
            |b, &data| {
                b.iter(|| {
                    let res: IResult<_, Json> = parse(black_box(data));
                    let _ = res.unwrap();
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, run_benchmark);
criterion_main!(benches);
