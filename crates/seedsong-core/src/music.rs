use crate::{dna::Dna, rng::Stream};
mod refinement;
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

pub const MODES: [&str; 7] = [
    "Major",
    "Minor",
    "Dorian",
    "Mixolydian",
    "Major Pentatonic",
    "Minor Pentatonic",
    "Lydian",
];

/// Version of the deterministic note algorithm, stored in SNG1 header byte 4.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum MusicVersion {
    V1 = 1,
    #[default]
    V2 = 2,
}
impl MusicVersion {
    pub fn specification(self) -> &'static str {
        match self {
            Self::V1 => "seedsong-music/1",
            Self::V2 => "seedsong-music/2",
        }
    }
}
impl TryFrom<u8> for MusicVersion {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::V1),
            2 => Ok(Self::V2),
            _ => anyhow::bail!("music version must be 1 or 2"),
        }
    }
}
pub const KEYS: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
pub const FORMS: [[usize; 4]; 4] = [[0, 0, 1, 0], [0, 1, 0, 1], [0, 0, 1, 1], [0, 1, 0, 2]];
pub const LEADS: [&str; 6] = [
    "Soft Sine",
    "Warm Organ",
    "E-Piano",
    "Glass Bell",
    "Music Box",
    "Crystal",
];
pub const SCALES: [&[i16]; 7] = [
    &[0, 2, 4, 5, 7, 9, 11],
    &[0, 2, 3, 5, 7, 8, 10],
    &[0, 2, 3, 5, 7, 9, 10],
    &[0, 2, 4, 5, 7, 9, 10],
    &[0, 2, 4, 7, 9],
    &[0, 3, 5, 7, 10],
    &[0, 2, 4, 6, 7, 9, 11],
];
const MAJOR: [[usize; 4]; 8] = [
    [0, 4, 5, 3],
    [0, 5, 3, 4],
    [5, 3, 0, 4],
    [0, 3, 5, 4],
    [0, 4, 3, 4],
    [0, 2, 3, 4],
    [3, 0, 4, 5],
    [0, 3, 0, 4],
];
const MINOR: [[usize; 4]; 8] = [
    [0, 5, 2, 6],
    [0, 3, 6, 2],
    [0, 6, 5, 6],
    [0, 3, 4, 0],
    [5, 6, 0, 0],
    [0, 2, 6, 3],
    [0, 5, 3, 4],
    [0, 3, 0, 4],
];
const RHYTHMS: [[&str; 4]; 3] = [
    [
        "x---x---x-------",
        "x-------x---x---",
        "x---x-------x---",
        "x-----x-x-------",
    ],
    [
        "x-x-x---x-x-x---",
        "x---x-x-x---x---",
        "x-x---x-x-x-x---",
        "x--x--x-x---x-x-",
    ],
    [
        "x-x-x-x-x-x-x-x-",
        "x-xxx-x-x-xxx-x-",
        "x-x-xx-xx-x-x---",
        "xxx-x-x-xxx-x---",
    ],
];
const ENDINGS: [&str; 3] = ["x---------------", "x-------x-------", "x---x-----------"];

