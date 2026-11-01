//! Private wasm bridge. The supported JavaScript API is wasm/seedsong-js.
use crate::dob::{self, DobOutput};
use crate::{
    backend::{MusicBackend, PcmAudio, web_audio::WebAudioBackend},
    dna::Dna,
    music::{MusicVersion, Params, Song},
    synth::{self, Options, Timbre},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::prelude::*;
use web_sys::{AudioBuffer, AudioContext};

/// Version of the JS decoder contract, separate from the music algorithm version.
#[wasm_bindgen(js_name = decoderAbiVersion)]
pub fn decoder_abi_version() -> u32 {
    1
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dob0Request {
    dna: String,
    pattern: Vec<Vec<Value>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dob1Request {
    traits: Vec<DobOutput>,
    pattern: Vec<Vec<Value>>,
}
fn dob_result(value: &impl Serialize) -> Result<JsValue, JsValue> {
    // Standard DOB output must be ordinary JS objects, not ES Maps. Unsafe
    // 64-bit numbers are rejected by serde-wasm-bindgen rather than rounded.
    value
        .serialize(&serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true))
        .map_err(error)
}
#[wasm_bindgen(js_name = decodeDob0)]
pub fn decode_dob0(request: JsValue) -> Result<JsValue, JsValue> {
    validate_keys(&request, &["dna", "pattern"])?;
    let request: Dob0Request = serde_wasm_bindgen::from_value(request).map_err(error)?;
    let dna = dob::parse_dna(&request.dna).map_err(error)?;
    dob_result(&dob::decode_dob0(&dna, &request.pattern).map_err(|e| error(format!("{e:#}")))?)
}
#[wasm_bindgen(js_name = decodeDob1)]
pub fn decode_dob1(request: JsValue) -> Result<JsValue, JsValue> {
    validate_keys(&request, &["traits", "pattern"])?;
    let request: Dob1Request = serde_wasm_bindgen::from_value(request).map_err(error)?;
    dob_result(&dob::decode_dob1(&request.traits, &request.pattern).map_err(error)?)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MusicResult {
    dna: String,
    music_version: u8,
    params: Params,
    events: Vec<crate::music::Event>,
    sources: [String; 4],
    #[serde(with = "serde_wasm_bindgen::preserve")]
    sng1: js_sys::Uint8Array,
}
/// Generate deterministic events and synthesis selectors without rendering PCM.
#[wasm_bindgen(js_name = generateMusic)]
pub fn generate_music(request: JsValue) -> Result<JsValue, JsValue> {
    validate_keys(&request, &["dna", "musicVersion", "participants"])?;
    let (song, _, dna) = RenderRequest::decode(request, Options::default().rate)?;
    let sng1 = js_sys::Uint8Array::from(song.encode().as_slice());
    dob_result(&MusicResult {
        dna,
        music_version: song.version as u8,
        sources: song.sources.map(|d| d.to_string()),
        params: song.params,
        events: song.events,
        sng1,
    })
}

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
    #[serde(default = "default_music_version")]
    music_version: u8,
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
fn default_music_version() -> u8 {
    MusicVersion::default() as u8
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
                "musicVersion",
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
        let version = MusicVersion::try_from(request.music_version).map_err(error)?;
        Ok((
            Song::generate_with_version(dna, participants, version),
            options,
            dna.to_string(),
        ))
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WavResult {
    #[serde(with = "serde_wasm_bindgen::preserve")]
    data: js_sys::Uint8Array,
    dna: String,
    music_version: u8,
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
        music_version: song.version as u8,
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
            r#""musicVersion":0"#,
            r#""musicVersion":3"#,
            r#""musicVersion":1.5"#,
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
