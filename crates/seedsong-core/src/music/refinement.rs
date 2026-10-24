//! Integer-only, whole-melody voice leading for seedsong-music/2.
use super::{Chord, Event, Params};

const INFINITY: u32 = u32::MAX / 4;

/// Find the least-cost pitch path around the v1 draft. Rhythm, velocities and
/// random-stream consumption are unchanged. Checking the complete phrase chain
/// catches repetitions and leaps introduced by snapping and phrase boundaries.
pub(super) fn refine(p: &Params, chords: &[Chord; 16], notes: &mut [Event]) {
    if notes.is_empty() {
        return;
    }
    let degrees = p.degrees();
    let pitches: Vec<i16> = degrees.iter().map(|&degree| p.pitch(degree)).collect();
    let width = degrees.len();
    let budget = (notes.len() - 1) * 8 / 100;
    // State: large leaps used, previous scale degree, identical-note run (1/2).
    let states = (budget + 1) * width * 2;
    let mut costs = vec![INFINITY; states];
    let mut predecessors = vec![vec![usize::MAX; states]; notes.len()];

    for (i, note) in notes.iter().enumerate() {
        let last = i + 1 == notes.len();
        let chord = chords[note.start as usize / 16];
        let phrase_ending =
            note.start / 16 % 4 == 3 && (last || notes[i + 1].start / 16 != note.start / 16);
        let needs_chord = [0, 8].contains(&(note.start % 16)) || phrase_ending;
        let candidates: Vec<usize> = pitches
            .iter()
            .enumerate()
            .filter(|&(_, &pitch)| {
                (!needs_chord || chord.contains(pitch))
                    && (!last || [p.tonic, p.tonic + 12].contains(&pitch))
            })
            .map(|(index, _)| index)
            .collect();
        let mut next = vec![INFINITY; states];
        for &candidate in &candidates {
            let deviation = (pitches[candidate] - note.pitch as i16).unsigned_abs() as u32 * 16;
            if i == 0 {
                next[candidate * 2] = deviation;
                continue;
            }
            for (previous, &cost) in costs.iter().enumerate() {
                if cost == INFINITY {
                    continue;
                }
                let prev_pitch = previous / 2 % width;
                let repeated = candidate == prev_pitch;
                if repeated && previous % 2 == 1 {
                    continue;
                }
                let interval = degrees[candidate] - degrees[prev_pitch];
                let large = usize::from(interval.abs() >= 4);
                let used = previous / (width * 2) + large;
                if used > budget {
                    continue;
                }
                let original_direction = (note.pitch as i16 - notes[i - 1].pitch as i16).signum();
                let reversed = interval != 0
                    && original_direction != 0
                    && interval.signum() != original_direction;
                let score = cost
                    + deviation
                    + 2 * (interval as i32).pow(2) as u32
                    + 64 * large as u32
                    + 6 * u32::from(repeated)
                    + 8 * u32::from(reversed);
                let state = (used * width + candidate) * 2 + usize::from(repeated);
                if score < next[state] {
                    next[state] = score;
                    predecessors[i][state] = previous;
                }
            }
        }
        costs = next;
    }

    let (mut state, &cost) = costs
        .iter()
        .enumerate()
        .min_by_key(|&(state, cost)| (*cost, state))
        .unwrap();
    assert!(cost < INFINITY, "no valid v2 melody path");
    for i in (0..notes.len()).rev() {
        notes[i].pitch = pitches[state / 2 % width] as u8;
        if i > 0 {
            state = predecessors[i][state];
        }
    }
}
