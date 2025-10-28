/// Discrete simulation tick in Chronos time.
///
/// The engine advances in integer ticks with a fixed `dt` per branch. This
/// newtype ensures explicit tick passing across APIs.
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick {
    index: u64,
}

impl Tick {
    /// Creates a new tick with the given index.
    #[must_use]
    pub const fn new(index: u64) -> Self {
        Self { index }
    }

    /// Returns the tick index.
    #[must_use]
    pub const fn index(&self) -> u64 {
        self.index
    }
}

impl From<u64> for Tick {
    fn from(value: u64) -> Self {
        Self { index: value }
    }
}

impl From<Tick> for u64 {
    fn from(t: Tick) -> Self {
        t.index
    }
}
