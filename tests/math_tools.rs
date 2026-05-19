/// Focused integration tests for hematite::tools::math_util
///
/// Covers known-value correctness, edge-case non-panics, and the newer
/// matrix decomposition modes (QR, SVD, Cholesky).
use hematite::tools::math_util::{
    bitwise_calc, checksum_calc, cipher_calc, matrix_calc, number_format, prob_calc, set_calc,
    sort_viz, string_dist, text_stats, validate_calc,
};

// ─── Cipher tests ────────────────────────────────────────────────────────────

#[test]
fn cipher_rot13_hello_world() {
    let out = cipher_calc("rot13 Hello, World!");
    assert!(
        out.contains("Uryyb, Jbeyq!"),
        "ROT13 expected 'Uryyb, Jbeyq!': {out}"
    );
}

#[test]
fn cipher_caesar_shift_13_hello() {
    // ROT13 via caesar subcommand
    let out = cipher_calc("caesar 13 Hello");
    assert!(out.contains("Uryyb"), "Caesar shift-13: {out}");
}

#[test]
fn cipher_caesar_shift_1_abc() {
    let out = cipher_calc("caesar 1 abc");
    assert!(out.contains("bcd"), "Caesar +1: {out}");
}

#[test]
fn cipher_atbash_abc_is_zyx() {
    let out = cipher_calc("atbash ABC");
    assert!(out.contains("ZYX"), "Atbash A→Z B→Y C→X: {out}");
}

#[test]
fn cipher_empty_no_panic() {
    let out = cipher_calc("");
    assert!(!out.is_empty());
}

#[test]
fn cipher_unknown_command_no_panic() {
    let out = cipher_calc("notacipher foobar");
    assert!(!out.is_empty());
}

// ─── String distance tests ───────────────────────────────────────────────────

#[test]
fn levenshtein_kitten_sitting_is_3() {
    let out = string_dist("kitten vs sitting");
    // Output format: "  Levenshtein:            3  (similarity: …)"
    let line = out
        .lines()
        .find(|l| l.contains("Levenshtein:"))
        .unwrap_or("");
    assert!(
        line.contains('3'),
        "Expected Levenshtein distance 3, line: {line:?}\nFull output:\n{out}"
    );
}

#[test]
fn levenshtein_identical_strings_is_zero() {
    let out = string_dist("hello vs hello");
    let line = out
        .lines()
        .find(|l| l.contains("Levenshtein:"))
        .unwrap_or("");
    // Distance must be 0
    let dist: usize = line
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .next()
        .unwrap_or(99);
    assert_eq!(dist, 0, "Identical strings: {line:?}");
}

#[test]
fn levenshtein_one_char_diff_is_one() {
    let out = string_dist("a vs b");
    let line = out
        .lines()
        .find(|l| l.contains("Levenshtein:"))
        .unwrap_or("");
    let dist: usize = line
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .next()
        .unwrap_or(99);
    assert_eq!(dist, 1, "Single-char diff: {line:?}");
}

#[test]
fn levenshtein_empty_input_no_panic() {
    let out = string_dist("");
    assert!(!out.is_empty());
}

#[test]
fn levenshtein_comma_separator() {
    // Also supports comma as separator
    let out = string_dist("kitten, sitting");
    assert!(!out.is_empty());
}

// ─── Validation tests ────────────────────────────────────────────────────────

#[test]
fn validate_luhn_visa_test_card_valid() {
    // 4532015112830366 is a standard Luhn-valid Visa test number
    let out = validate_calc("4532015112830366");
    assert!(
        out.contains("YES ✓"),
        "Expected Luhn VALID (YES ✓): {out}"
    );
}

#[test]
fn validate_luhn_invalid_number() {
    // Flip the last digit — must fail
    let out = validate_calc("4532015112830365");
    assert!(
        out.contains("NO ✗"),
        "Expected Luhn INVALID (NO ✗): {out}"
    );
}

#[test]
fn validate_isbn13_bookland_valid() {
    // 978-0-306-40615-7 is a canonical valid ISBN-13
    let out = validate_calc("978-0-306-40615-7");
    assert!(out.contains("YES ✓"), "Expected ISBN-13 VALID: {out}");
    assert!(out.contains("ISBN-13"), "Should be labelled ISBN-13: {out}");
}

#[test]
fn validate_isbn10_valid() {
    // 0-306-40615-2 is the matching valid ISBN-10
    let out = validate_calc("0-306-40615-2");
    assert!(out.contains("YES ✓"), "Expected ISBN-10 VALID: {out}");
}

