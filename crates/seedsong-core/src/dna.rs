use anyhow::{Result, bail};
use rand::RngCore;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dna(pub [u8; 32]);
impl Dna {
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();
        let s = s
            .strip_prefix("0x")
            .or_else(|| s.strip_prefix("0X"))
            .unwrap_or(s);
        let s: String = s
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
            .collect();
        if s.len() != 64 || !s.is_ascii() {
            bail!("DNA 必须是 32 字节，即 64 个十六进制字符（可带 0x、空格或分隔线）");
        }
        let mut bytes = [0; 32];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
                .map_err(|_| anyhow::anyhow!("DNA 第 {} 字节不是十六进制数", i + 1))?;
        }
        Ok(Self(bytes))
    }
    pub fn random() -> Self {
        let mut b = [0; 32];
        rand::rngs::OsRng.fill_bytes(&mut b);
        Self(b)
    }
}
impl fmt::Display for Dna {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn parse_roundtrip_and_reject_invalid() {
        let dna = Dna([0xab; 32]);
        assert_eq!(Dna::parse(&format!("0x{dna}")).unwrap(), dna);
        for s in ["é", "zz", "", &"z".repeat(64), &"a".repeat(63)] {
            assert!(Dna::parse(s).is_err());
        }
    }
}
