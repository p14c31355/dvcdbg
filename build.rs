// build.rs
use std::env;

fn main() {
    let features = [
        "arduino-uno",
        "arduino-nano",
        "arduino-mega",
        "arduino-leonardo",
    ];

    let mut enabled_count = 0;
    for feature in &features {
        if env::var(format!(
            "CARGO_CFG_FEATURE_{}",
            feature.to_uppercase().replace('-', "_")
        ))
        .is_ok()
        {
            enabled_count += 1;
        }
    }

    if enabled_count > 1 {
        panic!("Only one Arduino board feature can be enabled at a time.");
    }
}