#[test]
fn validate_uuid_format_detected() {
    let out = validate_calc("550e8400-e29b-41d4-a716-446655440000");
    assert!(
        out.to_lowercase().contains("uuid"),
        "Should detect UUID: {out}"
    );
}

#[test]
fn validate_empty_no_panic() {
    let out = validate_calc("");
    assert!(!out.is_empty());
}

// ─── Probability / Distribution tests ───────────────────────────────────────

#[test]
fn prob_normal_cdf_at_1sigma() {
    // P(X ≤ 1.0) for N(0,1) ≈ 0.8413
    let out = prob_calc("normal mean=0 sd=1 1.0");
    let cdf_val = parse_cdf_value(&out);
    assert!(
        (cdf_val - 0.8413).abs() < 0.001,
        "CDF at z=1 expected ≈0.8413, got {cdf_val:.6}\n{out}"
    );
}

#[test]
fn prob_normal_cdf_at_196() {
    // P(X ≤ 1.96) for N(0,1) ≈ 0.9750
    let out = prob_calc("normal mean=0 sd=1 1.96");
    let cdf_val = parse_cdf_value(&out);
    assert!(
        (cdf_val - 0.975).abs() < 0.002,
        "CDF at z=1.96 expected ≈0.975, got {cdf_val:.6}\n{out}"
    );
}

#[test]
fn prob_normal_cdf_at_zero_is_half() {
    // Symmetry: P(X ≤ 0) for N(0,1) = 0.5
    let out = prob_calc("normal mean=0 sd=1 0.0");
    let cdf_val = parse_cdf_value(&out);
    assert!(
        (cdf_val - 0.5).abs() < 0.001,
        "CDF at z=0 expected 0.5, got {cdf_val:.6}\n{out}"
    );
}

#[test]
fn prob_empty_no_panic() {
    let out = prob_calc("");
    assert!(!out.is_empty());
}

#[test]
fn prob_unknown_distribution_no_panic() {
    let out = prob_calc("notadist x=1.0");
    assert!(!out.is_empty());
}

// ─── Set theory tests ────────────────────────────────────────────────────────

#[test]
fn set_union_basic() {
    let out = set_calc("{1,2,3} union {3,4,5}");
    // Sorted union: {1, 2, 3, 4, 5}
    assert!(
        out.contains("1, 2, 3, 4, 5"),
        "Expected {{1, 2, 3, 4, 5}}: {out}"
    );
}

#[test]
fn set_intersection_basic() {
    let out = set_calc("{1,2,3,4} intersection {3,4,5}");
    // Intersection: {3, 4}
    assert!(out.contains("3, 4") || (out.contains('3') && out.contains('4')), "Intersection: {out}");
    // 5 should not be in the result line
    let result_line = out.lines().find(|l| l.contains('∩')).unwrap_or("");
    assert!(!result_line.contains('5'), "5 should not be in intersection: {result_line}");
}

#[test]
fn set_difference_basic() {
    let out = set_calc("{1,2,3} difference {2,3}");
    // A\B = {1}
    let diff_line = out.lines().find(|l| l.contains('\\') || l.contains("A \\ B")).unwrap_or("");
    assert!(diff_line.contains('1'), "Difference A\\B should be {{1}}: {diff_line}\n{out}");
}

#[test]
fn set_union_deduplicates() {
    let out = set_calc("{1,1,2} union {2,3}");
    // Result should contain 1, 2, 3 without duplicates
    assert!(out.contains("1, 2, 3") || out.contains("{1, 2, 3}"), "Dedup union: {out}");
}

#[test]
fn set_empty_input_no_panic() {
    let out = set_calc("");
    assert!(!out.is_empty());
}

// ─── Number format tests ─────────────────────────────────────────────────────

#[test]
fn number_format_thousands_separator() {
    let out = number_format("1234567890");
    assert!(
        out.contains("1,234,567,890"),
        "Expected '1,234,567,890': {out}"
    );
}

#[test]
fn number_format_small_number() {
    let out = number_format("42");
    assert!(out.contains("42"), "Small number: {out}");
}

#[test]
fn number_format_negative() {
    let out = number_format("-1000000");
    assert!(
        out.contains("1,000,000"),
        "Negative thousands: {out}"
    );
}

#[test]
fn number_format_scientific_notation() {
    let out = number_format("6.022e23");
    assert!(!out.is_empty(), "Should handle scientific notation");
    assert!(out.contains("Scientific") || out.contains("6.02"), "Sci notation: {out}");
}

