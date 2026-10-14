//! A deterministic in-crate PRNG: splitmix64 seeding and xorshift64* output.

/// One mixing step of splitmix64.
pub fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// Combine a seed, an input index, and a stream id into one seed.
pub fn mix(seed: u64, index: u64, stream: u64) -> u64 {
    splitmix(splitmix(seed ^ splitmix(index)) ^ stream.wrapping_mul(0x2545_f491_4f6c_dd1d))
}

/// Stream ids. Each use of randomness for one input has its own stream.
pub const STREAM_INPUT: u64 = 1;
/// The run choices of an input (limits, fixture results, arguments).
pub const STREAM_AUX: u64 = 2;
/// The witness sample choice.
pub const STREAM_WITNESS: u64 = 3;

/// The generator. The same seed gives the same sequence.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// A generator from a seed. A zero state is replaced by a constant.
    pub fn new(seed: u64) -> Self {
        let s = splitmix(seed);
        Self(if s == 0 { 0x1234_5678_9abc_def1 } else { s })
    }
    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    /// A value in `0..n`. Zero `n` gives zero.
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    /// A value in `lo..=hi`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi.saturating_sub(lo) + 1)
    }
    /// True with probability `1 / n`.
    pub fn one_in(&mut self, n: usize) -> bool {
        self.below(n) == 0
    }
    /// One element of a non-empty slice.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
    /// A binary64 value, special values included.
    pub fn number(&mut self) -> f64 {
        match self.below(12) {
            0 => f64::NAN,
            1 => -0.0,
            2 => f64::INFINITY,
            3 => f64::NEG_INFINITY,
            4 => f64::MAX,
            5 => f64::MIN_POSITIVE / 2.0,
            6 => 9_007_199_254_740_993.0,
            7 => f64::from_bits(self.next_u64()),
            _ => (self.below(2001) as f64) - 1000.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_seed_gives_same_sequence() {
        let mut a = Rng::new(mix(7, 3, STREAM_INPUT));
        let mut b = Rng::new(mix(7, 3, STREAM_INPUT));
        let mut c = Rng::new(mix(7, 4, STREAM_INPUT));
        let left: Vec<_> = (0..16).map(|_| a.next_u64()).collect();
        let right: Vec<_> = (0..16).map(|_| b.next_u64()).collect();
        let other: Vec<_> = (0..16).map(|_| c.next_u64()).collect();
        assert_eq!(left, right);
        assert_ne!(left, other);
        assert!((0..1000).all(|_| a.below(7) < 7));
    }
}
