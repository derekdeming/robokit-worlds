use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

macro_rules! unit {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(f64);

        impl $name {
            #[doc = concat!("Creates a new `", stringify!($name), "`; world validation enforces its range.")]
            pub const fn new(value: f64) -> Self {
                Self(value)
            }

            /// Returns the scalar value.
            pub const fn get(self) -> f64 {
                self.0
            }

            /// Returns whether the value is finite and non-negative.
            pub fn is_non_negative_finite(self) -> bool {
                self.0.is_finite() && self.0 >= 0.0
            }
        }
    };
}

unit!(Grams, "A mass expressed in grams.");
unit!(Meters, "A distance expressed in meters.");
unit!(Radians, "An angle expressed in radians.");
unit!(Seconds, "A duration expressed in seconds.");
