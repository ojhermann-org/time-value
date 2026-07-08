//! Periodicity markers — the zero-sized type tag that keeps a [`Rate`] and the
//! [`Cashflows`] it discounts on the same clock.
//!
//! See `docs/adr/0005-domain-modelling-and-strong-typing.md`. Applying a rate of
//! one periodicity to cashflows of another is the headline TVM bug; because the
//! periodicity is part of the type, that mismatch is a compile error.
//!
//! [`Rate`]: crate::Rate
//! [`Cashflows`]: crate::Cashflows

use core::fmt;

mod sealed {
    pub trait Sealed {}
}

/// A compounding / cashflow frequency, used as a zero-sized type tag on
/// [`Rate<P>`](crate::Rate) and [`Cashflows<P>`](crate::Cashflows).
///
/// This trait is **sealed**: the set of periodicities is fixed by this crate and
/// cannot be extended downstream.
pub trait Periodicity: sealed::Sealed + Copy + fmt::Debug {
    /// The number of periods of this frequency in one year (e.g. `12` for
    /// [`Monthly`]).
    const PERIODS_PER_YEAR: u16;
    /// A lower-case human-readable name (e.g. `"monthly"`).
    const NAME: &'static str;
}

macro_rules! periodicity {
    ($(#[$doc:meta])* $name:ident => $ppy:literal, $label:literal) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name;

        impl sealed::Sealed for $name {}

        impl Periodicity for $name {
            const PERIODS_PER_YEAR: u16 = $ppy;
            const NAME: &'static str = $label;
        }
    };
}

periodicity!(
    /// Once per year.
    Annual => 1, "annual"
);
periodicity!(
    /// Twice per year.
    SemiAnnual => 2, "semi-annual"
);
periodicity!(
    /// Four times per year.
    Quarterly => 4, "quarterly"
);
periodicity!(
    /// Twelve times per year.
    Monthly => 12, "monthly"
);
periodicity!(
    /// Fifty-two times per year.
    Weekly => 52, "weekly"
);
periodicity!(
    /// 365 times per year.
    Daily => 365, "daily"
);
