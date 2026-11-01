//! Test-only adapter around the unmodified, commit-pinned official libraries.
//! Never call Seedsong here: this is the independent side of the comparison.
use anyhow::{Result, anyhow};
use serde_json::Value;

pub fn decode(dna: &str, dob0: &Value, dob1: &Value) -> Result<(Value, Value)> {
    let pattern0 = serde_json::to_vec(dob0)?;
    let args0 = vec![dna.as_bytes(), pattern0.as_slice()];
    let parameters = spore_dob_0::decoder::dobs_parse_parameters(args0)
        .map_err(|error| anyhow!("Official DOB/0 parameter error {}", error as u64))?;
    let traits = spore_dob_0::decoder::dobs_decode(parameters)
        .map_err(|error| anyhow!("Official DOB/0 decoding error {}", error as u64))?;
    let pattern1 = serde_json::to_vec(dob1)?;
    let args1 = vec![dna.as_bytes(), pattern1.as_slice(), traits.as_slice()];
    let parameters = spore_dob_1::decoder::dobs_parse_parameters(args1)
        .map_err(|error| anyhow!("Official DOB/1 parameter error {}", error as u64))?;
    let decoded = spore_dob_1::decoder::dobs_decode(parameters)
        .map_err(|error| anyhow!("Official DOB/1 decoding error {}", error as u64))?;
    Ok((
        serde_json::from_slice(&traits)?,
        serde_json::from_slice(&decoded)?,
    ))
}