#[derive(Clone, Debug)]
pub struct Params {
    pub bpm: u8,
    pub pc: u8,
    pub mode: u8,
    pub form: u8,
    pub lead: u8,
    pub bass: u8,
    pub pad_pattern: u8,
    pub bass_pattern: u8,
    pub drums: u8,
    pub swing: u8,
    pub role: u8,
    pub tonic: i16,
}
impl Params {
    pub fn new(d: &Dna) -> Self {
        let d = d.0;
        let mode = match d[0] % 32 {
            0..=7 => 0,
            8..=15 => 1,
            16..=19 => 2,
            20..=23 => 3,
            24..=27 => 4,
            28..=30 => 5,
            _ => 6,
        };
        let lead = match d[4] % 32 {
            0..=9 => 0,
            10..=16 => 1,
            17..=22 => 2,
            23..=27 => 3,
            28..=30 => 4,
            _ => 5,
        };
        let pc = d[17] % 12;
        Self {
            bpm: 60 + d[1] % 81,
            pc,
            mode,
            form: d[18] % 4,
            lead,
            bass: match d[5] % 4 {
                0 | 1 => 0,
                2 => 1,
                _ => 2,
            },
            pad_pattern: d[18] / 16 % 4,
            bass_pattern: d[18] / 4 % 4,
            drums: d[18] / 64 % 4,
            swing: [0, 0, 12, 25][(d[19] % 4) as usize],
            role: d[16] % 4,
            tonic: if pc <= 7 {
                60 + pc as i16
            } else {
                48 + pc as i16
            },
        }
    }
    pub fn pitch(&self, s: i16) -> i16 {
        let scale = SCALES[self.mode as usize];
        self.tonic
            + 12 * s.div_euclid(scale.len() as i16)
            + scale[s.rem_euclid(scale.len() as i16) as usize]
    }
    pub fn degrees(&self) -> Vec<i16> {
        (-14..=21)
            .filter(|&s| (self.tonic - 5..=self.tonic + 16).contains(&self.pitch(s)))
            .collect()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub start: u16,
    pub duration: u16,
    pub part: u8,
    pub pitch: u8,
    pub velocity: u8,
}
impl Event {
    fn new(start: usize, duration: usize, part: u8, pitch: i16, velocity: u8) -> Self {
        Self {
            start: start as u16,
            duration: duration as u16,
            part,
            pitch: pitch as u8,
            velocity,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Chord {
    pub pcs: [i16; 3],
}
impl Chord {
    fn contains(&self, pitch: i16) -> bool {
        self.pcs.contains(&pitch.rem_euclid(12))
    }
}
pub fn harmony(p: &Params, dna: &Dna) -> [Chord; 16] {
    let scale = SCALES[match p.mode {
        4 => 0,
        5 => 1,
        m => m,
    } as usize];
    let templates = if [1, 2, 5].contains(&p.mode) {
        MINOR
    } else {
        MAJOR
    };
    std::array::from_fn(|bar| {
        let letter = FORMS[p.form as usize][bar / 4];
        let t = (dna.0[2] as usize + [0, 3, 5][letter]) % 8;
        let mut degree = if bar == 15 { 0 } else { templates[t][bar % 4] };
        if (scale[(degree + 4) % 7] - scale[degree]).rem_euclid(12) == 6 {
            degree = (degree + 5) % 7;
        }
        Chord {
            pcs: std::array::from_fn(|i| {
                (p.pc as i16 + scale[(degree + 2 * i) % 7]).rem_euclid(12)
            }),
        }
    })
}
fn onsets(pattern: &str) -> Vec<(usize, usize)> {
    let b = pattern.as_bytes();
    b.iter()
        .enumerate()
        .filter(|(_, v)| **v == b'x')
        .map(|(i, _)| {
            let end = (i + 1..16).find(|&j| b[j] != b'-').unwrap_or(16);
            (i, end - i)
        })
        .collect()
}
#[derive(Clone, Default)]
struct History {
    prev: i16,
    before: Option<i16>,
    last_iv: i16,
    seen: bool,
}
impl History {
    fn push(&mut self, s: i16) {
        self.last_iv = s - self.prev;
        self.before = self.seen.then_some(self.prev);
        self.seen = true;
        self.prev = s;
    }
}
fn snap(p: &Params, chord: Chord, s: i16, prev: i16, degrees: &[i16]) -> i16 {
    *degrees
        .iter()
        .filter(|&&d| chord.contains(p.pitch(d)))
        .min_by_key(|&&d| {
            (
                (p.pitch(d) - p.pitch(s)).abs(),
                (p.pitch(d) - p.pitch(prev)).abs(),
                p.pitch(d),
            )
        })
        .unwrap()
}
#[allow(clippy::too_many_arguments)]
fn melody_bar(
    p: &Params,
    final_tonic: bool,
    dna: &Dna,
    chord: Chord,
    bar: usize,
    letter: usize,
    pattern: &str,
    rng: &mut Stream,
    h: &mut History,
) -> Vec<(usize, usize, i16)> {
    let degrees = p.degrees();
    let contour: &[u32] = match dna.0[3] % 8 {
        0..=2 => &[15, 60, 20, 5, 0],
        3..=6 => &[10, 50, 25, 10, 5],
        _ => &[8, 40, 25, 15, 12],
    };
    let center = p.tonic + 5 + if letter > 0 { 3 } else { 0 };
    let notes = onsets(pattern);
    let mut out = Vec::new();
    for (index, &(step, duration)) in notes.iter().enumerate() {
        let mut iv = if h.last_iv.abs() >= 3 {
            -h.last_iv.signum()
        } else {
            let mag = rng.weighted(contour) as i16;
            let up = if p.pitch(h.prev) > center + 7 {
                !rng.chance(3, 4)
            } else if p.pitch(h.prev) < center - 2 {
                rng.chance(3, 4)
            } else {
                rng.chance(1, 2)
            };
            if up { mag } else { -mag }
        };
        if iv == 0 && h.before == Some(h.prev) {
            iv = if p.pitch(h.prev) < center { 1 } else { -1 };
        }
        let mut s = h.prev + iv;
        let in_range = |s| (p.tonic - 5..=p.tonic + 16).contains(&p.pitch(s));
        if !in_range(s) {
            s = h.prev - iv;
        }
        if !in_range(s) {
            s = *degrees
                .iter()
                .min_by_key(|&&d| ((p.pitch(d) - p.pitch(s)).abs(), p.pitch(d)))
                .unwrap();
        }
        if step == 0 || step == 8 || (bar % 4 == 3 && index + 1 == notes.len()) {
            s = snap(p, chord, s, h.prev, &degrees);
        }
        if final_tonic && index + 1 == notes.len() {
            s = *degrees
                .iter()
                .filter(|&&d| [p.tonic, p.tonic + 12].contains(&p.pitch(d)))
                .min_by_key(|&&d| ((p.pitch(d) - p.pitch(h.prev)).abs(), p.pitch(d)))
                .unwrap();
        }
        h.push(s);
        out.push((step, duration, s));
    }
    out
}
fn melody(p: &Params, dna: &Dna, chords: &[Chord; 16]) -> Vec<Event> {
    let mut mel = Stream::new("sng/melody", &dna.0[6..10]);
    let rhythm_bytes: Vec<u8> = dna.0[6..10].iter().chain(&dna.0[20..24]).copied().collect();
    let mut rhythm = Stream::new("sng/rhythm", &rhythm_bytes);
    let mut vary = Stream::new("sng/vary", &dna.0[20..32]);
    let mut human = Stream::new("sng/human", &dna.0[24..32]);
    type Bar = Vec<(usize, usize, i16)>;
    let mut cache: [Option<Vec<Bar>>; 3] = [None, None, None];
    let mut patterns: [Option<[&str; 4]>; 3] = [None, None, None];
    let mut h = History::default();
    let mut output = Vec::new();
    for (phrase, &letter) in FORMS[p.form as usize].iter().enumerate() {
        let first = cache[letter].is_none();
        if first {
            let mut weights = if p.bpm < 80 {
                [20, 50, 30]
            } else if p.bpm < 110 {
                [30, 50, 20]
            } else {
                [50, 40, 10]
            };
            let density = [0, -1, 1, 0][(dna.0[19] / 4 % 4) as usize];
            let density = if letter > 0 {
                (density + 1).min(1)
            } else {
                density
            };
            if density < 0 {
                weights[0] += weights[2];
                weights[2] = 0;
            }
            if density > 0 {
                weights[2] += weights[0];
                weights[0] = 0;
            }
            let r0 = RHYTHMS[rhythm.weighted(&weights)][rhythm.below(4) as usize];
            let r1 = RHYTHMS[rhythm.weighted(&weights)][rhythm.below(4) as usize];
            let e = ENDINGS[rhythm.below(3) as usize];
            patterns[letter] = Some([r0, r1, r0, e]);
        }
        let patterns = patterns[letter].unwrap();
        let mut phrase_notes = Vec::new();
        for (local, &pattern) in patterns.iter().enumerate() {
            let bar = phrase * 4 + local;
            let notes = if !first && local < 3 {
                let notes = cache[letter].as_ref().unwrap()[local].clone();
                for &(_, _, s) in &notes {
                    h.push(s);
                }
                notes
            } else {
                let before = h.clone();
                let rng = if first { &mut mel } else { &mut vary };
                let notes = melody_bar(
                    p,
                    !first && bar == 15,
                    dna,
                    chords[bar],
                    bar,
                    letter,
                    pattern,
                    rng,
                    &mut h,
                );
                // ABAC introduces C in the final phrase: create its original bar,
                // then regenerate the ending with the variation stream (§5.6).
                if first && bar == 15 {
                    h = before;
                    melody_bar(
                        p,
                        true,
                        dna,
                        chords[bar],
                        bar,
                        letter,
                        pattern,
                        &mut vary,
                        &mut h,
                    )
                } else {
                    notes
                }
            };
            for &(step, duration, s) in &notes {
                let base = match step {
                    0 => 100,
                    8 => 92,
                    4 | 12 => 84,
                    s if s % 2 == 0 => 76,
                    _ => 68,
                };
                let velocity = (base + human.below(9) as i16 - 4).clamp(1, 127) as u8;
                output.push(Event::new(
                    bar * 16 + step,
                    duration,
                    0,
                    p.pitch(s),
                    velocity,
                ));
            }
            phrase_notes.push(notes);
        }
        if first {
            cache[letter] = Some(phrase_notes);
        }
    }
    output
}
fn bass(own: &Params, chords: &[Chord; 16]) -> Vec<Event> {
    let mut out = Vec::new();
    for (bar, c) in chords.iter().enumerate() {
        let root = 36 + c.pcs[0];
        let fifth = 36 + (c.pcs[0] + 7) % 12;
        let pattern = if bar == 15 { 0 } else { own.bass_pattern };
        let notes = match pattern {
            0 => vec![(0, 16, root)],
            1 => vec![(0, 8, root), (8, 8, root)],
            2 => vec![(0, 8, root), (8, 8, fifth)],
            _ => vec![
                (0, 4, root),
                (4, 4, root + 12),
                (8, 4, root),
                (12, 4, root + 12),
            ],
        };
        for (step, dur, pitch) in notes {
            out.push(Event::new(
                bar * 16 + step,
                dur,
                1,
                pitch,
                if step == 0 { 96 } else { 84 },
            ));
        }
    }
    out
}
fn voicing(c: Chord, inversion: usize) -> [i16; 3] {
    let mut notes = [
        c.pcs[0],
        c.pcs[0] + (c.pcs[1] - c.pcs[0]).rem_euclid(12),
        c.pcs[0] + (c.pcs[2] - c.pcs[0]).rem_euclid(12),
    ];
    for n in notes.iter_mut().take(inversion) {
        *n += 12;
    }
    notes.sort();
    let shift = 52 + (notes[0] - 52).rem_euclid(12) - notes[0];
    notes.map(|n| n + shift)
}
fn pad(own: &Params, chords: &[Chord; 16]) -> Vec<Event> {
    let mut out = Vec::new();
    let mut previous = voicing(chords[0], 0);
    for (bar, &c) in chords.iter().enumerate() {
        let notes = if bar == 0 {
            previous
        } else {
            (0..3)
                .map(|i| voicing(c, i))
                .min_by_key(|v| {
                    v.iter()
                        .zip(previous)
                        .map(|(&a, b)| (a - b).abs())
                        .sum::<i16>()
                })
                .unwrap()
        };
        previous = notes;
        let mut add = |step, dur, pitch| out.push(Event::new(bar * 16 + step, dur, 2, pitch, 64));
        match own.pad_pattern {
            0 => {
                for n in notes {
                    add(0, 16, n)
                }
            }
            1 => {
                for step in [0, 8] {
                    for n in notes {
                        add(step, 8, n)
                    }
                }
            }
            2 => {
                for i in 0..8 {
                    add(i * 2, 2, notes[[0, 1, 2, 1][i % 4]])
                }
            }
            _ => {
                for step in [4, 12] {
                    for n in notes {
                        add(step, 2, n)
                    }
                }
            }
        }
    }
    out
}
fn drums(own: &Params, dna: &Dna, melody_count: usize) -> Vec<Event> {
    let patterns = [
        ["x-------x-------", "----x-------x---", "x-x-x-x-x-x-x-x-"],
        ["x---------x-----", "--------x-------", "x---x---x---x---"],
        ["x---x---x---x---", "----x-------x---", "--x---x---x---x-"],
        ["x---------------", "................", "--------x-------"],
    ];
    let mut human = Stream::new("sng/human", &dna.0[24..32]);
    // The human stream consumes all melody velocities first, then hats in time order.
    for _ in 0..melody_count {
        human.below(9);
    }
    let mut out = Vec::new();
    for bar in 0..16 {
        for step in 0..16 {
            for (i, pattern) in patterns[own.drums as usize].iter().enumerate() {
                let fill = i == 1
                    && [0, 2].contains(&own.drums)
                    && bar % 4 == 3
                    && bar != 15
                    && step >= 14;
                if pattern.as_bytes()[step] == b'x' || fill {
                    let vel = match i {
                        0 => 100,
                        1 => 90,
                        _ => 60 + human.below(16) as u8,
                    };
                    out.push(Event::new(bar * 16 + step, 1, 3, [36, 38, 42][i], vel));
                }
            }
        }
    }
    out
}
#[derive(Clone)]
pub struct Song {
    pub version: MusicVersion,
    pub params: Params,
    pub events: Vec<Event>,
    pub sources: [Dna; 4],
}
impl Song {
    pub fn generate(conductor: Dna, participants: [Option<Dna>; 4]) -> Self {
        Self::generate_with_version(conductor, participants, MusicVersion::V2)
    }

    pub fn generate_with_version(
        conductor: Dna,
        participants: [Option<Dna>; 4],
        version: MusicVersion,
    ) -> Self {
        let p = Params::new(&conductor);
        let chords = harmony(&p, &conductor);
        let mut sources = participants.map(|d| d.unwrap_or(conductor));
        sources[p.role as usize] = conductor;
        let parts: Vec<Vec<Event>> = platform_iter!((0..4usize).into_par_iter(), 0..4usize)
            .map(|part| {
                let own = Params::new(&sources[part]);
                match part {
                    0 => {
                        let mut notes = melody(&p, &sources[0], &chords);
                        if version == MusicVersion::V2 {
                            refinement::refine(&p, &chords, &mut notes);
                        }
                        notes
                    }
                    1 => bass(&own, &chords),
                    2 => pad(&own, &chords),
                    _ => {
                        let count = melody(&p, &sources[3], &chords).len();
                        drums(&own, &sources[3], count)
                    }
                }
            })
            .collect();
        let mut events: Vec<_> = parts.into_iter().flatten().collect();
        events.sort_by_key(|e| (e.start, e.part, e.pitch));
        Self {
            version,
            params: p,
            events,
            sources,
        }
    }
    pub fn encode(&self) -> Vec<u8> {
        let p = &self.params;
        let mut b = b"SNG1".to_vec();
        b.extend([
            self.version as u8,
            p.bpm,
            p.pc,
            p.mode,
            p.form,
            p.lead,
            p.bass,
            p.pad_pattern,
            p.bass_pattern,
            p.drums,
            p.swing,
            p.role,
        ]);
        b.extend((self.events.len() as u16).to_le_bytes());
        for e in &self.events {
            b.extend(e.start.to_le_bytes());
            b.extend(e.duration.to_le_bytes());
            b.extend([e.part, e.pitch, e.velocity, 0]);
        }
        b
    }
}
