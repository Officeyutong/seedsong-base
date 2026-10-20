use crate::{
    music::{Event, Params, Song},
    rng::Stream,
};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use std::{f64::consts::TAU, sync::OnceLock};

const TABLE_SIZE: usize = 16384;
fn sine(phase: f64) -> f64 {
    static TABLE: OnceLock<Vec<f64>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        (0..=TABLE_SIZE)
            .map(|i| (TAU * i as f64 / TABLE_SIZE as f64).sin())
            .collect()
    });
    let x = phase.rem_euclid(1.0) * TABLE_SIZE as f64;
    let i = x as usize;
    table[i] + (table[i + 1] - table[i]) * (x - i as f64)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Timbre {
    Spec,
    Piano,
}
#[derive(Clone, Copy)]
pub struct Options {
    pub rate: u32,
    pub timbre: Timbre,
    pub looping: bool,
    pub reverb: f64,
}
struct Voice {
    start: usize,
    samples: Vec<f32>,
    left: f32,
    right: f32,
}
#[derive(Clone, Copy)]
struct Envelope {
    attack: f64,
    decay: f64,
    sustain: f64,
    release: f64,
}
impl Envelope {
    fn at(self, t: f64, gate: f64) -> f64 {
        let held = |x: f64| {
            if x < self.attack {
                x / self.attack
            } else if self.decay > 0.0 {
                let d = ((x - self.attack) / self.decay).clamp(0.0, 1.0);
                1.0 + (self.sustain - 1.0) * d
            } else {
                self.sustain
            }
        };
        if t <= gate {
            held(t)
        } else {
            held(gate) * (1.0 - (t - gate) / self.release).max(0.0)
        }
    }
}
fn envelope(e: Event, p: &Params) -> Envelope {
    let a = match e.part {
        0 => match p.lead {
            0 => [10., 300., 600., 200.],
            1 => [15., 50., 900., 120.],
            2 => [5., 900., 300., 300.],
            3 => [2., 1800., 0., 600.],
            4 => [2., 700., 0., 300.],
            _ => [2., 2500., 0., 800.],
        },
        1 => match p.bass {
            0 => [5., 100., 900., 100.],
            1 => [5., 200., 700., 100.],
            _ => [2., 400., 200., 100.],
        },
        _ => [400., 0., 1000., 800.],
    };
    Envelope {
        attack: a[0] / 1000.,
        decay: a[1] / 1000.,
        sustain: a[2] / 1000.,
        release: a[3] / 1000.,
    }
}
fn noise(song: &Song, part: usize, rate: u32) -> Vec<[f64; 2]> {
    let mut rng = Stream::new("sng/noise", &song.sources[part].0[20..32]);
    let raw: Vec<f64> = (0..44100)
        .map(|_| (rng.next_u64() >> 48) as u16 as i16 as f64 / 32768.)
        .collect();
    let mut hp7 = 0.;
    let mut hp1 = 0.;
    let mut prev = 0.;
    let mut band = 0.;
    let high = |hz: f64| 1. / (1. + TAU * hz / rate as f64);
    (0..rate as usize)
        .map(|i| {
            let x = i as f64 * 44100. / rate as f64;
            let j = x as usize % 44100;
            let v = raw[j] + (raw[(j + 1) % 44100] - raw[j]) * x.fract();
            hp7 = high(7000.) * (hp7 + v - prev);
            hp1 = high(1200.) * (hp1 + v - prev);
            band += (1. - (-TAU * 2400. / rate as f64).exp()) * (hp1 - band);
            prev = v;
            [band, hp7]
        })
        .collect()
}
fn render_voice(e: Event, song: &Song, opt: Options, noise: &[[f64; 2]]) -> Voice {
    let p = Params::new(&song.sources[e.part as usize]);
    let step = 60. / song.params.bpm as f64 / 4.;
    let swing = if e.start % 4 == 2 {
        song.params.swing as f64 / 100. * step
    } else {
        0.
    };
    let start = ((e.start as f64 * step + swing) * opt.rate as f64).round() as usize;
    let gate = e.duration as f64 * step;
    let env = envelope(e, &p);
    let piano = e.part == 0 && matches!(opt.timbre, Timbre::Piano);
    let length = if e.part == 3 {
        match e.pitch {
            36 => 0.65,
            38 => 0.4,
            _ => 0.18,
        }
    } else {
        gate + if piano { 1.2 } else { env.release }
    };
    let mut samples = vec![0f32; (length * opt.rate as f64).ceil() as usize];
    let freq = 440. * 2f64.powf((e.pitch as f64 - 69.) / 12.);
    let nyquist = opt.rate as f64 * 0.47;
    if piano {
        // Frequency-dependent stiffness and double decay, interpolated across registers.
        let x = ((e.pitch as f64 - 36.) / 60.).clamp(0., 1.);
        let stiffness = 0.00004 + (0.0012 - 0.00004) * x * x;
        let decay = 4.8 + (0.85 - 4.8) * x;
        let brightness = 0.55 + e.velocity as f64 / 127. * 0.65;
        for partial in 1..=24 {
            let k = partial as f64;
            let f = freq * k * ((1. + stiffness * k * k) / (1. + stiffness)).sqrt();
            if f > nyquist {
                break;
            }
            let amplitude = k.powf(-1.35) * (-k / (10. * brightness)).exp();
            let tau = decay / (1. + 0.22 * (k - 1.));
            let fast = (-1. / (opt.rate as f64 * tau * 0.35)).exp();
            let slow = (-1. / (opt.rate as f64 * tau)).exp();
            let release = (-1. / (opt.rate as f64 * 0.18)).exp();
            let mut a = 0.72 * amplitude;
            let mut b = 0.28 * amplitude;
            let mut phase = 0.;
            let mut detuned = 0.;
            let detune = if e.pitch < 48 { 0.00025 } else { 0.00075 };
            for (i, sample) in samples.iter_mut().enumerate() {
                let t = i as f64 / opt.rate as f64;
                let attack = (t / 0.003).min(1.);
                let pair = if partial <= 8 {
                    0.78 * sine(phase) + 0.22 * sine(detuned)
                } else {
                    sine(phase)
                };
                *sample += (pair * (a + b) * attack) as f32;
                phase = (phase + f / opt.rate as f64).fract();
                detuned = (detuned + f * (1. + detune) / opt.rate as f64).fract();
                a *= fast;
                b *= slow;
                if t > gate {
                    a *= release;
                    b *= release;
                }
            }
        }
        for (i, sample) in samples
            .iter_mut()
            .take((opt.rate as f64 * 0.035) as usize)
            .enumerate()
        {
            let t = i as f64 / opt.rate as f64;
            *sample += (noise[i % noise.len()][0] * 0.045 * (-t / 0.008).exp()) as f32;
        }
    } else if e.part == 3 {
        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f64 / opt.rate as f64;
            *sample = match e.pitch {
                36 => {
                    // Integral of the exponential pitch sweep (phase stays continuous).
                    let a = (3f64).ln() / 0.12;
                    let phase = if t <= 0.12 {
                        150. / a * (1. - (-a * t).exp())
                    } else {
                        150. / a * (1. - (-a * 0.12).exp()) + 50. * (t - 0.12)
                    };
                    (sine(phase) * (-t / 0.16).exp() * (t / 0.002).min(1.)) as f32
                }
                38 => {
                    ((noise[i % noise.len()][0] * 1.7 + 0.23 * sine(180. * t))
                        * (-t / 0.065).exp()
                        * (t / 0.001).min(1.)) as f32
                }
                _ => (noise[i % noise.len()][1] * (-t / 0.022).exp() * (t / 0.0005).min(1.)) as f32,
            };
        }
    } else {
        let partials: Vec<(f64, f64)> = match (e.part, p.lead, p.bass) {
            (0, 0, _) => vec![(1., 1.), (2., 0.25), (3., 0.08)],
            (0, 1, _) => [1., 0.5, 0.33, 0.25, 0.2, 0.16]
                .into_iter()
                .enumerate()
                .map(|(i, a)| ((i + 1) as f64, a))
                .collect(),
            (0, 5, _) => vec![(1., 1.), (2.756, 0.5), (5.404, 0.25), (8.933, 0.12)],
            (1, _, 0) => vec![(1., 1.), (2., 0.15)],
            (2, _, _) => vec![(1., 1.), (2., 0.4), (3., 0.2)],
            _ => Vec::new(),
        }
        .into_iter()
        .filter(|(ratio, _)| freq * ratio < nyquist)
        .collect();
        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f64 / opt.rate as f64;
            let carrier = freq * t;
            let value = if partials.is_empty() {
                let (ratio, index) = match e.part {
                    0 => match p.lead {
                        2 => (1., 2. - 1.7 * (t / 0.8).min(1.)),
                        3 => (3.5, 3. * (1. - (t / 1.5).min(1.))),
                        _ => (4., 1.5 * (1. - (t / 0.4).min(1.))),
                    },
                    _ => {
                        if p.bass == 1 {
                            (1., 1.)
                        } else {
                            (2., 2.5 - 2.3 * (t / 0.3).min(1.))
                        }
                    }
                };
                // Limit FM bandwidth near Nyquist by reducing modulation depth.
                let index = index * ((nyquist / freq - 1.) / ratio).clamp(0., 1.);
                sine(carrier + index / TAU * sine(carrier * ratio))
            } else {
                partials
                    .iter()
                    .map(|&(r, a)| a * sine(carrier * r))
                    .sum::<f64>()
            };
            *sample = (value * env.at(t, gate)) as f32;
        }
    }
    // End every finite voice at zero, including percussion and truncated piano tails.
    let fade = (opt.rate as usize / 200).min(samples.len());
    let len = samples.len();
    for (j, s) in samples[len - fade..].iter_mut().enumerate() {
        *s *= (fade - 1 - j) as f32 / fade as f32;
    }
    let gain = [1., 0.8, 0.45, 0.7][e.part as usize]
        * if e.part == song.params.role {
            10f64.powf(3. / 20.)
        } else {
            1.
        };
    let gain = (gain * e.velocity as f64 / 127. * 0.25) as f32;
    let pan = match e.part {
        0 => ((e.pitch as f64 - 64.) / 100.).clamp(-0.25, 0.25),
        1 => 0.,
        2 => ((e.pitch as f64 - 64.) / 45.).clamp(-0.6, 0.6),
        _ => {
            if e.pitch == 42 {
                0.25
            } else {
                -0.05
            }
        }
    };
    let angle = (pan + 1.) * std::f64::consts::FRAC_PI_4;
    Voice {
        start,
        samples,
        left: gain * angle.cos() as f32,
        right: gain * angle.sin() as f32,
    }
}
pub fn render(song: &Song, opt: Options) -> Vec<[f32; 2]> {
    let noise = noise(song, 3, opt.rate);
    let melody_noise = noise_for_melody(song, opt.rate);
    let voices: Vec<_> = platform_iter!(song.events.par_iter(), song.events.iter())
        .map(|&e| {
            render_voice(
                e,
                song,
                opt,
                if e.part == 0 { &melody_noise } else { &noise },
            )
        })
        .collect();
    let song_frames = (256. * 60. / song.params.bpm as f64 / 4. * opt.rate as f64).round() as usize;
    let frames = if opt.looping {
        song_frames
    } else {
        voices
            .iter()
            .map(|v| v.start + v.samples.len())
            .max()
            .unwrap_or(song_frames)
            + opt.rate as usize / 2
    };
    const BLOCK: usize = 4096;
    // Bounded memory: disjoint output chunks, stable event order, no locked shared mixer.
    let mut out = vec![[0f32; 2]; frames];
    platform_iter!(out.par_chunks_mut(BLOCK), out.chunks_mut(BLOCK))
        .enumerate()
        .for_each(|(block, chunk)| {
            let begin = block * BLOCK;
            let end = begin + chunk.len();
            for v in &voices {
                let count = if opt.looping { 2 } else { 1 };
                for wrap in 0..count {
                    let shifted = begin + wrap * frames;
                    let upper = end + wrap * frames;
                    let lo = shifted.max(v.start);
                    let hi = upper.min(v.start + v.samples.len());
                    if lo >= hi {
                        continue;
                    }
                    for (dst, &sample) in chunk[lo - shifted..hi - shifted]
                        .iter_mut()
                        .zip(&v.samples[lo - v.start..hi - v.start])
                    {
                        dst[0] += sample * v.left;
                        dst[1] += sample * v.right;
                    }
                }
            }
        });
    if opt.reverb > 0. {
        let dry = &out;
        out = platform_iter!((0..frames).into_par_iter(), 0..frames)
            .map(|i| {
                let mut x = dry[i];
                for (j, delay) in [0.071, 0.113, 0.173, 0.233, 0.347].iter().enumerate() {
                    let d = (delay * opt.rate as f64) as usize;
                    let source = if i >= d {
                        Some(i - d)
                    } else if opt.looping {
                        Some((i + frames - d % frames) % frames)
                    } else {
                        None
                    };
                    if let Some(source) = source {
                        let a = opt.reverb * (0.32 / (1. + j as f64 * 0.5));
                        x[0] += dry[source][j % 2] * a as f32;
                        x[1] += dry[source][1 - j % 2] * a as f32;
                    }
                }
                x
            })
            .collect();
    }
    let peaks = platform_iter!(out.par_iter(), out.iter()).map(|x| x[0].abs().max(x[1].abs()));
    #[cfg(not(target_arch = "wasm32"))]
    let peak = peaks.reduce(|| 0f32, f32::max);
    #[cfg(target_arch = "wasm32")]
    let peak = peaks.fold(0f32, f32::max);
    let gain = if peak > 0. { 0.95 / peak } else { 1. };
    platform_iter!(out.par_iter_mut(), out.iter_mut()).for_each(|x| {
        x[0] *= gain;
        x[1] *= gain;
    });
    out
}
fn noise_for_melody(song: &Song, rate: u32) -> Vec<[f64; 2]> {
    noise(song, 0, rate)
}

impl Default for Options {
    fn default() -> Self {
        Self {
            rate: 44100,
            timbre: Timbre::Spec,
            looping: false,
            reverb: 0.18,
        }
    }
}
impl Options {
    /// Validate before allocating audio buffers; both backends share this contract.
    pub fn validate(self) -> anyhow::Result<()> {
        anyhow::ensure!(
            (22050..=96000).contains(&self.rate),
            "sample rate must be 22050–96000 Hz"
        );
        anyhow::ensure!(
            self.reverb.is_finite() && (0.0..=1.0).contains(&self.reverb),
            "reverb must be finite and between 0 and 1"
        );
        Ok(())
    }
}