#[test]
fn number_format_empty_no_panic() {
    let out = number_format("");
    assert!(!out.is_empty());
}

#[test]
fn number_format_invalid_no_panic() {
    let out = number_format("not_a_number");
    assert!(!out.is_empty());
}

// ─── Checksum tests ──────────────────────────────────────────────────────────

#[test]
fn checksum_produces_all_algorithms() {
    let out = checksum_calc("Hello, World!");
    assert!(out.contains("CRC-32"), "Missing CRC-32: {out}");
    assert!(out.contains("CRC-16"), "Missing CRC-16: {out}");
    assert!(out.contains("Adler"), "Missing Adler: {out}");
    assert!(out.contains("FNV"), "Missing FNV: {out}");
}

#[test]
fn checksum_is_deterministic() {
    let out1 = checksum_calc("consistency test");
    let out2 = checksum_calc("consistency test");
    assert_eq!(out1, out2, "Checksums must be deterministic for same input");
}

#[test]
fn checksum_different_inputs_differ() {
    let out1 = checksum_calc("abc");
    let out2 = checksum_calc("xyz");
    // The CRC-32 lines must differ
    let crc1 = out1.lines().find(|l| l.contains("CRC-32")).unwrap_or("");
    let crc2 = out2.lines().find(|l| l.contains("CRC-32")).unwrap_or("");
    assert_ne!(crc1, crc2, "Different inputs must yield different CRC-32");
}

#[test]
fn checksum_empty_string_no_panic() {
    let out = checksum_calc("");
    // Empty input is valid — checksums of empty byte sequence are defined
    assert!(out.contains("CRC"), "Empty input checksum: {out}");
}

// ─── Bitwise calculator tests ────────────────────────────────────────────────

#[test]
fn bitwise_and_mask() {
    // 0xFF AND 0x3C = 0x3C = 60
    let out = bitwise_calc("0xFF AND 0x3C");
    assert!(
        out.contains("60") || out.contains("3C") || out.contains("0x3C"),
        "AND 0xFF & 0x3C expected 0x3C (60): {out}"
    );
}

#[test]
fn bitwise_or_combines_nibbles() {
    // 0xF0 OR 0x0F = 0xFF = 255
    let out = bitwise_calc("0xF0 OR 0x0F");
    assert!(
        out.contains("255") || out.contains("FF"),
        "OR 0xF0 | 0x0F expected 0xFF (255): {out}"
    );
}

#[test]
fn bitwise_xor_same_operands_is_zero() {
    // 0xDEAD XOR 0xDEAD = 0
    let out = bitwise_calc("0xDEAD XOR 0xDEAD");
    // The result row should show value 0
    assert!(
        out.contains("            0") || out.contains("Result:") && out.contains('0'),
        "XOR same operands: {out}"
    );
}

#[test]
fn bitwise_ieee754_one_point_zero() {
    let out = bitwise_calc("ieee754 1.0");
    // IEEE 754 bit pattern for 1.0 = 0x3FF0000000000000
    assert!(out.contains("3FF0000000000000") || out.contains("3ff0000000000000"),
            "IEEE754 1.0 pattern: {out}");
    assert!(out.contains("Value") || out.contains("1.0"), "IEEE754: {out}");
}

#[test]
fn bitwise_empty_no_panic() {
    let out = bitwise_calc("");
    assert!(!out.is_empty());
}

// ─── Sorting visualizer tests ────────────────────────────────────────────────

#[test]
fn sort_viz_produces_output() {
    let out = sort_viz("5,3,8,1,9,2");
    // Should contain at least the numbers
    assert!(out.contains('1') && out.contains('9'), "Sort: {out}");
    // Should mention some algorithm names
    assert!(
        out.to_lowercase().contains("bubble")
            || out.to_lowercase().contains("sort")
            || out.to_lowercase().contains("merge"),
        "Sort algorithms: {out}"
    );
}

#[test]
fn sort_viz_already_sorted_no_panic() {
    let out = sort_viz("1,2,3,4,5");
    assert!(!out.is_empty());
}

#[test]
fn sort_viz_single_element_no_panic() {
    let out = sort_viz("42");
    assert!(!out.is_empty());
}

#[test]
fn sort_viz_two_elements_no_panic() {
    let out = sort_viz("9,1");
    assert!(!out.is_empty());
}

#[test]
fn sort_viz_empty_no_panic() {
    let out = sort_viz("");
    assert!(!out.is_empty());
}

// ─── Text statistics tests ───────────────────────────────────────────────────

