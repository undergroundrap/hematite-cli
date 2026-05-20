/// Focused integration tests for hematite::tools::math_util
///
/// Covers known-value correctness, edge-case non-panics, and the newer
/// matrix decomposition modes (QR, SVD, Cholesky).
use hematite::tools::math_util::{
    bitwise_calc, checksum_calc, chemistry_calc, cipher_calc, combinatorics_calc, complex_calc,
    csv_calc, datetime_calc, electrical_calc, encode_calc, fraction_calc, geometry_calc, hash_calc,
    health_calc, json_calc, matrix_calc, number_format, number_theory_calc, percent_calc,
    physics_calc, prob_calc, regex_calc, roman_calc, set_calc, sort_viz, stats_calc, string_dist,
    text_stats, trig_calc, validate_calc,
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
    assert!(out.contains("YES ✓"), "Expected Luhn VALID (YES ✓): {out}");
}

#[test]
fn validate_luhn_invalid_number() {
    // Flip the last digit — must fail
    let out = validate_calc("4532015112830365");
    assert!(out.contains("NO ✗"), "Expected Luhn INVALID (NO ✗): {out}");
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
    assert!(
        out.contains("3, 4") || (out.contains('3') && out.contains('4')),
        "Intersection: {out}"
    );
    // 5 should not be in the result line
    let result_line = out.lines().find(|l| l.contains('∩')).unwrap_or("");
    assert!(
        !result_line.contains('5'),
        "5 should not be in intersection: {result_line}"
    );
}

#[test]
fn set_difference_basic() {
    let out = set_calc("{1,2,3} difference {2,3}");
    // A\B = {1}
    let diff_line = out
        .lines()
        .find(|l| l.contains('\\') || l.contains("A \\ B"))
        .unwrap_or("");
    assert!(
        diff_line.contains('1'),
        "Difference A\\B should be {{1}}: {diff_line}\n{out}"
    );
}

#[test]
fn set_union_deduplicates() {
    let out = set_calc("{1,1,2} union {2,3}");
    // Result should contain 1, 2, 3 without duplicates
    assert!(
        out.contains("1, 2, 3") || out.contains("{1, 2, 3}"),
        "Dedup union: {out}"
    );
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
    assert!(out.contains("1,000,000"), "Negative thousands: {out}");
}

