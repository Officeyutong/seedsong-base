use seedsong_core::{
    dna::Dna,
    music::{Params, Song, harmony},
    rng::Stream,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn spec_parameter_example_and_harmony() {
    let mut d = [0; 32];
    d[..10].copy_from_slice(&[0x9a, 0x40, 0x1c, 0x07, 0xe8, 0x55, 0x3f, 0xa2, 0x71, 0x0c]);
    d[16..20].copy_from_slice(&[2, 9, 0xb6, 0x2d]);
    let dna = Dna(d);
    let p = Params::new(&dna);
    assert_eq!((p.mode, p.pc, p.bpm, p.tonic), (4, 9, 124, 57));
    assert_eq!(
        (p.form, p.bass_pattern, p.pad_pattern, p.drums, p.swing),
        (2, 1, 3, 2, 0)
    );
    let chords = harmony(&p, &dna);
    assert_eq!(
        chords[..4].iter().map(|c| c.pcs[0]).collect::<Vec<_>>(),
        [9, 4, 2, 4]
    );
    assert_eq!(
        chords[8..12].iter().map(|c| c.pcs[0]).collect::<Vec<_>>(),
        [9, 2, 9, 4]
    );
    assert_eq!(chords[15].pcs[0], 9);
    let song = Song::generate(dna, [None; 4]);
    assert_eq!(
        &song.encode()[..16],
        &[b'S', b'N', b'G', b'1', 1, 124, 9, 4, 2, 0, 0, 3, 1, 2, 0, 2]
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn integer_decoder_invariants_across_modes_and_patterns() {
    let mut rng = Stream::new("test", &[0]);
    for i in 0..=255 {
        let mut d = [0u8; 32];
        for b in &mut d {
            *b = rng.next_u64() as u8;
        }
        d[0] = i;
        d[18] = i;
        let dna = Dna(d);
        let song = Song::generate(dna, [None; 4]);
        let p = &song.params;
        let chords = harmony(p, &dna);
        let encoded = song.encode();
        assert_eq!(encoded.len(), 18 + song.events.len() * 8);
        assert_eq!(
            u16::from_le_bytes([encoded[16], encoded[17]]) as usize,
            song.events.len()
        );
        assert!(song.events.windows(2).all(|w|(w[0].start,w[0].part,w[0].pitch)<=(w[1].start,w[1].part,w[1].pitch)));
        let mel: Vec<_> = song.events.iter().filter(|e| e.part == 0).collect();
        assert_eq!(mel.last().unwrap().pitch % 12, p.pc);
        for e in &song.events {
            assert!(e.start < 256 && e.duration > 0 && e.start + e.duration <= 256);
            assert!((1..128).contains(&e.velocity));
            if e.part == 0 {
                assert!((p.tonic - 5..=p.tonic + 16).contains(&(e.pitch as i16)));
                assert!(p.degrees().iter().any(|&s| p.pitch(s) == e.pitch as i16));
                if [0, 8].contains(&(e.start % 16)) {
                    assert!(
                        chords[e.start as usize / 16]
                            .pcs
                            .contains(&(e.pitch as i16 % 12))
                    );
                }
            }
        }
        let bass: Vec<_> = song
            .events
            .iter()
            .filter(|e| e.part == 1 && e.start >= 240)
            .collect();
        assert_eq!(bass.len(), 1);
        assert_eq!(bass[0].duration, 16);
        // Cover bytes must have no musical effect.
        let mut other = dna;
        other.0[10..16].fill(99);
        assert_eq!(encoded, Song::generate(other, [None; 4]).encode());
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn repeated_phrase_pitch_and_rhythm_are_copied() {
    let dna = Dna([0; 32]);
    let song = Song::generate(dna, [None; 4]);
    let phrase = |start: u16| {
        song.events
            .iter()
            .filter(|e| e.part == 0 && (start..start + 48).contains(&e.start))
            .map(|e| (e.start - start, e.duration, e.pitch))
            .collect::<Vec<_>>()
    };
    assert_eq!(phrase(0), phrase(64));
    assert_eq!(phrase(0), phrase(192));
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn ensemble_uses_conductor_context_and_preserves_its_role() {
    let conductor = Dna([0; 32]);
    let solo = Song::generate(conductor, [None; 4]);
    let mut bass = Dna([255; 32]);
    bass.0[16] = 1;
    let ensemble = Song::generate(conductor, [Some(bass), Some(bass), None, None]);
    assert_eq!(solo.params.bpm, ensemble.params.bpm);
    assert_eq!(&solo.encode()[..16], &ensemble.encode()[..16]);
    assert_eq!(
        solo.events
            .iter()
            .filter(|e| e.part == 0)
            .collect::<Vec<_>>(),
        ensemble
            .events
            .iter()
            .filter(|e| e.part == 0)
            .collect::<Vec<_>>()
    );
    assert_eq!(ensemble.sources[0], conductor);
    assert_eq!(ensemble.sources[1], bass);
    let chords = harmony(&solo.params, &conductor);
    assert!(
        ensemble
            .events
            .iter()
            .filter(|e| e.part == 1)
            .all(|e| e.pitch as i16 % 12 == chords[e.start as usize / 16].pcs[0])
    );
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn regression_vectors() {
    // Locally generated regression vectors; these are not independent acceptance vectors.
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../../vectors/music-v1.json")).unwrap();
    for v in vectors["vectors"].as_array().unwrap() {
        let song = Song::generate(Dna::parse(v["dna"].as_str().unwrap()).unwrap(), [None; 4]);
        let hex: String = song.encode().iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, v["sng1_hex"].as_str().unwrap());
    }
}