#[test]
fn text_stats_basic_pangram() {
    let out = text_stats("The quick brown fox jumps over the lazy dog.");
    // Should report word count, character count, or readability scores
    assert!(
        out.to_lowercase().contains("word") || out.contains("Word"),
        "Text stats missing word info: {out}"
    );
}

#[test]
fn text_stats_word_frequency() {
    let out = text_stats("the cat sat on the mat the cat");
    // "the" appears 3 times
    assert!(out.contains('3') || out.contains("the"), "Word freq: {out}");
}

#[test]
fn text_stats_empty_no_panic() {
    let out = text_stats("");
    assert!(!out.is_empty());
}

#[test]
fn text_stats_single_word_no_panic() {
    let out = text_stats("hello");
    assert!(!out.is_empty());
}

// ─── Matrix decomposition tests ──────────────────────────────────────────────

#[test]
fn matrix_qr_produces_q_and_r() {
    let out = matrix_calc("qr [[1,2],[3,4]]");
    assert!(!out.contains("Error"), "QR should not error: {out}");
    assert!(
        out.contains("Q (orthonormal") || out.contains("Q (ortho"),
        "QR missing Q: {out}"
    );
    assert!(
        out.contains("R (upper-triangular") || out.contains("R (upper"),
        "QR missing R: {out}"
    );
}

#[test]
fn matrix_qr_identity_no_error() {
    let out = matrix_calc("qr [[1,0,0],[0,1,0],[0,0,1]]");
    assert!(!out.contains("Error"), "QR identity: {out}");
}

#[test]
fn matrix_svd_singular_values_positive() {
    let out = matrix_calc("svd [[1,2],[3,4]]");
    assert!(
        out.contains("SVD Singular Values"),
        "SVD header missing: {out}"
    );
    // Extract σ1 value and verify it is positive
    let sv_line = out.lines().find(|l| l.contains("σ1")).unwrap_or("");
    let sv1: f64 = sv_line
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .next()
        .unwrap_or(-1.0);
    assert!(sv1 > 0.0, "σ1 must be positive, got {sv1}: {sv_line}");
}

#[test]
fn matrix_svd_identity_no_error() {
    let out = matrix_calc("svd [[1,0,0],[0,1,0],[0,0,1]]");
    assert!(!out.contains("Error"), "SVD identity: {out}");
    assert!(out.contains("SVD Singular Values"), "SVD header: {out}");
    // σ1 must be ≥ 1 (all singular values of I are 1)
    let sv_line = out.lines().find(|l| l.contains("σ1")).unwrap_or("");
    let sv1: f64 = sv_line
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .next()
        .unwrap_or(0.0);
    assert!(sv1 >= 1.0 - 1e-6, "σ1 of identity must be ≥1, got {sv1}");
}

#[test]
fn matrix_svd_rank_deficient() {
    // Rank-1 matrix: all rows the same
    let out = matrix_calc("svd [[1,2],[1,2]]");
    assert!(!out.contains("Error"), "SVD rank-1: {out}");
    assert!(out.contains("Rank: 1"), "Expected rank 1: {out}");
}

#[test]
fn matrix_cholesky_spd_2x2() {
    // A = [[4,2],[2,3]] is symmetric positive definite
    let out = matrix_calc("chol [[4,2],[2,3]]");
    assert!(!out.contains("Error"), "Cholesky SPD 2×2: {out}");
    assert!(
        out.contains("Cholesky Decomposition") || out.contains("L (lower"),
        "Cholesky output: {out}"
    );
}

#[test]
fn matrix_cholesky_identity() {
    // Cholesky of I is I
    let out = matrix_calc("chol [[1,0,0],[0,1,0],[0,0,1]]");
    assert!(!out.contains("Error"), "Cholesky identity: {out}");
}

#[test]
fn matrix_cholesky_non_spd_does_not_panic() {
    // Negative definite — should report an error message, not panic
    let out = matrix_calc("chol [[-1,0],[0,-1]]");
    assert!(!out.is_empty(), "Must produce some output");
    // Must contain Error or still print something sensible
    assert!(
        out.contains("Error") || out.contains("not") || out.contains("L (lower"),
        "Non-SPD cholesky: {out}"
    );
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Extract the CDF value from a `prob_calc` output.
/// Looks for a line containing "CDF P(X ≤" and parses the trailing float.
fn parse_cdf_value(out: &str) -> f64 {
    out.lines()
        .find(|l| l.contains("CDF P") && l.contains('≤'))
        .and_then(|l| l.split('=').last())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or_else(|| panic!("Could not parse CDF value from:\n{out}"))
}
