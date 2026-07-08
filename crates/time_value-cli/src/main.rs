//! `time-value` — the command-line interface for the [`time_value`] library.
//!
//! Placeholder entry point. The time-value-of-money subcommands (`pv`, `fv`,
//! `npv`, `irr`, annuity/payment, …) land in a later phase; their surface is
//! designed in `docs/adr/0010-cli-surface.md` before implementation.

fn main() {
    println!("time-value: CLI not yet implemented");
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_builds() {
        // The real integration tests (assert_cmd) arrive with the subcommands.
        assert_eq!(2 + 2, 4);
    }
}
