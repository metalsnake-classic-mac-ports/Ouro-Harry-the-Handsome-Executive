//! The QuickDraw random generator the game uses (`Random`, through SAT's
//! `SATRand`); seeded from the clock in the original (`Randomize`).

/// QuickDraw `Random`: Park–Miller (16807, 2³¹ − 1) on `randSeed`; the
/// result is the low 16 bits, −32768 becomes 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Random {
    pub seed: u32,
}

impl Random {
    pub fn new(seed: u32) -> Random {
        Random { seed }
    }

    pub fn next(&mut self) -> i16 {
        self.seed = ((u64::from(self.seed) * 16807) % 0x7fff_ffff) as u32;
        let v = self.seed as u16;
        if v == 0x8000 { 0 } else { v as i16 }
    }

    /// `SATRand(n)` (PPC 0x10062dc8): |Random()| mod n.
    pub fn below(&mut self, n: i16) -> i16 {
        let r = self.next();
        let r = if r < 0 { r.wrapping_neg() } else { r };
        r % n
    }
}
