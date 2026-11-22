//! A tiny seeded random number generator (xorshift64*), so the simulation is deterministic and
//! needs no crate: the same seed always gives the same sequence on every platform.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // xorshift must never start from zero; mix the seed so that small seeds differ a lot.
        let s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
        Self(if s == 0 { 0x2545_F491_4F6C_DD1D } else { s })
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `0..n` (`n` must be > 0; the tiny modulo bias is irrelevant for a game).
    pub fn below(&mut self, n: u32) -> u32 {
        debug_assert!(n > 0);
        ((self.next_u64() >> 32) % u64::from(n.max(1))) as u32
    }

    /// A whole number in `lo..=hi` (`lo <= hi`).
    #[allow(dead_code)] // used by the level generator
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        lo + self.below((hi - lo + 1) as u32) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        assert!((0..100).all(|_| a.next_u64() == b.next_u64()));
    }

    #[test]
    fn different_seeds_differ() {
        assert_ne!(Rng::new(1).next_u64(), Rng::new(2).next_u64());
        // Seed 0 works (does not get stuck at zero).
        let mut z = Rng::new(0);
        assert_ne!(z.next_u64(), z.next_u64());
    }

    #[test]
    fn range_is_inclusive() {
        let mut r = Rng::new(3);
        let v: Vec<i32> = (0..300).map(|_| r.range(-2, 2)).collect();
        assert!(v.iter().all(|x| (-2..=2).contains(x)));
        assert!([-2, 2].iter().all(|e| v.contains(e)));
        assert_eq!(r.range(5, 5), 5);
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut r = Rng::new(7);
        let mut seen = [false; 5];
        for _ in 0..500 {
            let v = r.below(5);
            assert!(v < 5);
            seen[v as usize] = true;
        }
        assert!(seen.iter().all(|&s| s));
    }
}
