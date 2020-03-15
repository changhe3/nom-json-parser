use criterion::black_box;
use nom_json_parser::{parse, IResult, Json};
use std::fs::File;
use std::io::Read;

fn main() {
    let paths: &[&str] = &[
        "benches/data/canada.json",
        "benches/data/citm_catalog.json",
        "benches/data/twitter.json",
    ];
    for path in paths {
        let data = {
            let mut s = String::new();
            File::open(path).unwrap().read_to_string(&mut s).unwrap();
            s
        };
        for _ in 0..100 {
            let res: IResult<_, Json> = parse(black_box(&data));
            let _ = res.unwrap();
        }
    }
}
