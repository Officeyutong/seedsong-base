//! Native unit tests against independent official decoders. The JS suite uses
//! the same oracle to compare the actual compiled WASM, not just native Rust.
#[path = "../tests/support/official_dob.rs"]
mod official;
use crate::dob::{decode_dob0, decode_dob1, parse_dna};
use serde_json::Value;

fn compare(dna: &str, dob0: &Value, dob1: &Value) {
    let (expected0, expected1) = official::decode(dna, dob0, dob1)
        .unwrap_or_else(|error| panic!("Official decoding failed for DNA {dna}: {error:#}"));
    let traits = decode_dob0(
        &parse_dna(dna).unwrap(),
        &serde_json::from_value::<Vec<Vec<Value>>>(dob0.clone()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&traits).unwrap(),
        expected0,
        "DOB/0 DNA={dna}"
    );
    let decoded = decode_dob1(
        &traits,
        &serde_json::from_value::<Vec<Vec<Value>>>(dob1.clone()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&decoded).unwrap(),
        expected1,
        "DOB/1 DNA={dna}"
    );
    // Compare SVG UTF-8 bytes verbatim: never trim, canonicalize or parse XML.
    let actual = serde_json::to_value(decoded).unwrap();
    let svgs = |value: &Value| -> Vec<Vec<u8>> {
        value
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|entry| entry["traits"].as_array().unwrap())
            .filter_map(|value| value["SVG"].as_str().map(|text| text.as_bytes().to_vec()))
            .collect()
    };
    assert_eq!(svgs(&actual), svgs(&expected1), "SVG bytes DNA={dna}");
}

#[test]
fn official_dob_collection_corpus_and_byte_boundaries() {
    let document = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../pattern.md"));
    let patterns: Vec<Value> = document
        .split("```json\n")
        .skip(1)
        .map(|block| serde_json::from_str(block.split("```").next().unwrap()).unwrap())
        .filter(Value::is_array)
        .collect();
    assert_eq!(
        patterns.len(),
        2,
        "Expected the two published DOB pattern arrays"
    );
    let report: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../reports/m1-v2/quality-report.json"
    )))
    .unwrap();
    assert_eq!(report["songs"].as_array().unwrap().len(), 1000);
    for song in report["songs"].as_array().unwrap() {
        compare(song["dna"].as_str().unwrap(), &patterns[0], &patterns[1]);
    }
    let reference = "9a401c07e8553fa2710c0000000000000209b62d000000000000000000000000";
    compare(reference, &patterns[0], &patterns[1]);
    for byte in ["00", "ff"] {
        compare(&byte.repeat(32), &patterns[0], &patterns[1]);
    }
    for offset in 0..32 {
        for byte in [0u8, 1, 254, 255] {
            let mut dna = parse_dna(reference).unwrap();
            dna[offset] = byte;
            let hex: String = dna.iter().map(|value| format!("{value:02x}")).collect();
            compare(&hex, &patterns[0], &patterns[1]);
        }
    }
}

#[test]
fn official_dob_semantic_vectors() {
    let vectors: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/dob-parity.json"
    )))
    .unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        for dna in case["dnas"].as_array().unwrap() {
            compare(dna.as_str().unwrap(), &case["dob0"], &case["dob1"]);
        }
    }
}
