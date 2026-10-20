//! Native file I/O and ffmpeg conversion. Never compiled for wasm.
use super::{MusicBackend, PcmAudio};
use crate::{
    music::Song,
    synth::{self, Options},
};
use anyhow::{Context, Result, ensure};
use std::{
    fs::{self, File},
    io::BufWriter,
    path::Path,
};

#[derive(Default)]
pub struct NativeBackend;
impl MusicBackend for NativeBackend {
    type Output = PcmAudio;
    type Error = anyhow::Error;
    fn render(&mut self, song: &Song, options: Options) -> Result<PcmAudio> {
        options.validate()?;
        Ok(PcmAudio {
            frames: synth::render(song, options),
            rate: options.rate,
        })
    }
}
impl PcmAudio {
    pub fn write_wav(&self, path: &Path) -> Result<()> {
        write_wav(path, &self.frames, self.rate)
    }
}
pub fn write_wav(path: &Path, frames: &[[f32; 2]], rate: u32) -> Result<()> {
    ensure!(!path.exists(), "文件已存在：{}", path.display());
    let file = BufWriter::new(File::options().write(true).create_new(true).open(path)?);
    crate::wav::write(file, frames, rate)
}
pub fn export(wav: &Path, output: &Path) -> Result<()> {
    let ext = output
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let codec: &[&str] = match ext.as_str() {
        "flac" => &["-c:a", "flac"],
        "opus" | "ogg" => &["-c:a", "libopus", "-b:a", "128k"],
        "mp3" => &["-c:a", "libmp3lame", "-q:a", "2"],
        "m4a" | "aac" => &["-c:a", "aac", "-b:a", "192k"],
        _ => {
            anyhow::bail!("不支持的输出格式：{ext}，可用 wav / flac / opus / ogg / mp3 / m4a / aac")
        }
    };
    ensure!(!output.exists(), "文件已存在：{}", output.display());
    let result = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-n", "-i"])
        .arg(wav)
        .args(codec)
        .arg(output)
        .output()
        .context("无法调用 ffmpeg；WAV 已保留，请安装 ffmpeg 后转换")?;
    if !result.status.success() {
        anyhow::bail!(
            "ffmpeg 转换失败，WAV 已保留：{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let _ = fs::metadata(output).context("ffmpeg 没有生成输出文件")?;
    Ok(())
}
