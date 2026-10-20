/// Independent, integer-only streams specified by seedsong-music/1.
#[derive(Clone)]
pub struct Stream(u64);
impl Stream {
    pub fn new(label: &str, bytes: &[u8]) -> Self {
        let hash = label
            .bytes()
            .chain(bytes.iter().copied())
            .fold(0xcbf29ce484222325u64, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
            });
        let mut z = hash.wrapping_add(0x9e3779b97f4a7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^= z >> 31;
        Self(if z == 0 { 0x9e3779b97f4a7c15 } else { z })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545f4914f6cdd1d)
    }
    pub fn below(&mut self, n: u32) -> u32 {
        (((self.next_u64() >> 32) * u64::from(n)) >> 32) as u32
    }
    pub fn chance(&mut self, a: u32, b: u32) -> bool {
        self.below(b) < a
    }
    pub fn weighted(&mut self, weights: &[u32]) -> usize {
        let mut r = self.below(weights.iter().sum());
        for (i, &w) in weights.iter().enumerate() {
            if r < w {
                return i;
            }
            r -= w;
        }
        unreachable!()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn published_xorshift_vector() {
        let mut r = Stream(1);
        assert_eq!(r.next_u64(), 0x47e4ce4b896cdd1d);
        assert_eq!(r.0, 0x2000001);
    }
}
