//! DOB/0 and DOB/1 pattern interpretation using the official decoder semantics.
//! SVG fragments are collection-provided markup; this module does not sanitize them.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Standard DOB output. Keep the vector form: duplicate names and image order matter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DobOutput {
    pub name: String,
    pub traits: Vec<BTreeMap<String, Value>>,
}
fn output(name: &str, kind: &str, value: Value) -> DobOutput {
    DobOutput {
        name: name.to_owned(),
        traits: vec![BTreeMap::from([(kind.to_owned(), value)])],
    }
}
fn text(value: &Value) -> Result<&str> {
    value.as_str().context("Expected a string")
}
fn uint(value: &Value) -> Result<u64> {
    value.as_u64().context("Expected an unsigned integer")
}
fn number(bytes: &[u8]) -> Result<u64> {
    ensure!(
        (1..=8).contains(&bytes.len()),
        "Numeric DNA segments must contain 1–8 bytes"
    );
    let mut buffer = [0; 8];
    buffer[..bytes.len()].copy_from_slice(bytes);
    Ok(u64::from_le_bytes(buffer))
}
/// Decode a nonempty, even-length hex string. DOB DNA need not be 32 bytes.
pub fn parse_dna(hex: &str) -> Result<Vec<u8>> {
    ensure!(
        !hex.is_empty() && hex.len().is_multiple_of(2) && hex.is_ascii(),
        "Expected nonempty hexadecimal DNA"
    );
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).context("Invalid hexadecimal DNA"))
        .collect()
}
/// Interpret DOB/0. Offsets are clamped to DNA bounds; integers are little-endian.
/// `range` excludes its upper bound. `options` retains arbitrary JSON values.
pub fn decode_dob0(dna: &[u8], pattern: &[Vec<Value>]) -> Result<Vec<DobOutput>> {
    ensure!(!dna.is_empty(), "DNA must not be empty");
    pattern
        .iter()
        .enumerate()
        .map(|(index, row)| -> Result<_> {
            let decode = || -> Result<_> {
                ensure!(row.len() >= 5, "DOB/0 entry needs at least five fields");
                let name = text(&row[0])?;
                let kind = text(&row[1])?;
                let offset = uint(&row[2])?.min(dna.len() as u64) as usize;
                let length = uint(&row[3])?.min((dna.len() - offset) as u64) as usize;
                let bytes = &dna[offset..offset + length];
                let args = row
                    .get(5)
                    .map(|v| v.as_array().context("DOB/0 arguments must be an array"))
                    .transpose()?;
                let value = match text(&row[4])? {
                    "rawNumber" => json!(number(bytes)?),
                    "rawString" => {
                        Value::String(bytes.iter().map(|b| format!("{b:02x}")).collect())
                    }
                    "utf8" => {
                        let end = bytes.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
                        Value::String(
                            std::str::from_utf8(&bytes[..end])
                                .context("Invalid UTF-8 DNA")?
                                .to_owned(),
                        )
                    }
                    "range" => {
                        let args = args.context("Missing range arguments")?;
                        ensure!(args.len() == 2, "Range requires two arguments");
                        let (lower, upper) = (uint(&args[0])?, uint(&args[1])?);
                        ensure!(upper > lower, "Range upper bound must exceed lower bound");
                        json!(lower + number(bytes)? % (upper - lower))
                    }
                    "options" => {
                        let args = args.context("Missing options")?;
                        ensure!(!args.is_empty(), "Options must not be empty");
                        args[(number(bytes)? % args.len() as u64) as usize].clone()
                    }
                    _ => bail!("Unknown DOB/0 pattern"),
                };
                Ok(output(name, kind, value))
            };
            decode().with_context(|| format!("DOB/0 entry {index}"))
        })
        .collect()
}
fn first_value<'a>(outputs: &'a [DobOutput], name: &str) -> Option<&'a Value> {
    outputs.iter().find_map(|o| {
        if o.name == name {
            o.traits.first()?.first_key_value().map(|(_, v)| v)
        } else {
            None
        }
    })
}
fn select(args: &Value, value: &Value) -> Result<Option<String>> {
    for branch in args.as_array().context("DOB/1 options must be an array")? {
        let branch = branch.as_array().context("Invalid DOB/1 option")?;
        ensure!(
            branch.len() >= 2,
            "DOB/1 option needs condition and fragment"
        );
        let fragment = text(&branch[1])?;
        let matches = match &branch[0] {
            Value::Number(_) => uint(value)? == uint(&branch[0])?,
            Value::String(condition) => text(value)? == condition,
            Value::Array(range) => {
                if range.first().and_then(Value::as_str) == Some("*") {
                    true
                } else {
                    ensure!(range.len() == 2, "DOB/1 range needs two bounds");
                    let (start, end, value) = (uint(&range[0])?, uint(&range[1])?, uint(value)?);
                    start <= value && value <= end
                }
            }
            _ => bail!("Invalid DOB/1 condition"),
        };
        if matches {
            return Ok(Some(fragment.to_owned()));
        }
    }
    Ok(None)
}
/// Interpret DOB/1, preserving public DOB/0 traits and appending SVG traits.
/// Consecutive equal image names form one SVG. Missing named traits end that
/// image group; unmatched options omit a fragment, as in the official decoder.
pub fn decode_dob1(traits: &[DobOutput], pattern: &[Vec<Value>]) -> Result<Vec<DobOutput>> {
    // Validate schema before rendering, including entries after a missing trait.
    for row in pattern {
        ensure!(row.len() >= 5, "DOB/1 entry needs at least five fields");
        text(&row[0])?;
        ensure!(
            matches!(text(&row[1])?, "attributes" | "elements"),
            "Invalid SVG field"
        );
        text(&row[2])?;
        ensure!(
            matches!(text(&row[3])?, "raw" | "range" | "options"),
            "Unknown DOB/1 pattern"
        );
    }
    let mut result: Vec<_> = traits
        .iter()
        .filter(|t| !t.name.starts_with('_'))
        .cloned()
        .collect();
    for group in pattern.chunk_by(|a, b| a[0] == b[0]) {
        let name = text(&group[0][0])?;
        let mut attributes = String::new();
        let mut elements = String::new();
        for row in group {
            let trait_name = text(&row[2])?;
            let kind = text(&row[3])?;
            let fragment = if let Some(value) = first_value(traits, trait_name) {
                if kind == "raw" {
                    text(&row[4])?.replace("{value}", text(value)?)
                } else if let Some(fragment) = select(&row[4], value)? {
                    fragment
                } else {
                    continue;
                }
            } else {
                if !trait_name.is_empty() {
                    break;
                }
                ensure!(kind == "raw", "Unnamed DOB/1 entries must use raw");
                text(&row[4])?.to_owned()
            };
            if text(&row[1])? == "attributes" {
                attributes.push(' ');
                attributes.push_str(&fragment);
            } else {
                elements.push_str(&fragment);
            }
        }
        result.push(output(
            name.split('.').next().unwrap_or(name),
            "SVG",
            json!(format!("<svg{attributes}>{elements}</svg>")),
        ));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pattern(value: Value) -> Vec<Vec<Value>> {
        serde_json::from_value(value).unwrap()
    }
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn dob0_little_endian_clamping_utf8_and_ranges() {
        let rows = pattern(json!([
            ["n", "Number", 0, 2, "rawNumber"],
            ["r", "Number", 0, 1, "range", [60, 141]],
            ["o", "String", 0, 2, "options", ["A", "B", "C"]],
            ["s", "String", 1, 99, "rawString"],
            ["u", "String", 2, 99, "utf8"],
            ["empty", "String", 999, 1, "rawString"]
        ]));
        let decoded = decode_dob0(&[0xff, 1, b'h', b'i', 0, 0], &rows).unwrap();
        assert_eq!(decoded[0], output("n", "Number", json!(511)));
        assert_eq!(decoded[1], output("r", "Number", json!(72)));
        assert_eq!(decoded[2], output("o", "String", json!("B")));
        assert_eq!(decoded[3], output("s", "String", json!("0168690000")));
        assert_eq!(decoded[4], output("u", "String", json!("hi")));
        assert_eq!(decoded[5], output("empty", "String", json!("")));
        for rows in [
            json!([["x", "Number", 0, 1, "range", [1, 1]]]),
            json!([["x", "Number", 99, 1, "rawNumber"]]),
            json!([["x", "String", 0, 1, "utf8"]]),
            json!([["x", "String", 0, 1, "options", []]]),
        ] {
            assert!(decode_dob0(&[255], &pattern(rows)).is_err());
        }
    }
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn dob1_templates_grouping_hidden_traits_and_skip_semantics() {
        let traits = vec![
            output("_color", "String", json!("red")),
            output("Tempo", "Number", json!(79)),
        ];
        let rows = pattern(json!([
            [
                "IMAGE.0",
                "attributes",
                "",
                "raw",
                "xmlns='http://www.w3.org/2000/svg'"
            ],
            [
                "IMAGE.0",
                "elements",
                "_color",
                "raw",
                "<g fill='{value}'/>"
            ],
            [
                "IMAGE.0",
                "elements",
                "Tempo",
                "range",
                [[[60, 79], "<a/>"], [["*"], "<b/>"]]
            ],
            ["IMAGE.0", "elements", "Tempo", "options", [[80, "unused"]]],
            ["IMAGE.0", "elements", "missing", "raw", "ignored"],
            ["IMAGE.0", "elements", "", "raw", "also ignored"],
            ["IMAGE.1", "elements", "Tempo", "options", [[["*"], "<c/>"]]],
            ["IMAGE.0", "elements", "", "raw", "<d/>"]
        ]));
        let result = decode_dob1(&traits, &rows).unwrap();
        assert_eq!(
            result,
            vec![
                traits[1].clone(),
                output(
                    "IMAGE",
                    "SVG",
                    json!("<svg xmlns='http://www.w3.org/2000/svg'><g fill='red'/><a/></svg>")
                ),
                output("IMAGE", "SVG", json!("<svg><c/></svg>")),
                output("IMAGE", "SVG", json!("<svg><d/></svg>"))
            ]
        );
        assert!(
            decode_dob1(
                &traits,
                &pattern(json!([["IMAGE.0", "elements", "Tempo", "raw", "{value}"]]))
            )
            .is_err()
        );
        assert!(
            decode_dob1(
                &traits,
                &pattern(json!([["IMAGE.0", "invalid", "", "raw", ""]]))
            )
            .is_err()
        );
    }
}
