//! JSON stdin/stdout adapter for the Node WASM parity unit tests.
//! Kept as a native example: neither upstream crate enters production WASM.
#[cfg(not(target_arch = "wasm32"))]
#[path = "../tests/support/official_dob.rs"]
mod official;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    use serde::Deserialize;
    use serde_json::{Value, json};
    use std::io::{self, Write};
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request {
        dnas: Vec<String>,
        dob0: Value,
        dob1: Value,
    }
    let request: Request = serde_json::from_reader(io::stdin().lock())?;
    let mut output = io::BufWriter::new(io::stdout().lock());
    for dna in request.dnas {
        let (dob0, dob1) = official::decode(&dna, &request.dob0, &request.dob1)?;
        serde_json::to_writer(
            &mut output,
            &json!({ "dna": dna, "dob0": dob0, "dob1": dob1 }),
        )?;
        writeln!(output)?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
