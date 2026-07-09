//! Property-based tests over the public numeric API.
//!
//! These assert the *laws* the operations obey for whole families of inputs,
//! rather than a handful of worked examples: present and future value invert
//! each other, net present value is monotone in the discount rate and collapses
//! to the plain sum at a zero rate, the internal rate of return zeroes the net
//! present value, and the annuity payment inverts the annuity present value.
//!
//! `proptest` is a dev-dependency only, so it never reaches the published
//! crate's dependency tree (the zero-dependency promise is about distribution,
//! not test tooling — `docs/adr/0009-no_std-and-optional-libm.md`). The `std`/
//! `libm`-gated operations (single sum, annuity) are tested only when a
//! transcendental-math feature is on.

use proptest::prelude::*;
use time_value::{Cashflows, Money, Monthly, Rate};

/// Absolute closeness check, mirroring the crate's own `no_std`-safe tolerance
/// helper (`f64::abs` is not in `core`).
fn close(a: f64, b: f64, tolerance: f64) -> bool {
    let d = a - b;
    d < tolerance && d > -tolerance
}

proptest! {
    /// At a zero discount rate nothing is discounted, so the net present value is
    /// exactly the arithmetic sum of the cashflows.
    #[test]
    fn npv_at_zero_rate_is_the_plain_sum(
        amounts in prop::collection::vec(-1e6f64..1e6, 1..=16),
    ) {
        let flows: Vec<Money> = amounts.iter().map(|&a| Money::new(a).unwrap()).collect();
        let series = Cashflows::<Monthly>::new(&flows);
        let sum: f64 = amounts.iter().sum();
        let npv = series
            .net_present_value(Rate::<Monthly>::new(0.0).unwrap())
            .unwrap()
            .value();
        // Up to 16 addends, each |·| ≤ 1e6, so accumulated rounding stays well
        // under this tolerance.
        prop_assert!(close(npv, sum, 1e-6));
    }

    /// With every cashflow positive (and at least one discounted), raising the
    /// discount rate can only lower the net present value.
    #[test]
    fn npv_does_not_increase_with_the_rate(
        amounts in prop::collection::vec(1.0f64..1e5, 2..=16),
        low in 0.0f64..0.5,
        bump in 1e-3f64..0.5,
    ) {
        let flows: Vec<Money> = amounts.iter().map(|&a| Money::new(a).unwrap()).collect();
        let series = Cashflows::<Monthly>::new(&flows);
        let npv_low = series
            .net_present_value(Rate::<Monthly>::new(low).unwrap())
            .unwrap()
            .value();
        let npv_high = series
            .net_present_value(Rate::<Monthly>::new(low + bump).unwrap())
            .unwrap()
            .value();
        // Each discounted term shrinks as the rate rises; the undiscounted t=0
        // term is unchanged. A tiny epsilon absorbs rounding.
        prop_assert!(npv_high <= npv_low + 1e-6);
    }

    /// A conventional series — an outflow now, then inflows that more than repay
    /// it — has an internal rate of return, and discounting at it zeroes the NPV.
    #[test]
    fn irr_zeroes_the_npv(
        inflows in prop::collection::vec(1.0f64..1e3, 1..=10),
        fraction in 0.05f64..0.95,
    ) {
        // Outflow strictly below the total inflow: NPV > 0 at r = 0 and tends to
        // the (negative) initial outflow as r → ∞, so a root is guaranteed.
        let total: f64 = inflows.iter().sum();
        let outflow = total * fraction;
        let mut flows = vec![Money::new(-outflow).unwrap()];
        flows.extend(inflows.iter().map(|&a| Money::new(a).unwrap()));
        let series = Cashflows::<Monthly>::new(&flows);

        let irr = series.internal_rate_of_return().unwrap();
        // The solver converges to a magnitude-relative tolerance (ADR-0021), so
        // the residual NPV is bounded relative to the cashflow scale, not by a
        // fixed absolute epsilon.
        prop_assert!(close(
            series.net_present_value(irr).unwrap().value(),
            0.0,
            1e-6 * total
        ));
    }
}

#[cfg(any(feature = "std", feature = "libm"))]
proptest! {
    /// Present value undoes future value: compounding an amount forward then
    /// discounting it back recovers the original, for any rate and horizon.
    #[test]
    fn present_value_inverts_future_value(
        amount in 1.0f64..1e6,
        rate in -0.9f64..1.0,
        periods in 0.0f64..60.0,
    ) {
        use time_value::{single_sum, Period};

        let rate = Rate::<Monthly>::new(rate).unwrap();
        let periods = Period::new(periods).unwrap();
        let amount = Money::new(amount).unwrap();

        let future = single_sum::future_value(rate, periods, amount).unwrap();
        let back = single_sum::present_value(rate, periods, future).unwrap();
        // Round-trips through the same compound factor, so the error is a few
        // ulps of the amount — a relative tolerance keeps it scale-independent.
        prop_assert!(close(back.value(), amount.value(), 1e-6 * amount.value()));
    }

    /// The level annuity payment is the inverse of the annuity present value:
    /// pricing a payment stream then amortising that price recovers the payment.
    #[test]
    fn annuity_payment_inverts_present_value(
        payment in 1.0f64..1e5,
        rate in -0.9f64..1.0,
        periods in 1.0f64..120.0,
    ) {
        use time_value::{annuity, Period};

        let rate = Rate::<Monthly>::new(rate).unwrap();
        // At least one period, so the amortisation is not degenerate.
        let periods = Period::new(periods).unwrap();
        let payment = Money::new(payment).unwrap();

        let present = annuity::present_value(rate, periods, payment).unwrap();
        let recovered = annuity::payment(rate, periods, present).unwrap();
        prop_assert!(close(recovered.value(), payment.value(), 1e-6 * payment.value()));
    }
}
