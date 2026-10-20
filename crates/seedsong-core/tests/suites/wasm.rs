#![cfg(target_arch = "wasm32")]

use seedsong_core::{
    backend::web_audio::{encode_dna, random_dna, render_wav},
    dna::Dna,
    music::Song,
};
use wasm_bindgen_test::wasm_bindgen_test;

const DNA: &str = "9a401c07e8553fa2710c0000000000000209b62d000000000000000000000000";

#[wasm_bindgen_test]
fn exported_encoder_matches_core() {
    let expected = Song::generate(Dna::parse(DNA).unwrap(), [None; 4]).encode();
    assert_eq!(encode_dna(DNA).unwrap(), expected);
}

#[wasm_bindgen_test]
fn exported_functions_reject_invalid_dna() {
    for invalid in ["", "not a DNA", &"z".repeat(64)] {
        assert!(encode_dna(invalid).is_err());
        assert!(render_wav(invalid, false, false).is_err());
    }
}

#[wasm_bindgen_test]
fn random_dna_uses_js_entropy_and_roundtrips() {
    for _ in 0..16 {
        let dna = random_dna();
        assert_eq!(dna.len(), 64);
        assert!(dna.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(Dna::parse(&dna).unwrap().to_string(), dna);
    }
}

fn assert_loop_wav(piano: bool) {
    let bytes = render_wav(DNA, piano, true).unwrap();
    let u32_at = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(u32_at(4) as usize, bytes.len() - 8);
    assert_eq!(&bytes[20..24], &[1, 0, 2, 0]); // PCM16, stereo
    assert_eq!(u32_at(24), 44100);
    assert_eq!(&bytes[34..36], &16u16.to_le_bytes());
    assert_eq!(u32_at(40) as usize, bytes.len() - 44);
    let bpm = encode_dna(DNA).unwrap()[5] as f64;
    let frames = (256. * 60. / bpm / 4. * 44100.).round() as usize;
    assert_eq!(bytes.len(), 44 + frames * 4);
    let peak = bytes[44..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| i16::from_le_bytes([b[0], b[1]]).unsigned_abs())
        .max()
        .unwrap();
    assert_eq!(peak, (0.95_f64 * 32767.).round() as u16);
}

#[wasm_bindgen_test]
fn spec_wav_is_normalized_and_has_exact_loop_length() {
    assert_loop_wav(false);
}

#[wasm_bindgen_test]
fn piano_wav_is_normalized_and_has_exact_loop_length() {
    assert_loop_wav(true);
}
