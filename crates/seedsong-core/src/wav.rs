use anyhow::{Context, Result, ensure};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;
use std::io::Write;

pub(crate) fn write(mut file: impl Write, frames: &[[f32; 2]], rate: u32) -> Result<()> {
    let data_len = frames.len().checked_mul(4).context("WAV 过大")?;
    ensure!(
        data_len <= u32::MAX as usize - 36,
        "音频超过 WAV RIFF 的 4 GiB 限制"
    );
    file.write_all(b"RIFF")?;
    file.write_all(&(data_len as u32 + 36).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?;
    file.write_all(&rate.to_le_bytes())?;
    file.write_all(&(rate * 4).to_le_bytes())?;
    file.write_all(&4u16.to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&(data_len as u32).to_le_bytes())?;
    for block in frames.chunks(65536) {
        let bytes: Vec<[u8; 4]> = platform_iter!(block.par_iter(), block.iter())
            .map(|x| {
                let l = (x[0].clamp(-1., 1.) * 32767.).round() as i16;
                let r = (x[1].clamp(-1., 1.) * 32767.).round() as i16;
                let [a, b] = l.to_le_bytes();
                let [c, d] = r.to_le_bytes();
                [a, b, c, d]
            })
            .collect();
        for sample in bytes {
            file.write_all(&sample)?;
        }
    }
    file.flush()?;
    Ok(())
}
