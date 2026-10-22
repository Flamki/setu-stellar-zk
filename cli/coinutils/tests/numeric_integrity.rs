use coinutils::{bls_scalar_to_decimal_string, decimal_string_to_bls_scalar};
use num_bigint::BigUint;
use soroban_sdk::Env;

fn assert_exact_conversion(decimal_str: &str) {
    let env = Env::default();
    let scalar = decimal_string_to_bls_scalar(&env, decimal_str)
        .expect("a canonical in-field integer must be convertible");
    let actual = BigUint::from_bytes_be(&scalar.to_bytes().to_array()).to_str_radix(10);
    assert_eq!(
        actual, decimal_str,
        "decimal_str -> scalar changed the integer"
    );
    assert_eq!(bls_scalar_to_decimal_string(&scalar), decimal_str);
}

macro_rules! exact_conversion {
    ($name:ident, $decimal_str:literal) => {
        #[test]
        fn $name() {
            assert_exact_conversion($decimal_str);
        }
    };
}

macro_rules! rejected_conversion {
    ($name:ident, $decimal_str:literal) => {
        #[test]
        fn $name() {
            let env = Env::default();
            if let Ok(scalar) = decimal_string_to_bls_scalar(&env, $decimal_str) {
                panic!(
                    "invalid decimal_str {:?} was accepted as {}",
                    $decimal_str,
                    bls_scalar_to_decimal_string(&scalar)
                );
            }
        }
    };
}

exact_conversion!(preserves_zero, "0");
exact_conversion!(preserves_value_100, "100");
exact_conversion!(preserves_u32_max, "4294967295");
exact_conversion!(preserves_u32_max_plus_one, "4294967296");
exact_conversion!(preserves_100_plus_two_to_32, "4294967396");
exact_conversion!(preserves_js_safe_integer_max, "9007199254740991");
exact_conversion!(preserves_above_js_safe_integer, "9007199254740993");
exact_conversion!(
    preserves_u128_max,
    "340282366920938463463374607431768211455"
);
exact_conversion!(
    preserves_bls_scalar_max,
    "52435875175126190479447740508185965837690552500527637822603658699938581184512"
);

rejected_conversion!(rejects_empty_decimal, "");
rejected_conversion!(rejects_negative_decimal, "-1");
rejected_conversion!(rejects_fractional_decimal, "100.5");
rejected_conversion!(rejects_exponent_notation, "1e2");
rejected_conversion!(
    rejects_more_than_256_bits,
    "115792089237316195423570985008687907853269984665640564039457584007913129639936"
);
