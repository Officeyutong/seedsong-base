use rayon::prelude::*;
use seedsong_core::{
    dna::Dna,
    music::{MusicVersion, Song, harmony},
    rng::Stream,
};
use serde_json::{Value, json};

pub fn report(count: usize, version: MusicVersion) -> Value {
    let rows:Vec<Value>=(0..count).into_par_iter().map(|i| {
        let mut rng=Stream::new("sng/quality",&(i as u64).to_le_bytes());
        let mut dna=[0;32];
        for chunk in dna.chunks_mut(8) {chunk.copy_from_slice(&rng.next_u64().to_le_bytes());}
        let dna=Dna(dna);
        let song=Song::generate_with_version(dna,[None;4],version);
        let p=&song.params;
        let chords=harmony(p,&dna);
        let notes:Vec<_>=song.events.iter().filter(|e|e.part==0).collect();
        let degrees=p.degrees();
        let ds:Vec<_>=notes.iter().map(|e| *degrees.iter().find(|&&d|p.pitch(d)==e.pitch as i16).unwrap()).collect();
        let leaps=ds.windows(2).filter(|w|(w[1]-w[0]).abs()>=4).count();
        let strong=notes.iter().filter(|e| [0,8].contains(&(e.start%16)) && !chords[e.start as usize/16].pcs.contains(&(e.pitch as i16%12))).count();
        let range=notes.iter().filter(|e|!(p.tonic-5..=p.tonic+16).contains(&(e.pitch as i16))).count();
        let repeats=ds.windows(3).filter(|w|w[0]==w[1] && w[1]==w[2]).count();
        let tonic=notes.last().unwrap().pitch%12==p.pc;
        json!({"dna":dna.to_string(),"notes":notes.len(),"strong_non_chord":strong,"out_of_range":range,
            "large_leaps":leaps,"intervals":ds.len()-1,"triple_repeats":repeats,"ends_on_tonic":tonic})
    }).collect();
    let sum = |key: &str| rows.iter().map(|r| r[key].as_u64().unwrap()).sum::<u64>();
    let max_leap = rows
        .iter()
        .map(|r| r["large_leaps"].as_f64().unwrap() / r["intervals"].as_f64().unwrap())
        .fold(0., f64::max);
    let length_failures = rows
        .iter()
        .filter(|r| !(40..=160).contains(&r["notes"].as_u64().unwrap()))
        .count();
    let strong = sum("strong_non_chord");
    let range = sum("out_of_range");
    let triples = sum("triple_repeats");
    let tonic = rows.iter().filter(|r| r["ends_on_tonic"] == true).count();
    json!({"version":version.specification(),"count":count,"seed":"sng/quality + little-endian u64 index",
        "summary":{"strong_non_chord":strong,"out_of_range":range,"large_leap_ratio":sum("large_leaps") as f64/sum("intervals") as f64,
        "max_song_large_leap_ratio":max_leap,"triple_repeat_windows":triples,"ends_on_tonic":tonic,"note_count_failures":length_failures,
        "all_targets_pass":strong==0 && range==0 && triples==0 && tonic==count && length_failures==0 && max_leap<=0.08},
        "notes":match version {
            MusicVersion::V1 => "Quality targets are observations, not extra rules: chord snapping and phrase copies can introduce repetitions and large leaps. No official vectors were supplied.",
            MusicVersion::V2 => "Final v2 melody events are checked, including phrase boundaries. Whole-melody refinement enforces no triple repeats and a per-song large-leap budget. No independent acceptance vectors were supplied.",
        },
        "songs":rows})
}
