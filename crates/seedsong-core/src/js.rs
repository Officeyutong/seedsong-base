//! Private wasm bridge. The supported JavaScript API is wasm/seedsong-js.
use crate::{
    backend::{MusicBackend, PcmAudio, web_audio::WebAudioBackend},
    dna::Dna,
    music::{Params, Song},
    synth::{self, Options, Timbre},
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{AudioBuffer, AudioContext};

#[derive(Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum JsTimbre {
    #[default]
    Spec,
    Piano,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Participants {
    melody: Option<String>,
    bass: Option<String>,
    pad: Option<String>,
    drums: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RenderRequest {
    dna: String,
    #[serde(default)]
    timbre: JsTimbre,
    sample_rate: Option<u32>,
    #[serde(default, rename = "loop")]
    looping: bool,
    #[serde(default = "default_reverb")]
    reverb: f64,
    #[serde(default)]
    participants: Participants,
}
fn default_reverb() -> f64 {
    Options::default().reverb
}
fn error(error: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}
// serde-wasm-bindgen reads known struct fields directly, so deny_unknown_fields
// alone does not inspect extra JS properties. Check enumerable keys explicitly.
fn validate_keys(value: &JsValue, allowed: &[&str]) -> Result<(), JsValue> {
    if !value.is_object() || value.is_null() || js_sys::Array::is_array(value) {
        return Err(error("Expected an options object"));
    }
    for key in js_sys::Object::keys(value.unchecked_ref::<js_sys::Object>()) {
        let key = key
            .as_string()
            .ok_or_else(|| error("Expected a string property name"))?;
        if !allowed.contains(&key.as_str()) {
            return Err(error(format!("Unknown option: {key}")));
        }
    }
    Ok(())
}
impl RenderRequest {
    fn decode(value: JsValue, default_rate: u32) -> Result<(Song, Options, String), JsValue> {
        validate_keys(
            &value,
            &[
                "dna",
                "timbre",
                "sampleRate",
                "loop",
                "reverb",
                "participants",
            ],
        )?;
        let participants = js_sys::Reflect::get(&value, &JsValue::from_str("participants"))?;
        if !participants.is_undefined() {
            validate_keys(&participants, &["melody", "bass", "pad", "drums"])?;
        }
        let request: Self = serde_wasm_bindgen::from_value(value).map_err(error)?;
        let options = Options {
            rate: request.sample_rate.unwrap_or(default_rate),
            timbre: match request.timbre {
                JsTimbre::Spec => Timbre::Spec,
                JsTimbre::Piano => Timbre::Piano,
            },
            looping: request.looping,
            reverb: request.reverb,
        };
        options.validate().map_err(error)?;
        let dna = Dna::parse(&request.dna).map_err(error)?;
        let role = Params::new(&dna).role as usize;
        let mut participants = [None; 4];
        for (part, source) in [
            request.participants.melody,
            request.participants.bass,
            request.participants.pad,
            request.participants.drums,
        ]
        .into_iter()
        .enumerate()
        {
            if let Some(source) = source {
                if part == role {
                    return Err(error("The conductor's own part cannot be replaced"));
                }
                let participant = Dna::parse(&source).map_err(error)?;
                if Params::new(&participant).role as usize != part {
                    return Err(error(format!(
                        "Participant DNA role does not match {}",
                        ["melody", "bass", "pad", "drums"][part]
                    )));
                }
                participants[part] = Some(participant);
            }
        }
        Ok((Song::generate(dna, participants), options, dna.to_string()))
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WavResult {
    #[serde(with = "serde_wasm_bindgen::preserve")]
    data: js_sys::Uint8Array,
    dna: String,
    sample_rate: u32,
    channels: u8,
    duration_seconds: f64,
}
/// Decode a structured JS request and return stereo PCM16 WAV with metadata.
#[wasm_bindgen(js_name = renderWav)]
pub fn render_wav(request: JsValue) -> Result<JsValue, JsValue> {
    let (song, options, dna) = RenderRequest::decode(request, Options::default().rate)?;
    let audio = PcmAudio {
        frames: synth::render(&song, options),
        rate: options.rate,
    };
    let result = WavResult {
        data: js_sys::Uint8Array::from(audio.to_wav().map_err(error)?.as_slice()),
        dna,
        sample_rate: audio.rate,
        channels: 2,
        duration_seconds: audio.frames.len() as f64 / audio.rate as f64,
    };
    serde_wasm_bindgen::to_value(&result).map_err(error)
}
/// Use a caller-owned AudioContext; the bridge never resumes or closes it.
#[wasm_bindgen(js_name = renderAudioBuffer)]
pub fn render_audio_buffer(
    context: AudioContext,
    request: JsValue,
) -> Result<AudioBuffer, JsValue> {
    let (song, options, _) = RenderRequest::decode(request, context.sample_rate() as u32)?;
    WebAudioBackend::from_context(context).render(&song, options)
}
#[wasm_bindgen(js_name = randomDna)]
pub fn random_dna() -> String {
    Dna::random().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn structured_options_and_defaults() {
        let value = js_sys::JSON::parse(&format!(
            r#"{{"dna":"{}","timbre":"piano","loop":true,"sampleRate":22050}}"#,
            "00".repeat(32)
        ))
        .unwrap();
        let (song, options, dna) = RenderRequest::decode(value, 44100).unwrap();
        assert!(!song.events.is_empty());
        assert_eq!(options.timbre, Timbre::Piano);
        assert_eq!(options.rate, 22050);
        assert!(options.looping);
        assert_eq!(options.reverb, 0.18);
        assert_eq!(dna, "00".repeat(32));
    }
    #[wasm_bindgen_test]
    fn structured_options_reject_typos_and_wrong_types() {
        for field in [
            r#""timbre":"unknown""#,
            r#""sampleRate":22050.5"#,
            r#""reverb":2"#,
            r#""loop":"true""#,
            r#""sample_rate":44100"#,
            r#""participants":{"unknown":"00"}"#,
        ] {
            let value = js_sys::JSON::parse(&format!(r#"{{"dna":"{}",{field}}}"#, "00".repeat(32)))
                .unwrap();
            assert!(
                RenderRequest::decode(value, 44100).is_err(),
                "accepted {field}"
            );
        }
    }
    #[wasm_bindgen_test]
    fn structured_ensemble_rejects_incorrect_roles() {
        for role in ["melody", "bass"] {
            let value = js_sys::JSON::parse(&format!(
                r#"{{"dna":"{}","participants":{{"{role}":"{}"}}}}"#,
                "00".repeat(32),
                "00".repeat(32)
            ))
            .unwrap();
            assert!(RenderRequest::decode(value, 44100).is_err());
        }
    }
    #[wasm_bindgen_test]
    async fn structured_request_renders_into_a_caller_owned_context() {
        let context = AudioContext::new().unwrap();
        let value = js_sys::JSON::parse(&format!(
            r#"{{"dna":"{}","sampleRate":22050,"timbre":"piano","loop":true}}"#,
            "9a401c07e8553fa2710c0000000000000209b62d000000000000000000000000"
        ))
        .unwrap();
        let buffer = render_audio_buffer(context.clone(), value).unwrap();
        assert_eq!(buffer.number_of_channels(), 2);
        assert_eq!(buffer.sample_rate(), 22050.);
        let expected = (256.0_f64 * 60. / 124. / 4. * 22050.).round() as u32;
        assert_eq!(buffer.length(), expected);
        // Context lifecycle still belongs to the caller.
        wasm_bindgen_futures::JsFuture::from(context.close().unwrap())
            .await
            .unwrap();
    }
}