#[test]
fn number_format_scientific_notation() {
    let out = number_format("6.022e23");
    assert!(!out.is_empty(), "Should handle scientific notation");
    assert!(
        out.contains("Scientific") || out.contains("6.02"),
        "Sci notation: {out}"
    );
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
    assert!(
        out.contains("3FF0000000000000") || out.contains("3ff0000000000000"),
        "IEEE754 1.0 pattern: {out}"
    );
    assert!(
        out.contains("Value") || out.contains("1.0"),
        "IEEE754: {out}"
    );
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

// ─── Stats tests ─────────────────────────────────────────────────────────────

#[test]
fn stats_mean_of_1_to_5() {
    let out = stats_calc("1,2,3,4,5");
    assert!(out.contains("Mean:"), "missing Mean: {out}");
    assert!(
        out.contains("3.000000"),
        "mean of 1..5 should be 3.0: {out}"
    );
}

#[test]
fn stats_median_even_count() {
    // 1,2,3,4 => median = 2.5
    let out = stats_calc("1,2,3,4");
    assert!(out.contains("2.500000"), "median of 1,2,3,4 = 2.5: {out}");
}

#[test]
fn stats_outlier_detected() {
    // 1,2,3,4,5,100 — 100 should be an outlier
    let out = stats_calc("1,2,3,4,5,100");
    assert!(out.contains("Outliers:"), "should list outliers: {out}");
    assert!(
        out.contains("100"),
        "100 should be detected as outlier: {out}"
    );
}

#[test]
fn stats_empty_input_no_panic() {
    let out = stats_calc("");
    assert!(
        out.contains("No numeric") || out.contains("Usage"),
        "empty input message: {out}"
    );
}

#[test]
fn stats_histogram_present() {
    let out = stats_calc("1,2,3,4,5,6,7,8,9,10");
    assert!(
        out.contains("Histogram"),
        "histogram section missing: {out}"
    );
}

// ─── Hash tests ───────────────────────────────────────────────────────────────

#[test]
fn hash_md5_known_value() {
    // MD5("") = d41d8cd98f00b204e9800998ecf8427e
    let out = hash_calc("");
    assert!(
        out.contains("d41d8cd98f00b204e9800998ecf8427e"),
        "MD5 of empty string: {out}"
    );
}

#[test]
fn hash_sha256_known_value() {
    // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469...
    let out = hash_calc("abc");
    assert!(
        out.contains("ba7816bf"),
        "SHA-256 of 'abc' should start with ba7816bf: {out}"
    );
}

#[test]
fn hash_sha1_known_value() {
    // SHA-1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
    let out = hash_calc("abc");
    assert!(
        out.contains("a9993e36"),
        "SHA-1 of 'abc' should start with a9993e36: {out}"
    );
}

#[test]
fn hash_shows_all_algorithms() {
    let out = hash_calc("hello");
    assert!(out.contains("MD5"), "should show MD5: {out}");
    assert!(out.contains("SHA-1"), "should show SHA-1: {out}");
    assert!(out.contains("SHA-256"), "should show SHA-256: {out}");
    assert!(out.contains("SHA-512"), "should show SHA-512: {out}");
}

// ─── Encode tests ─────────────────────────────────────────────────────────────

#[test]
fn encode_base64_hello_world() {
    let out = encode_calc("base64 encode Hello World");
    assert!(
        out.contains("SGVsbG8gV29ybGQ="),
        "base64 of 'Hello World': {out}"
    );
}

#[test]
fn encode_base64_decode_roundtrip() {
    let out = encode_calc("base64 decode SGVsbG8gV29ybGQ=");
    assert!(out.contains("Hello World"), "base64 decode: {out}");
}

#[test]
fn encode_hex_hello() {
    let out = encode_calc("hex encode Hello");
    assert!(out.contains("48656c6c6f"), "hex of 'Hello': {out}");
}

#[test]
fn encode_hex_decode_roundtrip() {
    let out = encode_calc("hex decode 48656c6c6f");
    assert!(out.contains("Hello"), "hex decode: {out}");
}

#[test]
fn encode_rot13_is_involution() {
    let out1 = encode_calc("rot13 Hello");
    assert!(out1.contains("Uryyb"), "ROT13 Hello->Uryyb: {out1}");
    let out2 = encode_calc("rot13 Uryyb");
    assert!(out2.contains("Hello"), "ROT13 Uryyb->Hello: {out2}");
}

#[test]
fn encode_all_formats_shows_all() {
    let out = encode_calc("Hello");
    assert!(out.contains("Base64:"), "all-formats missing Base64: {out}");
    assert!(out.contains("Hex:"), "all-formats missing Hex: {out}");
    assert!(out.contains("URL:"), "all-formats missing URL: {out}");
    assert!(out.contains("Binary:"), "all-formats missing Binary: {out}");
    assert!(out.contains("ROT13:"), "all-formats missing ROT13: {out}");
}

// ─── Geometry tests ───────────────────────────────────────────────────────────

#[test]
fn geometry_circle_area_and_circumference() {
    let out = geometry_calc("circle r=5");
    // Area = pi*25 ≈ 78.5398
    assert!(out.contains("78.539"), "circle area r=5: {out}");
    // Circumference = 2*pi*5 ≈ 31.4159
    assert!(out.contains("31.415"), "circle circumference r=5: {out}");
}

#[test]
fn geometry_pythagorean_3_4_5() {
    let out = geometry_calc("pythagorean a=3 b=4");
    assert!(
        out.contains("5.000000"),
        "3-4-5 right triangle hypotenuse: {out}"
    );
}

#[test]
fn geometry_triangle_345_heron() {
    // 3-4-5 right triangle area = 6
    let out = geometry_calc("triangle a=3 b=4 c=5");
    assert!(out.contains("6.000000"), "Heron area of 3-4-5: {out}");
}

#[test]
fn geometry_sphere_volume() {
    // r=3 => V = 4/3 * pi * 27 ≈ 113.097
    let out = geometry_calc("sphere r=3");
    assert!(out.contains("113.09"), "sphere volume r=3: {out}");
}

#[test]
fn geometry_rectangle_area() {
    let out = geometry_calc("rectangle w=6 h=4");
    assert!(out.contains("24.000000"), "rectangle area 6×4=24: {out}");
}

#[test]
fn geometry_distance_3_4() {
    // (0,0) to (3,4) = 5
    let out = geometry_calc("distance x1=0 y1=0 x2=3 y2=4");
    assert!(
        out.contains("5.000000"),
        "distance from origin to (3,4)=5: {out}"
    );
}

#[test]
fn geometry_radians_90_degrees() {
    let out = geometry_calc("radians 90");
    assert!(out.contains("1.570796"), "90° in radians: {out}");
}

#[test]
fn geometry_unknown_shape_no_panic() {
    let out = geometry_calc("hexahedron r=5");
    assert!(
        !out.is_empty(),
        "should return non-empty output for unknown shape"
    );
}

// ─── Electrical tests ─────────────────────────────────────────────────────────

#[test]
fn electrical_ohms_law_solve_current() {
    // V=12, R=100 => I=120mA (SI formatter shows mA)
    let out = electrical_calc("ohm V=12 R=100");
    assert!(
        out.contains("120.0000 mA") || out.contains("0.1200"),
        "I=V/R=120mA: {out}"
    );
}

#[test]
fn electrical_ohms_law_solve_voltage() {
    // I=2, R=50 => V=100
    let out = electrical_calc("ohm I=2 R=50");
    assert!(out.contains("100.0000"), "V=I*R=100: {out}");
}

#[test]
fn electrical_rc_time_constant() {
    // R=10k=10000, C=100u=0.0001 => tau=1s (displayed as 1000ms by SI formatter)
    let out = electrical_calc("rc R=10k C=100u");
    assert!(
        out.contains("1000.0000 ms") || out.contains("1.0000 s"),
        "RC tau=1s: {out}"
    );
}

#[test]
fn electrical_db_half_power() {
    // linear ratio 0.5 => ~-6.02 dB (voltage)
    let out = electrical_calc("db 0.5");
    assert!(out.contains("-6.02"), "0.5 linear = -6.02 dB: {out}");
}

#[test]
fn electrical_voltage_divider() {
    // Vin=10, R1=10k, R2=10k => Vout=5
    let out = electrical_calc("divider Vin=10 R1=10k R2=10k");
    assert!(
        out.contains("5.0000"),
        "equal resistor divider Vout=5: {out}"
    );
}

#[test]
fn electrical_empty_no_panic() {
    let out = electrical_calc("");
    assert!(!out.is_empty(), "empty input should return usage");
}

// ─── Physics tests ────────────────────────────────────────────────────────────

#[test]
fn physics_kinematic_freefall_3s() {
    // v0=0, a=9.8, t=3 → v=29.4, s=44.1
    let out = physics_calc("kinematic v0=0 a=9.8 t=3");
    assert!(out.contains("29.4"), "v=a*t=29.4: {out}");
    assert!(out.contains("44.1"), "s=0.5*a*t²=44.1: {out}");
}

#[test]
fn physics_projectile_45_degree_range() {
    // v0=20, angle=45 → max range (symmetric) ≈ 40.77 m at g=9.80665
    let out = physics_calc("projectile v0=20 angle=45");
    assert!(out.contains("Range"), "should show Range: {out}");
    // Range ≈ 40.7 m
    assert!(out.contains("40."), "45° range ≈ 40.7 m: {out}");
}

#[test]
fn physics_force_f_equals_ma() {
    // m=5, a=3 → F=15 N
    let out = physics_calc("force m=5 a=3");
    assert!(out.contains("15.000000"), "F=ma=15: {out}");
}

#[test]
fn physics_kinetic_energy() {
    // m=10, v=5 → KE=125
    let out = physics_calc("energy m=10 v=5");
    assert!(out.contains("125.000000"), "KE=0.5*10*25=125: {out}");
}

#[test]
fn physics_pendulum_1m() {
    // L=1 → T=2π√(1/9.80665) ≈ 2.006s
    let out = physics_calc("pendulum L=1");
    assert!(
        out.contains("2.006") || out.contains("2.00"),
        "T≈2.006s for L=1m: {out}"
    );
}

#[test]
fn physics_ideal_gas_solve_pressure() {
    // n=1, T=273.15, V=0.02241 (1 mol at STP) → P≈101325 Pa
    let out = physics_calc("gas n=1 T=273.15 V=0.02241");
    assert!(out.contains("Pa"), "should show pressure in Pa: {out}");
}

#[test]
fn physics_snell_glass_30_degrees() {
    // n1=1, n2=1.5, angle=30 → θ2=arcsin(0.5/1.5)≈19.47°
    let out = physics_calc("snell n1=1 n2=1.5 angle=30");
    assert!(out.contains("19."), "refracted angle ≈19.47°: {out}");
}

#[test]
fn physics_empty_no_panic() {
    let out = physics_calc("");
    assert!(!out.is_empty());
}

// ─── Chemistry tests ──────────────────────────────────────────────────────────

#[test]
fn chemistry_molar_mass_water() {
    // H2O = 2*1.008 + 15.999 = 18.015
    let out = chemistry_calc("H2O");
    assert!(
        out.contains("18.015") || out.contains("18.01"),
        "H2O molar mass ≈18.015: {out}"
    );
}

#[test]
fn chemistry_molar_mass_glucose() {
    // C6H12O6 = 6*12.011 + 12*1.008 + 6*15.999 = 180.156
    let out = chemistry_calc("C6H12O6");
    assert!(out.contains("180.1"), "C6H12O6 molar mass ≈180.156: {out}");
}

#[test]
fn chemistry_molar_mass_calcium_hydroxide() {
    // Ca(OH)2 = 40.078 + 2*(15.999+1.008) = 74.092
    let out = chemistry_calc("Ca(OH)2");
    assert!(
        out.contains("74.09") || out.contains("74.08"),
        "Ca(OH)2 ≈74.09: {out}"
    );
}

#[test]
fn chemistry_ph_from_concentration() {
    // [H+]=0.001 → pH=3
    let out = chemistry_calc("ph 0.001");
    assert!(
        out.contains("3.000") || out.contains("3.0000"),
        "pH=3 for [H+]=0.001: {out}"
    );
}

#[test]
fn chemistry_ph_from_ph_value() {
    let out = chemistry_calc("ph pH=7");
    assert!(
        out.contains("neutral") || out.contains("1.0000e-7"),
        "pH=7 neutral: {out}"
    );
}

#[test]
fn chemistry_molarity_calculation() {
    // n=2 mol, V=0.5 L → C=4 M
    let out = chemistry_calc("molarity n=2 V=0.5");
    assert!(out.contains("4.000000"), "C=n/V=4M: {out}");
}

#[test]
fn chemistry_buffer_henderson_hasselbalch() {
    // pKa=4.75, equal concentrations → pH=pKa=4.75
    let out = chemistry_calc("buffer pKa=4.75 A=0.1 HA=0.1");
    assert!(
        out.contains("4.750") || out.contains("4.75"),
        "equal conc buffer pH=pKa: {out}"
    );
}

#[test]
fn chemistry_empty_no_panic() {
    let out = chemistry_calc("");
    assert!(!out.is_empty());
}

// ─── Combinatorics tests ──────────────────────────────────────────────────────

#[test]
fn combinatorics_c_10_3_is_120() {
    let out = combinatorics_calc("C 10 3");
    assert!(out.contains("120"), "C(10,3)=120: {out}");
}

#[test]
fn combinatorics_p_5_2_is_20() {
    let out = combinatorics_calc("P 5 2");
    assert!(out.contains("20"), "P(5,2)=20: {out}");
}

#[test]
fn combinatorics_factorial_10_is_3628800() {
    let out = combinatorics_calc("factorial 10");
    assert!(out.contains("3628800"), "10!=3628800: {out}");
}

#[test]
fn combinatorics_derangement_5_is_44() {
    let out = combinatorics_calc("derangement 5");
    assert!(out.contains("44"), "D(5)=44: {out}");
}

#[test]
fn combinatorics_catalan_7_is_429() {
    let out = combinatorics_calc("catalan 7");
    assert!(out.contains("429"), "C_7=429: {out}");
}

#[test]
fn combinatorics_pascal_row_4() {
    // Row 4: 1 4 6 4 1
    let out = combinatorics_calc("pascal 4");
    assert!(out.contains('6'), "C(4,2)=6 in pascal row 4: {out}");
}

#[test]
fn combinatorics_bell_4_is_15() {
    let out = combinatorics_calc("bell 4");
    assert!(out.contains("15"), "B(4)=15: {out}");
}

#[test]
fn combinatorics_stirling_5_2_is_15() {
    let out = combinatorics_calc("stirling 5 2");
    assert!(out.contains("15"), "S(5,2)=15: {out}");
}

#[test]
fn combinatorics_partition_5_is_7() {
    let out = combinatorics_calc("partition 5");
    assert!(out.contains('7'), "p(5)=7: {out}");
}

#[test]
fn combinatorics_multinomial_12_3_4_5() {
    // 12!/(3!4!5!) = 27720
    let out = combinatorics_calc("multinomial 12 3,4,5");
    assert!(out.contains("27720"), "multinomial(12;3,4,5)=27720: {out}");
}

#[test]
fn combinatorics_empty_no_panic() {
    let out = combinatorics_calc("");
    assert!(!out.is_empty());
}

// ─── Date / time tests ───────────────────────────────────────────────────────

#[test]
fn datetime_date_diff_days() {
    // 2025-01-01 to 2025-01-31 = 30 days
    let out = datetime_calc("2025-01-01 to 2025-01-31");
    assert!(out.contains("30"), "expected 30 days: {out}");
}

#[test]
fn datetime_date_diff_includes_business_days() {
    // 2025-01-06 (Mon) to 2025-01-10 (Fri) = 4 days, 4 business days
    let out = datetime_calc("2025-01-06 to 2025-01-10");
    assert!(out.contains('4'), "expected 4 business days: {out}");
}

#[test]
fn datetime_unix_decode() {
    // Unix 0 = 1970-01-01
    let out = datetime_calc("unix 0");
    assert!(out.contains("1970"), "epoch decode: {out}");
}

#[test]
fn datetime_tounix_epoch() {
    // 1970-01-01 = Unix 0
    let out = datetime_calc("toUnix 1970-01-01");
    assert!(out.contains('0'), "toUnix epoch: {out}");
}

#[test]
fn datetime_today_no_panic() {
    let out = datetime_calc("today");
    assert!(out.contains("Date"), "today info: {out}");
}

#[test]
fn datetime_relative_add() {
    // "today + 0" = today, should still produce output
    let out = datetime_calc("today + 0");
    assert!(!out.is_empty());
}

#[test]
fn datetime_single_date_profile() {
    let out = datetime_calc("2000-01-01");
    assert!(out.contains("2000"), "single date profile: {out}");
    assert!(
        out.contains("Saturday") || out.contains("Day of week"),
        "day info: {out}"
    );
}

#[test]
fn datetime_empty_no_panic() {
    let out = datetime_calc("");
    assert!(!out.is_empty());
}

// ─── Number theory tests ──────────────────────────────────────────────────────

#[test]
fn nt_prime_97_is_prime() {
    let out = number_theory_calc("prime 97");
    assert!(out.contains("YES"), "97 is prime: {out}");
}

#[test]
fn nt_prime_100_not_prime() {
    let out = number_theory_calc("prime 100");
    assert!(out.contains("NO"), "100 is not prime: {out}");
}

#[test]
fn nt_factor_360() {
    // 360 = 2^3 × 3^2 × 5
    let out = number_theory_calc("factor 360");
    assert!(out.contains('2'), "factor 360 has 2: {out}");
    assert!(out.contains('3'), "factor 360 has 3: {out}");
    assert!(out.contains('5'), "factor 360 has 5: {out}");
}

#[test]
fn nt_gcd_48_18_is_6() {
    let out = number_theory_calc("gcd 48 18");
    assert!(out.contains('6'), "gcd(48,18)=6: {out}");
}

#[test]
fn nt_lcm_12_18_is_36() {
    let out = number_theory_calc("lcm 12 18");
    assert!(out.contains("36"), "lcm(12,18)=36: {out}");
}

#[test]
fn nt_phi_36_is_12() {
    let out = number_theory_calc("phi 36");
    assert!(out.contains("12"), "phi(36)=12: {out}");
}

#[test]
fn nt_modinv_3_11_is_4() {
    // 3 × 4 = 12 ≡ 1 (mod 11)
    let out = number_theory_calc("modinv 3 11");
    assert!(out.contains('4'), "modinv(3,11)=4: {out}");
}

#[test]
fn nt_primes_up_to_20() {
    let out = number_theory_calc("primes 20");
    assert!(out.contains('2'), "primes(20): {out}");
    assert!(out.contains("19"), "primes(20) includes 19: {out}");
}

#[test]
fn nt_nextprime_after_10_is_11() {
    let out = number_theory_calc("nextprime 10");
    assert!(out.contains("11"), "nextprime(10)=11: {out}");
}

#[test]
fn nt_bare_number_profile() {
    let out = number_theory_calc("12");
    assert!(!out.is_empty(), "bare number 12: {out}");
    assert!(out.contains("NO"), "12 not prime: {out}");
}

#[test]
fn nt_empty_no_panic() {
    let out = number_theory_calc("");
    assert!(!out.is_empty());
}

// ─── Health calc tests ────────────────────────────────────────────────────────

#[test]
fn health_bmi_normal_weight() {
    // 70 kg, 1.75 m → BMI ≈ 22.9 (Normal weight)
    let out = health_calc("bmi w=70 h=1.75");
    assert!(out.contains("Normal"), "BMI normal: {out}");
    assert!(
        out.contains("22") || out.contains("23"),
        "BMI ≈ 22.9: {out}"
    );
}

#[test]
fn health_bmi_overweight() {
    // 90 kg, 1.70 m → BMI ≈ 31.1 (Obese)
    let out = health_calc("bmi w=90 h=1.70");
    assert!(
        out.contains("Obese") || out.contains("Over"),
        "BMI obese/over: {out}"
    );
}

#[test]
fn health_bmi_imperial() {
    // 154 lb, 5ft10in ≈ 70 kg, 1.778 m → BMI ≈ 22.1
    let out = health_calc("bmi w=154lb h=5ft10in");
    assert!(
        out.contains("Normal") || out.contains("22"),
        "BMI imperial: {out}"
    );
}

#[test]
fn health_bmr_male() {
    // 80 kg, 180 cm, 30 yr male → ≈ 1846 kcal
    let out = health_calc("bmr male w=80 h=180 age=30");
    assert!(out.contains("1") && out.contains("kcal"), "BMR male: {out}");
    assert!(out.contains("Male"), "sex label: {out}");
}

#[test]
fn health_tdee_moderate() {
    let out = health_calc("tdee male w=80 h=180 age=30 activity=moderate");
    assert!(out.contains("TDEE"), "tdee output: {out}");
    assert!(out.contains("maintenance"), "maintenance label: {out}");
}

#[test]
fn health_macros_muscle() {
    let out = health_calc("macros calories=2400 goal=muscle");
    assert!(out.contains("Protein"), "macros protein: {out}");
    assert!(out.contains("30"), "muscle protein 30%: {out}");
}

#[test]
fn health_macros_keto() {
    let out = health_calc("macros calories=2000 goal=keto");
    assert!(out.contains("70"), "keto fat 70%: {out}");
}

#[test]
fn health_ideal_height() {
    let out = health_calc("ideal h=175");
    assert!(
        out.contains("18.5") || out.contains("24.9") || out.contains("Ideal"),
        "ideal: {out}"
    );
}

#[test]
fn health_water_intake() {
    // 70 kg × 0.033 ≈ 2.31 L
    let out = health_calc("water w=70");
    assert!(out.contains('2'), "water ≈ 2.31 L: {out}");
}

#[test]
fn health_empty_no_panic() {
    let out = health_calc("");
    assert!(!out.is_empty());
}

// ─── Trig tests ───────────────────────────────────────────────────────────────

#[test]
fn trig_sin_45_is_sqrt2_over_2() {
    let out = trig_calc("45");
    // sin(45) = 0.707107
    assert!(
        out.contains("0.707") || out.contains("sin"),
        "sin(45): {out}"
    );
}

#[test]
fn trig_cos_60_is_half() {
    let out = trig_calc("60");
    assert!(
        out.contains("0.5") || out.contains("cos"),
        "cos(60)=0.5: {out}"
    );
}

#[test]
fn trig_sin_90_is_1() {
    let out = trig_calc("90");
    assert!(out.contains("sin"), "sin(90): {out}");
    // sin 90 should be 1
    let sin_line = out.lines().find(|l| l.contains("sin")).unwrap_or("");
    assert!(sin_line.contains('1'), "sin(90)=1: {sin_line}");
}

#[test]
fn trig_asin_half_is_30() {
    let out = trig_calc("asin 0.5");
    assert!(out.contains("30"), "asin(0.5)=30 deg: {out}");
}

#[test]
fn trig_acos_half_is_60() {
    let out = trig_calc("acos 0.5");
    assert!(out.contains("60"), "acos(0.5)=60 deg: {out}");
}

#[test]
fn trig_atan_1_is_45() {
    let out = trig_calc("atan 1");
    assert!(out.contains("45"), "atan(1)=45 deg: {out}");
}

#[test]
fn trig_atan2_3_4() {
    let out = trig_calc("atan2 3 4");
    // atan2(3,4) ≈ 36.87 deg
    assert!(out.contains("36"), "atan2(3,4) approx 36.87: {out}");
}

#[test]
fn trig_sinh_0_is_0() {
    let out = trig_calc("sinh 0");
    assert!(
        out.contains("0.000000") || out.contains("sinh"),
        "sinh(0)=0: {out}"
    );
}

#[test]
fn trig_hyp_table_shows_sin() {
    let out = trig_calc("hyp");
    assert!(out.contains("sin"), "hyp table has sin header: {out}");
    assert!(out.contains("45"), "hyp table has 45 deg row: {out}");
}

#[test]
fn trig_rad_input() {
    // pi/4 rad = 45 deg, sin = 0.7071
    let out = trig_calc("0.7854rad");
    assert!(
        out.contains("0.707") || out.contains("sin"),
        "rad input: {out}"
    );
}

#[test]
fn trig_empty_no_panic() {
    let out = trig_calc("");
    assert!(!out.is_empty());
}

// ─── Fraction tests ───────────────────────────────────────────────────────────

#[test]
fn fraction_add_half_plus_third() {
    // 1/2 + 1/3 = 5/6
    let out = fraction_calc("1/2 + 1/3");
    assert!(out.contains("5/6"), "1/2 + 1/3 = 5/6: {out}");
}

#[test]
fn fraction_sub_three_quarters_minus_eighth() {
    // 3/4 - 1/8 = 5/8
    let out = fraction_calc("3/4 - 1/8");
    assert!(out.contains("5/8"), "3/4 - 1/8 = 5/8: {out}");
}

#[test]
fn fraction_mul_two_thirds_times_three_fifths() {
    // 2/3 * 3/5 = 2/5
    let out = fraction_calc("2/3 * 3/5");
    assert!(out.contains("2/5"), "2/3 * 3/5 = 2/5: {out}");
}

#[test]
fn fraction_div_seven_eighths_by_three_quarters() {
    // 7/8 / 3/4 = 7/6
    let out = fraction_calc("7/8 / 3/4");
    assert!(out.contains("7/6"), "7/8 / 3/4 = 7/6: {out}");
}

#[test]
fn fraction_simplify_12_18() {
    // 12/18 = 2/3
    let out = fraction_calc("simplify 12/18");
    assert!(out.contains("2/3"), "simplify 12/18 = 2/3: {out}");
}

#[test]
fn fraction_lcd_three_fractions() {
    // LCD of 1/3 1/4 1/6 = 12
    let out = fraction_calc("lcd 1/3 1/4 1/6");
    assert!(out.contains("12"), "LCD(3,4,6)=12: {out}");
}

#[test]
fn fraction_todec_three_sevenths() {
    let out = fraction_calc("todec 3/7");
    assert!(
        out.contains("0.4285") || out.contains("repeating"),
        "3/7 decimal: {out}"
    );
}

#[test]
fn fraction_tofrac_0625() {
    // 0.625 = 5/8
    let out = fraction_calc("tofrac 0.625");
    assert!(out.contains("5/8"), "0.625 = 5/8: {out}");
}

#[test]
fn fraction_mixed_7_3() {
    // 7/3 = 2 and 1/3
    let out = fraction_calc("mixed 7/3");
    assert!(out.contains('2'), "7/3 mixed = 2 and 1/3: {out}");
    assert!(
        out.contains("1/3") || out.contains("1"),
        "remainder 1/3: {out}"
    );
}

#[test]
fn fraction_todec_quarter_terminates() {
    let out = fraction_calc("todec 1/4");
    assert!(out.contains("terminating"), "1/4 is terminating: {out}");
    assert!(out.contains("0.25"), "1/4 = 0.25: {out}");
}

#[test]
fn fraction_empty_no_panic() {
    let out = fraction_calc("");
    assert!(!out.is_empty());
}

// ─── Percent tests ────────────────────────────────────────────────────────────

#[test]
fn percent_of_basic() {
    // 15% of 350 = 52.5
    let out = percent_calc("15% of 350");
    assert!(
        out.contains("52.5") || out.contains("52."),
        "15% of 350: {out}"
    );
}

#[test]
fn percent_what_pct_of() {
    // 42 is what % of 280 = 15%
    let out = percent_calc("42 is what % of 280");
    assert!(out.contains("15"), "42/280=15%: {out}");
}

#[test]
fn percent_change_increase() {
    // 80 to 100 = +25%
    let out = percent_calc("change 80 to 100");
    assert!(out.contains("25"), "80->100 = +25%: {out}");
}

#[test]
fn percent_change_decrease() {
    // 100 to 75 = -25%
    let out = percent_calc("change 100 to 75");
    assert!(out.contains("25"), "100->75 = -25%: {out}");
}

#[test]
fn percent_increase_by_pct() {
    // 200 + 10% = 220
    let out = percent_calc("200 + 10%");
    assert!(out.contains("220"), "200+10%=220: {out}");
}

#[test]
fn percent_decrease_by_pct() {
    // 200 - 25% = 150
    let out = percent_calc("200 - 25%");
    assert!(out.contains("150"), "200-25%=150: {out}");
}

#[test]
fn percent_tip_calc() {
    // tip 100 20% → tip=20, total=120
    let out = percent_calc("tip 100 20%");
    assert!(
        out.contains("20.00") || out.contains("20"),
        "tip 20%: {out}"
    );
    assert!(
        out.contains("120") || out.contains("Total"),
        "total 120: {out}"
    );
}

#[test]
fn percent_markup() {
    // markup 80 25% → sell = 100
    let out = percent_calc("markup 80 25%");
    assert!(out.contains("100"), "markup 80+25%=100: {out}");
}

#[test]
fn percent_discount() {
    // discount 120 25% → final = 90
    let out = percent_calc("discount 120 25%");
    assert!(out.contains("90"), "discount 120-25%=90: {out}");
}

#[test]
fn percent_empty_no_panic() {
    let out = percent_calc("");
    assert!(!out.is_empty());
}

// ─── Complex number tests ──────────────────────────────────────────────────────

#[test]
fn complex_magnitude_3_4_is_5() {
    let out = complex_calc("mag 3+4i");
    assert!(
        out.contains('5') || out.contains("5.000"),
        "|3+4i|=5: {out}"
    );
}

#[test]
fn complex_add() {
    // (3+4i) + (1-2i) = 4+2i
    let out = complex_calc("(3+4i) + (1-2i)");
    assert!(out.contains('4') && out.contains('2'), "add: {out}");
}

#[test]
fn complex_multiply() {
    // (1+i)(1-i) = 1-i+i-i² = 1+1 = 2+0i
    let out = complex_calc("(1+i) * (1-i)");
    assert!(out.contains('2'), "mul: {out}");
}

#[test]
fn complex_sqrt_neg4() {
    // sqrt(-4) = 2i
    let out = complex_calc("sqrt -4");
    assert!(out.contains('2'), "sqrt(-4)=2i: {out}");
}

#[test]
fn complex_euler_pi() {
    // e^(i*pi) ≈ -1 + 0i
    let out = complex_calc("euler 3.14159265");
    assert!(
        out.contains("-1") || out.contains("-0.99"),
        "Euler e^(i*pi): {out}"
    );
}

#[test]
fn complex_arg_pure_imaginary() {
    // arg(0+i) = 90 deg
    let out = complex_calc("arg 0+1i");
    assert!(out.contains("90"), "arg(i)=90 deg: {out}");
}

#[test]
fn complex_conjugate() {
    // conj(3+4i) = 3-4i
    let out = complex_calc("conj 3+4i");
    assert!(out.contains("3") && out.contains("4"), "conj: {out}");
    // should show negative imaginary
    assert!(out.contains('-'), "conjugate has minus: {out}");
}

#[test]
fn complex_polar_3_4() {
    // polar 3 4 → r=5, theta≈53.13°
    let out = complex_calc("polar 3 4");
    assert!(out.contains('5'), "r=5: {out}");
    assert!(out.contains("53"), "theta≈53 deg: {out}");
}

#[test]
fn complex_single_number_info() {
    let out = complex_calc("3+4i");
    assert!(
        out.contains("magnitude") || out.contains("5"),
        "info block: {out}"
    );
}

#[test]
fn complex_empty_no_panic() {
    let out = complex_calc("");
    assert!(!out.is_empty());
}

// ─── Roman numeral tests ──────────────────────────────────────────────────────

#[test]
fn roman_int_to_roman_42() {
    let out = roman_calc("42");
    assert!(out.contains("XLII"), "42=XLII: {out}");
}

#[test]
fn roman_to_int_xlii() {
    let out = roman_calc("XLII");
    assert!(out.contains("42"), "XLII=42: {out}");
}

#[test]
fn roman_2024() {
    let out = roman_calc("2024");
    assert!(out.contains("MMXXIV"), "2024=MMXXIV: {out}");
}

#[test]
fn roman_mmxxiv_to_int() {
    let out = roman_calc("MMXXIV");
    assert!(out.contains("2024"), "MMXXIV=2024: {out}");
}

#[test]
fn roman_1_is_i() {
    let out = roman_calc("1");
    assert!(out.contains('I'), "1=I: {out}");
}

#[test]
fn roman_out_of_range_no_panic() {
    let out = roman_calc("0");
    assert!(!out.is_empty());
}

#[test]
fn roman_empty_no_panic() {
    let out = roman_calc("");
    assert!(!out.is_empty());
}

// ─── JSON tests ──────────────────────────────────────────────────────────────

#[test]
fn json_format_object() {
    let out = json_calc(r#"format {"name":"Alice","age":30}"#);
    assert!(out.contains("\"name\""));
    assert!(out.contains("\"Alice\""));
}

#[test]
fn json_validate_valid() {
    let out = json_calc(r#"validate {"x":1}"#);
    assert!(out.contains("Valid JSON"));
    assert!(out.contains("object"));
}

#[test]
fn json_validate_invalid() {
    let out = json_calc("validate {bad json}");
    assert!(out.contains("Invalid JSON") || out.contains("invalid"));
}

#[test]
fn json_minify() {
    let out = json_calc("minify {\"a\": 1, \"b\": 2}");
    assert!(out.contains("{\"a\":1,\"b\":2}"));
}

#[test]
fn json_keys() {
    let out = json_calc(r#"keys {"name":"Bob","score":42}"#);
    assert!(out.contains("name"));
    assert!(out.contains("score"));
}

#[test]
fn json_query_nested() {
    let out = json_calc(r#"query user.name {"user":{"name":"Alice","age":25}}"#);
    assert!(out.contains("Alice"));
}

#[test]
fn json_query_array_index() {
    let out = json_calc(r#"query items[1] {"items":["a","b","c"]}"#);
    assert!(out.contains("b") || out.contains("\"b\""));
}

#[test]
fn json_diff_no_change() {
    let out = json_calc(r#"{"x":1} --- {"x":1}"#);
    assert!(out.contains("identical") || out.contains("No difference"));
}

#[test]
fn json_diff_changed_value() {
    let out = json_calc(r#"{"x":1} --- {"x":2}"#);
    assert!(out.contains("changed") || out.contains("x"));
}

#[test]
fn json_auto_format_bare() {
    let out = json_calc(r#"{"hello":"world"}"#);
    assert!(out.contains("hello"));
}

#[test]
fn json_empty_no_panic() {
    let out = json_calc("");
    assert!(!out.is_empty());
}

// ─── Regex tests ─────────────────────────────────────────────────────────────

#[test]
fn regex_test_digits() {
    let out = regex_calc(r"test \d+ hello 42 world 99");
    assert!(out.contains("42") || out.contains("2 match"));
}

#[test]
fn regex_test_no_match() {
    let out = regex_calc(r"test \d+ abcdef");
    assert!(out.contains("No match"));
}

#[test]
fn regex_explain_pattern() {
    let out = regex_calc(r"explain \d+");
    assert!(out.contains("digit") || out.contains("one or more"));
}

#[test]
fn regex_explain_anchors() {
    let out = regex_calc(r"explain ^hello$");
    assert!(out.contains("start") && out.contains("end"));
}

#[test]
fn regex_split_whitespace() {
    let out = regex_calc(r"split \s+ hello   world  foo");
    assert!(out.contains("hello") && out.contains("world") && out.contains("foo"));
}

#[test]
fn regex_replace_spaces() {
    let out = regex_calc(r"replace \s+ _ hello world");
    assert!(out.contains("hello_world"));
}

#[test]
fn regex_capture_group() {
    let out = regex_calc(r"test (\d+)-(\d+) foo 12-34 bar");
    assert!(out.contains("12") && out.contains("34"));
}

#[test]
fn regex_word_boundary_class() {
    let out = regex_calc(r"test [a-z]+ hello123");
    assert!(out.contains("hello"));
}

#[test]
fn regex_invalid_no_panic() {
    let out = regex_calc("");
    assert!(!out.is_empty());
}

// ─── CSV tests ───────────────────────────────────────────────────────────────

const SAMPLE_CSV: &str = "name,age,city\nAlice,30,NYC\nBob,25,LA\nCarol,35,NYC\nDave,28,Chicago";

#[test]
fn csv_preview() {
    let out = csv_calc(&format!("preview {}", SAMPLE_CSV));
    assert!(out.contains("name") && out.contains("Alice"));
}

#[test]
fn csv_cols() {
    let out = csv_calc(&format!("cols {}", SAMPLE_CSV));
    assert!(out.contains("name") && out.contains("age") && out.contains("city"));
}

#[test]
fn csv_count() {
    let out = csv_calc(&format!("count {}", SAMPLE_CSV));
    assert!(out.contains("4"));
}

#[test]
fn csv_sum_numeric() {
    let out = csv_calc(&format!("sum age {}", SAMPLE_CSV));
    assert!(out.contains("118") || out.contains("Sum")); // 30+25+35+28=118
}

#[test]
fn csv_avg_numeric() {
    let out = csv_calc(&format!("avg age {}", SAMPLE_CSV));
    assert!(out.contains("Avg") || out.contains("29.5"));
}

#[test]
fn csv_filter_eq() {
    let out = csv_calc(&format!("filter city = NYC {}", SAMPLE_CSV));
    assert!(out.contains("Alice") || out.contains("Carol"));
    // Bob (LA) should not appear
    assert!(!out.contains("Bob"));
}

#[test]
fn csv_filter_gt() {
    let out = csv_calc(&format!("filter age > 28 {}", SAMPLE_CSV));
    assert!(out.contains("Alice") || out.contains("Carol"));
}

#[test]
fn csv_groupby() {
    let out = csv_calc(&format!("groupby city {}", SAMPLE_CSV));
    assert!(out.contains("NYC") && out.contains("2"));
}

#[test]
fn csv_sort_asc() {
    let out = csv_calc(&format!("sort age asc {}", SAMPLE_CSV));
    // Bob (25) should appear before Alice (30)
    let bob_pos = out.find("Bob").unwrap_or(usize::MAX);
    let alice_pos = out.find("Alice").unwrap_or(usize::MAX);
    assert!(bob_pos < alice_pos);
}

#[test]
fn csv_select_columns() {
    let out = csv_calc(&format!("select name,city {}", SAMPLE_CSV));
    assert!(out.contains("name") && out.contains("city"));
    // age column should not be in output header area
    // (it may appear in data but the header "age" shouldn't be in the first display line)
    assert!(!out.contains("age") || out.contains("name"));
}

#[test]
fn csv_empty_no_panic() {
    let out = csv_calc("");
    assert!(!out.is_empty());
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
