// build.rs
use std::env;

fn main() {
    let features = [
        "arduino-uno",
        "arduino-nano",
        "arduino-mega",
        "arduino-leonardo",
    ];

        let enabled_count = features
        .iter()
        .filter(|feature| {
            env::var(format!(
                "CARGO_CFG_FEATURE_{}",
                feature.to_uppercase().replace('-', "_")
            ))
            .is_ok()
        })
        .count();

    if enabled_count > 1 {
        panic!("Only one Arduino board feature can be enabled at a time.");
    }
}
