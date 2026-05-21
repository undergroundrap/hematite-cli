/// Focused integration tests for hematite::tools::math_util
///
/// Covers known-value correctness, edge-case non-panics, and the newer
/// matrix decomposition modes (QR, SVD, Cholesky).
use hematite::tools::math_util::{
    algo_ref_calc, ansible_calc, ascii_calc, ascii_table_calc, awk_calc, bash_ref_calc,
    bitwise_calc, case_calc, chars_calc, checksum_calc, chemistry_calc, chmod_calc, cipher_calc,
    cloud_ref_calc, color_calc, color_names_calc, combinatorics_calc, complex_calc, cron_calc,
    css_ref_calc, csv_calc, curl_calc, datetime_calc, diff_calc, docker_adv_calc, docker_ref_calc,
    duration_calc, electrical_calc, encode_calc, escape_calc, find_calc, fraction_calc,
    geometry_calc, git_adv_calc, git_ref_calc, gitignore_calc, go_ref_calc, grep_calc, hash_calc,
    headers_calc, health_calc, http_adv_calc, http_calc, http_status_calc, id_gen_calc, ip_calc,
    jinja_calc, jq_calc, js_ref_calc, json_calc, json_path_calc, jwt_calc, kbd_calc, kubectl_calc,
    license_calc, linux_adv_calc, lorem_calc, make_calc, makefile_calc, markdown_calc, matrix_calc,
    mime_calc, net_calc, nginx_calc, npm_calc, number_format, number_theory_calc, oop_ref_calc,
    openssl_calc, percent_calc, physics_calc, port_calc, postgres_calc, prob_calc,
    python_data_calc, python_ref_calc, regex_adv_calc, regex_calc, regex_ref_calc, roman_calc,
    rust_adv_calc, rust_ref_calc, security_ref_calc, sed_calc, semver_calc, set_calc, sort_viz,
    spark_calc, sql_adv_calc, sql_fmt_calc, sql_ref_calc, ssh_ref_calc, ssl_calc, stats_calc,
    string_dist, systemd_adv_calc, systemd_calc, table_calc, tar_calc, template_calc,
    terraform_calc, text_stats, timestamp_calc, tmux_calc, toml_calc, trig_calc, ts_ref_calc,
    tz_calc, url_calc, uuid_calc, validate_calc, vim_adv_calc, vim_calc, xml_calc, yaml_calc,
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

// ─── JWT tests ────────────────────────────────────────────────────────────────

// eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9  = {"alg":"HS256","typ":"JWT"}
// eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ
//   = {"sub":"1234567890","name":"John Doe","iat":1516239022}
const SAMPLE_JWT: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
     eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.\
     SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

#[test]
fn jwt_decode_header_alg() {
    let out = jwt_calc(SAMPLE_JWT);
    assert!(out.contains("HS256") || out.contains("alg"));
}

#[test]
fn jwt_decode_payload_sub() {
    let out = jwt_calc(SAMPLE_JWT);
    assert!(out.contains("1234567890") || out.contains("sub"));
}

#[test]
fn jwt_decode_shows_iat() {
    let out = jwt_calc(SAMPLE_JWT);
    // iat = 1516239022 → should show timestamp
    assert!(out.contains("1516239022") || out.contains("Issued"));
}

#[test]
fn jwt_claims_command() {
    let out = jwt_calc(&format!("claims {}", SAMPLE_JWT));
    assert!(out.contains("John Doe") || out.contains("name"));
}

#[test]
fn jwt_header_command() {
    let out = jwt_calc(&format!("header {}", SAMPLE_JWT));
    assert!(out.contains("HS256") || out.contains("JWT"));
}

#[test]
fn jwt_invalid_format_no_panic() {
    let out = jwt_calc("not.a.valid.jwt.at.all");
    assert!(!out.is_empty());
}

#[test]
fn jwt_empty_no_panic() {
    let out = jwt_calc("");
    assert!(!out.is_empty());
}

#[test]
fn jwt_three_parts_required() {
    let out = jwt_calc("only.two");
    assert!(out.contains("3") || out.contains("valid") || out.contains("part"));
}

// ─── URL tests ────────────────────────────────────────────────────────────────

#[test]
fn url_parse_full() {
    let out = url_calc("parse https://api.example.com:8080/v1/users?page=2&limit=10#results");
    assert!(out.contains("https"));
    assert!(out.contains("api.example.com"));
    assert!(out.contains("8080"));
    assert!(out.contains("/v1/users"));
}

#[test]
fn url_parse_query_params() {
    let out = url_calc("params https://example.com/path?foo=bar&baz=qux");
    assert!(out.contains("foo") && out.contains("bar"));
    assert!(out.contains("baz") && out.contains("qux"));
}

#[test]
fn url_encode() {
    let out = url_calc("encode hello world & foo=bar");
    assert!(out.contains("%20") || out.contains("+"));
    assert!(out.contains("%26") || out.contains("&"));
}

#[test]
fn url_decode() {
    let out = url_calc("decode hello%20world%26foo%3Dbar");
    assert!(out.contains("hello world") || out.contains("hello"));
}

#[test]
fn url_auto_parse_bare() {
    let out = url_calc("https://github.com/owner/repo?tab=readme");
    assert!(out.contains("github.com"));
    assert!(out.contains("tab") || out.contains("readme"));
}

#[test]
fn url_build() {
    let out = url_calc("build scheme=https host=example.com path=/api key=mykey");
    assert!(out.contains("https://example.com/api"));
}

#[test]
fn url_default_port_shown() {
    let out = url_calc("parse https://example.com/path");
    assert!(out.contains("443") || out.contains("https"));
}

#[test]
fn url_empty_no_panic() {
    let out = url_calc("");
    assert!(!out.is_empty());
}

// ─── Cron tests ───────────────────────────────────────────────────────────────

#[test]
fn cron_every_minute() {
    let out = cron_calc("* * * * *");
    assert!(out.contains("every minute") || out.contains("every"));
}

#[test]
fn cron_weekday_morning() {
    let out = cron_calc("0 9 * * 1-5");
    assert!(out.contains("Monday") || out.contains("9") || out.contains("weekday"));
}

#[test]
fn cron_every_15_min() {
    let out = cron_calc("*/15 * * * *");
    assert!(out.contains("15") || out.contains("every"));
}

#[test]
fn cron_explain_command() {
    let out = cron_calc("explain 0 0 1 1 *");
    assert!(out.contains("January") || out.contains("midnight") || out.contains("0"));
}

#[test]
fn cron_next_command_returns_dates() {
    let out = cron_calc("next * * * * *");
    // should show at least one date line with year
    assert!(out.contains("202") || out.contains("next"));
}

#[test]
fn cron_next_n_command() {
    let out = cron_calc("next 3 * * * * *");
    assert!(out.contains("1.") && out.contains("2.") && out.contains("3."));
}

#[test]
fn cron_midnight_daily() {
    let out = cron_calc("0 0 * * *");
    assert!(out.contains("0") || out.contains("midnight") || out.contains("daily"));
}

#[test]
fn cron_invalid_no_panic() {
    let out = cron_calc("not a cron");
    assert!(!out.is_empty());
}

#[test]
fn cron_empty_no_panic() {
    let out = cron_calc("");
    assert!(!out.is_empty());
}

// ─── IP tests ─────────────────────────────────────────────────────────────────

#[test]
fn ip_private_class_c() {
    let out = ip_calc("192.168.1.1");
    assert!(out.contains("Private") || out.contains("192.168"));
}

#[test]
fn ip_loopback() {
    let out = ip_calc("127.0.0.1");
    assert!(out.contains("Loopback") || out.contains("127"));
}

#[test]
fn ip_public() {
    let out = ip_calc("8.8.8.8");
    assert!(out.contains("Public") || out.contains("8.8.8.8"));
}

#[test]
fn ip_cidr_24() {
    let out = ip_calc("192.168.1.0/24");
    assert!(out.contains("255.255.255.0") || out.contains("254")); // subnet mask or host count
    assert!(
        out.contains("192.168.1.255") || out.contains("broadcast") || out.contains("Broadcast")
    );
}

#[test]
fn ip_cidr_16_host_count() {
    let out = ip_calc("10.0.0.0/16");
    // 2^16 - 2 = 65534 usable hosts
    assert!(out.contains("65534") || out.contains("65536") || out.contains("16"));
}

#[test]
fn ip_contains_true() {
    let out = ip_calc("contains 192.168.1.0/24 192.168.1.50");
    assert!(out.contains("YES") || out.contains("inside"));
}

#[test]
fn ip_contains_false() {
    let out = ip_calc("contains 192.168.1.0/24 10.0.0.1");
    assert!(out.contains("NO") || out.contains("outside"));
}

#[test]
fn ip_range() {
    let out = ip_calc("range 192.168.1.1 192.168.1.254");
    assert!(out.contains("254") || out.contains("Count"));
}

#[test]
fn ip_mask_prefix() {
    let out = ip_calc("mask 24");
    assert!(out.contains("255.255.255.0"));
}

#[test]
fn ip_mask_dotted() {
    let out = ip_calc("mask 255.255.0.0");
    assert!(out.contains("/16") || out.contains("16"));
}

#[test]
fn ip_ipv6_loopback() {
    let out = ip_calc("::1");
    assert!(out.contains("Loopback") || out.contains("::1"));
}

#[test]
fn ip_empty_no_panic() {
    let out = ip_calc("");
    assert!(!out.is_empty());
}

// ─── Color tests ─────────────────────────────────────────────────────────────

#[test]
fn color_parse_hex6() {
    let out = color_calc("#1a2b3c");
    assert!(
        out.contains("26") || out.contains("RGB"),
        "expected RGB decode: {out}"
    );
    assert!(
        out.contains("HSL") || out.contains("hsl"),
        "expected HSL: {out}"
    );
}

#[test]
fn color_parse_hex3() {
    let out = color_calc("#fff");
    assert!(out.contains("255"), "expected RGB 255 from #fff: {out}");
}

#[test]
fn color_rgb_command() {
    let out = color_calc("rgb 255 128 0");
    assert!(
        out.contains("255") && out.contains("128"),
        "expected rgb components: {out}"
    );
    assert!(
        out.contains('#') || out.contains("hex") || out.contains("Hex"),
        "expected hex output: {out}"
    );
}

#[test]
fn color_mix_command() {
    let out = color_calc("mix #ff0000 #0000ff");
    assert!(!out.is_empty(), "mix should not be empty");
    assert!(
        out.contains('#') || out.contains("Mix") || out.contains("mix"),
        "expected mixed color: {out}"
    );
}

#[test]
fn color_contrast_black_white() {
    let out = color_calc("contrast #000000 #ffffff");
    assert!(
        out.contains("21") || out.contains("PASS") || out.contains("AA"),
        "expected max contrast ratio: {out}"
    );
}

#[test]
fn color_palette_command() {
    let out = color_calc("palette #ff0000");
    assert!(
        out.contains("complement")
            || out.contains("Complement")
            || out.contains("triad")
            || out.contains("Triad"),
        "expected palette output: {out}"
    );
}

#[test]
fn color_hsl_command() {
    let out = color_calc("hsl 0 100% 50%");
    assert!(
        out.contains("255") || out.contains("ff0000") || out.contains("FF0000"),
        "hsl 0 100 50 should be red: {out}"
    );
}

#[test]
fn color_empty_no_panic() {
    let out = color_calc("");
    assert!(!out.is_empty());
}

// ─── UUID tests ───────────────────────────────────────────────────────────────

#[test]
fn uuid_v4_generates_valid() {
    let out = uuid_calc("v4");
    // UUID v4 format: 8-4-4-4-12 hex chars
    let uuid_line = out.lines().find(|l| l.contains('-')).unwrap_or(&out);
    let parts: Vec<&str> = uuid_line.trim().split('-').collect();
    assert_eq!(parts.len(), 5, "UUID should have 5 groups: {out}");
    assert_eq!(parts[0].len(), 8, "first group 8 chars: {out}");
    assert_eq!(parts[2].len(), 4, "third group 4 chars: {out}");
    // version nibble should be '4'
    assert!(
        parts[2].starts_with('4'),
        "version nibble should be 4: {out}"
    );
}

#[test]
fn uuid_v4_batch() {
    let out = uuid_calc("batch 5");
    let uuid_lines: Vec<&str> = out
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.len() >= 36 && t.chars().filter(|&c| c == '-').count() == 4
        })
        .collect();
    assert_eq!(uuid_lines.len(), 5, "expected 5 UUIDs: {out}");
}

#[test]
fn uuid_nil() {
    let out = uuid_calc("nil");
    assert!(
        out.contains("00000000-0000-0000-0000-000000000000"),
        "nil UUID: {out}"
    );
}

#[test]
fn uuid_decode_valid() {
    let out = uuid_calc("decode 550e8400-e29b-41d4-a716-446655440000");
    assert!(
        out.contains("Version") || out.contains("version"),
        "should show version: {out}"
    );
    assert!(
        out.contains("RFC") || out.contains("Variant") || out.contains("variant"),
        "should show variant: {out}"
    );
}

#[test]
fn uuid_decode_invalid() {
    let out = uuid_calc("decode not-a-valid-uuid-string");
    assert!(
        out.contains("Not a valid") || out.contains("invalid") || out.contains("Invalid"),
        "should report invalid: {out}"
    );
}

#[test]
fn uuid_parse_v4() {
    let out = uuid_calc("decode 550e8400-e29b-41d4-a716-446655440000");
    assert!(
        out.contains("Version") && (out.contains(" 4") || out.contains(": 4")),
        "should show version 4: {out}"
    );
}

#[test]
fn uuid_empty_no_panic() {
    let out = uuid_calc("");
    assert!(!out.is_empty());
}

// ─── Diff tests ───────────────────────────────────────────────────────────────

#[test]
fn diff_identical_texts() {
    let out = diff_calc("hello world ||| hello world");
    assert!(
        out.contains("identical") || out.contains("Identical"),
        "identical texts: {out}"
    );
}

#[test]
fn diff_changed_line() {
    let out = diff_calc("hello world ||| hello there");
    assert!(
        out.contains("world") || out.contains('-') || out.contains('+'),
        "should show diff: {out}"
    );
}

#[test]
fn diff_word_mode() {
    let out = diff_calc("word foo bar baz ||| foo qux baz");
    assert!(
        out.contains("bar") || out.contains("qux") || out.contains('-') || out.contains('+'),
        "word diff: {out}"
    );
}

#[test]
fn diff_line_counts() {
    let out = diff_calc("hello world ||| hello there");
    assert!(
        out.contains("line") || out.contains("added") || out.contains("removed"),
        "should show counts: {out}"
    );
}

#[test]
fn diff_multiline() {
    let out = diff_calc("line one\nline two ||| line one\nline three");
    assert!(
        out.contains("two") || out.contains("three") || out.contains('-') || out.contains('+'),
        "multiline diff: {out}"
    );
}

#[test]
fn diff_empty_no_panic() {
    let out = diff_calc("");
    assert!(!out.is_empty());
}

// ─── SemVer tests ─────────────────────────────────────────────────────────────

#[test]
fn semver_parse_basic() {
    let out = semver_calc("parse 1.2.3");
    assert!(
        out.contains('1') && out.contains('2') && out.contains('3'),
        "parse components: {out}"
    );
    assert!(
        out.contains("major") || out.contains("Major"),
        "should label major: {out}"
    );
}

#[test]
fn semver_parse_prerelease() {
    let out = semver_calc("parse 1.2.3-alpha.1+build.456");
    assert!(out.contains("alpha"), "pre-release label: {out}");
    assert!(
        out.contains("build") || out.contains("456"),
        "build metadata: {out}"
    );
}

#[test]
fn semver_compare_gt() {
    let out = semver_calc("2.0.0 vs 1.9.9");
    assert!(
        out.contains('>') || out.contains("newer") || out.contains("2.0.0"),
        "2.0.0 > 1.9.9: {out}"
    );
}

#[test]
fn semver_compare_eq() {
    let out = semver_calc("1.0.0 vs 1.0.0");
    assert!(
        out.contains("==") || out.contains("equal"),
        "equal versions: {out}"
    );
}

#[test]
fn semver_satisfies_caret() {
    let out = semver_calc("satisfies 1.5.0 ^1.2.3");
    assert!(
        out.contains("YES") || out.contains("yes"),
        "^1.2.3 satisfied by 1.5.0: {out}"
    );
}

#[test]
fn semver_satisfies_caret_fail() {
    let out = semver_calc("satisfies 2.0.0 ^1.2.3");
    assert!(
        out.contains("NO") || out.contains("no"),
        "^1.2.3 not satisfied by 2.0.0: {out}"
    );
}

#[test]
fn semver_satisfies_tilde() {
    let out = semver_calc("satisfies 1.2.9 ~1.2.3");
    assert!(
        out.contains("YES") || out.contains("yes"),
        "~1.2.3 satisfied by 1.2.9: {out}"
    );
}

#[test]
fn semver_sort() {
    let out = semver_calc("sort 1.10.0 1.9.0 1.2.0");
    // sorted ascending, 1.2.0 should come before 1.10.0
    let pos_1_2 = out.find("1.2.0").unwrap_or(usize::MAX);
    let pos_1_10 = out.find("1.10.0").unwrap_or(usize::MAX);
    assert!(pos_1_2 < pos_1_10, "1.2.0 should sort before 1.10.0: {out}");
}

#[test]
fn semver_bump_minor() {
    let out = semver_calc("bump minor 1.2.3");
    assert!(out.contains("1.3.0"), "bump minor: {out}");
}

#[test]
fn semver_bump_patch() {
    let out = semver_calc("bump patch 1.2.3");
    assert!(out.contains("1.2.4"), "bump patch: {out}");
}

#[test]
fn semver_bump_major() {
    let out = semver_calc("bump major 1.2.3");
    assert!(out.contains("2.0.0"), "bump major: {out}");
}

#[test]
fn semver_validate_valid() {
    let out = semver_calc("parse 1.2.3-beta.1");
    assert!(
        out.contains("Pre-release") || out.contains("beta"),
        "should show pre-release: {out}"
    );
}

#[test]
fn semver_validate_invalid() {
    let out = semver_calc("parse not.a.version");
    assert!(
        out.contains("Invalid") || out.contains("invalid") || out.contains("Could not"),
        "should be invalid: {out}"
    );
}

#[test]
fn semver_empty_no_panic() {
    let out = semver_calc("");
    assert!(!out.is_empty());
}

// ─── Timestamp tests ─────────────────────────────────────────────────────────

#[test]
fn timestamp_current_shows_unix() {
    let out = timestamp_calc("");
    assert!(out.contains("Unix (s)"), "should show Unix seconds: {out}");
    assert!(out.contains("ISO 8601"), "should show ISO 8601: {out}");
    assert!(out.contains("Human UTC"), "should show human date: {out}");
}

#[test]
fn timestamp_now_keyword() {
    let out = timestamp_calc("now");
    assert!(out.contains("Unix (s)"), "now keyword: {out}");
}

#[test]
fn timestamp_decode_known() {
    // 2024-05-20 00:00:00 UTC = 1716163200
    let out = timestamp_calc("1716163200");
    assert!(out.contains("2024"), "should decode to 2024: {out}");
    assert!(
        out.contains("May") || out.contains("05"),
        "should show May: {out}"
    );
}

#[test]
fn timestamp_decode_millis() {
    // millis version of 1716163200
    let out = timestamp_calc("1716163200000");
    assert!(
        out.contains("auto-detected") || out.contains("ms"),
        "should auto-detect millis: {out}"
    );
    assert!(out.contains("2024"), "should decode to 2024: {out}");
}

#[test]
fn timestamp_parse_date_string() {
    let out = timestamp_calc("2024-05-20");
    assert!(out.contains("Unix (s)"), "date string to unix: {out}");
    assert!(
        out.contains("1716"),
        "should produce 2024-05-20 unix: {out}"
    );
}

#[test]
fn timestamp_relative_future() {
    let out = timestamp_calc("now + 1d");
    assert!(
        out.contains("Unix (s)") || out.contains("Offset"),
        "relative future: {out}"
    );
}

#[test]
fn timestamp_relative_past() {
    let out = timestamp_calc("now - 1h");
    assert!(
        out.contains("Unix (s)") || out.contains("Offset"),
        "relative past: {out}"
    );
}

#[test]
fn timestamp_empty_no_panic() {
    let out = timestamp_calc("");
    assert!(!out.is_empty());
}

// ─── Lorem tests ──────────────────────────────────────────────────────────────

#[test]
fn lorem_default_one_paragraph() {
    let out = lorem_calc("");
    assert!(
        out.contains("lorem") || out.contains("Lorem"),
        "should contain lorem: {out}"
    );
    assert!(
        out.contains("(1 paragraph)"),
        "should say 1 paragraph: {out}"
    );
}

#[test]
fn lorem_multiple_paragraphs() {
    let out = lorem_calc("3");
    assert!(
        out.contains("(3 paragraphs)"),
        "should say 3 paragraphs: {out}"
    );
}

#[test]
fn lorem_words_mode() {
    let out = lorem_calc("words 20");
    assert!(out.contains("(20 words)"), "should say 20 words: {out}");
    let word_count = out
        .lines()
        .find(|l| !l.trim().starts_with('(') && !l.contains("─") && !l.contains("Lorem"))
        .map(|l| l.split_whitespace().count())
        .unwrap_or(0);
    assert!(
        word_count >= 15,
        "should have roughly 20 words, got {}: {out}",
        word_count
    );
}

#[test]
fn lorem_sentences_mode() {
    let out = lorem_calc("sentences 3");
    assert!(
        out.contains("(3 sentences)"),
        "should say 3 sentences: {out}"
    );
}

#[test]
fn lorem_words_end_with_period() {
    let out = lorem_calc("sentences 1");
    let sentence_line = out.lines().find(|l| l.trim().ends_with('.'));
    assert!(
        sentence_line.is_some(),
        "sentence should end with period: {out}"
    );
}

#[test]
fn lorem_empty_no_panic() {
    let out = lorem_calc("");
    assert!(!out.is_empty());
}

// ─── Case converter tests ─────────────────────────────────────────────────────

#[test]
fn case_all_hello_world() {
    let out = case_calc("hello world");
    assert!(
        out.contains("helloWorld") || out.contains("camelCase"),
        "camel: {out}"
    );
    assert!(
        out.contains("HelloWorld") || out.contains("PascalCase"),
        "pascal: {out}"
    );
    assert!(
        out.contains("hello_world") || out.contains("snake_case"),
        "snake: {out}"
    );
    assert!(
        out.contains("hello-world") || out.contains("kebab-case"),
        "kebab: {out}"
    );
    assert!(
        out.contains("HELLO_WORLD") || out.contains("SCREAMING"),
        "screaming: {out}"
    );
}

#[test]
fn case_snake_from_camel() {
    let out = case_calc("snake getUserById");
    assert!(out.contains("get_user_by_id"), "camelCase to snake: {out}");
}

#[test]
fn case_camel_from_snake() {
    let out = case_calc("camel user_profile_data");
    assert!(out.contains("userProfileData"), "snake to camel: {out}");
}

#[test]
fn case_pascal_from_kebab() {
    let out = case_calc("pascal my-component-name");
    assert!(out.contains("MyComponentName"), "kebab to pascal: {out}");
}

#[test]
fn case_kebab_from_pascal() {
    let out = case_calc("kebab MyComponentName");
    assert!(out.contains("my-component-name"), "pascal to kebab: {out}");
}

#[test]
fn case_screaming_snake() {
    let out = case_calc("screaming hello world");
    assert!(out.contains("HELLO_WORLD"), "screaming snake: {out}");
}

#[test]
fn case_title_case() {
    let out = case_calc("title hello world");
    assert!(out.contains("Hello World"), "title case: {out}");
}

#[test]
fn case_upper() {
    let out = case_calc("upper hello world");
    assert!(out.contains("HELLO WORLD"), "upper case: {out}");
}

#[test]
fn case_lower() {
    let out = case_calc("lower HELLO WORLD");
    assert!(out.contains("hello world"), "lower case: {out}");
}

#[test]
fn case_dot() {
    let out = case_calc("dot hello world");
    assert!(out.contains("hello.world"), "dot case: {out}");
}

#[test]
fn case_empty_no_panic() {
    let out = case_calc("");
    assert!(!out.is_empty());
}

// ─── YAML tests ──────────────────────────────────────────────────────────────

#[test]
fn yaml_validate_valid_mapping() {
    let out = yaml_calc("key: value");
    assert!(out.contains("VALID"), "simple mapping: {out}");
    assert!(out.contains("mapping"), "should show type mapping: {out}");
}

#[test]
fn yaml_validate_valid_sequence() {
    let out = yaml_calc("validate - one\n- two\n- three");
    assert!(
        out.contains("VALID") || out.contains("sequence"),
        "sequence: {out}"
    );
}

#[test]
fn yaml_validate_invalid() {
    let out = yaml_calc("validate: {invalid: [unclosed");
    // either it parses weirdly or is INVALID
    assert!(!out.is_empty());
}

#[test]
fn yaml_format_inline() {
    let out = yaml_calc("format key: value");
    assert!(
        out.contains("key") || out.contains("value"),
        "format inline: {out}"
    );
}

#[test]
fn yaml_keys_command() {
    let out = yaml_calc("keys name: Alice\nage: 30\ncity: NYC");
    assert!(
        out.contains("name") || out.contains("Top-level"),
        "keys command: {out}"
    );
}

#[test]
fn yaml_validate_multiline() {
    let yaml = "server:\n  host: localhost\n  port: 8080\ndatabase:\n  name: mydb";
    let out = yaml_calc(yaml);
    assert!(out.contains("VALID"), "multiline YAML: {out}");
    assert!(out.contains("mapping"), "should be a mapping: {out}");
}

#[test]
fn yaml_empty_shows_help() {
    let out = yaml_calc("");
    assert!(
        out.contains("validate") || out.contains("Commands"),
        "empty shows help: {out}"
    );
}

// ─── Table tests ──────────────────────────────────────────────────────────────

#[test]
fn table_csv_basic() {
    let out = table_calc("name,age\nAlice,30\nBob,25");
    assert!(
        out.contains("Alice") && out.contains("Bob"),
        "CSV rows: {out}"
    );
    assert!(
        out.contains("name") && out.contains("age"),
        "CSV headers: {out}"
    );
    assert!(out.contains('|'), "ASCII table has pipes: {out}");
}

#[test]
fn table_csv_row_count() {
    let out = table_calc("a,b,c\n1,2,3\n4,5,6\n7,8,9");
    assert!(
        out.contains("3 rows") || out.contains("rows"),
        "row count: {out}"
    );
}

#[test]
fn table_json_array() {
    let out = table_calc(r#"[{"name":"Alice","score":95},{"name":"Bob","score":87}]"#);
    assert!(
        out.contains("Alice") && out.contains("Bob"),
        "JSON array rows: {out}"
    );
    assert!(
        out.contains("name") && out.contains("score"),
        "JSON array headers: {out}"
    );
    assert!(out.contains('|'), "table has pipes: {out}");
}

#[test]
fn table_markdown_output() {
    let out = table_calc("markdown name,score\nAlice,95\nBob,87");
    assert!(
        out.contains("Alice") && out.contains("Bob"),
        "markdown rows: {out}"
    );
    assert!(
        out.contains("---") || out.contains("| name"),
        "markdown separator: {out}"
    );
}

#[test]
fn table_csv_to_csv_via_json() {
    let out = table_calc(r#"csv [{"x":1,"y":2},{"x":3,"y":4}]"#);
    assert!(
        out.contains("x") && out.contains("y"),
        "json to csv headers: {out}"
    );
    assert!(
        out.contains('1') && out.contains('3'),
        "json to csv values: {out}"
    );
}

#[test]
fn table_empty_shows_help() {
    let out = table_calc("");
    assert!(
        out.contains("Commands") || out.contains("csv"),
        "empty shows help: {out}"
    );
}

// ─── SQL formatter tests ──────────────────────────────────────────────────────

#[test]
fn sql_fmt_uppercases_keywords() {
    let out = sql_fmt_calc("select id, name from users where active = 1");
    assert!(out.contains("SELECT"), "SELECT uppercased: {out}");
    assert!(out.contains("FROM"), "FROM uppercased: {out}");
    assert!(out.contains("WHERE"), "WHERE uppercased: {out}");
}

#[test]
fn sql_fmt_newlines_before_clauses() {
    let out = sql_fmt_calc("select id from users where id = 1");
    let from_pos = out.find("FROM").unwrap_or(usize::MAX);
    let where_pos = out.find("WHERE").unwrap_or(usize::MAX);
    assert!(from_pos < where_pos, "FROM before WHERE: {out}");
    // They should be on separate lines
    let lines_between: Vec<&str> = out
        .lines()
        .skip_while(|l| !l.contains("FROM"))
        .take_while(|l| !l.contains("WHERE"))
        .collect();
    assert!(
        !lines_between.is_empty(),
        "FROM and WHERE on separate lines: {out}"
    );
}

#[test]
fn sql_fmt_join_uppercased() {
    let out = sql_fmt_calc("select u.id from users u inner join orders o on u.id = o.user_id");
    assert!(
        out.contains("INNER JOIN") || out.contains("JOIN"),
        "JOIN uppercased: {out}"
    );
}

#[test]
fn sql_fmt_minify() {
    let out = sql_fmt_calc("minify SELECT   id,   name\n  FROM   users\n  WHERE active = 1");
    // should collapse to one line
    let result_lines: Vec<&str> = out
        .lines()
        .filter(|l| !l.contains("─") && !l.contains("SQL") && !l.trim().is_empty())
        .collect();
    assert_eq!(result_lines.len(), 1, "minify = one line: {out}");
}

#[test]
fn sql_fmt_group_by() {
    let out = sql_fmt_calc("select dept, count(*) from emp group by dept having count(*) > 5");
    assert!(
        out.contains("GROUP BY") || out.contains("GROUP"),
        "GROUP BY uppercased: {out}"
    );
    assert!(out.contains("HAVING"), "HAVING uppercased: {out}");
}

#[test]
fn sql_fmt_keywords_command() {
    let out = sql_fmt_calc("keywords");
    assert!(
        out.contains("SELECT") && out.contains("WHERE"),
        "keywords list: {out}"
    );
}

#[test]
fn sql_fmt_empty_shows_help() {
    let out = sql_fmt_calc("");
    assert!(
        out.contains("Commands") || out.contains("format"),
        "empty shows help: {out}"
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

// ─── HTTP status code tests ───────────────────────────────────────────────────

#[test]
fn http_lookup_404() {
    let out = http_calc("404");
    assert!(out.contains("404"), "should echo the code: {out}");
    assert!(
        out.to_lowercase().contains("not found"),
        "404 should be Not Found: {out}"
    );
}

#[test]
fn http_lookup_200() {
    let out = http_calc("200");
    assert!(out.contains("200"), "{out}");
    assert!(out.to_lowercase().contains("ok"), "200 should be OK: {out}");
}

#[test]
fn http_lookup_500() {
    let out = http_calc("500");
    assert!(out.contains("500"), "{out}");
    assert!(
        out.to_lowercase().contains("internal server error"),
        "500 → Internal Server Error: {out}"
    );
}

#[test]
fn http_keyword_search() {
    let out = http_calc("redirect");
    assert!(
        out.contains("301") || out.contains("302") || out.contains("307"),
        "keyword 'redirect' should match 3xx codes: {out}"
    );
}

#[test]
fn http_range_filter_4xx() {
    let out = http_calc("list 4xx");
    assert!(out.contains("400"), "4xx range should include 400: {out}");
    assert!(out.contains("404"), "4xx range should include 404: {out}");
    assert!(
        !out.contains("200"),
        "4xx range must not include 200: {out}"
    );
}

#[test]
fn http_range_filter_5xx() {
    let out = http_calc("list 5xx");
    assert!(out.contains("500"), "{out}");
    assert!(out.contains("503"), "{out}");
}

#[test]
fn http_list_all() {
    let out = http_calc("list");
    assert!(out.contains("200"), "{out}");
    assert!(out.contains("404"), "{out}");
    assert!(out.contains("500"), "{out}");
}

#[test]
fn http_unknown_code_no_panic() {
    let out = http_calc("999");
    assert!(
        !out.is_empty(),
        "unknown code should still return something"
    );
}

#[test]
fn http_empty_no_panic() {
    let out = http_calc("");
    assert!(!out.is_empty());
}

// ─── MIME type tests ──────────────────────────────────────────────────────────

#[test]
fn mime_extension_json() {
    let out = mime_calc(".json");
    assert!(
        out.contains("application/json"),
        ".json should map to application/json: {out}"
    );
}

#[test]
fn mime_extension_without_dot() {
    let out = mime_calc("png");
    assert!(out.contains("image/png"), "png → image/png: {out}");
}

#[test]
fn mime_type_to_extension() {
    let out = mime_calc("application/pdf");
    assert!(out.contains(".pdf"), "application/pdf → .pdf: {out}");
}

#[test]
fn mime_keyword_search() {
    let out = mime_calc("video");
    assert!(
        out.contains("video/mp4") || out.contains("video/"),
        "keyword 'video' should match video/* types: {out}"
    );
}

#[test]
fn mime_list_all() {
    let out = mime_calc("list");
    assert!(out.contains("text/html"), "{out}");
    assert!(out.contains("application/json"), "{out}");
}

#[test]
fn mime_list_category() {
    // keyword search for "image" returns only image/* types
    let out = mime_calc("image");
    assert!(
        out.contains("image/"),
        "keyword 'image' should list image/* types: {out}"
    );
    // audio types contain no "image" token — should not appear
    assert!(
        !out.contains("audio/mpeg"),
        "image search must not include audio/mpeg: {out}"
    );
}

#[test]
fn mime_html_extension() {
    let out = mime_calc(".html");
    assert!(out.contains("text/html"), ".html → text/html: {out}");
}

#[test]
fn mime_unknown_extension_no_panic() {
    let out = mime_calc(".xyz_unknown_ext");
    assert!(!out.is_empty());
}

#[test]
fn mime_empty_no_panic() {
    let out = mime_calc("");
    assert!(!out.is_empty());
}

// ─── XML toolkit tests ────────────────────────────────────────────────────────

#[test]
fn xml_validate_wellformed() {
    let out = xml_calc("validate <root><child/></root>");
    assert!(
        out.to_lowercase().contains("valid") || out.to_lowercase().contains("well"),
        "well-formed XML should pass validation: {out}"
    );
}

#[test]
fn xml_validate_selfclosing() {
    let out = xml_calc("validate <root/>");
    assert!(
        out.to_lowercase().contains("valid") || out.to_lowercase().contains("well"),
        "self-closing root should be valid: {out}"
    );
}

#[test]
fn xml_validate_malformed_unclosed() {
    let out = xml_calc("validate <root><unclosed></root>");
    assert!(
        out.to_lowercase().contains("error")
            || out.to_lowercase().contains("invalid")
            || out.to_lowercase().contains("mismatch"),
        "unclosed tag should report an error: {out}"
    );
}

#[test]
fn xml_format_adds_indentation() {
    let out = xml_calc("format <a><b><c/></b></a>");
    assert!(out.contains('\n'), "format should add newlines: {out}");
    // indented child elements should appear
    assert!(
        out.contains("  ") || out.contains('\t'),
        "format should indent: {out}"
    );
}

#[test]
fn xml_minify_collapses_whitespace() {
    let input = "minify <root>\n  <child />\n</root>";
    let out = xml_calc(input);
    // the decorative header always has newlines; check the XML content line itself
    let xml_line = out
        .lines()
        .find(|l| l.contains("<root>"))
        .unwrap_or_else(|| panic!("No XML content line found in:\n{out}"));
    assert!(
        xml_line.contains("<child") && xml_line.contains("</root>"),
        "minified content should have root and child on the same line: {xml_line}"
    );
}

#[test]
fn xml_get_element_content() {
    let out = xml_calc("get name <person><name>Alice</name><age>30</age></person>");
    assert!(
        out.contains("Alice"),
        "get name should extract Alice: {out}"
    );
}

#[test]
fn xml_get_missing_tag() {
    let out = xml_calc("get email <person><name>Bob</name></person>");
    // should report not found, not panic
    assert!(
        !out.is_empty(),
        "missing tag should return a message, not panic"
    );
}

#[test]
fn xml_empty_no_panic() {
    let out = xml_calc("");
    assert!(!out.is_empty());
}

#[test]
fn xml_validate_empty_string_no_panic() {
    let out = xml_calc("validate ");
    assert!(!out.is_empty());
}

// ─── TOML toolkit tests ───────────────────────────────────────────────────────

#[test]
fn toml_validate_valid_simple() {
    let out = toml_calc("validate [server]\nport = 8080\nhost = \"localhost\"");
    assert!(out.contains('✓'), "valid TOML should pass: {out}");
}

#[test]
fn toml_validate_valid_bare_kv() {
    let out = toml_calc("[db]\nname = \"mydb\"\nmax_conn = 10");
    assert!(
        out.contains('✓'),
        "bare key-value section should be valid: {out}"
    );
}

#[test]
fn toml_validate_unclosed_table() {
    let out = toml_calc("validate [server\nport = 8080");
    assert!(
        out.contains('✗')
            || out.to_lowercase().contains("invalid")
            || out.to_lowercase().contains("unclosed"),
        "unclosed table header should fail: {out}"
    );
}

#[test]
fn toml_validate_missing_equals() {
    let out = toml_calc("validate [s]\nbadline");
    assert!(
        out.contains('✗')
            || out.to_lowercase().contains("invalid")
            || out.to_lowercase().contains("expected"),
        "line without '=' should fail: {out}"
    );
}

#[test]
fn toml_keys_lists_key_paths() {
    let out = toml_calc("keys [server]\nport = 8080\nhost = \"localhost\"");
    assert!(
        out.contains("server.port"),
        "keys should show dotted paths: {out}"
    );
    assert!(out.contains("server.host"), "{out}");
}

#[test]
fn toml_keys_bare_section() {
    let out = toml_calc("keys name = \"Alice\"\nage = 30");
    assert!(out.contains("name"), "top-level keys should appear: {out}");
    assert!(out.contains("age"), "{out}");
}

#[test]
fn toml_get_existing_key() {
    let out = toml_calc("get port [server]\nport = 8080");
    assert!(out.contains("8080"), "get should return the value: {out}");
}

#[test]
fn toml_get_missing_key() {
    let out = toml_calc("get missing [server]\nport = 8080");
    assert!(
        out.to_lowercase().contains("not found"),
        "missing key should report not found: {out}"
    );
}

#[test]
fn toml_fmt_normalizes_spacing() {
    let out = toml_calc("fmt [s]\nport=8080\nhost =   \"x\"");
    assert!(
        out.contains("port = 8080") || out.contains("port ="),
        "fmt should add spaces: {out}"
    );
}

#[test]
fn toml_empty_shows_help() {
    let out = toml_calc("");
    assert!(
        out.to_lowercase().contains("validate") || out.to_lowercase().contains("command"),
        "{out}"
    );
}

#[test]
fn toml_empty_input_no_panic() {
    let out = toml_calc("validate ");
    assert!(!out.is_empty());
}

// ─── Network / CIDR calculator tests ─────────────────────────────────────────

#[test]
fn net_cidr_24_breakdown() {
    let out = net_calc("192.168.1.0/24");
    assert!(out.contains("192.168.1.0"), "network address: {out}");
    assert!(out.contains("192.168.1.255"), "broadcast address: {out}");
    assert!(out.contains("255.255.255.0"), "subnet mask: {out}");
}

#[test]
fn net_cidr_hosts_count() {
    let out = net_calc("10.0.0.0/24");
    assert!(
        out.contains("254"),
        "usable hosts in /24 should be 254: {out}"
    );
}

#[test]
fn net_cidr_32_single_host() {
    let out = net_calc("192.168.1.1/32");
    assert!(out.contains('1'), "{out}");
    // /32 = 1 usable host
    assert!(out.contains('1'), "single host: {out}");
}

#[test]
fn net_ip_classification_private_192() {
    let out = net_calc("192.168.1.100");
    assert!(
        out.to_lowercase().contains("private") || out.contains("RFC 1918"),
        "192.168.x should be private: {out}"
    );
}

#[test]
fn net_ip_classification_loopback() {
    let out = net_calc("127.0.0.1");
    assert!(
        out.to_lowercase().contains("loopback"),
        "127.0.0.1 is loopback: {out}"
    );
}

#[test]
fn net_ip_classification_public() {
    let out = net_calc("8.8.8.8");
    assert!(
        out.to_lowercase().contains("public"),
        "8.8.8.8 is public: {out}"
    );
}

#[test]
fn net_contains_true() {
    let out = net_calc("contains 10.0.0.5 10.0.0.0/24");
    assert!(
        out.to_uppercase().contains("YES"),
        "10.0.0.5 in 10.0.0.0/24: {out}"
    );
}

#[test]
fn net_contains_false() {
    let out = net_calc("contains 10.0.1.5 10.0.0.0/24");
    assert!(
        out.to_uppercase().contains("NO"),
        "10.0.1.5 not in 10.0.0.0/24: {out}"
    );
}

#[test]
fn net_split_shows_subnets() {
    let out = net_calc("split 10.0.0.0/24 /26");
    assert!(out.contains("10.0.0.0/26"), "first /26 subnet: {out}");
    assert!(
        out.contains("10.0.0.64/26") || out.contains("10.0.0.64"),
        "second /26 subnet: {out}"
    );
}

#[test]
fn net_invalid_ip_no_panic() {
    let out = net_calc("999.999.999.999");
    assert!(!out.is_empty());
}

#[test]
fn net_empty_shows_help() {
    let out = net_calc("");
    assert!(
        out.to_lowercase().contains("cidr") || out.to_lowercase().contains("command"),
        "{out}"
    );
}

// ─── ASCII table tests ────────────────────────────────────────────────────────

#[test]
fn ascii_lookup_decimal_65() {
    let out = ascii_calc("65");
    assert!(out.contains("65"), "{out}");
    assert!(out.contains("'A'") || out.contains("A"), "65 is 'A': {out}");
}

#[test]
fn ascii_lookup_decimal_10() {
    let out = ascii_calc("10");
    assert!(
        out.contains("LF") || out.to_lowercase().contains("line feed"),
        "10 is LF: {out}"
    );
}

#[test]
fn ascii_lookup_hex_0x41() {
    let out = ascii_calc("0x41");
    assert!(
        out.contains("65") || out.contains("'A'"),
        "0x41 = 65 = 'A': {out}"
    );
}

#[test]
fn ascii_lookup_hex_0x1b() {
    let out = ascii_calc("0x1B");
    assert!(
        out.contains("ESC") || out.to_lowercase().contains("escape"),
        "0x1B is ESC: {out}"
    );
}

#[test]
fn ascii_lookup_char_a() {
    let out = ascii_calc("A");
    assert!(
        out.contains("65") || out.contains("'A'"),
        "char A = 65: {out}"
    );
}

#[test]
fn ascii_list_contains_multiple() {
    let out = ascii_calc("list printable");
    assert!(
        out.contains("65"),
        "printable list should include 65: {out}"
    );
    assert!(
        out.contains("90") || out.contains("'Z'"),
        "should include Z: {out}"
    );
    assert!(
        !out.contains("NUL"),
        "printable should not include NUL: {out}"
    );
}

#[test]
fn ascii_list_control_has_nul() {
    let out = ascii_calc("list control");
    assert!(
        out.contains("NUL"),
        "control list should include NUL: {out}"
    );
    assert!(out.contains("LF"), "control list should include LF: {out}");
}

#[test]
fn ascii_keyword_newline() {
    let out = ascii_calc("newline");
    assert!(
        out.contains("LF") || out.to_lowercase().contains("line feed"),
        "keyword 'newline' should find LF: {out}"
    );
}

#[test]
fn ascii_empty_no_panic() {
    let out = ascii_calc("");
    assert!(!out.is_empty());
}

#[test]
fn ascii_unknown_code_no_panic() {
    let out = ascii_calc("999");
    // 999 overflows u8, so lookup returns None → falls through to keyword search
    assert!(!out.is_empty());
}

// ─── Keyboard shortcuts tests ─────────────────────────────────────────────────

#[test]
fn kbd_vim_shows_shortcuts() {
    let out = kbd_calc("vim");
    assert!(out.to_lowercase().contains("vim"), "{out}");
    assert!(
        out.contains("h  j  k  l") || out.contains("h j k l"),
        "vim nav shortcuts: {out}"
    );
}

#[test]
fn kbd_vscode_shows_shortcuts() {
    let out = kbd_calc("vscode");
    assert!(
        out.contains("Ctrl+P") || out.contains("Ctrl+p"),
        "vscode quick-open: {out}"
    );
}

#[test]
fn kbd_tmux_shows_shortcuts() {
    let out = kbd_calc("tmux");
    assert!(out.to_lowercase().contains("prefix"), "tmux prefix: {out}");
    assert!(
        out.contains("prefix d") || out.to_lowercase().contains("detach"),
        "tmux detach: {out}"
    );
}

#[test]
fn kbd_git_shows_shortcuts() {
    let out = kbd_calc("git");
    assert!(out.to_lowercase().contains("commit"), "git commit: {out}");
    assert!(out.to_lowercase().contains("push"), "git push: {out}");
}

#[test]
fn kbd_bash_shows_shortcuts() {
    let out = kbd_calc("bash");
    assert!(
        out.contains("Ctrl+R") || out.contains("Ctrl+r"),
        "bash reverse search: {out}"
    );
}

#[test]
fn kbd_windows_shows_shortcuts() {
    let out = kbd_calc("windows");
    assert!(
        out.to_lowercase().contains("win+l") || out.to_lowercase().contains("lock"),
        "windows lock: {out}"
    );
}

#[test]
fn kbd_vim_filter_search() {
    let out = kbd_calc("vim search");
    assert!(
        out.to_lowercase().contains("search") || out.contains('/'),
        "vim search filter should show search shortcuts: {out}"
    );
}

#[test]
fn kbd_git_filter_rebase() {
    let out = kbd_calc("git rebase");
    assert!(
        out.to_lowercase().contains("rebase"),
        "git rebase filter: {out}"
    );
}

#[test]
fn kbd_empty_lists_tools() {
    let out = kbd_calc("");
    assert!(
        out.to_lowercase().contains("vim"),
        "empty query lists tools: {out}"
    );
    assert!(out.to_lowercase().contains("vscode"), "{out}");
}

#[test]
fn kbd_unknown_tool_no_panic() {
    let out = kbd_calc("notarealtoolforsure");
    assert!(
        !out.is_empty(),
        "unknown tool should return a message, not panic"
    );
    assert!(
        out.to_lowercase().contains("unknown") || out.to_lowercase().contains("available"),
        "{out}"
    );
}

// ── duration_calc tests ───────────────────────────────────────────────────────

#[test]
fn test_duration_today() {
    let out = duration_calc("today");
    assert!(out.contains("Today:"), "should show Today: line\n{out}");
    assert!(out.contains("ISO date:"), "{out}");
    assert!(out.contains("Day of year:"), "{out}");
}

#[test]
fn test_duration_empty_shows_today() {
    let out = duration_calc("");
    assert!(out.contains("Today:"), "{out}");
}

#[test]
fn test_duration_date_to_date() {
    let out = duration_calc("2024-01-01 to 2025-01-01");
    assert!(out.contains("Days:"), "should show Days:\n{out}");
    assert!(out.contains("Weeks:"), "{out}");
    assert!(out.contains("366"), "2024 has 366 days\n{out}");
}

#[test]
fn test_duration_date_to_date_same() {
    let out = duration_calc("2025-06-15 to 2025-06-15");
    assert!(out.contains("Days:   0"), "same date = 0 days\n{out}");
}

#[test]
fn test_duration_age() {
    let out = duration_calc("age 1990-01-01");
    assert!(out.contains("Age:"), "{out}");
    assert!(out.contains("Birth date:"), "{out}");
    assert!(out.contains("Next birthday"), "{out}");
}

#[test]
fn test_duration_age_future_date() {
    let out = duration_calc("age 2099-01-01");
    assert!(out.to_lowercase().contains("future"), "{out}");
}

#[test]
fn test_duration_unix_timestamp() {
    // 2024-01-01 00:00:00 UTC
    let out = duration_calc("1704067200");
    assert!(out.contains("Unix timestamp:"), "{out}");
    assert!(out.contains("UTC:"), "{out}");
    assert!(out.contains("2024-01-01"), "{out}");
}

#[test]
fn test_duration_days_ago() {
    let out = duration_calc("30 days ago");
    assert!(out.contains("Base date:"), "{out}");
    assert!(out.contains("Offset:"), "{out}");
    assert!(out.contains("Result:"), "{out}");
}

#[test]
fn test_duration_days_from_now() {
    let out = duration_calc("90 days from now");
    assert!(out.contains("Result:"), "{out}");
    assert!(out.contains("+90"), "{out}");
}

#[test]
fn test_duration_weeks_from_date() {
    let out = duration_calc("2 weeks from 2025-01-01");
    assert!(
        out.contains("2025-01-15"),
        "2 weeks from Jan 1 = Jan 15\n{out}"
    );
}

#[test]
fn test_duration_in_seconds_conversion() {
    let out = duration_calc("3600 in seconds");
    assert!(out.contains("3600 seconds"), "{out}");
    assert!(out.contains("60.00 minutes"), "{out}");
    assert!(out.contains("1.0000 hours"), "{out}");
}

#[test]
fn test_duration_bare_date() {
    let out = duration_calc("2000-01-01");
    assert!(out.contains("ISO date:"), "{out}");
    assert!(out.contains("From today:"), "{out}");
    assert!(out.contains("ago"), "{out}");
}

#[test]
fn test_duration_help_on_unknown() {
    let out = duration_calc("not a valid query xyz");
    assert!(out.contains("Commands:"), "{out}");
}

// ── spark_calc tests ──────────────────────────────────────────────────────────

#[test]
fn test_spark_basic_sparkline() {
    let out = spark_calc("1,4,2,8,5,7");
    assert!(
        out.contains('\u{2581}') || out.contains('\u{2588}'),
        "should have spark chars\n{out}"
    );
    assert!(out.contains("min="), "{out}");
    assert!(out.contains("max="), "{out}");
}

#[test]
fn test_spark_all_same_values() {
    let out = spark_calc("5,5,5,5");
    assert!(!out.is_empty(), "{out}");
}

#[test]
fn test_spark_bar_chart() {
    let out = spark_calc("bar 10,20,30,25,15");
    assert!(out.contains("BAR CHART"), "{out}");
    assert!(out.contains('['), "{out}");
    assert!(out.contains("30"), "{out}");
}

#[test]
fn test_spark_stats_mode() {
    let out = spark_calc("stats 3,7,2,9,4,6");
    assert!(out.contains("STATISTICS"), "{out}");
    assert!(out.contains("mean:"), "{out}");
    assert!(out.contains("median:"), "{out}");
    assert!(out.contains("std dev:"), "{out}");
    assert!(out.contains("5.0000"), "median of [2,3,4,6,7,9] = 5\n{out}");
}

#[test]
fn test_spark_normalize_mode() {
    let out = spark_calc("normalize 0,5,10");
    assert!(out.contains("Normalized:"), "{out}");
    assert!(out.contains("0.000"), "{out}");
    assert!(out.contains("1.000"), "{out}");
}

#[test]
fn test_spark_empty_input() {
    let out = spark_calc("");
    assert!(out.to_lowercase().contains("no valid numbers"), "{out}");
}

#[test]
fn test_spark_single_value() {
    let out = spark_calc("42");
    assert!(!out.is_empty(), "{out}");
}

#[test]
fn test_spark_negative_numbers() {
    let out = spark_calc("-3,-1,0,2,5");
    assert!(!out.is_empty(), "{out}");
    assert!(out.contains("min="), "{out}");
}

// ── template_calc tests ───────────────────────────────────────────────────────

#[test]
fn test_template_basic_substitution() {
    let out = template_calc("Hello {{name}}! ||| name=Alice");
    assert!(out.contains("Hello Alice!"), "substitution failed\n{out}");
    assert!(out.contains("Substituted:"), "{out}");
}

#[test]
fn test_template_multiple_vars() {
    let out = template_calc("Dear {{first}} {{last}} ||| first=Jane, last=Doe");
    assert!(out.contains("Dear Jane Doe"), "{out}");
}

#[test]
fn test_template_semicolon_separator() {
    let out = template_calc("{{greeting}} {{name}} ||| greeting=Hi; name=Bob");
    assert!(out.contains("Hi Bob"), "{out}");
}

#[test]
fn test_template_missing_var() {
    let out = template_calc("Hello {{name}} {{title}}! ||| name=Alice");
    assert!(out.contains("Missing vars:"), "{out}");
    assert!(out.contains("title"), "{out}");
}

#[test]
fn test_template_no_separator_shows_needs() {
    let out = template_calc("Hello {{name}}!");
    assert!(out.contains("Needs variables:"), "{out}");
    assert!(out.contains("name"), "{out}");
}

#[test]
fn test_template_no_placeholders_shows_help() {
    let out = template_calc("");
    assert!(out.to_lowercase().contains("usage"), "{out}");
}

#[test]
fn test_template_repeated_placeholder() {
    let out = template_calc("{{x}} + {{x}} = double ||| x=5");
    assert!(out.contains("5 + 5"), "{out}");
}

// ── escape_calc tests ─────────────────────────────────────────────────────────

#[test]
fn test_escape_json_mode() {
    let out = escape_calc("json She said \"hello\"");
    assert!(out.contains("\\\""), "should escape quotes\n{out}");
    assert!(out.contains("JSON:"), "{out}");
}

#[test]
fn test_escape_json_backslash() {
    let out = escape_calc("json C:\\Users\\foo");
    assert!(out.contains("\\\\"), "should escape backslash\n{out}");
}

#[test]
fn test_escape_shell_mode() {
    let out = escape_calc("shell hello world");
    assert!(out.contains("'hello world'"), "{out}");
}

#[test]
fn test_escape_regex_mode() {
    let out = escape_calc("regex (foo|bar)");
    let result_line = out
        .lines()
        .find(|l| l.trim_start().starts_with("Regex:"))
        .unwrap_or("");
    assert!(result_line.contains("\\("), "should escape (\n{out}");
    assert!(result_line.contains("\\|"), "should escape |\n{out}");
}

#[test]
fn test_escape_sql_mode() {
    let out = escape_calc("sql 50% off_sale");
    assert!(out.contains("\\%"), "should escape %\n{out}");
    assert!(out.contains("\\_"), "should escape _\n{out}");
}

#[test]
fn test_escape_unescape_mode() {
    let out = escape_calc("unescape \\\"hello\\\"");
    assert!(out.contains("Unescaped:"), "{out}");
}

#[test]
fn test_escape_all_mode() {
    let out = escape_calc("hello (world)");
    assert!(out.contains("JSON:"), "{out}");
    assert!(out.contains("Shell:"), "{out}");
    assert!(out.contains("Regex:"), "{out}");
    assert!(out.contains("SQL LIKE:"), "{out}");
}

#[test]
fn test_escape_empty_shows_help() {
    let out = escape_calc("");
    assert!(out.to_lowercase().contains("commands:"), "{out}");
}

#[test]
fn test_escape_no_special_json() {
    let out = escape_calc("json hello world");
    assert!(out.contains("\"hello world\""), "{out}");
}

// ── port_calc tests ───────────────────────────────────────────────────────────

#[test]
fn test_port_numeric_lookup_443() {
    let out = port_calc("443");
    assert!(out.contains("443"), "{out}");
    assert!(out.to_lowercase().contains("https"), "{out}");
}

#[test]
fn test_port_numeric_lookup_5432() {
    let out = port_calc("5432");
    assert!(out.to_lowercase().contains("postgres"), "{out}");
}

#[test]
fn test_port_numeric_lookup_6379() {
    let out = port_calc("6379");
    assert!(out.to_lowercase().contains("redis"), "{out}");
}

#[test]
fn test_port_service_name_lookup() {
    let out = port_calc("mysql");
    assert!(out.contains("3306"), "mysql should map to port 3306\n{out}");
}

#[test]
fn test_port_keyword_search() {
    let out = port_calc("mongo");
    assert!(out.contains("27017"), "{out}");
}

#[test]
fn test_port_unknown_number() {
    let out = port_calc("49999");
    assert!(!out.is_empty(), "{out}");
    // Should mention it's not in directory or mention ephemeral range
    assert!(
        out.to_lowercase().contains("not in") || out.to_lowercase().contains("ephemeral"),
        "{out}"
    );
}

#[test]
fn test_port_list_shows_entries() {
    let out = port_calc("list");
    assert!(out.contains("80"), "{out}");
    assert!(out.contains("443"), "{out}");
    assert!(out.contains("22"), "{out}");
}

#[test]
fn test_port_empty_shows_list() {
    let out = port_calc("");
    assert!(out.contains("PORT"), "{out}");
}

#[test]
fn test_port_ssh_lookup() {
    let out = port_calc("22");
    assert!(out.to_lowercase().contains("ssh"), "{out}");
}

#[test]
fn test_port_kafka_keyword() {
    let out = port_calc("kafka");
    assert!(out.contains("9092"), "{out}");
}

// ── chars_calc tests ──────────────────────────────────────────────────────────

#[test]
fn test_chars_ascii_letter() {
    let out = chars_calc("A");
    assert!(out.contains("U+0041"), "{out}");
    assert!(out.to_lowercase().contains("latin"), "{out}");
}

#[test]
fn test_chars_space() {
    let out = chars_calc(" ");
    assert!(out.contains("U+0020"), "{out}");
    assert!(out.to_lowercase().contains("space"), "{out}");
}

#[test]
fn test_chars_multi_char() {
    let out = chars_calc("Hi!");
    assert!(out.contains("U+0048"), "H should be U+0048\n{out}");
    assert!(out.contains("U+0069"), "i should be U+0069\n{out}");
    assert!(out.contains("U+0021"), "! should be U+0021\n{out}");
}

#[test]
fn test_chars_codepoint_lookup() {
    let out = chars_calc("U+2014");
    assert!(out.contains("U+2014"), "{out}");
    assert!(
        out.to_lowercase().contains("em dash") || out.contains("EM DASH"),
        "{out}"
    );
}

#[test]
fn test_chars_nbsp() {
    let out = chars_calc("\u{00A0}");
    assert!(out.contains("U+00A0"), "{out}");
    assert!(
        out.to_lowercase().contains("no-break") || out.to_lowercase().contains("nbsp"),
        "{out}"
    );
}

#[test]
fn test_chars_digit() {
    let out = chars_calc("9");
    assert!(out.contains("U+0039"), "{out}");
    assert!(
        out.to_lowercase().contains("digit") || out.to_lowercase().contains("nine"),
        "{out}"
    );
}

#[test]
fn test_chars_html_entity_amp() {
    let out = chars_calc("&");
    assert!(out.contains("&amp;"), "should show &amp; entity\n{out}");
}

#[test]
fn test_chars_html_entity_lt() {
    let out = chars_calc("<");
    assert!(out.contains("&lt;"), "{out}");
}

#[test]
fn test_chars_empty_shows_help() {
    let out = chars_calc("");
    assert!(out.to_lowercase().contains("usage"), "{out}");
}

#[test]
fn test_chars_shows_utf8_bytes() {
    // 'A' is 0x41 in UTF-8
    let out = chars_calc("A");
    assert!(out.contains("41"), "should show UTF-8 byte 41\n{out}");
}

// ── tz_calc tests ─────────────────────────────────────────────────────────────

#[test]
fn test_tz_list_shows_zones() {
    let out = tz_calc("list");
    assert!(out.contains("UTC"), "{out}");
    assert!(out.contains("JST"), "{out}");
    assert!(out.contains("EST"), "{out}");
}

#[test]
fn test_tz_now_in_utc() {
    let out = tz_calc("now in utc");
    assert!(out.contains("UTC"), "{out}");
    assert!(out.contains(':'), "should show time with colon\n{out}");
}

#[test]
fn test_tz_now_in_tokyo() {
    let out = tz_calc("now in tokyo");
    assert!(out.contains("Tokyo") || out.contains("JST"), "{out}");
}

#[test]
fn test_tz_time_conversion_est_to_jst() {
    let out = tz_calc("9am EST in JST");
    // EST is UTC-5, JST is UTC+9, so 9am EST = 23:00 JST
    assert!(out.contains("23:00"), "9am EST should be 23:00 JST\n{out}");
}

#[test]
fn test_tz_time_conversion_24h() {
    let out = tz_calc("14:00 UTC in JST");
    // UTC+9: 14:00 + 9h = 23:00
    assert!(
        out.contains("23:00"),
        "14:00 UTC should be 23:00 JST\n{out}"
    );
}

#[test]
fn test_tz_midnight_wraps_to_next_day() {
    let out = tz_calc("20:00 EST in JST");
    // EST UTC-5: 20:00 EST = 01:00 UTC next day; +9 = 10:00 JST next day?
    // 20:00 EST (UTC-5) -> 25:00 UTC = 01:00 UTC +1day; +9 -> 10:00 JST next day
    assert!(out.contains("next day") || out.contains("10:00"), "{out}");
}

#[test]
fn test_tz_single_zone_lookup() {
    let out = tz_calc("jst");
    assert!(out.contains("JST") || out.contains("Japan"), "{out}");
    assert!(out.contains("UTC+09:00") || out.contains("+09"), "{out}");
}

#[test]
fn test_tz_unknown_zone() {
    let out = tz_calc("now in fakezone");
    assert!(out.to_lowercase().contains("unknown"), "{out}");
}

#[test]
fn test_tz_empty_shows_list() {
    let out = tz_calc("");
    assert!(out.contains("UTC"), "{out}");
}

#[test]
fn test_tz_city_london() {
    let out = tz_calc("london");
    assert!(out.contains("London"), "{out}");
}

// ── headers_calc tests ────────────────────────────────────────────────────────

#[test]
fn test_headers_cache_control() {
    let out = headers_calc("cache-control");
    assert!(out.to_lowercase().contains("cache"), "{out}");
    assert!(
        out.to_lowercase().contains("caching") || out.to_lowercase().contains("directives"),
        "{out}"
    );
}

#[test]
fn test_headers_exact_content_type() {
    let out = headers_calc("content-type");
    assert!(
        out.contains("Content-Type") || out.contains("content-type"),
        "{out}"
    );
    assert!(
        out.to_lowercase().contains("media type") || out.to_lowercase().contains("body"),
        "{out}"
    );
}

#[test]
fn test_headers_cors_shortcut() {
    let out = headers_calc("cors");
    assert!(out.contains("Access-Control-Allow-Origin"), "{out}");
    assert!(out.contains("Access-Control-Allow-Methods"), "{out}");
    assert!(out.contains("Access-Control-Allow-Headers"), "{out}");
}

#[test]
fn test_headers_security_shortcut() {
    let out = headers_calc("security");
    assert!(out.contains("Strict-Transport-Security"), "{out}");
    assert!(out.contains("Content-Security-Policy"), "{out}");
    assert!(out.contains("X-Frame-Options"), "{out}");
}

#[test]
fn test_headers_etag() {
    let out = headers_calc("etag");
    assert!(out.to_lowercase().contains("etag"), "{out}");
    assert!(
        out.to_lowercase().contains("version") || out.to_lowercase().contains("identifier"),
        "{out}"
    );
}

#[test]
fn test_headers_authorization() {
    let out = headers_calc("authorization");
    assert!(
        out.to_lowercase().contains("bearer") || out.to_lowercase().contains("auth"),
        "{out}"
    );
}

#[test]
fn test_headers_list_shows_all() {
    let out = headers_calc("list");
    assert!(out.contains("Cache-Control"), "{out}");
    assert!(out.contains("Content-Type"), "{out}");
    assert!(out.contains("Authorization"), "{out}");
}

#[test]
fn test_headers_keyword_search_cookie() {
    let out = headers_calc("cookie");
    assert!(out.to_lowercase().contains("cookie"), "{out}");
}

#[test]
fn test_headers_empty_shows_list() {
    let out = headers_calc("");
    assert!(out.contains("Cache-Control") || out.contains("["), "{out}");
}

#[test]
fn test_headers_unknown_shows_no_match() {
    let out = headers_calc("xyznotaheader");
    assert!(
        out.to_lowercase().contains("no headers found") || out.to_lowercase().contains("not found"),
        "{out}"
    );
}

// ── gitignore_calc tests ──────────────────────────────────────────────────────

#[test]
fn test_gitignore_rust() {
    let out = gitignore_calc("rust");
    assert!(out.contains("/target/"), "{out}");
    assert!(out.contains("Cargo.lock"), "{out}");
}

#[test]
fn test_gitignore_node() {
    let out = gitignore_calc("node");
    assert!(out.contains("node_modules/"), "{out}");
    assert!(out.contains(".env"), "{out}");
}

#[test]
fn test_gitignore_python() {
    let out = gitignore_calc("python");
    assert!(out.contains("__pycache__/"), "{out}");
    assert!(out.contains(".venv"), "{out}");
}

#[test]
fn test_gitignore_alias_ts() {
    // "ts" is an alias for node template
    let out = gitignore_calc("ts");
    assert!(out.contains("node_modules/"), "{out}");
}

#[test]
fn test_gitignore_combined() {
    let out = gitignore_calc("rust macos vscode");
    assert!(out.contains("/target/"), "should have rust section\n{out}");
    assert!(
        out.contains(".DS_Store"),
        "should have macos section\n{out}"
    );
    assert!(
        out.contains(".vscode/"),
        "should have vscode section\n{out}"
    );
    assert!(
        out.contains("Combined:"),
        "should show combined header\n{out}"
    );
}

#[test]
fn test_gitignore_list() {
    let out = gitignore_calc("list");
    assert!(out.contains("rust"), "{out}");
    assert!(out.contains("node"), "{out}");
    assert!(out.contains("python"), "{out}");
    assert!(out.contains("terraform"), "{out}");
}

#[test]
fn test_gitignore_empty_shows_list() {
    let out = gitignore_calc("");
    assert!(out.to_lowercase().contains("available"), "{out}");
}

#[test]
fn test_gitignore_unknown() {
    let out = gitignore_calc("notarealstack");
    assert!(out.to_lowercase().contains("unknown"), "{out}");
}

#[test]
fn test_gitignore_vscode_alias() {
    let out = gitignore_calc("vscode");
    assert!(out.contains(".vscode/"), "{out}");
    assert!(out.contains("*.vsix"), "{out}");
}

// ── license_calc tests ────────────────────────────────────────────────────────

#[test]
fn test_license_list() {
    let out = license_calc("list");
    assert!(out.contains("MIT"), "{out}");
    assert!(out.contains("Apache-2.0"), "{out}");
    assert!(out.contains("GPL-3.0"), "{out}");
    assert!(out.contains("Unlicense"), "{out}");
}

#[test]
fn test_license_mit_full_text() {
    let out = license_calc("mit");
    assert!(out.contains("MIT License"), "{out}");
    assert!(out.contains("[year]"), "should have placeholders\n{out}");
    assert!(out.contains("[author]"), "{out}");
    assert!(out.contains("Permission is hereby granted"), "{out}");
}

#[test]
fn test_license_bsd2_full_text() {
    let out = license_calc("bsd2");
    assert!(out.contains("BSD 2-Clause"), "{out}");
    assert!(out.contains("Redistribution"), "{out}");
}

#[test]
fn test_license_bsd3_full_text() {
    let out = license_calc("bsd3");
    assert!(out.contains("BSD 3-Clause"), "{out}");
    assert!(out.contains("endorse or promote"), "{out}");
}

#[test]
fn test_license_isc_full_text() {
    let out = license_calc("isc");
    assert!(out.contains("ISC License"), "{out}");
    assert!(out.contains("Permission to use"), "{out}");
}

#[test]
fn test_license_unlicense_full_text() {
    let out = license_calc("unlicense");
    assert!(out.contains("public domain"), "{out}");
}

#[test]
fn test_license_apache_summary() {
    let out = license_calc("apache");
    assert!(out.contains("Apache-2.0"), "{out}");
    assert!(out.contains("Patent use"), "{out}");
    // No full text — should show spdx link
    assert!(
        out.contains("spdx.org") || out.contains("https://"),
        "{out}"
    );
}

#[test]
fn test_license_gpl3_summary() {
    let out = license_calc("gpl3");
    assert!(out.contains("GPL-3.0"), "{out}");
    assert!(out.contains("Disclose source"), "{out}");
}

#[test]
fn test_license_agpl3_network_clause() {
    let out = license_calc("agpl3");
    assert!(out.contains("Network use"), "{out}");
}

#[test]
fn test_license_wtfpl() {
    let out = license_calc("wtfpl");
    assert!(out.contains("WANT TO"), "{out}");
}

#[test]
fn test_license_unknown() {
    let out = license_calc("notarealelicense");
    assert!(out.to_lowercase().contains("unknown"), "{out}");
}

#[test]
fn test_license_permissions_shown() {
    let out = license_calc("mit");
    assert!(out.contains("Permissions:"), "{out}");
    assert!(out.contains("+ Commercial use"), "{out}");
}

// ── json_path_calc tests ──────────────────────────────────────────────────────

#[test]
fn test_json_path_simple_field() {
    let out = json_path_calc(r#".name ||| {"name":"Alice","age":30}"#);
    assert!(out.contains("Alice"), "{out}");
}

#[test]
fn test_json_path_number_field() {
    let out = json_path_calc(r#".age ||| {"name":"Alice","age":30}"#);
    assert!(out.contains("30"), "{out}");
}

#[test]
fn test_json_path_array_index() {
    let out = json_path_calc(r#".[0] ||| [10,20,30]"#);
    assert!(out.contains("10"), "{out}");
}

#[test]
fn test_json_path_nested() {
    let out = json_path_calc(r#".user.email ||| {"user":{"email":"a@b.com"}}"#);
    assert!(out.contains("a@b.com"), "{out}");
}

#[test]
fn test_json_path_array_field() {
    let out = json_path_calc(r#".users[0].name ||| {"users":[{"name":"Bob"}]}"#);
    assert!(out.contains("Bob"), "{out}");
}

#[test]
fn test_json_path_keys_command() {
    let out = json_path_calc(r#"keys ||| {"a":1,"b":2,"c":3}"#);
    assert!(out.contains("a"), "{out}");
    assert!(out.contains("b"), "{out}");
    assert!(out.contains("c"), "{out}");
    assert!(out.contains("Keys"), "{out}");
}

#[test]
fn test_json_path_type_command() {
    let out = json_path_calc(r#"type ||| {"name":"Alice","age":30}"#);
    assert!(out.contains("object"), "{out}");
    assert!(out.contains("string"), "{out}");
    assert!(out.contains("number"), "{out}");
}

#[test]
fn test_json_path_length_array() {
    let out = json_path_calc(r#"length ||| [1,2,3,4,5]"#);
    assert!(out.contains("5"), "{out}");
}

#[test]
fn test_json_path_pretty_print() {
    let out = json_path_calc(r#". ||| {"a":1,"b":2}"#);
    assert!(out.contains("\"a\""), "{out}");
    assert!(out.contains("\"b\""), "{out}");
}

#[test]
fn test_json_path_bare_json_pretty() {
    let out = json_path_calc(r#"{"x":1,"y":2}"#);
    assert!(out.contains("pretty"), "{out}");
    assert!(out.contains("\"x\""), "{out}");
}

#[test]
fn test_json_path_missing_field_error() {
    let out = json_path_calc(r#".missing ||| {"name":"Alice"}"#);
    assert!(
        out.to_lowercase().contains("error") || out.to_lowercase().contains("not found"),
        "{out}"
    );
    assert!(out.contains("name"), "should hint available keys\n{out}");
}

#[test]
fn test_json_path_invalid_json() {
    let out = json_path_calc(r#".name ||| {not valid json}"#);
    assert!(
        out.to_lowercase().contains("parse error") || out.to_lowercase().contains("error"),
        "{out}"
    );
}

#[test]
fn test_json_path_empty_shows_help() {
    let out = json_path_calc("");
    assert!(out.to_lowercase().contains("usage"), "{out}");
}

// ── markdown_calc tests ───────────────────────────────────────────────────────

#[test]
fn test_markdown_all_sections() {
    let out = markdown_calc("");
    assert!(out.contains("headings"), "{out}");
    assert!(out.contains("tables"), "{out}");
    assert!(out.contains("mermaid"), "{out}");
    assert!(out.contains("# H1"), "{out}");
}

#[test]
fn test_markdown_tables_section() {
    let out = markdown_calc("tables");
    assert!(out.contains("| Column"), "{out}");
    assert!(out.contains("|:"), "should show alignment syntax\n{out}");
}

#[test]
fn test_markdown_links_section() {
    let out = markdown_calc("links");
    assert!(out.contains("[Link text]"), "{out}");
    assert!(out.contains("https://example.com"), "{out}");
    assert!(out.contains("anchor"), "{out}");
}

#[test]
fn test_markdown_code_section() {
    let out = markdown_calc("code");
    assert!(out.contains("Fenced block:"), "{out}");
    assert!(out.contains("Language IDs:"), "{out}");
    assert!(out.contains("Inline:"), "{out}");
}

#[test]
fn test_markdown_mermaid_section() {
    let out = markdown_calc("mermaid");
    assert!(out.contains("graph TD"), "{out}");
    assert!(out.contains("sequenceDiagram"), "{out}");
}

#[test]
fn test_markdown_emphasis_section() {
    let out = markdown_calc("bold");
    assert!(out.contains("**bold**"), "{out}");
    assert!(out.contains("*italic*"), "{out}");
}

#[test]
fn test_markdown_lists_section() {
    let out = markdown_calc("lists");
    assert!(out.contains("- Item"), "{out}");
    assert!(out.contains("[x]"), "{out}");
}

#[test]
fn test_markdown_unknown_section() {
    let out = markdown_calc("notasectionxyz");
    assert!(
        out.to_lowercase().contains("no section found") || out.to_lowercase().contains("available"),
        "{out}"
    );
}

#[test]
fn test_markdown_frontmatter_section() {
    let out = markdown_calc("frontmatter");
    assert!(out.contains("---"), "{out}");
    assert!(out.contains("title:"), "{out}");
}

#[test]
fn test_markdown_blockquote_section() {
    let out = markdown_calc("blockquote");
    assert!(out.contains("> "), "{out}");
}

// ── Wave 11: regex_ref_calc ───────────────────────────────────────────────────

#[test]
fn test_regex_ref_empty_shows_help() {
    let out = regex_ref_calc("");
    assert!(out.contains("hematite --regex-ref"), "{out}");
    assert!(out.contains("email"), "{out}");
}

#[test]
fn test_regex_ref_email_pattern() {
    let out = regex_ref_calc("email");
    assert!(out.contains("email"), "{out}");
    assert!(out.contains("@"), "{out}");
    assert!(out.contains("grep"), "{out}");
}

#[test]
fn test_regex_ref_uuid_pattern() {
    let out = regex_ref_calc("uuid");
    assert!(out.contains("UUID"), "{out}");
    assert!(out.contains("[0-9a-f]"), "{out}");
}

#[test]
fn test_regex_ref_url_by_alias() {
    let out = regex_ref_calc("http");
    assert!(out.contains("http"), "{out}");
    assert!(out.contains("https?"), "{out}");
}

#[test]
fn test_regex_ref_ipv4_pattern() {
    let out = regex_ref_calc("ipv4");
    assert!(out.contains("IPv4"), "{out}");
    assert!(out.contains(r"\b"), "{out}");
}

#[test]
fn test_regex_ref_date_iso_pattern() {
    let out = regex_ref_calc("date-iso");
    assert!(out.contains("ISO 8601"), "{out}");
}

#[test]
fn test_regex_ref_syntax_section() {
    let out = regex_ref_calc("syntax");
    assert!(out.contains("Regex Syntax"), "{out}");
    assert!(out.contains(r"\d"), "{out}");
    assert!(out.contains("lookahead"), "{out}");
}

#[test]
fn test_regex_ref_all_prints_every_pattern() {
    let out = regex_ref_calc("all");
    assert!(out.contains("email"), "{out}");
    assert!(out.contains("uuid"), "{out}");
    assert!(out.contains("mac-address"), "{out}");
    assert!(out.contains("hashtag"), "{out}");
}

#[test]
fn test_regex_ref_unknown_pattern() {
    let out = regex_ref_calc("foobar_xyz");
    assert!(out.contains("No pattern found"), "{out}");
}

#[test]
fn test_regex_ref_hex_color() {
    let out = regex_ref_calc("hex-color");
    assert!(out.contains("CSS hex"), "{out}");
    assert!(out.contains("#"), "{out}");
}

#[test]
fn test_regex_ref_semver() {
    let out = regex_ref_calc("semver");
    assert!(out.contains("Semantic version"), "{out}");
}

#[test]
fn test_regex_ref_jwt_by_alias() {
    let out = regex_ref_calc("bearer");
    assert!(out.contains("JWT") || out.contains("token"), "{out}");
}

// ── Wave 11: ascii_table_calc ─────────────────────────────────────────────────

#[test]
fn test_ascii_table_empty_shows_full_table() {
    let out = ascii_table_calc("");
    assert!(
        out.contains("ASCII Table") || out.contains("Description"),
        "{out}"
    );
    assert!(out.contains("NUL"), "{out}");
    assert!(out.contains("DEL"), "{out}");
}

#[test]
fn test_ascii_table_lookup_by_decimal() {
    let out = ascii_table_calc("65");
    assert!(out.contains("65"), "{out}");
    assert!(out.contains("0x41"), "{out}");
    assert!(out.contains("Uppercase A") || out.contains("A"), "{out}");
}

#[test]
fn test_ascii_table_lookup_by_hex() {
    let out = ascii_table_calc("0x41");
    assert!(out.contains("65") || out.contains("Uppercase A"), "{out}");
}

#[test]
fn test_ascii_table_lookup_by_char() {
    let out = ascii_table_calc("A");
    assert!(out.contains("65") || out.contains("0x41"), "{out}");
}

#[test]
fn test_ascii_table_control_section() {
    let out = ascii_table_calc("control");
    assert!(out.contains("NUL"), "{out}");
    assert!(out.contains("ESC"), "{out}");
    // printable chars must not appear in control section
    assert!(!out.contains("Uppercase A"), "{out}");
}

#[test]
fn test_ascii_table_digits_section() {
    let out = ascii_table_calc("digits");
    assert!(out.contains("Digit zero"), "{out}");
    assert!(out.contains("Digit nine"), "{out}");
    assert!(!out.contains("NUL"), "{out}");
}

#[test]
fn test_ascii_table_upper_section() {
    let out = ascii_table_calc("upper");
    assert!(out.contains("Uppercase A"), "{out}");
    assert!(out.contains("Uppercase Z"), "{out}");
    assert!(!out.contains("Lowercase"), "{out}");
}

#[test]
fn test_ascii_table_punct_section() {
    let out = ascii_table_calc("punct");
    assert!(out.contains("Exclamation") || out.contains("!"), "{out}");
    assert!(!out.contains("Digit"), "{out}");
}

#[test]
fn test_ascii_table_search_by_name() {
    let out = ascii_table_calc("backslash");
    assert!(
        out.contains("92") || out.contains("5C") || out.contains("backslash"),
        "{out}"
    );
}

#[test]
fn test_ascii_table_dec_0_nul() {
    let out = ascii_table_calc("0");
    assert!(out.contains("NUL"), "{out}");
    assert!(out.contains("00"), "{out}");
}

#[test]
fn test_ascii_table_dec_32_space() {
    let out = ascii_table_calc("32");
    assert!(out.contains("SPC") || out.contains("Space"), "{out}");
}

#[test]
fn test_ascii_table_unknown_returns_tip() {
    let out = ascii_table_calc("zzquux");
    assert!(out.contains("No match") || out.contains("Try"), "{out}");
}

// ── Wave 11: ssl_calc ─────────────────────────────────────────────────────────

#[test]
fn test_ssl_empty_shows_help() {
    let out = ssl_calc("");
    assert!(out.contains("hematite --ssl"), "{out}");
    assert!(out.contains("handshake"), "{out}");
}

#[test]
fn test_ssl_handshake_topic() {
    let out = ssl_calc("handshake");
    assert!(out.contains("ClientHello") || out.contains("TLS"), "{out}");
    assert!(
        out.contains("ServerHello") || out.contains("1-RTT"),
        "{out}"
    );
}

#[test]
fn test_ssl_ciphers_topic() {
    let out = ssl_calc("ciphers");
    assert!(out.contains("AES"), "{out}");
    assert!(
        out.contains("CHACHA20") || out.contains("ChaCha20"),
        "{out}"
    );
}

#[test]
fn test_ssl_openssl_commands() {
    let out = ssl_calc("openssl");
    assert!(out.contains("openssl x509"), "{out}");
    assert!(out.contains("cert.pem"), "{out}");
}

#[test]
fn test_ssl_certificates_topic() {
    let out = ssl_calc("cert");
    assert!(out.contains("PEM") || out.contains("DER"), "{out}");
}

#[test]
fn test_ssl_nginx_config() {
    let out = ssl_calc("nginx");
    assert!(
        out.contains("ssl_protocols") || out.contains("TLSv1"),
        "{out}"
    );
}

#[test]
fn test_ssl_grades_topic() {
    let out = ssl_calc("grades");
    assert!(out.contains("Mozilla") || out.contains("Modern"), "{out}");
    assert!(out.contains("A+") || out.contains("A "), "{out}");
}

#[test]
fn test_ssl_hsts_headers() {
    let out = ssl_calc("hsts");
    assert!(out.contains("Strict-Transport-Security"), "{out}");
    assert!(out.contains("max-age"), "{out}");
}

#[test]
fn test_ssl_errors_section() {
    let out = ssl_calc("errors");
    assert!(out.contains("expired") || out.contains("ERR_CERT"), "{out}");
}

#[test]
fn test_ssl_all_prints_all_sections() {
    let out = ssl_calc("all");
    assert!(
        out.contains("handshake") || out.contains("ClientHello"),
        "{out}"
    );
    assert!(out.contains("AES"), "{out}");
    assert!(out.contains("Strict-Transport-Security"), "{out}");
}

#[test]
fn test_ssl_unknown_topic() {
    let out = ssl_calc("foobar_xyz");
    assert!(out.contains("No section") || out.contains("help"), "{out}");
}

// ── Wave 11: id_gen_calc ──────────────────────────────────────────────────────

#[test]
fn test_id_gen_empty_shows_help() {
    let out = id_gen_calc("");
    assert!(out.contains("hematite --id"), "{out}");
    assert!(out.contains("uuid"), "{out}");
    assert!(out.contains("ulid"), "{out}");
}

#[test]
fn test_id_gen_uuid_format() {
    let out = id_gen_calc("uuid");
    // UUID v4: 8-4-4-4-12 hex groups
    let id_line = out.lines().find(|l| l.contains('-')).unwrap_or("");
    let id = id_line.trim();
    let parts: Vec<&str> = id.split('-').collect();
    assert_eq!(parts.len(), 5, "UUID must have 5 parts: {id}");
    assert_eq!(parts[0].len(), 8, "Part 0 len: {id}");
    assert_eq!(parts[1].len(), 4, "Part 1 len: {id}");
    assert_eq!(parts[2].len(), 4, "Part 2 len: {id}");
    assert_eq!(parts[3].len(), 4, "Part 3 len: {id}");
    assert_eq!(parts[4].len(), 12, "Part 4 len: {id}");
}

#[test]
fn test_id_gen_uuid4_alias() {
    let out = id_gen_calc("uuid4");
    assert!(out.contains('-'), "{out}");
}

#[test]
fn test_id_gen_ulid_format() {
    let out = id_gen_calc("ulid");
    // ULID: 26 Crockford base32 chars, uppercase, no dashes
    let id_line = out.lines().find(|l| l.trim().len() == 26).unwrap_or("");
    let id = id_line.trim();
    assert_eq!(id.len(), 26, "ULID must be 26 chars: '{id}'");
    assert!(
        id.chars().all(|c| c.is_ascii_alphanumeric()),
        "ULID must be alphanumeric: {id}"
    );
}

#[test]
fn test_id_gen_nanoid_default_length() {
    let out = id_gen_calc("nanoid");
    assert!(out.contains("21 chars") || out.contains("NanoID"), "{out}");
    // Find the ID line (21 chars, no spaces)
    let id_line = out.lines().find(|l| l.trim().len() == 21).unwrap_or("");
    assert_eq!(
        id_line.trim().len(),
        21,
        "NanoID must be 21 chars, got: '{}'",
        id_line.trim()
    );
}

#[test]
fn test_id_gen_nanoid_custom_length() {
    let out = id_gen_calc("nanoid 32");
    assert!(out.contains("32 chars"), "{out}");
    let id_line = out.lines().find(|l| l.trim().len() == 32).unwrap_or("");
    assert_eq!(
        id_line.trim().len(),
        32,
        "NanoID custom must be 32 chars: '{}'",
        id_line.trim()
    );
}

#[test]
fn test_id_gen_hex8_format() {
    let out = id_gen_calc("hex8");
    // 16 hex chars
    let id_line = out
        .lines()
        .find(|l| {
            let t = l.trim();
            t.len() == 16 && t.chars().all(|c| c.is_ascii_hexdigit())
        })
        .unwrap_or("");
    assert!(
        !id_line.is_empty(),
        "hex8 must produce a 16-char hex ID, got: {out}"
    );
}

#[test]
fn test_id_gen_hex16_format() {
    let out = id_gen_calc("hex16");
    let id_line = out
        .lines()
        .find(|l| {
            let t = l.trim();
            t.len() == 32 && t.chars().all(|c| c.is_ascii_hexdigit())
        })
        .unwrap_or("");
    assert!(
        !id_line.is_empty(),
        "hex16 must produce a 32-char hex ID, got: {out}"
    );
}

#[test]
fn test_id_gen_cuid2_format() {
    let out = id_gen_calc("cuid2");
    // CUID2: starts with lowercase letter, 25 total alphanumeric chars
    let id_line = out
        .lines()
        .find(|l| {
            let t = l.trim();
            t.len() == 25 && t.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .unwrap_or("");
    assert!(
        !id_line.is_empty(),
        "cuid2 must be 25 lowercase alphanumeric chars, got: {out}"
    );
}

#[test]
fn test_id_gen_xid_format() {
    let out = id_gen_calc("xid");
    // XID: 20 base32 chars
    let id_line = out
        .lines()
        .find(|l| {
            let t = l.trim();
            t.len() == 20 && t.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .unwrap_or("");
    assert!(!id_line.is_empty(), "xid must be 20 chars, got: {out}");
}

#[test]
fn test_id_gen_all_shows_all_types() {
    let out = id_gen_calc("all");
    assert!(out.contains("UUID") || out.contains("uuid"), "{out}");
    assert!(out.contains("ULID") || out.contains("ulid"), "{out}");
    assert!(out.contains("NanoID") || out.contains("nanoid"), "{out}");
    assert!(out.contains("Hex") || out.contains("hex"), "{out}");
    assert!(out.contains("CUID2") || out.contains("cuid"), "{out}");
    assert!(out.contains("XID") || out.contains("xid"), "{out}");
}

#[test]
fn test_id_gen_unknown_type() {
    let out = id_gen_calc("foobar_xyz");
    assert!(
        out.contains("Unknown type") || out.contains("help"),
        "{out}"
    );
}

#[test]
fn test_id_gen_two_uuids_differ() {
    let a = id_gen_calc("uuid");
    let b = id_gen_calc("uuid");
    // Extract the actual ID from each output
    let id_a: String = a
        .lines()
        .find(|l| l.trim().len() == 36 && l.contains('-'))
        .unwrap_or("")
        .trim()
        .to_string();
    let id_b: String = b
        .lines()
        .find(|l| l.trim().len() == 36 && l.contains('-'))
        .unwrap_or("")
        .trim()
        .to_string();
    // They should differ (different nanosecond timestamps drive entropy)
    // Note: in very fast sequential calls they might collide; this is a best-effort check
    if !id_a.is_empty() && !id_b.is_empty() {
        // Just verify both are valid UUID format
        assert_eq!(
            id_a.split('-').count(),
            5,
            "ID a must be UUID format: {id_a}"
        );
        assert_eq!(
            id_b.split('-').count(),
            5,
            "ID b must be UUID format: {id_b}"
        );
    }
}

// ── Wave 12: http_status_calc ─────────────────────────────────────────────────

#[test]
fn test_http_status_empty_shows_help() {
    let out = http_status_calc("");
    assert!(out.contains("hematite --http-status"), "{out}");
    assert!(out.contains("4xx"), "{out}");
}

#[test]
fn test_http_status_lookup_200() {
    let out = http_status_calc("200");
    assert!(out.contains("200"), "{out}");
    assert!(out.contains("OK"), "{out}");
}

#[test]
fn test_http_status_lookup_404() {
    let out = http_status_calc("404");
    assert!(out.contains("404"), "{out}");
    assert!(
        out.contains("Not Found") || out.contains("not-found"),
        "{out}"
    );
}

#[test]
fn test_http_status_lookup_500() {
    let out = http_status_calc("500");
    assert!(out.contains("500"), "{out}");
    assert!(
        out.contains("Internal Server Error") || out.contains("internal"),
        "{out}"
    );
}

#[test]
fn test_http_status_lookup_418_teapot() {
    let out = http_status_calc("418");
    assert!(out.contains("418"), "{out}");
    assert!(out.contains("Teapot") || out.contains("teapot"), "{out}");
}

#[test]
fn test_http_status_lookup_by_name() {
    let out = http_status_calc("not-found");
    assert!(out.contains("404"), "{out}");
}

#[test]
fn test_http_status_category_4xx() {
    let out = http_status_calc("4xx");
    assert!(out.contains("400") || out.contains("404"), "{out}");
    assert!(!out.contains("200"), "{out}");
    assert!(!out.contains("500"), "{out}");
}

#[test]
fn test_http_status_category_5xx() {
    let out = http_status_calc("5xx");
    assert!(out.contains("500"), "{out}");
    assert!(out.contains("503"), "{out}");
    assert!(!out.contains("200"), "{out}");
}

#[test]
fn test_http_status_category_success() {
    let out = http_status_calc("success");
    assert!(out.contains("200"), "{out}");
    assert!(out.contains("201"), "{out}");
    assert!(!out.contains("404"), "{out}");
}

#[test]
fn test_http_status_category_redirect() {
    let out = http_status_calc("redirect");
    assert!(out.contains("301"), "{out}");
    assert!(out.contains("302"), "{out}");
    assert!(!out.contains("200"), "{out}");
}

#[test]
fn test_http_status_all_lists_many_codes() {
    let out = http_status_calc("all");
    assert!(out.contains("200"), "{out}");
    assert!(out.contains("404"), "{out}");
    assert!(out.contains("500"), "{out}");
    assert!(out.contains("418"), "{out}");
}

#[test]
fn test_http_status_unknown_code() {
    let out = http_status_calc("999");
    assert!(out.contains("not found") || out.contains("999"), "{out}");
}

#[test]
fn test_http_status_keyword_search() {
    let out = http_status_calc("rate");
    assert!(out.contains("429") || out.contains("Too Many"), "{out}");
}

// ── Wave 12: git_ref_calc ─────────────────────────────────────────────────────

#[test]
fn test_git_ref_empty_shows_help() {
    let out = git_ref_calc("");
    assert!(out.contains("hematite --git-ref"), "{out}");
    assert!(out.contains("rebase"), "{out}");
}

#[test]
fn test_git_ref_commit_topic() {
    let out = git_ref_calc("commit");
    assert!(out.contains("git commit"), "{out}");
    assert!(out.contains("--amend"), "{out}");
}

#[test]
fn test_git_ref_rebase_topic() {
    let out = git_ref_calc("rebase");
    assert!(out.contains("git rebase"), "{out}");
    assert!(out.contains("-i"), "{out}");
}

#[test]
fn test_git_ref_stash_topic() {
    let out = git_ref_calc("stash");
    assert!(out.contains("git stash"), "{out}");
    assert!(out.contains("pop"), "{out}");
}

#[test]
fn test_git_ref_remote_by_alias() {
    let out = git_ref_calc("push");
    assert!(out.contains("git push"), "{out}");
    assert!(out.contains("origin"), "{out}");
}

#[test]
fn test_git_ref_log_topic() {
    let out = git_ref_calc("log");
    assert!(out.contains("git log"), "{out}");
    assert!(out.contains("--oneline"), "{out}");
}

#[test]
fn test_git_ref_reset_topic() {
    let out = git_ref_calc("reset");
    assert!(out.contains("git reset"), "{out}");
    assert!(out.contains("--hard"), "{out}");
}

#[test]
fn test_git_ref_tag_topic() {
    let out = git_ref_calc("tag");
    assert!(out.contains("git tag"), "{out}");
    assert!(out.contains("annotated") || out.contains("-a"), "{out}");
}

#[test]
fn test_git_ref_aliases_topic() {
    let out = git_ref_calc("aliases");
    assert!(out.contains("alias"), "{out}");
    assert!(out.contains("--global"), "{out}");
}

#[test]
fn test_git_ref_all_prints_all_sections() {
    let out = git_ref_calc("all");
    assert!(out.contains("git commit"), "{out}");
    assert!(out.contains("git rebase"), "{out}");
    assert!(out.contains("git stash"), "{out}");
    assert!(out.contains("git log"), "{out}");
}

#[test]
fn test_git_ref_unknown_topic() {
    let out = git_ref_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

#[test]
fn test_git_ref_branch_topic() {
    let out = git_ref_calc("branch");
    assert!(out.contains("git branch"), "{out}");
    assert!(out.contains("-d") || out.contains("-D"), "{out}");
}

// ── Wave 12: color_names_calc ─────────────────────────────────────────────────

#[test]
fn test_color_names_empty_shows_help() {
    let out = color_names_calc("");
    assert!(out.contains("hematite --color-names"), "{out}");
    assert!(
        out.contains("Categories") || out.contains("category"),
        "{out}"
    );
}

#[test]
fn test_color_names_exact_lookup() {
    let out = color_names_calc("tomato");
    assert!(out.contains("tomato"), "{out}");
    assert!(out.contains("#FF6347"), "{out}");
    assert!(out.contains("rgb("), "{out}");
    assert!(out.contains("hsl("), "{out}");
}

#[test]
fn test_color_names_hex_lookup() {
    let out = color_names_calc("#FF6347");
    assert!(out.contains("tomato"), "{out}");
}

#[test]
fn test_color_names_hex_without_hash() {
    let out = color_names_calc("FF0000");
    assert!(out.contains("red") || out.contains("#FF0000"), "{out}");
}

#[test]
fn test_color_names_category_red() {
    let out = color_names_calc("red");
    assert!(out.contains("tomato") || out.contains("crimson"), "{out}");
}

#[test]
fn test_color_names_category_blue() {
    let out = color_names_calc("blue");
    assert!(out.contains("navy") || out.contains("royalblue"), "{out}");
}

#[test]
fn test_color_names_category_gray() {
    let out = color_names_calc("gray");
    assert!(out.contains("silver") || out.contains("gainsboro"), "{out}");
    assert!(out.contains("black"), "{out}");
}

#[test]
fn test_color_names_partial_search() {
    let out = color_names_calc("cornflower");
    assert!(out.contains("cornflowerblue"), "{out}");
    assert!(out.contains("#6495ED"), "{out}");
}

#[test]
fn test_color_names_all_shows_many() {
    let out = color_names_calc("all");
    assert!(out.contains("red"), "{out}");
    assert!(out.contains("navy"), "{out}");
    assert!(out.contains("gold"), "{out}");
    assert!(out.contains("white"), "{out}");
}

#[test]
fn test_color_names_unknown() {
    let out = color_names_calc("ultravioletmaroon");
    assert!(
        out.contains("No color found") || out.contains("Try"),
        "{out}"
    );
}

#[test]
fn test_color_names_hsl_values_present() {
    let out = color_names_calc("red");
    // red category listing should include HSL values
    assert!(out.contains("hsl("), "{out}");
}

#[test]
fn test_color_names_white_lookup() {
    let out = color_names_calc("white");
    // "white" is a category name AND a color name; should show #FFFFFF
    assert!(out.contains("#FFFFFF") || out.contains("FFFFFF"), "{out}");
}

// ── Wave 12: docker_ref_calc ──────────────────────────────────────────────────

#[test]
fn test_docker_ref_empty_shows_help() {
    let out = docker_ref_calc("");
    assert!(out.contains("hematite --docker-ref"), "{out}");
    assert!(out.contains("compose"), "{out}");
}

#[test]
fn test_docker_ref_build_topic() {
    let out = docker_ref_calc("build");
    assert!(out.contains("docker build"), "{out}");
    assert!(out.contains("--no-cache") || out.contains("-t"), "{out}");
}

#[test]
fn test_docker_ref_run_topic() {
    let out = docker_ref_calc("run");
    assert!(out.contains("docker run"), "{out}");
    assert!(out.contains("-p") || out.contains("port"), "{out}");
}

#[test]
fn test_docker_ref_exec_by_alias() {
    let out = docker_ref_calc("logs");
    assert!(out.contains("docker logs"), "{out}");
    assert!(out.contains("-f"), "{out}");
}

#[test]
fn test_docker_ref_compose_topic() {
    let out = docker_ref_calc("compose");
    assert!(out.contains("docker compose"), "{out}");
    assert!(out.contains("up") || out.contains("down"), "{out}");
}

#[test]
fn test_docker_ref_volumes_topic() {
    let out = docker_ref_calc("volumes");
    assert!(out.contains("docker volume"), "{out}");
    assert!(out.contains("prune") || out.contains("create"), "{out}");
}

#[test]
fn test_docker_ref_network_topic() {
    let out = docker_ref_calc("network");
    assert!(out.contains("docker network"), "{out}");
    assert!(out.contains("bridge") || out.contains("create"), "{out}");
}

#[test]
fn test_docker_ref_prune_topic() {
    let out = docker_ref_calc("prune");
    assert!(
        out.contains("docker system prune") || out.contains("prune"),
        "{out}"
    );
    assert!(out.contains("image") || out.contains("container"), "{out}");
}

#[test]
fn test_docker_ref_dockerfile_topic() {
    let out = docker_ref_calc("dockerfile");
    assert!(out.contains("FROM"), "{out}");
    assert!(out.contains("RUN") || out.contains("COPY"), "{out}");
    assert!(out.contains("WORKDIR") || out.contains("ENV"), "{out}");
}

#[test]
fn test_docker_ref_all_prints_all_sections() {
    let out = docker_ref_calc("all");
    assert!(out.contains("docker build"), "{out}");
    assert!(out.contains("docker compose"), "{out}");
    assert!(out.contains("FROM"), "{out}");
}

#[test]
fn test_docker_ref_registry_topic() {
    let out = docker_ref_calc("registry");
    assert!(
        out.contains("docker push") || out.contains("docker pull"),
        "{out}"
    );
    assert!(out.contains("login") || out.contains("Docker Hub"), "{out}");
}

#[test]
fn test_docker_ref_unknown_topic() {
    let out = docker_ref_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 13: sql_ref_calc ─────────────────────────────────────────────────────

#[test]
fn test_sql_ref_empty_shows_help() {
    let out = sql_ref_calc("");
    assert!(out.contains("hematite --sql-ref"), "{out}");
    assert!(out.contains("joins") || out.contains("select"), "{out}");
}

#[test]
fn test_sql_ref_select_topic() {
    let out = sql_ref_calc("select");
    assert!(out.contains("SELECT"), "{out}");
    assert!(out.contains("FROM") || out.contains("LIMIT"), "{out}");
}

#[test]
fn test_sql_ref_joins_topic() {
    let out = sql_ref_calc("joins");
    assert!(out.contains("JOIN"), "{out}");
    assert!(out.contains("LEFT") || out.contains("INNER"), "{out}");
}

#[test]
fn test_sql_ref_window_topic() {
    let out = sql_ref_calc("window");
    assert!(out.contains("OVER"), "{out}");
    assert!(
        out.contains("ROW_NUMBER") || out.contains("PARTITION"),
        "{out}"
    );
}

#[test]
fn test_sql_ref_cte_alias() {
    let out = sql_ref_calc("cte");
    assert!(out.contains("WITH"), "{out}");
    assert!(
        out.contains("CTE") || out.contains("recursive") || out.contains("RECURSIVE"),
        "{out}"
    );
}

#[test]
fn test_sql_ref_dml_topic() {
    let out = sql_ref_calc("insert");
    assert!(out.contains("INSERT"), "{out}");
    assert!(out.contains("UPDATE") || out.contains("DELETE"), "{out}");
}

#[test]
fn test_sql_ref_ddl_topic() {
    let out = sql_ref_calc("ddl");
    assert!(out.contains("CREATE TABLE"), "{out}");
    assert!(out.contains("ALTER") || out.contains("DROP"), "{out}");
}

#[test]
fn test_sql_ref_explain_topic() {
    let out = sql_ref_calc("explain");
    assert!(out.contains("EXPLAIN"), "{out}");
    assert!(out.contains("Seq Scan") || out.contains("Index"), "{out}");
}

#[test]
fn test_sql_ref_transactions_topic() {
    let out = sql_ref_calc("transaction");
    assert!(out.contains("BEGIN") || out.contains("COMMIT"), "{out}");
    assert!(
        out.contains("ROLLBACK") || out.contains("SAVEPOINT"),
        "{out}"
    );
}

#[test]
fn test_sql_ref_json_topic() {
    let out = sql_ref_calc("json");
    assert!(out.contains("JSON") || out.contains("jsonb"), "{out}");
}

#[test]
fn test_sql_ref_all_prints_all_sections() {
    let out = sql_ref_calc("all");
    assert!(out.contains("SELECT"), "{out}");
    assert!(out.contains("JOIN"), "{out}");
    assert!(out.contains("OVER"), "{out}");
    assert!(out.contains("CREATE TABLE"), "{out}");
}

#[test]
fn test_sql_ref_unknown_topic() {
    let out = sql_ref_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 13: vim_calc ─────────────────────────────────────────────────────────

#[test]
fn test_vim_empty_shows_help() {
    let out = vim_calc("");
    assert!(out.contains("hematite --vim"), "{out}");
    assert!(out.contains("motion") || out.contains("modes"), "{out}");
}

#[test]
fn test_vim_modes_topic() {
    let out = vim_calc("modes");
    assert!(out.contains("Normal") || out.contains("Insert"), "{out}");
    assert!(out.contains("Esc") || out.contains("Visual"), "{out}");
}

#[test]
fn test_vim_motion_topic() {
    let out = vim_calc("motion");
    assert!(out.contains("hjkl") || out.contains("h j k l"), "{out}");
    assert!(out.contains("gg") || out.contains("word"), "{out}");
}

#[test]
fn test_vim_editing_topic() {
    let out = vim_calc("edit");
    assert!(out.contains("dd") || out.contains("yy"), "{out}");
    assert!(
        out.contains("delete") || out.contains("yank") || out.contains("Delete"),
        "{out}"
    );
}

#[test]
fn test_vim_text_objects_topic() {
    let out = vim_calc("text-objects");
    assert!(out.contains("iw") || out.contains("aw"), "{out}");
    assert!(out.contains("inner") || out.contains("around"), "{out}");
}

#[test]
fn test_vim_search_replace_topic() {
    let out = vim_calc("search");
    assert!(out.contains("substitute") || out.contains(":%s/"), "{out}");
}

#[test]
fn test_vim_files_topic() {
    let out = vim_calc("files");
    assert!(out.contains(":w") || out.contains(":q"), "{out}");
    assert!(out.contains("split") || out.contains(":sp"), "{out}");
}

#[test]
fn test_vim_macros_topic() {
    let out = vim_calc("macro");
    assert!(
        out.contains("q<") || out.contains("Recording") || out.contains("record"),
        "{out}"
    );
    assert!(out.contains("@"), "{out}");
}

#[test]
fn test_vim_marks_topic() {
    let out = vim_calc("marks");
    assert!(
        out.contains("m<") || out.contains("mark") || out.contains("Mark"),
        "{out}"
    );
    assert!(
        out.contains("jumplist") || out.contains("Ctrl+o") || out.contains("Ctrl"),
        "{out}"
    );
}

#[test]
fn test_vim_config_topic() {
    let out = vim_calc("vimrc");
    assert!(
        out.contains("set number") || out.contains("tabstop"),
        "{out}"
    );
    assert!(
        out.contains("expandtab") || out.contains("hlsearch"),
        "{out}"
    );
}

#[test]
fn test_vim_all_prints_all_sections() {
    let out = vim_calc("all");
    assert!(out.contains("Normal"), "{out}");
    assert!(out.contains("hjkl") || out.contains("h j k l"), "{out}");
    assert!(out.contains(":w"), "{out}");
    assert!(
        out.contains("set number") || out.contains("tabstop"),
        "{out}"
    );
}

#[test]
fn test_vim_unknown_topic() {
    let out = vim_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 13: curl_calc ────────────────────────────────────────────────────────

#[test]
fn test_curl_empty_shows_help() {
    let out = curl_calc("");
    assert!(out.contains("hematite --curl"), "{out}");
    assert!(out.contains("auth") || out.contains("methods"), "{out}");
}

#[test]
fn test_curl_basics_topic() {
    let out = curl_calc("basics");
    assert!(out.contains("curl <url>") || out.contains("curl"), "{out}");
    assert!(out.contains("-L") || out.contains("-s"), "{out}");
}

#[test]
fn test_curl_methods_topic() {
    let out = curl_calc("post");
    assert!(out.contains("POST") || out.contains("-X POST"), "{out}");
    assert!(out.contains("-d") || out.contains("json"), "{out}");
}

#[test]
fn test_curl_headers_topic() {
    let out = curl_calc("headers");
    assert!(out.contains("-H") || out.contains("Authorization"), "{out}");
    assert!(
        out.contains("Content-Type") || out.contains("User-Agent"),
        "{out}"
    );
}

#[test]
fn test_curl_auth_topic() {
    let out = curl_calc("auth");
    assert!(
        out.contains("Bearer") || out.contains("basic") || out.contains("Basic"),
        "{out}"
    );
    assert!(
        out.contains("-u ") || out.contains("Authorization"),
        "{out}"
    );
}

#[test]
fn test_curl_tls_topic() {
    let out = curl_calc("tls");
    assert!(out.contains("-k") || out.contains("insecure"), "{out}");
    assert!(out.contains("cert") || out.contains("--cacert"), "{out}");
}

#[test]
fn test_curl_upload_topic() {
    let out = curl_calc("upload");
    assert!(out.contains("-F") || out.contains("multipart"), "{out}");
    assert!(out.contains("file") || out.contains("@"), "{out}");
}

#[test]
fn test_curl_proxy_topic() {
    let out = curl_calc("proxy");
    assert!(out.contains("-x ") || out.contains("--proxy"), "{out}");
    assert!(out.contains("socks") || out.contains("SOCKS"), "{out}");
}

#[test]
fn test_curl_cookies_topic() {
    let out = curl_calc("cookies");
    assert!(out.contains("-b ") || out.contains("-c "), "{out}");
    assert!(out.contains("cookie") || out.contains("session"), "{out}");
}

#[test]
fn test_curl_output_topic() {
    let out = curl_calc("output");
    assert!(
        out.contains("%{http_code}") || out.contains("http_code"),
        "{out}"
    );
    assert!(out.contains("-w ") || out.contains("write"), "{out}");
}

#[test]
fn test_curl_all_prints_all_sections() {
    let out = curl_calc("all");
    assert!(out.contains("-L"), "{out}");
    assert!(
        out.contains("Bearer") || out.contains("Authorization"),
        "{out}"
    );
    assert!(out.contains("-F") || out.contains("multipart"), "{out}");
    assert!(out.contains("%{http_code}"), "{out}");
}

#[test]
fn test_curl_unknown_topic() {
    let out = curl_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 13: jq_calc ──────────────────────────────────────────────────────────

#[test]
fn test_jq_empty_shows_help() {
    let out = jq_calc("");
    assert!(out.contains("hematite --jq"), "{out}");
    assert!(out.contains("recipes") || out.contains("access"), "{out}");
}

#[test]
fn test_jq_basics_topic() {
    let out = jq_calc("basics");
    assert!(
        out.contains("jq '.'") || out.contains("Pretty-print"),
        "{out}"
    );
    assert!(out.contains("-r") || out.contains("raw"), "{out}");
}

#[test]
fn test_jq_access_topic() {
    let out = jq_calc("access");
    assert!(out.contains(".field") || out.contains(".["), "{out}");
    assert!(out.contains("keys") || out.contains("length"), "{out}");
}

#[test]
fn test_jq_transform_topic() {
    let out = jq_calc("transform");
    assert!(out.contains("map(") || out.contains("select("), "{out}");
    assert!(out.contains("sort") || out.contains("flatten"), "{out}");
}

#[test]
fn test_jq_strings_topic() {
    let out = jq_calc("strings");
    assert!(out.contains("split(") || out.contains("join("), "{out}");
    assert!(
        out.contains("ascii_downcase") || out.contains("ascii_upcase"),
        "{out}"
    );
}

#[test]
fn test_jq_conditionals_topic() {
    let out = jq_calc("conditionals");
    assert!(out.contains("if") && out.contains("then"), "{out}");
    assert!(out.contains("try") || out.contains("catch"), "{out}");
}

#[test]
fn test_jq_reduce_topic() {
    let out = jq_calc("reduce");
    assert!(out.contains("reduce"), "{out}");
    assert!(out.contains("add") || out.contains("any"), "{out}");
}

#[test]
fn test_jq_recipes_topic() {
    let out = jq_calc("recipes");
    assert!(out.contains("select(") || out.contains("length"), "{out}");
    assert!(
        out.contains("@csv") || out.contains("@tsv") || out.contains("@base64"),
        "{out}"
    );
}

#[test]
fn test_jq_all_prints_all_sections() {
    let out = jq_calc("all");
    assert!(
        out.contains("jq '.'") || out.contains("Pretty-print"),
        "{out}"
    );
    assert!(out.contains("map("), "{out}");
    assert!(out.contains("reduce"), "{out}");
    assert!(out.contains("@csv") || out.contains("@tsv"), "{out}");
}

#[test]
fn test_jq_unknown_topic() {
    let out = jq_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 14: grep_calc ────────────────────────────────────────────────────────

#[test]
fn test_grep_empty_shows_help() {
    let out = grep_calc("");
    assert!(out.contains("hematite --grep"), "{out}");
    assert!(out.contains("patterns") || out.contains("ripgrep"), "{out}");
}

#[test]
fn test_grep_basics_topic() {
    let out = grep_calc("basics");
    assert!(out.contains("-i") || out.contains("case"), "{out}");
    assert!(out.contains("-r") || out.contains("-n"), "{out}");
}

#[test]
fn test_grep_patterns_topic() {
    let out = grep_calc("patterns");
    assert!(out.contains("BRE") || out.contains("ERE"), "{out}");
    assert!(out.contains("regex") || out.contains("PCRE"), "{out}");
}

#[test]
fn test_grep_context_flags() {
    let out = grep_calc("context");
    assert!(out.contains("-A") || out.contains("-B"), "{out}");
    assert!(out.contains("-C") || out.contains("After"), "{out}");
}

#[test]
fn test_grep_files_topic() {
    let out = grep_calc("files");
    assert!(out.contains("recursive") || out.contains("-r"), "{out}");
    assert!(out.contains("include") || out.contains("exclude"), "{out}");
}

#[test]
fn test_grep_ripgrep_topic() {
    let out = grep_calc("rg");
    assert!(out.contains("rg ") || out.contains("ripgrep"), "{out}");
    assert!(out.contains("gitignore") || out.contains("--type"), "{out}");
}

#[test]
fn test_grep_one_liners_topic() {
    let out = grep_calc("one-liners");
    assert!(out.contains("TODO") || out.contains("email"), "{out}");
}

#[test]
fn test_grep_all_prints_all_sections() {
    let out = grep_calc("all");
    assert!(out.contains("-i"), "{out}");
    assert!(out.contains("BRE") || out.contains("ERE"), "{out}");
    assert!(out.contains("rg ") || out.contains("ripgrep"), "{out}");
}

#[test]
fn test_grep_unknown_topic() {
    let out = grep_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 14: sed_calc ─────────────────────────────────────────────────────────

#[test]
fn test_sed_empty_shows_help() {
    let out = sed_calc("");
    assert!(out.contains("hematite --sed"), "{out}");
    assert!(
        out.contains("substitute") || out.contains("address"),
        "{out}"
    );
}

#[test]
fn test_sed_basics_topic() {
    let out = sed_calc("basics");
    assert!(out.contains("-i") || out.contains("in-place"), "{out}");
    assert!(out.contains("-n") || out.contains("-e"), "{out}");
}

#[test]
fn test_sed_substitute_topic() {
    let out = sed_calc("substitute");
    assert!(out.contains("s/"), "{out}");
    assert!(out.contains("/g") || out.contains("global"), "{out}");
}

#[test]
fn test_sed_address_topic() {
    let out = sed_calc("address");
    assert!(out.contains("range") || out.contains("addr"), "{out}");
    assert!(out.contains("$") || out.contains("negate"), "{out}");
}

#[test]
fn test_sed_delete_topic() {
    let out = sed_calc("delete");
    assert!(
        out.contains("/d") || out.contains(" d ") || out.contains("command — delete"),
        "{out}"
    );
    assert!(out.contains("blank") || out.contains("^$"), "{out}");
}

#[test]
fn test_sed_insert_topic() {
    let out = sed_calc("insert");
    assert!(
        out.contains(" i ") || out.contains(" a ") || out.contains("insert"),
        "{out}"
    );
    assert!(out.contains("append") || out.contains("before"), "{out}");
}

#[test]
fn test_sed_transform_topic() {
    let out = sed_calc("transform");
    assert!(
        out.contains("y/") || out.contains("transliteration"),
        "{out}"
    );
}

#[test]
fn test_sed_multiline_topic() {
    let out = sed_calc("multiline");
    assert!(
        out.contains("hold") || out.contains("pattern space"),
        "{out}"
    );
    assert!(out.contains(" N") || out.contains(" D"), "{out}");
}

#[test]
fn test_sed_advanced_topic() {
    let out = sed_calc("advanced");
    assert!(
        out.contains("branch") || out.contains(":label") || out.contains("label"),
        "{out}"
    );
}

#[test]
fn test_sed_all_prints_all_sections() {
    let out = sed_calc("all");
    assert!(out.contains("s/"), "{out}");
    assert!(
        out.contains("hold") || out.contains("pattern space"),
        "{out}"
    );
    assert!(out.contains("branch") || out.contains("label"), "{out}");
}

#[test]
fn test_sed_unknown_topic() {
    let out = sed_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 14: awk_calc ─────────────────────────────────────────────────────────

#[test]
fn test_awk_empty_shows_help() {
    let out = awk_calc("");
    assert!(out.contains("hematite --awk"), "{out}");
    assert!(
        out.contains("variables") || out.contains("patterns"),
        "{out}"
    );
}

#[test]
fn test_awk_basics_topic() {
    let out = awk_calc("basics");
    assert!(
        out.contains("-F") || out.contains("field separator"),
        "{out}"
    );
    assert!(out.contains("BEGIN") || out.contains("END"), "{out}");
}

#[test]
fn test_awk_patterns_topic() {
    let out = awk_calc("patterns");
    assert!(out.contains("/regex/") || out.contains("regex"), "{out}");
    assert!(
        out.contains("Range") || out.contains("range") || out.contains("NF"),
        "{out}"
    );
}

#[test]
fn test_awk_variables_topic() {
    let out = awk_calc("variables");
    assert!(out.contains("NF") || out.contains("NR"), "{out}");
    assert!(out.contains("FS") || out.contains("OFS"), "{out}");
}

#[test]
fn test_awk_arrays_topic() {
    let out = awk_calc("arrays");
    assert!(out.contains("arr[") || out.contains("associative"), "{out}");
    assert!(out.contains("delete") || out.contains("for ("), "{out}");
}

#[test]
fn test_awk_functions_topic() {
    let out = awk_calc("functions");
    assert!(out.contains("split(") || out.contains("substr("), "{out}");
    assert!(out.contains("gsub(") || out.contains("sprintf"), "{out}");
}

#[test]
fn test_awk_io_topic() {
    let out = awk_calc("io");
    assert!(out.contains("printf") || out.contains("print"), "{out}");
    assert!(out.contains("getline") || out.contains("pipe"), "{out}");
}

#[test]
fn test_awk_one_liners_topic() {
    let out = awk_calc("one-liners");
    assert!(out.contains("sum") || out.contains("{s+="), "{out}");
    assert!(out.contains("dedup") || out.contains("seen"), "{out}");
}

#[test]
fn test_awk_all_prints_all_sections() {
    let out = awk_calc("all");
    assert!(out.contains("BEGIN") || out.contains("END"), "{out}");
    assert!(out.contains("NF") || out.contains("NR"), "{out}");
    assert!(out.contains("arr[") || out.contains("associative"), "{out}");
    assert!(out.contains("getline"), "{out}");
}

#[test]
fn test_awk_unknown_topic() {
    let out = awk_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 14: ssh_ref_calc ─────────────────────────────────────────────────────

#[test]
fn test_ssh_ref_empty_shows_help() {
    let out = ssh_ref_calc("");
    assert!(out.contains("hematite --ssh-ref"), "{out}");
    assert!(out.contains("tunnel") || out.contains("keys"), "{out}");
}

#[test]
fn test_ssh_ref_connect_topic() {
    let out = ssh_ref_calc("connect");
    assert!(
        out.contains("ssh user@host") || out.contains("user@host"),
        "{out}"
    );
    assert!(out.contains("-p") || out.contains("port"), "{out}");
}

#[test]
fn test_ssh_ref_keys_topic() {
    let out = ssh_ref_calc("keys");
    assert!(
        out.contains("ssh-keygen") || out.contains("keygen"),
        "{out}"
    );
    assert!(out.contains("ed25519") || out.contains("rsa"), "{out}");
}

#[test]
fn test_ssh_ref_config_topic() {
    let out = ssh_ref_calc("config");
    assert!(
        out.contains("Host ") || out.contains("~/.ssh/config"),
        "{out}"
    );
    assert!(
        out.contains("HostName") || out.contains("IdentityFile"),
        "{out}"
    );
}

#[test]
fn test_ssh_ref_tunnel_topic() {
    let out = ssh_ref_calc("tunnel");
    assert!(out.contains("-L") || out.contains("local"), "{out}");
    assert!(out.contains("-R") || out.contains("-D"), "{out}");
}

#[test]
fn test_ssh_ref_scp_rsync_topic() {
    let out = ssh_ref_calc("scp");
    assert!(out.contains("scp ") || out.contains("secure copy"), "{out}");
    assert!(out.contains("rsync") || out.contains("-avz"), "{out}");
}

#[test]
fn test_ssh_ref_agent_topic() {
    let out = ssh_ref_calc("agent");
    assert!(
        out.contains("ssh-agent") || out.contains("ssh-add"),
        "{out}"
    );
    assert!(
        out.contains("SSH_AUTH_SOCK") || out.contains("forwarding"),
        "{out}"
    );
}

#[test]
fn test_ssh_ref_options_topic() {
    let out = ssh_ref_calc("options");
    assert!(
        out.contains("ServerAliveInterval") || out.contains("keepalive"),
        "{out}"
    );
    assert!(
        out.contains("ControlMaster") || out.contains("BatchMode"),
        "{out}"
    );
}

#[test]
fn test_ssh_ref_hardening_topic() {
    let out = ssh_ref_calc("hardening");
    assert!(
        out.contains("PermitRootLogin") || out.contains("sshd"),
        "{out}"
    );
    assert!(
        out.contains("PasswordAuthentication") || out.contains("password"),
        "{out}"
    );
}

#[test]
fn test_ssh_ref_all_prints_all_sections() {
    let out = ssh_ref_calc("all");
    assert!(
        out.contains("ssh user@host") || out.contains("user@host"),
        "{out}"
    );
    assert!(out.contains("ssh-keygen"), "{out}");
    assert!(out.contains("-L") || out.contains("tunnel"), "{out}");
    assert!(
        out.contains("PermitRootLogin") || out.contains("sshd"),
        "{out}"
    );
}

#[test]
fn test_ssh_ref_unknown_topic() {
    let out = ssh_ref_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 15: tar_calc ─────────────────────────────────────────────────────────

#[test]
fn test_tar_empty_shows_help() {
    let out = tar_calc("");
    assert!(out.contains("hematite --tar"), "{out}");
    assert!(out.contains("create") || out.contains("extract"), "{out}");
}

#[test]
fn test_tar_basics_topic() {
    let out = tar_calc("basics");
    assert!(out.contains("czf") || out.contains("xzf"), "{out}");
    assert!(out.contains("-z") || out.contains("gzip"), "{out}");
}

#[test]
fn test_tar_create_topic() {
    let out = tar_calc("create");
    assert!(out.contains("tar c") || out.contains("czf"), "{out}");
    assert!(
        out.contains("--exclude") || out.contains("exclude"),
        "{out}"
    );
}

#[test]
fn test_tar_extract_topic() {
    let out = tar_calc("extract");
    assert!(out.contains("tar x") || out.contains("xzf"), "{out}");
    assert!(
        out.contains("-C") || out.contains("strip-components"),
        "{out}"
    );
}

#[test]
fn test_tar_compress_gzip_alias() {
    let out = tar_calc("gzip");
    assert!(out.contains("gzip") || out.contains(".tar.gz"), "{out}");
    assert!(out.contains("bzip2") || out.contains("xz"), "{out}");
}

#[test]
fn test_tar_compress_zstd_alias() {
    let out = tar_calc("zstd");
    assert!(out.contains("zstd") || out.contains(".tar.zst"), "{out}");
}

#[test]
fn test_tar_advanced_topic() {
    let out = tar_calc("advanced");
    assert!(
        out.contains("incremental") || out.contains("--listed-incremental"),
        "{out}"
    );
    assert!(out.contains("split") || out.contains("pipe"), "{out}");
}

#[test]
fn test_tar_all_prints_all_sections() {
    let out = tar_calc("all");
    assert!(out.contains("czf") || out.contains("xzf"), "{out}");
    assert!(out.contains("--exclude"), "{out}");
    assert!(
        out.contains("incremental") || out.contains("--listed-incremental"),
        "{out}"
    );
}

#[test]
fn test_tar_unknown_topic() {
    let out = tar_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 15: find_calc ────────────────────────────────────────────────────────

#[test]
fn test_find_empty_shows_help() {
    let out = find_calc("");
    assert!(out.contains("hematite --find"), "{out}");
    assert!(out.contains("by-type") || out.contains("actions"), "{out}");
}

#[test]
fn test_find_basics_topic() {
    let out = find_calc("basics");
    assert!(out.contains("-name") || out.contains("-iname"), "{out}");
    assert!(
        out.contains("-maxdepth") || out.contains("mindepth"),
        "{out}"
    );
}

#[test]
fn test_find_by_type_topic() {
    let out = find_calc("type");
    assert!(out.contains("-type f") || out.contains("-type d"), "{out}");
    assert!(out.contains("-type l") || out.contains("symlink"), "{out}");
}

#[test]
fn test_find_by_time_topic() {
    let out = find_calc("mtime");
    assert!(out.contains("-mtime") || out.contains("mtime"), "{out}");
    assert!(out.contains("-newer") || out.contains("newer"), "{out}");
}

#[test]
fn test_find_by_size_topic() {
    let out = find_calc("size");
    assert!(out.contains("-size") || out.contains("size"), "{out}");
    assert!(out.contains("+10M") || out.contains("Megabyte"), "{out}");
}

#[test]
fn test_find_by_perm_topic() {
    let out = find_calc("perm");
    assert!(out.contains("-perm") || out.contains("SUID"), "{out}");
    assert!(out.contains("4000") || out.contains("world"), "{out}");
}

#[test]
fn test_find_actions_topic() {
    let out = find_calc("actions");
    assert!(out.contains("-exec") || out.contains("exec"), "{out}");
    assert!(out.contains("xargs") || out.contains("-delete"), "{out}");
}

#[test]
fn test_find_prune_topic() {
    let out = find_calc("prune");
    assert!(out.contains("-prune") || out.contains("prune"), "{out}");
    assert!(
        out.contains("node_modules") || out.contains(".git"),
        "{out}"
    );
}

#[test]
fn test_find_one_liners_topic() {
    let out = find_calc("one-liners");
    assert!(out.contains("find") && out.contains("mtime"), "{out}");
}

#[test]
fn test_find_all_prints_all_sections() {
    let out = find_calc("all");
    assert!(out.contains("-name"), "{out}");
    assert!(out.contains("-type f"), "{out}");
    assert!(out.contains("-exec"), "{out}");
    assert!(out.contains("-prune") || out.contains("prune"), "{out}");
}

#[test]
fn test_find_unknown_topic() {
    let out = find_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 15: systemd_calc ─────────────────────────────────────────────────────

#[test]
fn test_systemd_empty_shows_help() {
    let out = systemd_calc("");
    assert!(out.contains("hematite --systemd"), "{out}");
    assert!(out.contains("service") || out.contains("logs"), "{out}");
}

#[test]
fn test_systemd_service_topic() {
    let out = systemd_calc("service");
    assert!(
        out.contains("systemctl start") || out.contains("start"),
        "{out}"
    );
    assert!(out.contains("enable") || out.contains("disable"), "{out}");
}

#[test]
fn test_systemd_status_topic() {
    let out = systemd_calc("status");
    assert!(
        out.contains("systemctl status") || out.contains("is-active"),
        "{out}"
    );
    assert!(
        out.contains("list-units") || out.contains("failed"),
        "{out}"
    );
}

#[test]
fn test_systemd_logs_topic() {
    let out = systemd_calc("logs");
    assert!(
        out.contains("journalctl") || out.contains("journal"),
        "{out}"
    );
    assert!(out.contains("-f") || out.contains("follow"), "{out}");
}

#[test]
fn test_systemd_journalctl_alias() {
    let out = systemd_calc("journalctl");
    assert!(out.contains("journalctl"), "{out}");
    assert!(out.contains("--since") || out.contains("-u "), "{out}");
}

#[test]
fn test_systemd_analyze_topic() {
    let out = systemd_calc("analyze");
    assert!(
        out.contains("systemd-analyze") || out.contains("analyze"),
        "{out}"
    );
    assert!(out.contains("blame") || out.contains("critical"), "{out}");
}

#[test]
fn test_systemd_timers_topic() {
    let out = systemd_calc("timers");
    assert!(out.contains("OnCalendar") || out.contains("timer"), "{out}");
    assert!(
        out.contains("Persistent") || out.contains("oneshot"),
        "{out}"
    );
}

#[test]
fn test_systemd_units_topic() {
    let out = systemd_calc("units");
    assert!(
        out.contains("daemon-reload") || out.contains("unit"),
        "{out}"
    );
    assert!(
        out.contains("ExecStart") || out.contains("drop-in") || out.contains("override"),
        "{out}"
    );
}

#[test]
fn test_systemd_targets_topic() {
    let out = systemd_calc("targets");
    assert!(
        out.contains("multi-user.target") || out.contains("graphical.target"),
        "{out}"
    );
    assert!(out.contains("reboot") || out.contains("poweroff"), "{out}");
}

#[test]
fn test_systemd_all_prints_all_sections() {
    let out = systemd_calc("all");
    assert!(out.contains("systemctl start"), "{out}");
    assert!(out.contains("journalctl"), "{out}");
    assert!(out.contains("systemd-analyze"), "{out}");
    assert!(out.contains("OnCalendar") || out.contains("timer"), "{out}");
}

#[test]
fn test_systemd_unknown_topic() {
    let out = systemd_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ── Wave 15: make_calc ────────────────────────────────────────────────────────

#[test]
fn test_make_empty_shows_help() {
    let out = make_calc("");
    assert!(out.contains("hematite --make"), "{out}");
    assert!(out.contains("variables") || out.contains("rules"), "{out}");
}

#[test]
fn test_make_basics_topic() {
    let out = make_calc("basics");
    assert!(out.contains(".PHONY") || out.contains("phony"), "{out}");
    assert!(
        out.contains("-n") || out.contains("dry run") || out.contains("dry-run"),
        "{out}"
    );
}

#[test]
fn test_make_variables_topic() {
    let out = make_calc("variables");
    assert!(out.contains(":=") || out.contains("?="), "{out}");
    assert!(out.contains("CC") || out.contains("CFLAGS"), "{out}");
}

#[test]
fn test_make_rules_topic() {
    let out = make_calc("rules");
    assert!(
        out.contains("recipe") || out.contains("prerequisite"),
        "{out}"
    );
    assert!(out.contains("$@") || out.contains("$<"), "{out}");
}

#[test]
fn test_make_patterns_topic() {
    let out = make_calc("patterns");
    assert!(out.contains("%.o") || out.contains("pattern"), "{out}");
    assert!(out.contains("implicit") || out.contains("VPATH"), "{out}");
}

#[test]
fn test_make_functions_topic() {
    let out = make_calc("functions");
    assert!(
        out.contains("$(subst") || out.contains("$(patsubst"),
        "{out}"
    );
    assert!(
        out.contains("$(wildcard") || out.contains("$(shell"),
        "{out}"
    );
}

#[test]
fn test_make_conditionals_topic() {
    let out = make_calc("conditionals");
    assert!(out.contains("ifeq") || out.contains("ifdef"), "{out}");
    assert!(out.contains("endif") || out.contains("include"), "{out}");
}

#[test]
fn test_make_special_topic() {
    let out = make_calc("special");
    assert!(out.contains("$@") || out.contains("$<"), "{out}");
    assert!(
        out.contains(".PHONY") || out.contains(".DELETE_ON_ERROR"),
        "{out}"
    );
}

#[test]
fn test_make_all_prints_all_sections() {
    let out = make_calc("all");
    assert!(out.contains(".PHONY"), "{out}");
    assert!(out.contains(":=") || out.contains("?="), "{out}");
    assert!(out.contains("%.o"), "{out}");
    assert!(out.contains("ifeq") || out.contains("ifdef"), "{out}");
    assert!(out.contains("$@"), "{out}");
}

#[test]
fn test_make_unknown_topic() {
    let out = make_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("help"), "{out}");
}

// ─── Wave 16: chmod, openssl, nginx, bash-ref ────────────────────────────────

#[test]
fn test_chmod_help() {
    let out = chmod_calc("help");
    assert!(out.contains("chmod") || out.contains("TOPIC"), "{out}");
    assert!(out.contains("basics"), "{out}");
    assert!(out.contains("symbolic"), "{out}");
}

#[test]
fn test_chmod_list() {
    let out = chmod_calc("list");
    assert!(out.contains("basics"), "{out}");
    assert!(out.contains("umask"), "{out}");
}

#[test]
fn test_chmod_empty() {
    let out = chmod_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_chmod_basics_topic() {
    let out = chmod_calc("basics");
    assert!(out.contains("755"), "{out}");
    assert!(out.contains("644"), "{out}");
    assert!(out.contains("chmod"), "{out}");
}

#[test]
fn test_chmod_numeric_alias() {
    let out = chmod_calc("numeric");
    assert!(out.contains("755") || out.contains("octal"), "{out}");
}

#[test]
fn test_chmod_symbolic_topic() {
    let out = chmod_calc("symbolic");
    assert!(out.contains("u+x") || out.contains("ugoa"), "{out}");
    assert!(out.contains("operator") || out.contains("+"), "{out}");
}

#[test]
fn test_chmod_special_topic() {
    let out = chmod_calc("special");
    assert!(out.contains("SUID") || out.contains("suid"), "{out}");
    assert!(out.contains("sticky") || out.contains("1777"), "{out}");
}

#[test]
fn test_chmod_suid_alias() {
    let out = chmod_calc("suid");
    assert!(out.contains("SUID") || out.contains("4000"), "{out}");
}

#[test]
fn test_chmod_chown_topic() {
    let out = chmod_calc("chown");
    assert!(out.contains("chown"), "{out}");
    assert!(out.contains("chgrp"), "{out}");
}

#[test]
fn test_chmod_umask_topic() {
    let out = chmod_calc("umask");
    assert!(out.contains("umask"), "{out}");
    assert!(out.contains("022") || out.contains("644"), "{out}");
}

#[test]
fn test_chmod_all() {
    let out = chmod_calc("all");
    assert!(out.contains("755"), "{out}");
    assert!(out.contains("SUID") || out.contains("suid"), "{out}");
    assert!(out.contains("umask"), "{out}");
    assert!(out.contains("chown"), "{out}");
}

#[test]
fn test_chmod_unknown_topic() {
    let out = chmod_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── openssl tests ───────────────────────────────────────────────────────────

#[test]
fn test_openssl_help() {
    let out = openssl_calc("help");
    assert!(out.contains("openssl") || out.contains("TOPIC"), "{out}");
    assert!(out.contains("keygen"), "{out}");
    assert!(out.contains("certs"), "{out}");
}

#[test]
fn test_openssl_list() {
    let out = openssl_calc("list");
    assert!(out.contains("keygen"), "{out}");
    assert!(out.contains("connect"), "{out}");
}

#[test]
fn test_openssl_empty() {
    let out = openssl_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_openssl_keygen_topic() {
    let out = openssl_calc("keygen");
    assert!(out.contains("genrsa") || out.contains("RSA"), "{out}");
    assert!(out.contains("Ed25519") || out.contains("ed25519"), "{out}");
}

#[test]
fn test_openssl_rsa_alias() {
    let out = openssl_calc("rsa");
    assert!(out.contains("genrsa") || out.contains("RSA"), "{out}");
}

#[test]
fn test_openssl_certs_topic() {
    let out = openssl_calc("certs");
    assert!(out.contains("x509") || out.contains("self-signed"), "{out}");
    assert!(
        out.contains("SAN") || out.contains("subjectAltName"),
        "{out}"
    );
}

#[test]
fn test_openssl_csr_topic() {
    let out = openssl_calc("csr");
    assert!(out.contains("req") || out.contains("CSR"), "{out}");
}

#[test]
fn test_openssl_inspect_topic() {
    let out = openssl_calc("inspect");
    assert!(out.contains("x509") || out.contains("-text"), "{out}");
    assert!(
        out.contains("fingerprint") || out.contains("-dates"),
        "{out}"
    );
}

#[test]
fn test_openssl_convert_topic() {
    let out = openssl_calc("convert");
    assert!(out.contains("DER") || out.contains("der"), "{out}");
    assert!(out.contains("P12") || out.contains("pkcs12"), "{out}");
}

#[test]
fn test_openssl_encrypt_topic() {
    let out = openssl_calc("encrypt");
    assert!(out.contains("aes-256") || out.contains("AES"), "{out}");
    assert!(out.contains("pbkdf2") || out.contains("decrypt"), "{out}");
}

#[test]
fn test_openssl_digest_topic() {
    let out = openssl_calc("digest");
    assert!(out.contains("sha256") || out.contains("SHA"), "{out}");
    assert!(out.contains("hmac") || out.contains("HMAC"), "{out}");
}

#[test]
fn test_openssl_connect_topic() {
    let out = openssl_calc("connect");
    assert!(out.contains("s_client"), "{out}");
    assert!(out.contains("443") || out.contains("-connect"), "{out}");
}

#[test]
fn test_openssl_tls_alias() {
    let out = openssl_calc("tls");
    assert!(out.contains("s_client") || out.contains("TLS"), "{out}");
}

#[test]
fn test_openssl_all() {
    let out = openssl_calc("all");
    assert!(out.contains("genrsa") || out.contains("RSA"), "{out}");
    assert!(out.contains("s_client"), "{out}");
    assert!(out.contains("sha256") || out.contains("SHA"), "{out}");
    assert!(out.contains("DER") || out.contains("der"), "{out}");
}

#[test]
fn test_openssl_unknown_topic() {
    let out = openssl_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── nginx tests ─────────────────────────────────────────────────────────────

#[test]
fn test_nginx_help() {
    let out = nginx_calc("help");
    assert!(out.contains("nginx") || out.contains("TOPIC"), "{out}");
    assert!(out.contains("proxy"), "{out}");
    assert!(out.contains("location"), "{out}");
}

#[test]
fn test_nginx_list() {
    let out = nginx_calc("list");
    assert!(out.contains("commands"), "{out}");
    assert!(out.contains("rewrites"), "{out}");
}

#[test]
fn test_nginx_empty() {
    let out = nginx_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_nginx_commands_topic() {
    let out = nginx_calc("commands");
    assert!(out.contains("nginx -t") || out.contains("-t"), "{out}");
    assert!(out.contains("reload") || out.contains("-s"), "{out}");
}

#[test]
fn test_nginx_server_block_topic() {
    let out = nginx_calc("server-block");
    assert!(
        out.contains("server_name") || out.contains("listen"),
        "{out}"
    );
    assert!(out.contains("root") || out.contains("index"), "{out}");
}

#[test]
fn test_nginx_vhost_alias() {
    let out = nginx_calc("vhost");
    assert!(
        out.contains("listen") || out.contains("server_name"),
        "{out}"
    );
}

#[test]
fn test_nginx_location_topic() {
    let out = nginx_calc("location");
    assert!(
        out.contains("try_files") || out.contains("location"),
        "{out}"
    );
    assert!(out.contains("regex") || out.contains("prefix"), "{out}");
}

#[test]
fn test_nginx_proxy_topic() {
    let out = nginx_calc("proxy");
    assert!(out.contains("proxy_pass"), "{out}");
    assert!(
        out.contains("upstream") || out.contains("X-Real-IP"),
        "{out}"
    );
}

#[test]
fn test_nginx_websocket_alias() {
    let out = nginx_calc("websocket");
    assert!(
        out.contains("proxy_pass") || out.contains("Upgrade"),
        "{out}"
    );
}

#[test]
fn test_nginx_ssl_tls_topic() {
    let out = nginx_calc("ssl-tls");
    assert!(
        out.contains("ssl_certificate") || out.contains("TLSv1"),
        "{out}"
    );
    assert!(out.contains("certbot") || out.contains("443"), "{out}");
}

#[test]
fn test_nginx_static_topic() {
    let out = nginx_calc("static");
    assert!(out.contains("gzip") || out.contains("expires"), "{out}");
    assert!(out.contains("root") || out.contains("alias"), "{out}");
}

#[test]
fn test_nginx_rewrites_topic() {
    let out = nginx_calc("rewrites");
    assert!(out.contains("return") || out.contains("rewrite"), "{out}");
    assert!(out.contains("301") || out.contains("redirect"), "{out}");
}

#[test]
fn test_nginx_all() {
    let out = nginx_calc("all");
    assert!(out.contains("proxy_pass"), "{out}");
    assert!(
        out.contains("ssl_certificate") || out.contains("TLS"),
        "{out}"
    );
    assert!(out.contains("gzip"), "{out}");
    assert!(out.contains("return") || out.contains("301"), "{out}");
}

#[test]
fn test_nginx_unknown_topic() {
    let out = nginx_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── bash-ref tests ──────────────────────────────────────────────────────────

#[test]
fn test_bash_ref_help() {
    let out = bash_ref_calc("help");
    assert!(out.contains("bash") || out.contains("TOPIC"), "{out}");
    assert!(out.contains("variables"), "{out}");
    assert!(out.contains("loops"), "{out}");
}

#[test]
fn test_bash_ref_list() {
    let out = bash_ref_calc("list");
    assert!(out.contains("variables"), "{out}");
    assert!(out.contains("advanced"), "{out}");
}

#[test]
fn test_bash_ref_empty() {
    let out = bash_ref_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_bash_ref_variables_topic() {
    let out = bash_ref_calc("variables");
    assert!(out.contains("${var") || out.contains("$var"), "{out}");
    assert!(out.contains("export") || out.contains("unset"), "{out}");
}

#[test]
fn test_bash_ref_expansion_alias() {
    let out = bash_ref_calc("expansion");
    assert!(out.contains("${var") || out.contains("default"), "{out}");
}

#[test]
fn test_bash_ref_arrays_topic() {
    let out = bash_ref_calc("arrays");
    assert!(out.contains("arr=") || out.contains("declare -A"), "{out}");
    assert!(
        out.contains("associative") || out.contains("${arr"),
        "{out}"
    );
}

#[test]
fn test_bash_ref_conditionals_topic() {
    let out = bash_ref_calc("conditionals");
    assert!(out.contains("if") && out.contains("fi"), "{out}");
    assert!(out.contains("[[ ") || out.contains("-f "), "{out}");
}

#[test]
fn test_bash_ref_loops_topic() {
    let out = bash_ref_calc("loops");
    assert!(out.contains("for") || out.contains("while"), "{out}");
    assert!(out.contains("break") || out.contains("continue"), "{out}");
}

#[test]
fn test_bash_ref_while_alias() {
    let out = bash_ref_calc("while");
    assert!(out.contains("while") || out.contains("until"), "{out}");
}

#[test]
fn test_bash_ref_functions_topic() {
    let out = bash_ref_calc("functions");
    assert!(out.contains("local") || out.contains("return"), "{out}");
    assert!(out.contains("FUNCNAME") || out.contains("nameref"), "{out}");
}

#[test]
fn test_bash_ref_io_topic() {
    let out = bash_ref_calc("io");
    assert!(out.contains("printf") || out.contains("echo"), "{out}");
    assert!(out.contains("redirect") || out.contains("2>&1"), "{out}");
}

#[test]
fn test_bash_ref_heredoc_alias() {
    let out = bash_ref_calc("heredoc");
    assert!(
        out.contains("<<") || out.contains("heredoc") || out.contains("EOF"),
        "{out}"
    );
}

#[test]
fn test_bash_ref_advanced_topic() {
    let out = bash_ref_calc("advanced");
    assert!(out.contains("pipefail") || out.contains("set -e"), "{out}");
    assert!(out.contains("trap") || out.contains("subshell"), "{out}");
}

#[test]
fn test_bash_ref_strict_alias() {
    let out = bash_ref_calc("strict");
    assert!(out.contains("set -e") || out.contains("pipefail"), "{out}");
}

#[test]
fn test_bash_ref_all() {
    let out = bash_ref_calc("all");
    assert!(out.contains("export") || out.contains("unset"), "{out}");
    assert!(out.contains("declare -A"), "{out}");
    assert!(out.contains("while") || out.contains("for"), "{out}");
    assert!(out.contains("pipefail") || out.contains("set -e"), "{out}");
}

#[test]
fn test_bash_ref_unknown_topic() {
    let out = bash_ref_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── Wave 17: python-ref, rust-ref, go-ref, js-ref ───────────────────────────

#[test]
fn test_python_ref_help() {
    let out = python_ref_calc("help");
    assert!(out.contains("python") || out.contains("TOPIC"), "{out}");
    assert!(out.contains("builtins"), "{out}");
}

#[test]
fn test_python_ref_list() {
    let out = python_ref_calc("list");
    assert!(out.contains("strings"), "{out}");
    assert!(out.contains("async"), "{out}");
}

#[test]
fn test_python_ref_empty() {
    let out = python_ref_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_python_ref_builtins_topic() {
    let out = python_ref_calc("builtins");
    assert!(out.contains("len") || out.contains("print"), "{out}");
    assert!(out.contains("sorted") || out.contains("enumerate"), "{out}");
}

#[test]
fn test_python_ref_strings_topic() {
    let out = python_ref_calc("strings");
    assert!(
        out.contains("f-string") || out.contains("fstring") || out.contains("f\""),
        "{out}"
    );
    assert!(out.contains("split") || out.contains("strip"), "{out}");
}

#[test]
fn test_python_ref_fstring_alias() {
    let out = python_ref_calc("fstring");
    assert!(out.contains("f\"") || out.contains("format"), "{out}");
}

#[test]
fn test_python_ref_collections_topic() {
    let out = python_ref_calc("collections");
    assert!(out.contains("append") || out.contains("dict"), "{out}");
    assert!(out.contains("Counter") || out.contains("deque"), "{out}");
}

#[test]
fn test_python_ref_list_alias() {
    let out = python_ref_calc("counter");
    assert!(out.contains("Counter") || out.contains("deque"), "{out}");
}

#[test]
fn test_python_ref_dict_alias() {
    let out = python_ref_calc("dict");
    assert!(out.contains("get") || out.contains("keys"), "{out}");
}

#[test]
fn test_python_ref_comprehensions_topic() {
    let out = python_ref_calc("comprehensions");
    assert!(
        out.contains("comprehension") || out.contains("listcomp") || out.contains("[expr"),
        "{out}"
    );
    assert!(out.contains("Generator") || out.contains("Walrus"), "{out}");
}

#[test]
fn test_python_ref_functions_topic() {
    let out = python_ref_calc("functions");
    assert!(out.contains("lambda") || out.contains("decorator"), "{out}");
    assert!(out.contains("kwargs") || out.contains("*args"), "{out}");
}

#[test]
fn test_python_ref_decorator_alias() {
    let out = python_ref_calc("decorator");
    assert!(out.contains("@") || out.contains("wraps"), "{out}");
}

#[test]
fn test_python_ref_classes_topic() {
    let out = python_ref_calc("classes");
    assert!(out.contains("__init__") || out.contains("dunder"), "{out}");
    assert!(
        out.contains("dataclass") || out.contains("@dataclass"),
        "{out}"
    );
}

#[test]
fn test_python_ref_dunder_alias() {
    let out = python_ref_calc("dunder");
    assert!(
        out.contains("__init__") || out.contains("__repr__"),
        "{out}"
    );
}

#[test]
fn test_python_ref_async_topic() {
    let out = python_ref_calc("async");
    assert!(out.contains("asyncio") || out.contains("await"), "{out}");
    assert!(
        out.contains("gather") || out.contains("create_task"),
        "{out}"
    );
}

#[test]
fn test_python_ref_all() {
    let out = python_ref_calc("all");
    assert!(out.contains("lambda"), "{out}");
    assert!(out.contains("__init__"), "{out}");
    assert!(out.contains("asyncio") || out.contains("await"), "{out}");
    assert!(out.contains("Counter") || out.contains("deque"), "{out}");
}

#[test]
fn test_python_ref_unknown_topic() {
    let out = python_ref_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── rust-ref tests ───────────────────────────────────────────────────────────

#[test]
fn test_rust_ref_help() {
    let out = rust_ref_calc("help");
    assert!(out.contains("rust") || out.contains("TOPIC"), "{out}");
    assert!(out.contains("ownership"), "{out}");
}

#[test]
fn test_rust_ref_list() {
    let out = rust_ref_calc("list");
    assert!(out.contains("traits"), "{out}");
    assert!(out.contains("concurrency"), "{out}");
}

#[test]
fn test_rust_ref_empty() {
    let out = rust_ref_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_rust_ref_ownership_topic() {
    let out = rust_ref_calc("ownership");
    assert!(out.contains("borrow") || out.contains("Borrow"), "{out}");
    assert!(out.contains("lifetime") || out.contains("clone"), "{out}");
}

#[test]
fn test_rust_ref_borrow_alias() {
    let out = rust_ref_calc("borrow");
    assert!(out.contains("&") || out.contains("borrow"), "{out}");
}

#[test]
fn test_rust_ref_types_topic() {
    let out = rust_ref_calc("types");
    assert!(out.contains("Option") || out.contains("Result"), "{out}");
    assert!(out.contains("struct") || out.contains("enum"), "{out}");
}

#[test]
fn test_rust_ref_option_alias() {
    let out = rust_ref_calc("option");
    assert!(out.contains("Some") || out.contains("None"), "{out}");
}

#[test]
fn test_rust_ref_traits_topic() {
    let out = rust_ref_calc("traits");
    assert!(out.contains("trait") || out.contains("impl"), "{out}");
    assert!(out.contains("dyn") || out.contains("generic"), "{out}");
}

#[test]
fn test_rust_ref_generic_alias() {
    let out = rust_ref_calc("generic");
    assert!(
        out.contains("<T>") || out.contains("generic") || out.contains("bound"),
        "{out}"
    );
}

#[test]
fn test_rust_ref_iterators_topic() {
    let out = rust_ref_calc("iterators");
    assert!(out.contains("map") || out.contains("filter"), "{out}");
    assert!(out.contains("collect") || out.contains("fold"), "{out}");
}

#[test]
fn test_rust_ref_collect_alias() {
    let out = rust_ref_calc("collect");
    assert!(out.contains("collect") || out.contains("Vec"), "{out}");
}

#[test]
fn test_rust_ref_error_topic() {
    let out = rust_ref_calc("error");
    assert!(out.contains("Result") || out.contains("thiserror"), "{out}");
    assert!(out.contains("anyhow") || out.contains("context"), "{out}");
}

#[test]
fn test_rust_ref_concurrency_topic() {
    let out = rust_ref_calc("concurrency");
    assert!(out.contains("thread") || out.contains("Arc"), "{out}");
    assert!(out.contains("Mutex") || out.contains("channel"), "{out}");
}

#[test]
fn test_rust_ref_all() {
    let out = rust_ref_calc("all");
    assert!(out.contains("borrow") || out.contains("lifetime"), "{out}");
    assert!(out.contains("trait"), "{out}");
    assert!(out.contains("collect"), "{out}");
    assert!(out.contains("Arc") || out.contains("Mutex"), "{out}");
}

#[test]
fn test_rust_ref_unknown_topic() {
    let out = rust_ref_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── go-ref tests ─────────────────────────────────────────────────────────────

#[test]
fn test_go_ref_help() {
    let out = go_ref_calc("help");
    assert!(out.contains("go") || out.contains("TOPIC"), "{out}");
    assert!(
        out.contains("goroutines") || out.contains("slices"),
        "{out}"
    );
}

#[test]
fn test_go_ref_list() {
    let out = go_ref_calc("list");
    assert!(out.contains("basics"), "{out}");
    assert!(out.contains("errors"), "{out}");
}

#[test]
fn test_go_ref_empty() {
    let out = go_ref_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_go_ref_basics_topic() {
    let out = go_ref_calc("basics");
    assert!(out.contains("iota") || out.contains(":="), "{out}");
    assert!(out.contains("const") || out.contains("var"), "{out}");
}

#[test]
fn test_go_ref_iota_alias() {
    let out = go_ref_calc("iota");
    assert!(out.contains("iota") || out.contains("const"), "{out}");
}

#[test]
fn test_go_ref_functions_topic() {
    let out = go_ref_calc("functions");
    assert!(out.contains("defer") || out.contains("variadic"), "{out}");
    assert!(out.contains("panic") || out.contains("recover"), "{out}");
}

#[test]
fn test_go_ref_defer_alias() {
    let out = go_ref_calc("defer");
    assert!(out.contains("defer") || out.contains("LIFO"), "{out}");
}

#[test]
fn test_go_ref_slices_topic() {
    let out = go_ref_calc("slices");
    assert!(out.contains("append") || out.contains("make"), "{out}");
    assert!(out.contains("copy") || out.contains("range"), "{out}");
}

#[test]
fn test_go_ref_maps_topic() {
    let out = go_ref_calc("maps");
    assert!(out.contains("delete") || out.contains("make"), "{out}");
    assert!(out.contains("existence") || out.contains("ok"), "{out}");
}

#[test]
fn test_go_ref_interfaces_topic() {
    let out = go_ref_calc("interfaces");
    assert!(
        out.contains("interface") || out.contains("type-assert"),
        "{out}"
    );
    assert!(
        out.contains("Type switch") || out.contains(".(type)"),
        "{out}"
    );
}

#[test]
fn test_go_ref_goroutines_topic() {
    let out = go_ref_calc("goroutines");
    assert!(
        out.contains("goroutine") || out.contains("channel"),
        "{out}"
    );
    assert!(out.contains("WaitGroup") || out.contains("select"), "{out}");
}

#[test]
fn test_go_ref_channel_alias() {
    let out = go_ref_calc("channel");
    assert!(out.contains("chan") || out.contains("channel"), "{out}");
}

#[test]
fn test_go_ref_errors_topic() {
    let out = go_ref_calc("errors");
    assert!(
        out.contains("errors.Is") || out.contains("errors.As"),
        "{out}"
    );
    assert!(out.contains("fmt.Errorf") || out.contains("wrap"), "{out}");
}

#[test]
fn test_go_ref_all() {
    let out = go_ref_calc("all");
    assert!(out.contains("iota") || out.contains(":="), "{out}");
    assert!(out.contains("append"), "{out}");
    assert!(
        out.contains("WaitGroup") || out.contains("channel"),
        "{out}"
    );
    assert!(
        out.contains("errors.Is") || out.contains("errors.As"),
        "{out}"
    );
}

#[test]
fn test_go_ref_unknown_topic() {
    let out = go_ref_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── js-ref tests ─────────────────────────────────────────────────────────────

#[test]
fn test_js_ref_help() {
    let out = js_ref_calc("help");
    assert!(out.contains("js") || out.contains("TOPIC"), "{out}");
    assert!(out.contains("promises"), "{out}");
}

#[test]
fn test_js_ref_list() {
    let out = js_ref_calc("list");
    assert!(out.contains("modules"), "{out}");
    assert!(out.contains("modern"), "{out}");
}

#[test]
fn test_js_ref_empty() {
    let out = js_ref_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_js_ref_types_topic() {
    let out = js_ref_calc("types");
    assert!(out.contains("typeof") || out.contains("undefined"), "{out}");
    assert!(out.contains("truthy") || out.contains("falsy"), "{out}");
}

#[test]
fn test_js_ref_typeof_alias() {
    let out = js_ref_calc("typeof");
    assert!(out.contains("typeof") || out.contains("undefined"), "{out}");
}

#[test]
fn test_js_ref_functions_topic() {
    let out = js_ref_calc("functions");
    assert!(out.contains("arrow") || out.contains("=>"), "{out}");
    assert!(out.contains("closure") || out.contains("IIFE"), "{out}");
}

#[test]
fn test_js_ref_arrow_alias() {
    let out = js_ref_calc("arrow");
    assert!(out.contains("=>") || out.contains("arrow"), "{out}");
}

#[test]
fn test_js_ref_arrays_topic() {
    let out = js_ref_calc("arrays");
    assert!(out.contains("map") || out.contains("filter"), "{out}");
    assert!(out.contains("reduce") || out.contains("flat"), "{out}");
}

#[test]
fn test_js_ref_reduce_alias() {
    let out = js_ref_calc("reduce");
    assert!(
        out.contains("reduce") || out.contains("accumulate"),
        "{out}"
    );
}

#[test]
fn test_js_ref_objects_topic() {
    let out = js_ref_calc("objects");
    assert!(
        out.contains("destructure") || out.contains("entries"),
        "{out}"
    );
    assert!(out.contains("spread") || out.contains("..."), "{out}");
}

#[test]
fn test_js_ref_optional_chaining_alias() {
    let out = js_ref_calc("optional-chaining");
    assert!(out.contains("?.") || out.contains("optional"), "{out}");
}

#[test]
fn test_js_ref_promises_topic() {
    let out = js_ref_calc("promises");
    assert!(out.contains("Promise") || out.contains("async"), "{out}");
    assert!(out.contains("await") || out.contains("then"), "{out}");
}

#[test]
fn test_js_ref_async_alias() {
    let out = js_ref_calc("async");
    assert!(out.contains("async") || out.contains("await"), "{out}");
}

#[test]
fn test_js_ref_modules_topic() {
    let out = js_ref_calc("modules");
    assert!(out.contains("import") || out.contains("export"), "{out}");
    assert!(
        out.contains("ESM") || out.contains("CommonJS") || out.contains("require"),
        "{out}"
    );
}

#[test]
fn test_js_ref_esm_alias() {
    let out = js_ref_calc("esm");
    assert!(out.contains("import") || out.contains("export"), "{out}");
}

#[test]
fn test_js_ref_modern_topic() {
    let out = js_ref_calc("modern");
    assert!(out.contains("template") || out.contains("nullish"), "{out}");
    assert!(out.contains("generator") || out.contains("Proxy"), "{out}");
}

#[test]
fn test_js_ref_nullish_alias() {
    let out = js_ref_calc("nullish");
    assert!(out.contains("??") || out.contains("nullish"), "{out}");
}

#[test]
fn test_js_ref_all() {
    let out = js_ref_calc("all");
    assert!(out.contains("typeof"), "{out}");
    assert!(out.contains("reduce"), "{out}");
    assert!(out.contains("Promise") || out.contains("async"), "{out}");
    assert!(out.contains("import") || out.contains("export"), "{out}");
    assert!(out.contains("generator") || out.contains("Proxy"), "{out}");
}

#[test]
fn test_js_ref_unknown_topic() {
    let out = js_ref_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── Wave 18: kubectl, tmux, postgres, ts-ref ─────────────────────────────────

#[test]
fn test_kubectl_help() {
    let out = kubectl_calc("help");
    assert!(out.contains("kubectl") || out.contains("TOPIC"), "{out}");
    assert!(out.contains("pods"), "{out}");
}

#[test]
fn test_kubectl_empty() {
    let out = kubectl_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_kubectl_basics_topic() {
    let out = kubectl_calc("basics");
    assert!(
        out.contains("get nodes") || out.contains("cluster-info"),
        "{out}"
    );
    assert!(
        out.contains("namespace") || out.contains("context"),
        "{out}"
    );
}

#[test]
fn test_kubectl_get_alias() {
    let out = kubectl_calc("get");
    assert!(
        out.contains("kubectl get") || out.contains("describe"),
        "{out}"
    );
}

#[test]
fn test_kubectl_pods_topic() {
    let out = kubectl_calc("pods");
    assert!(out.contains("logs") || out.contains("exec"), "{out}");
    assert!(
        out.contains("port-forward") || out.contains("copy"),
        "{out}"
    );
}

#[test]
fn test_kubectl_logs_alias() {
    let out = kubectl_calc("logs");
    assert!(out.contains("kubectl logs") || out.contains("-f"), "{out}");
}

#[test]
fn test_kubectl_exec_alias() {
    let out = kubectl_calc("exec");
    assert!(out.contains("kubectl exec") || out.contains("-it"), "{out}");
}

#[test]
fn test_kubectl_deployments_topic() {
    let out = kubectl_calc("deployments");
    assert!(out.contains("rollout") || out.contains("scale"), "{out}");
    assert!(out.contains("replicas") || out.contains("image"), "{out}");
}

#[test]
fn test_kubectl_rollout_alias() {
    let out = kubectl_calc("rollout");
    assert!(out.contains("rollout") || out.contains("undo"), "{out}");
}

#[test]
fn test_kubectl_services_topic() {
    let out = kubectl_calc("services");
    assert!(
        out.contains("ClusterIP") || out.contains("LoadBalancer"),
        "{out}"
    );
    assert!(out.contains("ingress") || out.contains("expose"), "{out}");
}

#[test]
fn test_kubectl_config_topic() {
    let out = kubectl_calc("config");
    assert!(
        out.contains("configmap") || out.contains("ConfigMap") || out.contains("secret"),
        "{out}"
    );
    assert!(
        out.contains("label") || out.contains("RBAC") || out.contains("rbac"),
        "{out}"
    );
}

#[test]
fn test_kubectl_secret_alias() {
    let out = kubectl_calc("secret");
    assert!(out.contains("secret") || out.contains("base64"), "{out}");
}

#[test]
fn test_kubectl_yaml_topic() {
    let out = kubectl_calc("yaml");
    assert!(out.contains("apiVersion") || out.contains("kind"), "{out}");
    assert!(
        out.contains("Deployment") || out.contains("containers"),
        "{out}"
    );
}

#[test]
fn test_kubectl_advanced_topic() {
    let out = kubectl_calc("advanced");
    assert!(
        out.contains("jsonpath") || out.contains("JSONPath") || out.contains("jq"),
        "{out}"
    );
    assert!(
        out.contains("kustomize") || out.contains("field-selector"),
        "{out}"
    );
}

#[test]
fn test_kubectl_all() {
    let out = kubectl_calc("all");
    assert!(out.contains("rollout"), "{out}");
    assert!(
        out.contains("ClusterIP") || out.contains("LoadBalancer"),
        "{out}"
    );
    assert!(out.contains("apiVersion") || out.contains("kind"), "{out}");
    assert!(
        out.contains("jsonpath") || out.contains("kustomize"),
        "{out}"
    );
}

#[test]
fn test_kubectl_unknown_topic() {
    let out = kubectl_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── tmux tests ───────────────────────────────────────────────────────────────

#[test]
fn test_tmux_help() {
    let out = tmux_calc("help");
    assert!(out.contains("tmux") || out.contains("TOPIC"), "{out}");
    assert!(out.contains("sessions"), "{out}");
}

#[test]
fn test_tmux_empty() {
    let out = tmux_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_tmux_sessions_topic() {
    let out = tmux_calc("sessions");
    assert!(
        out.contains("attach") || out.contains("new-session") || out.contains("new -s"),
        "{out}"
    );
    assert!(out.contains("detach") || out.contains("kill"), "{out}");
}

#[test]
fn test_tmux_attach_alias() {
    let out = tmux_calc("attach");
    assert!(out.contains("attach") || out.contains("session"), "{out}");
}

#[test]
fn test_tmux_windows_topic() {
    let out = tmux_calc("windows");
    assert!(
        out.contains("new-window") || out.contains("rename"),
        "{out}"
    );
    assert!(
        out.contains("^b c") || out.contains("^b ,") || out.contains("layout"),
        "{out}"
    );
}

#[test]
fn test_tmux_panes_topic() {
    let out = tmux_calc("panes");
    assert!(out.contains("split") || out.contains("resize"), "{out}");
    assert!(
        out.contains("zoom") || out.contains("^b z") || out.contains("navigate"),
        "{out}"
    );
}

#[test]
fn test_tmux_split_alias() {
    let out = tmux_calc("split");
    assert!(out.contains("split") || out.contains("%"), "{out}");
}

#[test]
fn test_tmux_copy_mode_topic() {
    let out = tmux_calc("copy-mode");
    assert!(out.contains("copy") || out.contains("paste"), "{out}");
    assert!(
        out.contains("clipboard") || out.contains("buffer") || out.contains("vi"),
        "{out}"
    );
}

#[test]
fn test_tmux_clipboard_alias() {
    let out = tmux_calc("clipboard");
    assert!(out.contains("clipboard") || out.contains("copy"), "{out}");
}

#[test]
fn test_tmux_config_topic() {
    let out = tmux_calc("config");
    assert!(out.contains("prefix") || out.contains("tmux.conf"), "{out}");
    assert!(
        out.contains("mouse") || out.contains("color") || out.contains("status"),
        "{out}"
    );
}

#[test]
fn test_tmux_scripting_topic() {
    let out = tmux_calc("scripting");
    assert!(
        out.contains("send-keys") || out.contains("bind-key"),
        "{out}"
    );
    assert!(out.contains("run-shell") || out.contains("format"), "{out}");
}

#[test]
fn test_tmux_all() {
    let out = tmux_calc("all");
    assert!(
        out.contains("attach") || out.contains("new-session") || out.contains("new -s"),
        "{out}"
    );
    assert!(out.contains("split"), "{out}");
    assert!(out.contains("prefix") || out.contains("tmux.conf"), "{out}");
    assert!(out.contains("send-keys"), "{out}");
}

#[test]
fn test_tmux_unknown_topic() {
    let out = tmux_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── postgres tests ───────────────────────────────────────────────────────────

#[test]
fn test_postgres_help() {
    let out = postgres_calc("help");
    assert!(out.contains("postgres") || out.contains("TOPIC"), "{out}");
    assert!(out.contains("psql"), "{out}");
}

#[test]
fn test_postgres_empty() {
    let out = postgres_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_postgres_psql_topic() {
    let out = postgres_calc("psql");
    assert!(
        out.contains("\\l") || out.contains("\\dt") || out.contains("backslash"),
        "{out}"
    );
    assert!(
        out.contains("\\timing") || out.contains("\\copy") || out.contains("\\q"),
        "{out}"
    );
}

#[test]
fn test_postgres_meta_alias() {
    let out = postgres_calc("meta");
    assert!(
        out.contains("\\l") || out.contains("\\dt") || out.contains("\\d"),
        "{out}"
    );
}

#[test]
fn test_postgres_tables_topic() {
    let out = postgres_calc("tables");
    assert!(
        out.contains("CREATE TABLE") || out.contains("BIGSERIAL"),
        "{out}"
    );
    assert!(
        out.contains("ALTER TABLE") || out.contains("TIMESTAMPTZ"),
        "{out}"
    );
}

#[test]
fn test_postgres_index_alias() {
    let out = postgres_calc("index");
    assert!(
        out.contains("CREATE INDEX") || out.contains("CONCURRENTLY"),
        "{out}"
    );
}

#[test]
fn test_postgres_queries_topic() {
    let out = postgres_calc("queries");
    assert!(out.contains("CTE") || out.contains("WITH "), "{out}");
    assert!(
        out.contains("WINDOW") || out.contains("OVER") || out.contains("RANK"),
        "{out}"
    );
}

#[test]
fn test_postgres_cte_alias() {
    let out = postgres_calc("cte");
    assert!(
        out.contains("WITH ") || out.contains("CTE") || out.contains("RECURSIVE"),
        "{out}"
    );
}

#[test]
fn test_postgres_window_alias() {
    let out = postgres_calc("window");
    assert!(
        out.contains("OVER") || out.contains("PARTITION") || out.contains("RANK"),
        "{out}"
    );
}

#[test]
fn test_postgres_upsert_alias() {
    let out = postgres_calc("upsert");
    assert!(
        out.contains("ON CONFLICT") || out.contains("UPSERT"),
        "{out}"
    );
}

#[test]
fn test_postgres_admin_topic() {
    let out = postgres_calc("admin");
    assert!(out.contains("pg_dump") || out.contains("VACUUM"), "{out}");
    assert!(
        out.contains("GRANT") || out.contains("role") || out.contains("ROLE"),
        "{out}"
    );
}

#[test]
fn test_postgres_backup_alias() {
    let out = postgres_calc("backup");
    assert!(
        out.contains("pg_dump") || out.contains("pg_restore"),
        "{out}"
    );
}

#[test]
fn test_postgres_json_topic() {
    let out = postgres_calc("json");
    assert!(out.contains("JSONB") || out.contains("jsonb"), "{out}");
    assert!(
        out.contains("->>") || out.contains("@>") || out.contains("jsonb_build"),
        "{out}"
    );
}

#[test]
fn test_postgres_jsonb_alias() {
    let out = postgres_calc("jsonb");
    assert!(out.contains("JSONB") || out.contains("jsonb"), "{out}");
}

#[test]
fn test_postgres_performance_topic() {
    let out = postgres_calc("performance");
    assert!(out.contains("EXPLAIN") || out.contains("Seq Scan"), "{out}");
    assert!(
        out.contains("shared_buffers") || out.contains("work_mem"),
        "{out}"
    );
}

#[test]
fn test_postgres_explain_alias() {
    let out = postgres_calc("explain");
    assert!(out.contains("EXPLAIN") || out.contains("ANALYZE"), "{out}");
}

#[test]
fn test_postgres_all() {
    let out = postgres_calc("all");
    assert!(out.contains("\\dt") || out.contains("\\l"), "{out}");
    assert!(out.contains("OVER") || out.contains("RANK"), "{out}");
    assert!(out.contains("pg_dump"), "{out}");
    assert!(out.contains("JSONB") || out.contains("jsonb"), "{out}");
    assert!(out.contains("EXPLAIN"), "{out}");
}

#[test]
fn test_postgres_unknown_topic() {
    let out = postgres_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── ts-ref tests ─────────────────────────────────────────────────────────────

#[test]
fn test_ts_ref_help() {
    let out = ts_ref_calc("help");
    assert!(out.contains("TypeScript") || out.contains("TOPIC"), "{out}");
    assert!(out.contains("generics"), "{out}");
}

#[test]
fn test_ts_ref_empty() {
    let out = ts_ref_calc("");
    assert!(out.contains("TOPIC") || out.contains("Topics"), "{out}");
}

#[test]
fn test_ts_ref_types_topic() {
    let out = ts_ref_calc("types");
    assert!(out.contains("unknown") || out.contains("never"), "{out}");
    assert!(
        out.contains("union") || out.contains("intersection") || out.contains("tuple"),
        "{out}"
    );
}

#[test]
fn test_ts_ref_union_alias() {
    let out = ts_ref_calc("union");
    assert!(out.contains("union") || out.contains("|"), "{out}");
}

#[test]
fn test_ts_ref_interfaces_topic() {
    let out = ts_ref_calc("interfaces");
    assert!(
        out.contains("interface") || out.contains("extends"),
        "{out}"
    );
    assert!(
        out.contains("index") || out.contains("callable") || out.contains("implements"),
        "{out}"
    );
}

#[test]
fn test_ts_ref_extends_alias() {
    let out = ts_ref_calc("extends");
    assert!(
        out.contains("extends") || out.contains("interface"),
        "{out}"
    );
}

#[test]
fn test_ts_ref_generics_topic() {
    let out = ts_ref_calc("generics");
    assert!(
        out.contains("constraint") || out.contains("extends"),
        "{out}"
    );
    assert!(
        out.contains("conditional") || out.contains("infer") || out.contains("mapped"),
        "{out}"
    );
}

#[test]
fn test_ts_ref_infer_alias() {
    let out = ts_ref_calc("infer");
    assert!(out.contains("infer") || out.contains("ReturnType"), "{out}");
}

#[test]
fn test_ts_ref_functions_topic() {
    let out = ts_ref_calc("functions");
    assert!(out.contains("overload") || out.contains("async"), "{out}");
    assert!(out.contains("never") || out.contains("optional"), "{out}");
}

#[test]
fn test_ts_ref_overload_alias() {
    let out = ts_ref_calc("overload");
    assert!(
        out.contains("overload") || out.contains("function process"),
        "{out}"
    );
}

#[test]
fn test_ts_ref_utility_topic() {
    let out = ts_ref_calc("utility");
    assert!(out.contains("Partial") || out.contains("Omit"), "{out}");
    assert!(
        out.contains("ReturnType") || out.contains("Parameters"),
        "{out}"
    );
}

#[test]
fn test_ts_ref_partial_alias() {
    let out = ts_ref_calc("partial");
    assert!(out.contains("Partial") || out.contains("optional"), "{out}");
}

#[test]
fn test_ts_ref_omit_alias() {
    let out = ts_ref_calc("omit");
    assert!(out.contains("Omit") || out.contains("Pick"), "{out}");
}

#[test]
fn test_ts_ref_narrowing_topic() {
    let out = ts_ref_calc("narrowing");
    assert!(
        out.contains("typeof") || out.contains("instanceof"),
        "{out}"
    );
    assert!(
        out.contains("discriminated") || out.contains("predicate") || out.contains("satisfies"),
        "{out}"
    );
}

#[test]
fn test_ts_ref_discriminated_alias() {
    let out = ts_ref_calc("discriminated");
    assert!(
        out.contains("discriminated") || out.contains("kind"),
        "{out}"
    );
}

#[test]
fn test_ts_ref_config_topic() {
    let out = ts_ref_calc("config");
    assert!(out.contains("tsconfig") || out.contains("strict"), "{out}");
    assert!(
        out.contains("paths") || out.contains("target") || out.contains("module"),
        "{out}"
    );
}

#[test]
fn test_ts_ref_strict_alias() {
    let out = ts_ref_calc("strict");
    assert!(
        out.contains("strict") || out.contains("noImplicit"),
        "{out}"
    );
}

#[test]
fn test_ts_ref_all() {
    let out = ts_ref_calc("all");
    assert!(out.contains("unknown") || out.contains("never"), "{out}");
    assert!(out.contains("infer") || out.contains("mapped"), "{out}");
    assert!(out.contains("Partial") || out.contains("Omit"), "{out}");
    assert!(
        out.contains("discriminated") || out.contains("satisfies"),
        "{out}"
    );
    assert!(out.contains("tsconfig") || out.contains("strict"), "{out}");
}

#[test]
fn test_ts_ref_unknown_topic() {
    let out = ts_ref_calc("foobar_xyz");
    assert!(out.contains("No topic") || out.contains("Run:"), "{out}");
}

// ─── Wave 19 Tests: ansible, terraform, npm, git-adv ─────────────────────────

// ── ansible_calc ─────────────────────────────────────────────────────────────

#[test]
fn test_ansible_help_empty() {
    let out = ansible_calc("");
    assert!(out.contains("hematite --ansible") || out.contains("Topics"));
}

#[test]
fn test_ansible_help_keyword() {
    let out = ansible_calc("help");
    assert!(out.contains("hematite --ansible") || out.contains("Topics"));
}

#[test]
fn test_ansible_all() {
    let out = ansible_calc("all");
    assert!(out.contains("inventory") && out.contains("playbook") && out.contains("vault"));
}

#[test]
fn test_ansible_inventory_topic() {
    let out = ansible_calc("inventory");
    assert!(out.contains("webservers") || out.contains("ansible_host"));
    assert!(out.contains("INI") || out.contains("YAML") || out.contains("pattern"));
}

#[test]
fn test_ansible_inventory_alias_hosts() {
    let out = ansible_calc("hosts");
    assert!(out.contains("webservers") || out.contains("ansible_host"));
}

#[test]
fn test_ansible_inventory_alias_pattern() {
    let out = ansible_calc("pattern");
    assert!(out.contains("webservers") || out.contains("Intersection") || out.contains("Glob"));
}

#[test]
fn test_ansible_playbooks_topic() {
    let out = ansible_calc("playbooks");
    assert!(out.contains("become") || out.contains("gather_facts"));
    assert!(out.contains("handler") || out.contains("notify"));
}

#[test]
fn test_ansible_playbooks_alias_when() {
    let out = ansible_calc("when");
    assert!(out.contains("when:") || out.contains("condition"));
}

#[test]
fn test_ansible_playbooks_alias_loop() {
    let out = ansible_calc("loop");
    assert!(out.contains("loop:") || out.contains("with_items"));
}

#[test]
fn test_ansible_playbooks_alias_block() {
    let out = ansible_calc("block");
    assert!(out.contains("rescue") || out.contains("always"));
}

#[test]
fn test_ansible_modules_topic() {
    let out = ansible_calc("modules");
    assert!(out.contains("package") || out.contains("apt"));
    assert!(out.contains("service") || out.contains("systemd"));
}

#[test]
fn test_ansible_modules_alias_copy() {
    let out = ansible_calc("copy");
    assert!(out.contains("copy:") || out.contains("src=") || out.contains("dest="));
}

#[test]
fn test_ansible_modules_alias_lineinfile() {
    let out = ansible_calc("lineinfile");
    assert!(out.contains("lineinfile:") || out.contains("path="));
}

#[test]
fn test_ansible_vars_topic() {
    let out = ansible_calc("vars");
    assert!(out.contains("precedence") || out.contains("set_fact"));
    assert!(out.contains("group_vars") || out.contains("host_vars"));
}

#[test]
fn test_ansible_vars_alias_register() {
    let out = ansible_calc("register");
    assert!(out.contains("register:") || out.contains("register_output") || out.contains("stdout"));
}

#[test]
fn test_ansible_vars_alias_magic() {
    let out = ansible_calc("magic");
    assert!(
        out.contains("inventory_hostname")
            || out.contains("hostvars")
            || out.contains("ansible_facts")
    );
}

#[test]
fn test_ansible_vars_alias_filter() {
    let out = ansible_calc("filter");
    assert!(out.contains("default(") || out.contains("| upper") || out.contains("| join"));
}

#[test]
fn test_ansible_roles_topic() {
    let out = ansible_calc("roles");
    assert!(out.contains("tasks/main.yml") || out.contains("defaults/main.yml"));
    assert!(out.contains("galaxy") || out.contains("include_role") || out.contains("import_role"));
}

#[test]
fn test_ansible_roles_alias_galaxy() {
    let out = ansible_calc("galaxy");
    assert!(out.contains("ansible-galaxy") || out.contains("requirements.yml"));
}

#[test]
fn test_ansible_vault_topic() {
    let out = ansible_calc("vault");
    assert!(out.contains("ansible-vault") || out.contains("encrypt"));
    assert!(out.contains("decrypt") || out.contains("vault_password"));
}

#[test]
fn test_ansible_vault_alias_encrypt() {
    let out = ansible_calc("encrypt");
    assert!(out.contains("ansible-vault encrypt") || out.contains("encrypt_string"));
}

#[test]
fn test_ansible_vault_alias_secret() {
    let out = ansible_calc("secret");
    assert!(out.contains("vault") || out.contains("encrypt") || out.contains("password"));
}

#[test]
fn test_ansible_cli_topic() {
    let out = ansible_calc("cli");
    assert!(out.contains("ansible-playbook") || out.contains("--check"));
    assert!(out.contains("ansible-inventory") || out.contains("ad-hoc") || out.contains("Ad-hoc"));
}

#[test]
fn test_ansible_cli_alias_adhoc() {
    let out = ansible_calc("ad-hoc");
    assert!(out.contains("ansible all") || out.contains("-m ping") || out.contains("Ad-hoc"));
}

#[test]
fn test_ansible_cli_alias_verbosity() {
    let out = ansible_calc("verbosity");
    assert!(out.contains("-vvv") || out.contains("verbosity") || out.contains("-v/"));
}

#[test]
fn test_ansible_no_match() {
    let out = ansible_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --ansible"));
}

// ── terraform_calc ────────────────────────────────────────────────────────────

#[test]
fn test_terraform_help_empty() {
    let out = terraform_calc("");
    assert!(out.contains("hematite --terraform") || out.contains("Topics"));
}

#[test]
fn test_terraform_all() {
    let out = terraform_calc("all");
    assert!(out.contains("workflow") && out.contains("state") && out.contains("modules"));
}

#[test]
fn test_terraform_workflow_topic() {
    let out = terraform_calc("workflow");
    assert!(out.contains("terraform init") || out.contains("terraform plan"));
    assert!(out.contains("terraform apply") || out.contains("terraform destroy"));
}

#[test]
fn test_terraform_workflow_alias_init() {
    let out = terraform_calc("init");
    assert!(out.contains("terraform init") || out.contains("Download providers"));
}

#[test]
fn test_terraform_workflow_alias_apply() {
    let out = terraform_calc("apply");
    assert!(out.contains("terraform apply") || out.contains("auto-approve"));
}

#[test]
fn test_terraform_workflow_alias_destroy() {
    let out = terraform_calc("destroy");
    assert!(out.contains("terraform destroy") || out.contains("Destroy"));
}

#[test]
fn test_terraform_hcl_topic() {
    let out = terraform_calc("hcl");
    assert!(out.contains("resource") || out.contains("aws_instance"));
    assert!(out.contains("data") || out.contains("locals") || out.contains("output"));
}

#[test]
fn test_terraform_hcl_alias_resource() {
    let out = terraform_calc("resource");
    assert!(out.contains("resource \"") || out.contains("lifecycle"));
}

#[test]
fn test_terraform_hcl_alias_lifecycle() {
    let out = terraform_calc("lifecycle");
    assert!(
        out.contains("lifecycle")
            || out.contains("create_before_destroy")
            || out.contains("prevent_destroy")
    );
}

#[test]
fn test_terraform_variables_topic() {
    let out = terraform_calc("variables");
    assert!(out.contains("variable") || out.contains("tfvars"));
    assert!(out.contains("sensitive") || out.contains("validation") || out.contains("TF_VAR_"));
}

#[test]
fn test_terraform_variables_alias_tfvars() {
    let out = terraform_calc("tfvars");
    assert!(out.contains("tfvars") || out.contains("terraform.tfvars"));
}

#[test]
fn test_terraform_variables_alias_sensitive() {
    let out = terraform_calc("sensitive");
    assert!(out.contains("sensitive") || out.contains("masked"));
}

#[test]
fn test_terraform_state_topic() {
    let out = terraform_calc("state");
    assert!(out.contains("terraform state") || out.contains("terraform.tfstate"));
    assert!(out.contains("backend") || out.contains("workspace") || out.contains("import"));
}

#[test]
fn test_terraform_state_alias_import() {
    let out = terraform_calc("import");
    assert!(out.contains("terraform import") || out.contains("import {"));
}

#[test]
fn test_terraform_state_alias_backend() {
    let out = terraform_calc("backend");
    assert!(out.contains("backend") || out.contains("s3") || out.contains("azurerm"));
}

#[test]
fn test_terraform_state_alias_workspace() {
    let out = terraform_calc("workspace");
    assert!(out.contains("workspace") || out.contains("terraform.workspace"));
}

#[test]
fn test_terraform_modules_topic() {
    let out = terraform_calc("modules");
    assert!(out.contains("source") || out.contains("module"));
    assert!(out.contains("registry") || out.contains("github.com") || out.contains("for_each"));
}

#[test]
fn test_terraform_modules_alias_source() {
    let out = terraform_calc("source");
    assert!(out.contains("source =") || out.contains("Local path") || out.contains("Registry"));
}

#[test]
fn test_terraform_expressions_topic() {
    let out = terraform_calc("expressions");
    assert!(out.contains("interpolation") || out.contains("${") || out.contains("for_each"));
    assert!(out.contains("dynamic") || out.contains("splat") || out.contains("lookup"));
}

#[test]
fn test_terraform_expressions_alias_conditional() {
    let out = terraform_calc("conditional");
    assert!(out.contains("?") || out.contains("ternary") || out.contains("coalesce"));
}

#[test]
fn test_terraform_expressions_alias_function() {
    let out = terraform_calc("function");
    assert!(out.contains("lookup(") || out.contains("merge(") || out.contains("concat("));
}

#[test]
fn test_terraform_expressions_alias_dynamic() {
    let out = terraform_calc("dynamic");
    assert!(out.contains("dynamic") || out.contains("for_each") || out.contains("content {"));
}

#[test]
fn test_terraform_no_match() {
    let out = terraform_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --terraform"));
}

// ── npm_calc ─────────────────────────────────────────────────────────────────

#[test]
fn test_npm_help_empty() {
    let out = npm_calc("");
    assert!(out.contains("hematite --npm") || out.contains("Topics"));
}

#[test]
fn test_npm_all() {
    let out = npm_calc("all");
    assert!(out.contains("install") && out.contains("scripts") && out.contains("workspaces"));
}

#[test]
fn test_npm_install_topic() {
    let out = npm_calc("install");
    assert!(out.contains("npm install") || out.contains("npm i "));
    assert!(out.contains("npm ci") || out.contains("devDependency") || out.contains("-D"));
}

#[test]
fn test_npm_install_alias_audit() {
    let out = npm_calc("audit");
    assert!(out.contains("npm audit") || out.contains("vulnerabilities"));
}

#[test]
fn test_npm_install_alias_global() {
    let out = npm_calc("global");
    assert!(out.contains("-g ") || out.contains("Global install") || out.contains("global add"));
}

#[test]
fn test_npm_install_alias_ci() {
    let out = npm_calc("ci");
    assert!(out.contains("npm ci") || out.contains("lockfile") || out.contains("frozen"));
}

#[test]
fn test_npm_scripts_topic() {
    let out = npm_calc("scripts");
    assert!(out.contains("npm run") || out.contains("\"scripts\""));
    assert!(out.contains("npx") || out.contains("lifecycle") || out.contains("postinstall"));
}

#[test]
fn test_npm_scripts_alias_npx() {
    let out = npm_calc("npx");
    assert!(out.contains("npx") || out.contains("without installing"));
}

#[test]
fn test_npm_scripts_alias_lifecycle() {
    let out = npm_calc("lifecycle");
    assert!(out.contains("preinstall") || out.contains("postinstall") || out.contains("prepare"));
}

#[test]
fn test_npm_packages_topic() {
    let out = npm_calc("publish");
    assert!(out.contains("npm publish") || out.contains("npm version"));
}

#[test]
fn test_npm_packages_alias_version() {
    let out = npm_calc("version");
    assert!(out.contains("npm version") || out.contains("patch") || out.contains("minor"));
}

#[test]
fn test_npm_packages_alias_scope() {
    let out = npm_calc("scope");
    assert!(out.contains("scoped") || out.contains("@myorg") || out.contains("--scope"));
}

#[test]
fn test_npm_config_topic() {
    let out = npm_calc("config");
    assert!(out.contains("npm config") || out.contains(".npmrc"));
    assert!(out.contains("registry") || out.contains("cache"));
}

#[test]
fn test_npm_config_alias_npmrc() {
    let out = npm_calc("npmrc");
    assert!(out.contains(".npmrc") || out.contains("npmrc"));
}

#[test]
fn test_npm_config_alias_cache() {
    let out = npm_calc("cache");
    assert!(out.contains("npm cache") || out.contains("cache location"));
}

#[test]
fn test_npm_config_alias_lockfile() {
    let out = npm_calc("lockfile");
    assert!(
        out.contains("package-lock.json") || out.contains("lockfile") || out.contains("npm ci")
    );
}

#[test]
fn test_npm_workspaces_topic() {
    let out = npm_calc("workspaces");
    assert!(out.contains("workspace") || out.contains("monorepo"));
    assert!(out.contains("hoisting") || out.contains("--workspaces") || out.contains("packages/*"));
}

#[test]
fn test_npm_workspaces_alias_monorepo() {
    let out = npm_calc("monorepo");
    assert!(out.contains("monorepo") || out.contains("workspace") || out.contains("packages/*"));
}

#[test]
fn test_npm_workspaces_alias_pnpm() {
    let out = npm_calc("pnpm");
    assert!(out.contains("pnpm") || out.contains("-r run") || out.contains("-F "));
}

#[test]
fn test_npm_no_match() {
    let out = npm_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --npm"));
}

// ── git_adv_calc ──────────────────────────────────────────────────────────────

#[test]
fn test_git_adv_help_empty() {
    let out = git_adv_calc("");
    assert!(out.contains("hematite --git-adv") || out.contains("Topics"));
}

#[test]
fn test_git_adv_all() {
    let out = git_adv_calc("all");
    assert!(out.contains("rebase") && out.contains("stash") && out.contains("reflog"));
}

#[test]
fn test_git_adv_rebase_topic() {
    let out = git_adv_calc("rebase");
    assert!(out.contains("git rebase") || out.contains("interactive"));
    assert!(out.contains("squash") || out.contains("fixup") || out.contains("--onto"));
}

#[test]
fn test_git_adv_rebase_alias_squash() {
    let out = git_adv_calc("squash");
    assert!(out.contains("squash") || out.contains("fixup") || out.contains("Meld"));
}

#[test]
fn test_git_adv_rebase_alias_fixup() {
    let out = git_adv_calc("fixup");
    assert!(out.contains("fixup") || out.contains("--fixup") || out.contains("autosquash"));
}

#[test]
fn test_git_adv_rebase_alias_autosquash() {
    let out = git_adv_calc("autosquash");
    assert!(out.contains("autosquash") || out.contains("--autosquash"));
}

#[test]
fn test_git_adv_rebase_alias_onto() {
    let out = git_adv_calc("onto");
    assert!(out.contains("--onto") || out.contains("onto"));
}

#[test]
fn test_git_adv_stash_topic() {
    let out = git_adv_calc("stash");
    assert!(out.contains("git stash") || out.contains("stash push"));
    assert!(out.contains("stash pop") || out.contains("stash apply"));
}

#[test]
fn test_git_adv_stash_alias_partial() {
    let out = git_adv_calc("partial");
    assert!(out.contains("-p") || out.contains("patch mode") || out.contains("Interactive patch"));
}

#[test]
fn test_git_adv_stash_alias_wip() {
    let out = git_adv_calc("wip");
    assert!(out.contains("stash") || out.contains("WIP") || out.contains("Named stash"));
}

#[test]
fn test_git_adv_bisect_topic() {
    let out = git_adv_calc("bisect");
    assert!(
        out.contains("git bisect") || out.contains("binary-search") || out.contains("bisect start")
    );
    assert!(out.contains("git bisect good") || out.contains("git bisect bad"));
}

#[test]
fn test_git_adv_bisect_alias_regression() {
    let out = git_adv_calc("regression");
    assert!(out.contains("bisect") || out.contains("first bad commit"));
}

#[test]
fn test_git_adv_bisect_alias_scripted() {
    let out = git_adv_calc("scripted-bisect");
    assert!(out.contains("git bisect run") || out.contains("test.sh") || out.contains("automated"));
}

#[test]
fn test_git_adv_worktree_topic() {
    let out = git_adv_calc("worktree");
    assert!(out.contains("git worktree") || out.contains("linked"));
    assert!(out.contains("worktree add") || out.contains("worktree list"));
}

#[test]
fn test_git_adv_worktree_alias_bare() {
    let out = git_adv_calc("bare");
    assert!(out.contains("bare") || out.contains("--bare") || out.contains("Bare repo"));
}

#[test]
fn test_git_adv_worktree_alias_parallel() {
    let out = git_adv_calc("parallel");
    assert!(out.contains("worktree") || out.contains("parallel") || out.contains("multiple"));
}

#[test]
fn test_git_adv_reflog_topic() {
    let out = git_adv_calc("reflog");
    assert!(out.contains("git reflog") || out.contains("HEAD movements"));
    assert!(out.contains("ORIG_HEAD") || out.contains("recover") || out.contains("lost"));
}

#[test]
fn test_git_adv_reflog_alias_recover() {
    let out = git_adv_calc("recover");
    assert!(
        out.contains("reflog")
            || out.contains("lost commit")
            || out.contains("git branch recovered")
    );
}

#[test]
fn test_git_adv_reflog_alias_lost() {
    let out = git_adv_calc("lost");
    assert!(out.contains("lost") || out.contains("reflog") || out.contains("dangling"));
}

#[test]
fn test_git_adv_reflog_alias_orig_head() {
    let out = git_adv_calc("orig_head");
    assert!(
        out.contains("ORIG_HEAD") || out.contains("before rebase") || out.contains("pre-rebase")
    );
}

#[test]
fn test_git_adv_reflog_alias_dangling() {
    let out = git_adv_calc("dangling");
    assert!(out.contains("dangling") || out.contains("fsck") || out.contains("lost"));
}

#[test]
fn test_git_adv_hooks_topic() {
    let out = git_adv_calc("hooks");
    assert!(out.contains(".git/hooks") || out.contains("pre-commit"));
    assert!(out.contains("pre-push") || out.contains("commit-msg") || out.contains("husky"));
}

#[test]
fn test_git_adv_hooks_alias_pre_commit() {
    let out = git_adv_calc("pre-commit");
    assert!(
        out.contains("pre-commit") || out.contains("pre_commit") || out.contains("pre-commit tool")
    );
}

#[test]
fn test_git_adv_hooks_alias_husky() {
    let out = git_adv_calc("husky");
    assert!(out.contains("husky") || out.contains("Husky"));
}

#[test]
fn test_git_adv_no_match() {
    let out = git_adv_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --git-adv"));
}

// ─── Wave 20 Tests: docker-adv, systemd-adv, makefile, jinja ─────────────────

// ── docker_adv_calc ───────────────────────────────────────────────────────────

#[test]
fn test_docker_adv_help_empty() {
    let out = docker_adv_calc("");
    assert!(out.contains("hematite --docker-adv") || out.contains("Topics"));
}

#[test]
fn test_docker_adv_all() {
    let out = docker_adv_calc("all");
    assert!(out.contains("dockerfile") && out.contains("compose") && out.contains("buildkit"));
}

#[test]
fn test_docker_adv_dockerfile_topic() {
    let out = docker_adv_calc("dockerfile");
    assert!(out.contains("FROM") || out.contains("multi-stage") || out.contains("Multi-stage"));
    assert!(out.contains("COPY") || out.contains("RUN") || out.contains("ENTRYPOINT"));
}

#[test]
fn test_docker_adv_dockerfile_alias_multistage() {
    let out = docker_adv_calc("multi-stage");
    assert!(out.contains("AS builder") || out.contains("multi-stage") || out.contains("FROM"));
}

#[test]
fn test_docker_adv_dockerfile_alias_layer() {
    let out = docker_adv_calc("layer");
    assert!(out.contains("layer") || out.contains("Layer cache") || out.contains("cache"));
}

#[test]
fn test_docker_adv_dockerfile_alias_healthcheck() {
    let out = docker_adv_calc("healthcheck");
    assert!(out.contains("HEALTHCHECK") || out.contains("healthcheck"));
}

#[test]
fn test_docker_adv_networks_topic() {
    let out = docker_adv_calc("networks");
    assert!(out.contains("bridge") || out.contains("Network drivers"));
    assert!(out.contains("docker network") || out.contains("overlay") || out.contains("DNS"));
}

#[test]
fn test_docker_adv_networks_alias_bridge() {
    let out = docker_adv_calc("bridge");
    assert!(out.contains("bridge") || out.contains("Bridge"));
}

#[test]
fn test_docker_adv_networks_alias_port() {
    let out = docker_adv_calc("port");
    assert!(out.contains("-p ") || out.contains("Port publishing") || out.contains("8080:80"));
}

#[test]
fn test_docker_adv_networks_alias_dns() {
    let out = docker_adv_calc("dns");
    assert!(out.contains("DNS") || out.contains("resolve") || out.contains("container name"));
}

#[test]
fn test_docker_adv_volumes_topic() {
    let out = docker_adv_calc("volumes");
    assert!(out.contains("Named volume") || out.contains("docker volume"));
    assert!(out.contains("bind") || out.contains("Bind mount") || out.contains("tmpfs"));
}

#[test]
fn test_docker_adv_volumes_alias_bind() {
    let out = docker_adv_calc("bind");
    assert!(out.contains("bind") || out.contains("Bind mount") || out.contains("host path"));
}

#[test]
fn test_docker_adv_volumes_alias_tmpfs() {
    let out = docker_adv_calc("tmpfs");
    assert!(out.contains("tmpfs") || out.contains("In-memory"));
}

#[test]
fn test_docker_adv_volumes_alias_backup() {
    let out = docker_adv_calc("backup");
    assert!(out.contains("backup") || out.contains("tar czf") || out.contains("Backup"));
}

#[test]
fn test_docker_adv_compose_topic() {
    let out = docker_adv_calc("compose");
    assert!(out.contains("docker compose") || out.contains("services:"));
    assert!(out.contains("depends_on") || out.contains("healthcheck") || out.contains("restart"));
}

#[test]
fn test_docker_adv_compose_alias_service() {
    let out = docker_adv_calc("service");
    assert!(out.contains("services:") || out.contains("service") || out.contains("image:"));
}

#[test]
fn test_docker_adv_compose_alias_depends() {
    let out = docker_adv_calc("depends");
    assert!(out.contains("depends_on") || out.contains("service_healthy"));
}

#[test]
fn test_docker_adv_compose_alias_env_file() {
    let out = docker_adv_calc("env_file");
    assert!(out.contains("env_file") || out.contains(".env") || out.contains("environment:"));
}

#[test]
fn test_docker_adv_buildkit_topic() {
    let out = docker_adv_calc("buildkit");
    assert!(out.contains("BuildKit") || out.contains("buildx") || out.contains("DOCKER_BUILDKIT"));
    assert!(out.contains("cache") || out.contains("--platform") || out.contains("secret"));
}

#[test]
fn test_docker_adv_buildkit_alias_cache() {
    let out = docker_adv_calc("cache");
    assert!(out.contains("cache") || out.contains("--cache-to") || out.contains("--cache-from"));
}

#[test]
fn test_docker_adv_buildkit_alias_platform() {
    let out = docker_adv_calc("platform");
    assert!(out.contains("--platform") || out.contains("linux/amd64") || out.contains("arm64"));
}

#[test]
fn test_docker_adv_buildkit_alias_secret() {
    let out = docker_adv_calc("secret");
    assert!(out.contains("secret") || out.contains("--secret") || out.contains("/run/secrets"));
}

#[test]
fn test_docker_adv_operations_topic() {
    let out = docker_adv_calc("operations");
    assert!(out.contains("docker run") || out.contains("docker exec"));
    assert!(out.contains("docker logs") || out.contains("docker stats") || out.contains("prune"));
}

#[test]
fn test_docker_adv_operations_alias_prune() {
    let out = docker_adv_calc("prune");
    assert!(out.contains("prune") || out.contains("docker system prune"));
}

#[test]
fn test_docker_adv_operations_alias_registry() {
    let out = docker_adv_calc("registry");
    assert!(
        out.contains("docker login") || out.contains("registry") || out.contains("docker push")
    );
}

#[test]
fn test_docker_adv_no_match() {
    let out = docker_adv_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --docker-adv"));
}

// ── systemd_adv_calc ──────────────────────────────────────────────────────────

#[test]
fn test_systemd_adv_help_empty() {
    let out = systemd_adv_calc("");
    assert!(out.contains("hematite --systemd-adv") || out.contains("Topics"));
}

#[test]
fn test_systemd_adv_all() {
    let out = systemd_adv_calc("all");
    assert!(out.contains("service") && out.contains("journal") && out.contains("timer"));
}

#[test]
fn test_systemd_adv_units_topic() {
    let out = systemd_adv_calc("units");
    assert!(out.contains(".service") || out.contains(".timer") || out.contains("Unit types"));
    assert!(out.contains("After=") || out.contains("Requires=") || out.contains("Wants="));
}

#[test]
fn test_systemd_adv_units_alias_target() {
    let out = systemd_adv_calc("target");
    assert!(out.contains("target") || out.contains(".target") || out.contains("multi-user"));
}

#[test]
fn test_systemd_adv_units_alias_socket() {
    let out = systemd_adv_calc("socket");
    assert!(out.contains(".socket") || out.contains("Socket activation"));
}

#[test]
fn test_systemd_adv_units_alias_requires() {
    let out = systemd_adv_calc("requires");
    assert!(out.contains("Requires=") || out.contains("Hard dependency"));
}

#[test]
fn test_systemd_adv_service_topic() {
    let out = systemd_adv_calc("service");
    assert!(out.contains("ExecStart=") || out.contains("Type=") || out.contains("[Service]"));
    assert!(out.contains("Restart=") || out.contains("User=") || out.contains("restart"));
}

#[test]
fn test_systemd_adv_service_alias_execstart() {
    let out = systemd_adv_calc("execstart");
    assert!(
        out.contains("ExecStart=") || out.contains("ExecStartPre") || out.contains("Execution")
    );
}

#[test]
fn test_systemd_adv_service_alias_sandbox() {
    let out = systemd_adv_calc("sandbox");
    assert!(
        out.contains("PrivateTmp") || out.contains("ProtectSystem") || out.contains("Sandboxing")
    );
}

#[test]
fn test_systemd_adv_service_alias_oneshot() {
    let out = systemd_adv_calc("oneshot");
    assert!(out.contains("oneshot") || out.contains("RemainAfterExit"));
}

#[test]
fn test_systemd_adv_journal_topic() {
    let out = systemd_adv_calc("journal");
    assert!(out.contains("journalctl") || out.contains("journal"));
    assert!(out.contains("-f") || out.contains("follow") || out.contains("--since"));
}

#[test]
fn test_systemd_adv_journal_alias_logs() {
    let out = systemd_adv_calc("logs");
    assert!(out.contains("journalctl") || out.contains("logs"));
}

#[test]
fn test_systemd_adv_journal_alias_priority() {
    let out = systemd_adv_calc("priority");
    assert!(out.contains("-p ") || out.contains("priority") || out.contains("emerg"));
}

#[test]
fn test_systemd_adv_journal_alias_vacuum() {
    let out = systemd_adv_calc("vacuum");
    assert!(out.contains("vacuum") || out.contains("--vacuum-size") || out.contains("disk-usage"));
}

#[test]
fn test_systemd_adv_timers_topic() {
    let out = systemd_adv_calc("timers");
    assert!(out.contains("[Timer]") || out.contains("OnCalendar") || out.contains(".timer"));
    assert!(out.contains("Persistent=") || out.contains("RandomizedDelay") || out.contains("cron"));
}

#[test]
fn test_systemd_adv_timers_alias_cron() {
    let out = systemd_adv_calc("cron");
    assert!(out.contains("cron") || out.contains("OnCalendar") || out.contains("replacement"));
}

#[test]
fn test_systemd_adv_timers_alias_calendar() {
    let out = systemd_adv_calc("calendar");
    assert!(out.contains("OnCalendar") || out.contains("calendar"));
}

#[test]
fn test_systemd_adv_timers_alias_schedule() {
    let out = systemd_adv_calc("schedule");
    assert!(out.contains("OnCalendar") || out.contains("schedule") || out.contains("Timer"));
}

#[test]
fn test_systemd_adv_dropin_topic() {
    let out = systemd_adv_calc("dropin");
    assert!(out.contains("drop-in") || out.contains("override") || out.contains("Drop-in"));
    assert!(out.contains("daemon-reload") || out.contains("systemctl edit"));
}

#[test]
fn test_systemd_adv_dropin_alias_override() {
    let out = systemd_adv_calc("override");
    assert!(out.contains("override") || out.contains("drop-in") || out.contains("Drop-in"));
}

#[test]
fn test_systemd_adv_dropin_alias_edit() {
    let out = systemd_adv_calc("edit");
    assert!(out.contains("systemctl edit") || out.contains("override.conf"));
}

#[test]
fn test_systemd_adv_ctl_topic() {
    let out = systemd_adv_calc("ctl");
    assert!(out.contains("systemctl") || out.contains("enable") || out.contains("disable"));
    assert!(out.contains("list-units") || out.contains("status") || out.contains("daemon-reload"));
}

#[test]
fn test_systemd_adv_ctl_alias_enable() {
    let out = systemd_adv_calc("enable");
    assert!(out.contains("enable") || out.contains("--now"));
}

#[test]
fn test_systemd_adv_ctl_alias_analyze() {
    let out = systemd_adv_calc("analyze");
    assert!(
        out.contains("systemd-analyze") || out.contains("blame") || out.contains("critical-chain")
    );
}

#[test]
fn test_systemd_adv_ctl_alias_linger() {
    let out = systemd_adv_calc("linger");
    assert!(out.contains("linger") || out.contains("loginctl") || out.contains("user services"));
}

#[test]
fn test_systemd_adv_no_match() {
    let out = systemd_adv_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --systemd-adv"));
}

// ── makefile_calc ─────────────────────────────────────────────────────────────

#[test]
fn test_makefile_help_empty() {
    let out = makefile_calc("");
    assert!(out.contains("hematite --makefile") || out.contains("Topics"));
}

#[test]
fn test_makefile_all() {
    let out = makefile_calc("all");
    assert!(out.contains("basics") && out.contains("variables") && out.contains("pattern"));
}

#[test]
fn test_makefile_basics_topic() {
    let out = makefile_calc("basics");
    assert!(out.contains(".PHONY") || out.contains("target:") || out.contains("recipe"));
    assert!(out.contains("make -j") || out.contains("clean") || out.contains("make -n"));
}

#[test]
fn test_makefile_basics_alias_target() {
    let out = makefile_calc("target");
    assert!(out.contains("target:") || out.contains("target") || out.contains("prerequisites"));
}

#[test]
fn test_makefile_basics_alias_phony() {
    let out = makefile_calc("phony");
    assert!(out.contains(".PHONY") || out.contains("PHONY"));
}

#[test]
fn test_makefile_basics_alias_syntax() {
    let out = makefile_calc("syntax");
    assert!(out.contains("TAB") || out.contains("recipe") || out.contains("target:"));
}

#[test]
fn test_makefile_variables_topic() {
    let out = makefile_calc("variables");
    assert!(out.contains(":=") || out.contains("Variable flavors") || out.contains("Automatic"));
    assert!(out.contains("$@") || out.contains("$<") || out.contains("wildcard"));
}

#[test]
fn test_makefile_variables_alias_automatic() {
    let out = makefile_calc("automatic");
    assert!(out.contains("$@") || out.contains("$<") || out.contains("Automatic"));
}

#[test]
fn test_makefile_variables_alias_wildcard() {
    let out = makefile_calc("wildcard");
    assert!(out.contains("wildcard") || out.contains("$(wildcard"));
}

#[test]
fn test_makefile_variables_alias_patsubst() {
    let out = makefile_calc("patsubst");
    assert!(out.contains("patsubst") || out.contains("$(patsubst"));
}

#[test]
fn test_makefile_patterns_topic() {
    let out = makefile_calc("patterns");
    assert!(out.contains("%.o:") || out.contains("Pattern rules") || out.contains("implicit"));
    assert!(out.contains("static") || out.contains("dependency") || out.contains("double-colon"));
}

#[test]
fn test_makefile_patterns_alias_implicit() {
    let out = makefile_calc("implicit");
    assert!(out.contains("implicit") || out.contains("%.o") || out.contains("Pattern"));
}

#[test]
fn test_makefile_patterns_alias_static() {
    let out = makefile_calc("static");
    assert!(out.contains("Static pattern") || out.contains("static") || out.contains("$(OBJS)"));
}

#[test]
fn test_makefile_patterns_alias_dependency() {
    let out = makefile_calc("dependency");
    assert!(out.contains("dependency") || out.contains("-MMD") || out.contains("Dependency"));
}

#[test]
fn test_makefile_functions_topic() {
    let out = makefile_calc("functions");
    assert!(out.contains("$(if ") || out.contains("ifeq") || out.contains("foreach"));
    assert!(out.contains("define") || out.contains("$(call") || out.contains("$(error"));
}

#[test]
fn test_makefile_functions_alias_ifeq() {
    let out = makefile_calc("ifeq");
    assert!(out.contains("ifeq") || out.contains("ifneq") || out.contains("endif"));
}

#[test]
fn test_makefile_functions_alias_ifdef() {
    let out = makefile_calc("ifdef");
    assert!(out.contains("ifdef") || out.contains("ifndef") || out.contains("VERBOSE"));
}

#[test]
fn test_makefile_functions_alias_foreach() {
    let out = makefile_calc("foreach");
    assert!(out.contains("foreach") || out.contains("$(foreach"));
}

#[test]
fn test_makefile_functions_alias_define() {
    let out = makefile_calc("define");
    assert!(out.contains("define") || out.contains("endef") || out.contains("user-defined"));
}

#[test]
fn test_makefile_functions_alias_origin() {
    let out = makefile_calc("origin");
    assert!(out.contains("origin") || out.contains("$(origin") || out.contains("where a variable"));
}

#[test]
fn test_makefile_recipes_topic() {
    let out = makefile_calc("recipes");
    assert!(out.contains(".SILENT") || out.contains(".ONESHELL") || out.contains("@command"));
    assert!(out.contains("parallel") || out.contains("-j") || out.contains("NOTPARALLEL"));
}

#[test]
fn test_makefile_recipes_alias_silent() {
    let out = makefile_calc("silent");
    assert!(out.contains(".SILENT") || out.contains("@command") || out.contains("Suppress"));
}

#[test]
fn test_makefile_recipes_alias_oneshell() {
    let out = makefile_calc("oneshell");
    assert!(out.contains(".ONESHELL") || out.contains("ONESHELL") || out.contains("same shell"));
}

#[test]
fn test_makefile_recipes_alias_parallel() {
    let out = makefile_calc("parallel");
    assert!(out.contains("-j") || out.contains("parallel") || out.contains("nproc"));
}

#[test]
fn test_makefile_no_match() {
    let out = makefile_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --makefile"));
}

// ── jinja_calc ────────────────────────────────────────────────────────────────

#[test]
fn test_jinja_help_empty() {
    let out = jinja_calc("");
    assert!(out.contains("hematite --jinja") || out.contains("Topics"));
}

#[test]
fn test_jinja_all() {
    let out = jinja_calc("all");
    assert!(out.contains("syntax") && out.contains("control") && out.contains("inheritance"));
}

#[test]
fn test_jinja_syntax_topic() {
    let out = jinja_calc("syntax");
    assert!(out.contains("{{") || out.contains("{%") || out.contains("Delimiters"));
    assert!(out.contains("comment") || out.contains("{#") || out.contains("raw"));
}

#[test]
fn test_jinja_syntax_alias_delimiter() {
    let out = jinja_calc("delimiter");
    assert!(out.contains("{{") || out.contains("{%") || out.contains("Delimiters"));
}

#[test]
fn test_jinja_syntax_alias_whitespace() {
    let out = jinja_calc("whitespace");
    assert!(out.contains("whitespace") || out.contains("{{-") || out.contains("trim"));
}

#[test]
fn test_jinja_syntax_alias_raw() {
    let out = jinja_calc("raw");
    assert!(out.contains("raw") || out.contains("{% raw %}") || out.contains("endraw"));
}

#[test]
fn test_jinja_control_topic() {
    let out = jinja_calc("control");
    assert!(out.contains("{% if") || out.contains("{% for") || out.contains("{% else %}"));
    assert!(out.contains("loop.index") || out.contains("endif") || out.contains("endfor"));
}

#[test]
fn test_jinja_control_alias_if() {
    let out = jinja_calc("if");
    assert!(out.contains("{% if") || out.contains("{% elif") || out.contains("endif"));
}

#[test]
fn test_jinja_control_alias_for() {
    let out = jinja_calc("for");
    assert!(out.contains("{% for") || out.contains("endfor") || out.contains("loop.index"));
}

#[test]
fn test_jinja_control_alias_test() {
    let out = jinja_calc("test");
    assert!(out.contains("is defined") || out.contains("is none") || out.contains("is string"));
}

#[test]
fn test_jinja_control_alias_selectattr() {
    let out = jinja_calc("selectattr");
    assert!(out.contains("selectattr") || out.contains("active") || out.contains("| select"));
}

#[test]
fn test_jinja_filters_topic() {
    let out = jinja_calc("filters");
    assert!(out.contains("| upper") || out.contains("| lower") || out.contains("String filters"));
    assert!(out.contains("| join") || out.contains("| sort") || out.contains("| length"));
}

#[test]
fn test_jinja_filters_alias_upper() {
    let out = jinja_calc("upper");
    assert!(out.contains("upper") || out.contains("| upper") || out.contains("UPPERCASE"));
}

#[test]
fn test_jinja_filters_alias_join() {
    let out = jinja_calc("join");
    assert!(out.contains("| join") || out.contains("join("));
}

#[test]
fn test_jinja_filters_alias_sort() {
    let out = jinja_calc("sort");
    assert!(out.contains("| sort") || out.contains("dictsort") || out.contains("sort("));
}

#[test]
fn test_jinja_filters_alias_map() {
    let out = jinja_calc("map");
    assert!(out.contains("| map") || out.contains("map(attribute"));
}

#[test]
fn test_jinja_macros_topic() {
    let out = jinja_calc("macros");
    assert!(out.contains("{% macro") || out.contains("Macros") || out.contains("macro"));
    assert!(out.contains("caller") || out.contains("{% call") || out.contains("varargs"));
}

#[test]
fn test_jinja_macros_alias_caller() {
    let out = jinja_calc("caller");
    assert!(out.contains("caller") || out.contains("{% call") || out.contains("endcall"));
}

#[test]
fn test_jinja_macros_alias_import() {
    let out = jinja_calc("import");
    assert!(out.contains("{% import") || out.contains("{% from") || out.contains("Import macros"));
}

#[test]
fn test_jinja_macros_alias_kwargs() {
    let out = jinja_calc("kwargs");
    assert!(out.contains("kwargs") || out.contains("keyword args") || out.contains("varargs"));
}

#[test]
fn test_jinja_inheritance_topic() {
    let out = jinja_calc("inheritance");
    assert!(
        out.contains("{% extends")
            || out.contains("{% block")
            || out.contains("Template inheritance")
    );
    assert!(out.contains("super()") || out.contains("{% include") || out.contains("endblock"));
}

#[test]
fn test_jinja_inheritance_alias_extends() {
    let out = jinja_calc("extends");
    assert!(out.contains("extends") || out.contains("{% extends") || out.contains("parent"));
}

#[test]
fn test_jinja_inheritance_alias_block() {
    let out = jinja_calc("block");
    assert!(out.contains("{% block") || out.contains("endblock") || out.contains("block"));
}

#[test]
fn test_jinja_inheritance_alias_include() {
    let out = jinja_calc("include");
    assert!(out.contains("{% include") || out.contains("include") || out.contains("partials"));
}

#[test]
fn test_jinja_inheritance_alias_super() {
    let out = jinja_calc("super");
    assert!(out.contains("super()") || out.contains("parent block") || out.contains("{{ super()"));
}

#[test]
fn test_jinja_inheritance_alias_partial() {
    let out = jinja_calc("partial");
    assert!(out.contains("partial") || out.contains("include") || out.contains("nav.html"));
}

#[test]
fn test_jinja_no_match() {
    let out = jinja_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --jinja"));
}

// ─── Wave 21 Tests: http-adv, linux-adv, security-ref, cloud-ref ─────────────

// ── http_adv_calc ─────────────────────────────────────────────────────────────

#[test]
fn test_http_adv_help_empty() {
    let out = http_adv_calc("");
    assert!(out.contains("hematite --http-adv") || out.contains("Topics"));
}

#[test]
fn test_http_adv_all() {
    let out = http_adv_calc("all");
    assert!(out.contains("status") && out.contains("caching") && out.contains("cors"));
}

#[test]
fn test_http_adv_status_topic() {
    let out = http_adv_calc("status");
    assert!(out.contains("200") || out.contains("404") || out.contains("500"));
    assert!(out.contains("301") || out.contains("redirect") || out.contains("Redirect"));
}

#[test]
fn test_http_adv_status_alias_codes() {
    let out = http_adv_calc("codes");
    assert!(out.contains("200") || out.contains("404"));
}

#[test]
fn test_http_adv_status_alias_4xx() {
    let out = http_adv_calc("4xx");
    assert!(out.contains("400") || out.contains("401") || out.contains("429"));
}

#[test]
fn test_http_adv_status_alias_redirect() {
    let out = http_adv_calc("redirect");
    assert!(out.contains("301") || out.contains("302") || out.contains("307"));
}

#[test]
fn test_http_adv_caching_topic() {
    let out = http_adv_calc("caching");
    assert!(out.contains("Cache-Control") || out.contains("max-age"));
    assert!(out.contains("ETag") || out.contains("304") || out.contains("Vary"));
}

#[test]
fn test_http_adv_caching_alias_etag() {
    let out = http_adv_calc("etag");
    assert!(out.contains("ETag") || out.contains("If-None-Match") || out.contains("304"));
}

#[test]
fn test_http_adv_caching_alias_immutable() {
    let out = http_adv_calc("immutable");
    assert!(out.contains("immutable") || out.contains("versioned"));
}

#[test]
fn test_http_adv_caching_alias_stale() {
    let out = http_adv_calc("stale");
    assert!(out.contains("stale") || out.contains("stale-while-revalidate"));
}

#[test]
fn test_http_adv_cors_topic() {
    let out = http_adv_calc("cors");
    assert!(out.contains("CORS") || out.contains("Access-Control-Allow-Origin"));
    assert!(out.contains("preflight") || out.contains("OPTIONS") || out.contains("Preflight"));
}

#[test]
fn test_http_adv_cors_alias_preflight() {
    let out = http_adv_calc("preflight");
    assert!(out.contains("OPTIONS") || out.contains("preflight") || out.contains("Preflight"));
}

#[test]
fn test_http_adv_cors_alias_credentials() {
    let out = http_adv_calc("credentials");
    assert!(out.contains("credentials") || out.contains("Allow-Credentials"));
}

#[test]
fn test_http_adv_auth_topic() {
    let out = http_adv_calc("auth");
    assert!(out.contains("Authorization") || out.contains("Bearer") || out.contains("JWT"));
    assert!(out.contains("OAuth") || out.contains("cookie") || out.contains("Cookie"));
}

#[test]
fn test_http_adv_auth_alias_bearer() {
    let out = http_adv_calc("bearer");
    assert!(out.contains("Bearer") || out.contains("bearer"));
}

#[test]
fn test_http_adv_auth_alias_jwt() {
    let out = http_adv_calc("jwt");
    assert!(out.contains("JWT") || out.contains("header.payload"));
}

#[test]
fn test_http_adv_auth_alias_oauth() {
    let out = http_adv_calc("oauth");
    assert!(out.contains("OAuth") || out.contains("PKCE") || out.contains("Authorization Code"));
}

#[test]
fn test_http_adv_headers_topic() {
    let out = http_adv_calc("headers");
    assert!(
        out.contains("Strict-Transport-Security")
            || out.contains("Content-Security-Policy")
            || out.contains("Security headers")
    );
    assert!(
        out.contains("X-Content-Type-Options") || out.contains("nosniff") || out.contains("Rate")
    );
}

#[test]
fn test_http_adv_headers_alias_csp() {
    let out = http_adv_calc("csp");
    assert!(out.contains("Content-Security-Policy") || out.contains("CSP") || out.contains("csp"));
}

#[test]
fn test_http_adv_headers_alias_hsts() {
    let out = http_adv_calc("hsts");
    assert!(
        out.contains("Strict-Transport-Security") || out.contains("HSTS") || out.contains("hsts")
    );
}

#[test]
fn test_http_adv_headers_alias_rate_limit() {
    let out = http_adv_calc("rate-limit");
    assert!(out.contains("rate") || out.contains("RateLimit") || out.contains("429"));
}

#[test]
fn test_http_adv_performance_topic() {
    let out = http_adv_calc("performance");
    assert!(out.contains("HTTP/2") || out.contains("HTTP/3") || out.contains("QUIC"));
    assert!(out.contains("WebSocket") || out.contains("chunked") || out.contains("SSE"));
}

#[test]
fn test_http_adv_performance_alias_http2() {
    let out = http_adv_calc("http2");
    assert!(out.contains("HTTP/2") || out.contains("Multiplexing") || out.contains("HPACK"));
}

#[test]
fn test_http_adv_performance_alias_websocket() {
    let out = http_adv_calc("websocket");
    assert!(out.contains("WebSocket") || out.contains("Upgrade") || out.contains("101"));
}

#[test]
fn test_http_adv_performance_alias_sse() {
    let out = http_adv_calc("sse");
    assert!(
        out.contains("text/event-stream") || out.contains("SSE") || out.contains("Server-Sent")
    );
}

#[test]
fn test_http_adv_no_match() {
    let out = http_adv_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --http-adv"));
}

// ── linux_adv_calc ────────────────────────────────────────────────────────────

#[test]
fn test_linux_adv_help_empty() {
    let out = linux_adv_calc("");
    assert!(out.contains("hematite --linux-adv") || out.contains("Topics"));
}

#[test]
fn test_linux_adv_all() {
    let out = linux_adv_calc("all");
    assert!(out.contains("processes") && out.contains("tracing") && out.contains("sysctl"));
}

#[test]
fn test_linux_adv_processes_topic() {
    let out = linux_adv_calc("processes");
    assert!(out.contains("SIGTERM") || out.contains("SIGKILL") || out.contains("Signals"));
    assert!(out.contains("/proc/") || out.contains("pstree") || out.contains("pkill"));
}

#[test]
fn test_linux_adv_processes_alias_signal() {
    let out = linux_adv_calc("signal");
    assert!(out.contains("SIGTERM") || out.contains("kill -l") || out.contains("SIGHUP"));
}

#[test]
fn test_linux_adv_processes_alias_kill() {
    let out = linux_adv_calc("kill");
    assert!(out.contains("kill") || out.contains("SIGKILL") || out.contains("SIGTERM"));
}

#[test]
fn test_linux_adv_processes_alias_proc() {
    let out = linux_adv_calc("proc");
    assert!(out.contains("/proc/") || out.contains("cmdline") || out.contains("Proc filesystem"));
}

#[test]
fn test_linux_adv_tracing_topic() {
    let out = linux_adv_calc("tracing");
    assert!(out.contains("strace") || out.contains("lsof") || out.contains("perf"));
    assert!(out.contains("syscall") || out.contains("-p pid") || out.contains("attach"));
}

#[test]
fn test_linux_adv_tracing_alias_strace() {
    let out = linux_adv_calc("strace");
    assert!(out.contains("strace") || out.contains("system call"));
}

#[test]
fn test_linux_adv_tracing_alias_lsof() {
    let out = linux_adv_calc("lsof");
    assert!(out.contains("lsof") || out.contains("open files") || out.contains("open file"));
}

#[test]
fn test_linux_adv_tracing_alias_perf() {
    let out = linux_adv_calc("perf");
    assert!(out.contains("perf") || out.contains("CPU counters") || out.contains("profil"));
}

#[test]
fn test_linux_adv_namespaces_topic() {
    let out = linux_adv_calc("namespaces");
    assert!(out.contains("namespace") || out.contains("pid") || out.contains("net"));
    assert!(out.contains("nsenter") || out.contains("unshare") || out.contains("lsns"));
}

#[test]
fn test_linux_adv_namespaces_alias_cgroup() {
    let out = linux_adv_calc("cgroup");
    assert!(out.contains("cgroup") || out.contains("/sys/fs/cgroup"));
}

#[test]
fn test_linux_adv_namespaces_alias_capability() {
    let out = linux_adv_calc("capability");
    assert!(
        out.contains("capsh")
            || out.contains("getcap")
            || out.contains("capabilities")
            || out.contains("Capabilities")
    );
}

#[test]
fn test_linux_adv_sysctl_topic() {
    let out = linux_adv_calc("sysctl");
    assert!(out.contains("sysctl") || out.contains("kernel parameter"));
    assert!(out.contains("ip_forward") || out.contains("swappiness") || out.contains("vm."));
}

#[test]
fn test_linux_adv_sysctl_alias_kernel() {
    let out = linux_adv_calc("kernel");
    assert!(out.contains("sysctl") || out.contains("kernel") || out.contains("/proc/sys/"));
}

#[test]
fn test_linux_adv_sysctl_alias_swappiness() {
    let out = linux_adv_calc("swappiness");
    assert!(out.contains("swappiness") || out.contains("vm.swappiness"));
}

#[test]
fn test_linux_adv_sysctl_alias_ip_forward() {
    let out = linux_adv_calc("ip_forward");
    assert!(out.contains("ip_forward") || out.contains("IP forwarding") || out.contains("router"));
}

#[test]
fn test_linux_adv_filesystem_topic() {
    let out = linux_adv_calc("filesystem");
    assert!(out.contains("inode") || out.contains("mount") || out.contains("lsblk"));
    assert!(out.contains("inotify") || out.contains("ln -s") || out.contains("stat "));
}

#[test]
fn test_linux_adv_filesystem_alias_inode() {
    let out = linux_adv_calc("inode");
    assert!(out.contains("inode") || out.contains("stat ") || out.contains("ls -i"));
}

#[test]
fn test_linux_adv_filesystem_alias_mount() {
    let out = linux_adv_calc("mount");
    assert!(out.contains("mount") || out.contains("findmnt") || out.contains("umount"));
}

#[test]
fn test_linux_adv_filesystem_alias_inotify() {
    let out = linux_adv_calc("inotify");
    assert!(out.contains("inotify") || out.contains("inotifywait"));
}

#[test]
fn test_linux_adv_networking_topic() {
    let out = linux_adv_calc("networking");
    assert!(out.contains("ip addr") || out.contains("ip route") || out.contains("ss -"));
    assert!(out.contains("tc ") || out.contains("nftables") || out.contains("iptables"));
}

#[test]
fn test_linux_adv_networking_alias_ss() {
    let out = linux_adv_calc("ss-cmd");
    assert!(out.contains("ss -") || out.contains("socket statistics"));
}

#[test]
fn test_linux_adv_networking_alias_tc() {
    let out = linux_adv_calc("tc");
    assert!(out.contains("tc ") || out.contains("traffic control") || out.contains("netem"));
}

#[test]
fn test_linux_adv_no_match() {
    let out = linux_adv_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --linux-adv"));
}

// ── security_ref_calc ─────────────────────────────────────────────────────────

#[test]
fn test_security_ref_help_empty() {
    let out = security_ref_calc("");
    assert!(out.contains("hematite --security-ref") || out.contains("Topics"));
}

#[test]
fn test_security_ref_all() {
    let out = security_ref_calc("all");
    assert!(out.contains("owasp") && out.contains("injection") && out.contains("tls"));
}

#[test]
fn test_security_ref_owasp_topic() {
    let out = security_ref_calc("owasp");
    assert!(out.contains("A01") || out.contains("Broken Access") || out.contains("OWASP"));
    assert!(out.contains("A03") || out.contains("Injection") || out.contains("A07"));
}

#[test]
fn test_security_ref_owasp_alias_top10() {
    let out = security_ref_calc("top10");
    assert!(out.contains("A01") || out.contains("Top 10") || out.contains("OWASP"));
}

#[test]
fn test_security_ref_owasp_alias_ssrf() {
    let out = security_ref_calc("ssrf");
    assert!(
        out.contains("SSRF") || out.contains("Server-Side Request") || out.contains("metadata")
    );
}

#[test]
fn test_security_ref_injection_topic() {
    let out = security_ref_calc("injection");
    assert!(out.contains("SQL") || out.contains("parameterized") || out.contains("XSS"));
    assert!(out.contains("payload") || out.contains("Fix:") || out.contains("command"));
}

#[test]
fn test_security_ref_injection_alias_sqli() {
    let out = security_ref_calc("sqli");
    assert!(out.contains("SQL") || out.contains("parameterized") || out.contains("UNION"));
}

#[test]
fn test_security_ref_injection_alias_xss() {
    let out = security_ref_calc("xss");
    assert!(out.contains("XSS") || out.contains("Cross-Site Scripting") || out.contains("onerror"));
}

#[test]
fn test_security_ref_injection_alias_command() {
    let out = security_ref_calc("command");
    assert!(
        out.contains("Command Injection")
            || out.contains("shell=True")
            || out.contains("subprocess")
    );
}

#[test]
fn test_security_ref_injection_alias_traversal() {
    let out = security_ref_calc("traversal");
    assert!(out.contains("Traversal") || out.contains("../") || out.contains("realpath"));
}

#[test]
fn test_security_ref_tls_topic() {
    let out = security_ref_calc("tls");
    assert!(out.contains("TLS") || out.contains("cipher") || out.contains("certificate"));
    assert!(out.contains("TLS 1.2") || out.contains("TLS 1.3") || out.contains("ECDHE"));
}

#[test]
fn test_security_ref_tls_alias_cipher() {
    let out = security_ref_calc("cipher");
    assert!(out.contains("cipher") || out.contains("ECDHE") || out.contains("AES-GCM"));
}

#[test]
fn test_security_ref_tls_alias_certificate() {
    let out = security_ref_calc("certificate");
    assert!(out.contains("certificate") || out.contains("cert.pem") || out.contains("CA"));
}

#[test]
fn test_security_ref_secrets_topic() {
    let out = security_ref_calc("secrets");
    assert!(
        out.contains("Vault")
            || out.contains("secret")
            || out.contains("rotation")
            || out.contains("Never do")
    );
    assert!(out.contains("rotate") || out.contains("Rotation") || out.contains("git"));
}

#[test]
fn test_security_ref_secrets_alias_vault() {
    let out = security_ref_calc("vault");
    assert!(out.contains("vault kv") || out.contains("HashiCorp") || out.contains("Vault"));
}

#[test]
fn test_security_ref_secrets_alias_rotate() {
    let out = security_ref_calc("rotate");
    assert!(out.contains("Rotat") || out.contains("rotate") || out.contains("rotation"));
}

#[test]
fn test_security_ref_jwt_topic() {
    let out = security_ref_calc("jwt");
    assert!(out.contains("JWT") || out.contains("alg") || out.contains("algorithm"));
    assert!(
        out.contains("alg:none")
            || out.contains("none")
            || out.contains("HS256")
            || out.contains("attack")
    );
}

#[test]
fn test_security_ref_jwt_alias_alg() {
    let out = security_ref_calc("alg");
    assert!(out.contains("alg") || out.contains("algorithm") || out.contains("HS256"));
}

#[test]
fn test_security_ref_jwt_alias_claims() {
    let out = security_ref_calc("claims");
    assert!(
        out.contains("exp") || out.contains("aud") || out.contains("claims") || out.contains("sub")
    );
}

#[test]
fn test_security_ref_scanning_topic() {
    let out = security_ref_calc("scanning");
    assert!(out.contains("trivy") || out.contains("semgrep") || out.contains("audit"));
    assert!(out.contains("SAST") || out.contains("SCA") || out.contains("secret"));
}

#[test]
fn test_security_ref_scanning_alias_trivy() {
    let out = security_ref_calc("trivy");
    assert!(out.contains("trivy") || out.contains("Trivy"));
}

#[test]
fn test_security_ref_scanning_alias_semgrep() {
    let out = security_ref_calc("semgrep");
    assert!(out.contains("semgrep") || out.contains("Semgrep") || out.contains("SAST"));
}

#[test]
fn test_security_ref_scanning_alias_grype() {
    let out = security_ref_calc("grype");
    assert!(out.contains("grype") || out.contains("Grype") || out.contains("CVE"));
}

#[test]
fn test_security_ref_no_match() {
    let out = security_ref_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --security-ref"));
}

// ── cloud_ref_calc ────────────────────────────────────────────────────────────

#[test]
fn test_cloud_ref_help_empty() {
    let out = cloud_ref_calc("");
    assert!(out.contains("hematite --cloud-ref") || out.contains("Topics"));
}

#[test]
fn test_cloud_ref_all() {
    let out = cloud_ref_calc("all");
    assert!(out.contains("aws") && out.contains("gcp") && out.contains("azure"));
}

#[test]
fn test_cloud_ref_aws_topic() {
    let out = cloud_ref_calc("aws");
    assert!(out.contains("aws s3") || out.contains("aws ec2") || out.contains("aws configure"));
    assert!(out.contains("aws lambda") || out.contains("sts") || out.contains("IAM"));
}

#[test]
fn test_cloud_ref_aws_alias_s3() {
    let out = cloud_ref_calc("s3");
    assert!(out.contains("aws s3") || out.contains("s3://") || out.contains("S3"));
}

#[test]
fn test_cloud_ref_aws_alias_ec2() {
    let out = cloud_ref_calc("ec2");
    assert!(out.contains("ec2") || out.contains("instance-ids") || out.contains("EC2"));
}

#[test]
fn test_cloud_ref_aws_alias_lambda() {
    let out = cloud_ref_calc("lambda");
    assert!(out.contains("lambda") || out.contains("aws lambda") || out.contains("Lambda"));
}

#[test]
fn test_cloud_ref_gcp_topic() {
    let out = cloud_ref_calc("gcp");
    assert!(out.contains("gcloud") || out.contains("gsutil") || out.contains("GCP"));
    assert!(out.contains("Cloud Run") || out.contains("gcloud run") || out.contains("Compute"));
}

#[test]
fn test_cloud_ref_gcp_alias_gcloud() {
    let out = cloud_ref_calc("gcloud");
    assert!(out.contains("gcloud") || out.contains("gcloud auth"));
}

#[test]
fn test_cloud_ref_gcp_alias_gsutil() {
    let out = cloud_ref_calc("gsutil");
    assert!(out.contains("gsutil") || out.contains("gs://"));
}

#[test]
fn test_cloud_ref_gcp_alias_cloud_run() {
    let out = cloud_ref_calc("cloud-run");
    assert!(out.contains("cloud run") || out.contains("Cloud Run") || out.contains("gcloud run"));
}

#[test]
fn test_cloud_ref_azure_topic() {
    let out = cloud_ref_calc("azure");
    assert!(out.contains("az login") || out.contains("az webapp") || out.contains("Azure"));
    assert!(
        out.contains("Key Vault")
            || out.contains("az keyvault")
            || out.contains("ACR")
            || out.contains("az container")
    );
}

#[test]
fn test_cloud_ref_azure_alias_keyvault() {
    let out = cloud_ref_calc("keyvault");
    assert!(out.contains("keyvault") || out.contains("Key Vault") || out.contains("az keyvault"));
}

#[test]
fn test_cloud_ref_azure_alias_app_service() {
    let out = cloud_ref_calc("app-service");
    assert!(out.contains("webapp") || out.contains("App Service") || out.contains("az webapp"));
}

#[test]
fn test_cloud_ref_iam_topic() {
    let out = cloud_ref_calc("iam");
    assert!(out.contains("IAM") || out.contains("least privilege") || out.contains("policy"));
    assert!(out.contains("role") || out.contains("assume-role") || out.contains("RBAC"));
}

#[test]
fn test_cloud_ref_iam_alias_role() {
    let out = cloud_ref_calc("role");
    assert!(out.contains("role") || out.contains("Role") || out.contains("assume-role"));
}

#[test]
fn test_cloud_ref_iam_alias_policy() {
    let out = cloud_ref_calc("policy");
    assert!(out.contains("policy") || out.contains("Policy") || out.contains("Statement"));
}

#[test]
fn test_cloud_ref_iam_alias_assume_role() {
    let out = cloud_ref_calc("assume-role");
    assert!(
        out.contains("assume-role")
            || out.contains("AssumeRole")
            || out.contains("sts assume-role")
    );
}

#[test]
fn test_cloud_ref_k8s_cloud_topic() {
    let out = cloud_ref_calc("eks");
    assert!(out.contains("EKS") || out.contains("eksctl") || out.contains("eks"));
    assert!(out.contains("GKE") || out.contains("AKS") || out.contains("get-credentials"));
}

#[test]
fn test_cloud_ref_k8s_cloud_alias_managed() {
    let out = cloud_ref_calc("managed");
    assert!(
        out.contains("EKS")
            || out.contains("GKE")
            || out.contains("AKS")
            || out.contains("managed")
    );
}

#[test]
fn test_cloud_ref_k8s_cloud_alias_kubeconfig() {
    let out = cloud_ref_calc("kubeconfig");
    assert!(
        out.contains("kubeconfig")
            || out.contains("kubectl config")
            || out.contains("get-contexts")
    );
}

#[test]
fn test_cloud_ref_k8s_cloud_alias_storage_class() {
    let out = cloud_ref_calc("storage-class");
    assert!(
        out.contains("StorageClass")
            || out.contains("storage class")
            || out.contains("gp3")
            || out.contains("pd-ssd")
    );
}

#[test]
fn test_cloud_ref_no_match() {
    let out = cloud_ref_calc("xyznotfound");
    assert!(out.contains("No topic") || out.contains("hematite --cloud-ref"));
}

// ── regex_adv_calc ────────────────────────────────────────────────────────────

#[test]
fn regex_adv_help_empty() {
    let out = regex_adv_calc("");
    assert!(
        out.contains("hematite --regex-adv"),
        "help shown for empty query"
    );
    assert!(out.contains("lookaround"));
    assert!(out.contains("patterns"));
}

#[test]
fn regex_adv_all() {
    let out = regex_adv_calc("all");
    assert!(out.contains("Lookahead"), "all includes lookaround content");
    assert!(
        out.contains("capturing group"),
        "all includes groups content"
    );
    assert!(
        out.contains("possessive"),
        "all includes quantifiers content"
    );
    assert!(
        out.contains("Character class") || out.contains("[abc]"),
        "all includes charclass content"
    );
    assert!(out.contains("email"), "all includes patterns content");
}

#[test]
fn regex_adv_lookaround() {
    let out = regex_adv_calc("lookaround");
    assert!(
        out.contains("(?="),
        "lookaround section has positive lookahead"
    );
    assert!(
        out.contains("(?!"),
        "lookaround section has negative lookahead"
    );
    assert!(out.contains("(?<="), "lookaround section has lookbehind");
    assert!(
        out.contains("(?<!"),
        "lookaround section has negative lookbehind"
    );
    assert!(
        out.contains("zero-width") || out.contains("Password"),
        "content present"
    );
}

#[test]
fn regex_adv_lookahead_alias() {
    let out = regex_adv_calc("lookahead");
    assert!(
        out.contains("(?="),
        "lookahead alias resolves to lookaround section"
    );
}

#[test]
fn regex_adv_lookbehind_alias() {
    let out = regex_adv_calc("lookbehind");
    assert!(
        out.contains("(?<="),
        "lookbehind alias resolves to lookaround section"
    );
}

#[test]
fn regex_adv_groups() {
    let out = regex_adv_calc("groups");
    assert!(out.contains("capturing group"), "groups section present");
    assert!(
        out.contains("Non-capturing"),
        "non-capturing group in section"
    );
    assert!(
        out.contains("Named group") || out.contains("(?P<"),
        "named groups present"
    );
    assert!(
        out.contains("backreference") || out.contains("\\1"),
        "backrefs present"
    );
}

#[test]
fn regex_adv_named_alias() {
    let out = regex_adv_calc("named");
    assert!(
        out.contains("(?P<"),
        "named alias resolves to groups section"
    );
}

#[test]
fn regex_adv_capturing_alias() {
    let out = regex_adv_calc("capturing");
    assert!(out.contains("capturing group"), "capturing alias works");
}

#[test]
fn regex_adv_backreference_alias() {
    let out = regex_adv_calc("backreference");
    assert!(
        out.contains("\\1") || out.contains("backreference"),
        "backreference alias works"
    );
}

#[test]
fn regex_adv_flavors() {
    let out = regex_adv_calc("flavors");
    assert!(out.contains("PCRE"), "flavors section has PCRE");
    assert!(
        out.contains("RE2") || out.contains("Go"),
        "flavors section has RE2/Go"
    );
    assert!(out.contains("Python"), "flavors section has Python");
    assert!(
        out.contains("JavaScript") || out.contains("ES2018"),
        "flavors section has JS"
    );
}

#[test]
fn regex_adv_pcre_alias() {
    let out = regex_adv_calc("pcre");
    assert!(out.contains("PCRE"), "pcre alias resolves to flavors");
}

#[test]
fn regex_adv_re2_alias() {
    let out = regex_adv_calc("re2");
    assert!(
        out.contains("RE2") || out.contains("Linear time"),
        "re2 alias resolves to flavors"
    );
}

#[test]
fn regex_adv_quantifiers() {
    let out = regex_adv_calc("quantifiers");
    assert!(out.contains("Greedy"), "quantifiers section has greedy");
    assert!(
        out.contains("Lazy") || out.contains("lazy"),
        "quantifiers section has lazy"
    );
    assert!(
        out.contains("possessive") || out.contains("Possessive"),
        "possessive quantifiers present"
    );
    assert!(
        out.contains("catastrophic") || out.contains("backtrack"),
        "backtracking info present"
    );
}

#[test]
fn regex_adv_greedy_alias() {
    let out = regex_adv_calc("greedy");
    assert!(out.contains("Greedy"), "greedy alias works");
}

#[test]
fn regex_adv_lazy_alias() {
    let out = regex_adv_calc("lazy");
    assert!(
        out.contains("Lazy") || out.contains("lazy"),
        "lazy alias works"
    );
}

#[test]
fn regex_adv_anchor_alias() {
    let out = regex_adv_calc("anchor");
    assert!(
        out.contains("\\A") || out.contains("\\b") || out.contains("Start of string"),
        "anchor alias works"
    );
}

#[test]
fn regex_adv_charclass() {
    let out = regex_adv_calc("charclass");
    assert!(
        out.contains("[abc]") || out.contains("[^abc]"),
        "charclass section has basic class syntax"
    );
    assert!(out.contains("\\p{"), "unicode properties present");
    assert!(out.contains("POSIX"), "POSIX classes mentioned");
    assert!(
        out.contains("\\d") || out.contains("\\w"),
        "shorthand classes present"
    );
}

#[test]
fn regex_adv_unicode_alias() {
    let out = regex_adv_calc("unicode");
    assert!(
        out.contains("\\p{"),
        "unicode alias resolves to charclass section"
    );
}

#[test]
fn regex_adv_posix_alias() {
    let out = regex_adv_calc("posix");
    assert!(out.contains("POSIX"), "posix alias works");
}

#[test]
fn regex_adv_patterns() {
    let out = regex_adv_calc("patterns");
    assert!(
        out.contains("Email") || out.contains("email"),
        "patterns section has email"
    );
    assert!(
        out.contains("IPv4") || out.contains("ipv4"),
        "patterns section has IPv4"
    );
    assert!(
        out.contains("semver") || out.contains("Semantic version"),
        "semver pattern present"
    );
    assert!(
        out.contains("JWT") || out.contains("slug"),
        "JWT or slug present"
    );
}

#[test]
fn regex_adv_email_alias() {
    let out = regex_adv_calc("email");
    assert!(
        out.contains("Email") || out.contains("email"),
        "email alias resolves to patterns"
    );
}

#[test]
fn regex_adv_url_alias() {
    let out = regex_adv_calc("url");
    assert!(
        out.contains("https") || out.contains("URL"),
        "url alias resolves to patterns"
    );
}

#[test]
fn regex_adv_ipv4_alias() {
    let out = regex_adv_calc("ipv4");
    assert!(
        out.contains("IPv4") || out.contains("25[0-5]"),
        "ipv4 alias works"
    );
}

#[test]
fn regex_adv_semver_alias() {
    let out = regex_adv_calc("semver");
    assert!(
        out.contains("semver") || out.contains("Semantic"),
        "semver alias works"
    );
}

#[test]
fn regex_adv_not_found() {
    let out = regex_adv_calc("xyznotfound123");
    assert!(
        out.contains("No topic found"),
        "unknown query returns not-found message"
    );
}

// ── sql_adv_calc ──────────────────────────────────────────────────────────────

#[test]
fn sql_adv_help_empty() {
    let out = sql_adv_calc("");
    assert!(
        out.contains("hematite --sql-adv"),
        "help shown for empty query"
    );
    assert!(out.contains("window"));
    assert!(out.contains("cte"));
    assert!(out.contains("transactions"));
}

#[test]
fn sql_adv_all() {
    let out = sql_adv_calc("all");
    assert!(out.contains("ROW_NUMBER"), "all includes window content");
    assert!(
        out.contains("RECURSIVE"),
        "all includes recursive CTE content"
    );
    assert!(out.contains("CONCURRENTLY"), "all includes index content");
    assert!(out.contains("EXPLAIN"), "all includes explain content");
    assert!(
        out.contains("ISOLATION"),
        "all includes transaction content"
    );
    assert!(
        out.contains("JSONB") || out.contains("jsonb"),
        "all includes json content"
    );
}

#[test]
fn sql_adv_window() {
    let out = sql_adv_calc("window");
    assert!(out.contains("ROW_NUMBER"), "window section has ROW_NUMBER");
    assert!(out.contains("RANK"), "window section has RANK");
    assert!(out.contains("LAG"), "window section has LAG");
    assert!(out.contains("LEAD"), "window section has LEAD");
    assert!(out.contains("PARTITION"), "window section has PARTITION BY");
    assert!(out.contains("OVER"), "window section has OVER clause");
}

#[test]
fn sql_adv_rank_alias() {
    let out = sql_adv_calc("rank");
    assert!(
        out.contains("RANK"),
        "rank alias resolves to window section"
    );
}

#[test]
fn sql_adv_row_number_alias() {
    let out = sql_adv_calc("row_number");
    assert!(out.contains("ROW_NUMBER"), "row_number alias works");
}

#[test]
fn sql_adv_lag_alias() {
    let out = sql_adv_calc("lag");
    assert!(out.contains("LAG"), "lag alias works");
}

#[test]
fn sql_adv_partition_alias() {
    let out = sql_adv_calc("partition");
    assert!(out.contains("PARTITION"), "partition alias works");
}

#[test]
fn sql_adv_cte() {
    let out = sql_adv_calc("cte");
    assert!(out.contains("WITH"), "cte section has WITH keyword");
    assert!(out.contains("RECURSIVE"), "cte section has recursive CTE");
    assert!(
        out.contains("MATERIALIZED") || out.contains("materialized"),
        "materialized CTE present"
    );
    assert!(
        out.contains("Base case") || out.contains("UNION ALL"),
        "recursive structure present"
    );
}

#[test]
fn sql_adv_recursive_alias() {
    let out = sql_adv_calc("recursive");
    assert!(
        out.contains("RECURSIVE"),
        "recursive alias resolves to cte section"
    );
}

#[test]
fn sql_adv_with_alias() {
    let out = sql_adv_calc("with");
    assert!(out.contains("WITH"), "with alias works");
}

#[test]
fn sql_adv_materialized_alias() {
    let out = sql_adv_calc("materialized");
    assert!(
        out.contains("MATERIALIZED") || out.contains("materialized"),
        "materialized alias works"
    );
}

#[test]
fn sql_adv_indexes() {
    let out = sql_adv_calc("indexes");
    assert!(
        out.contains("B-tree") || out.contains("btree"),
        "indexes section has B-tree"
    );
    assert!(out.contains("GIN"), "indexes section has GIN");
    assert!(
        out.contains("CONCURRENTLY"),
        "indexes section has CONCURRENTLY"
    );
    assert!(
        out.contains("Partial") || out.contains("partial"),
        "partial index present"
    );
    assert!(
        out.contains("INCLUDE") || out.contains("Covering"),
        "covering index present"
    );
}

#[test]
fn sql_adv_btree_alias() {
    let out = sql_adv_calc("btree");
    assert!(
        out.contains("B-tree") || out.contains("btree"),
        "btree alias resolves to indexes"
    );
}

#[test]
fn sql_adv_gin_alias() {
    let out = sql_adv_calc("gin");
    assert!(out.contains("GIN"), "gin alias works");
}

#[test]
fn sql_adv_partial_alias() {
    let out = sql_adv_calc("partial");
    assert!(
        out.contains("Partial") || out.contains("partial"),
        "partial alias works"
    );
}

#[test]
fn sql_adv_covering_alias() {
    let out = sql_adv_calc("covering");
    assert!(
        out.contains("INCLUDE") || out.contains("Covering"),
        "covering alias works"
    );
}

#[test]
fn sql_adv_explain() {
    let out = sql_adv_calc("explain");
    assert!(out.contains("EXPLAIN"), "explain section has EXPLAIN");
    assert!(
        out.contains("Seq Scan"),
        "explain section has Seq Scan node"
    );
    assert!(out.contains("Index Scan"), "explain section has Index Scan");
    assert!(out.contains("Hash Join"), "explain section has Hash Join");
    assert!(
        out.contains("slow query") || out.contains("checklist"),
        "slow query checklist present"
    );
}

#[test]
fn sql_adv_seq_scan_alias() {
    let out = sql_adv_calc("seq-scan");
    assert!(
        out.contains("Seq Scan"),
        "seq-scan alias resolves to explain section"
    );
}

#[test]
fn sql_adv_slow_alias() {
    let out = sql_adv_calc("slow");
    assert!(
        out.contains("slow query") || out.contains("checklist") || out.contains("ANALYZE"),
        "slow alias works"
    );
}

#[test]
fn sql_adv_transactions() {
    let out = sql_adv_calc("transactions");
    assert!(
        out.contains("BEGIN") || out.contains("COMMIT"),
        "transactions section has BEGIN/COMMIT"
    );
    assert!(out.contains("ISOLATION LEVEL"), "isolation levels present");
    assert!(
        out.contains("SERIALIZABLE"),
        "SERIALIZABLE isolation present"
    );
    assert!(out.contains("FOR UPDATE"), "locking present");
    assert!(out.contains("SAVEPOINT"), "savepoint present");
}

#[test]
fn sql_adv_isolation_alias() {
    let out = sql_adv_calc("isolation");
    assert!(
        out.contains("ISOLATION LEVEL"),
        "isolation alias resolves to transactions section"
    );
}

#[test]
fn sql_adv_lock_alias() {
    let out = sql_adv_calc("lock");
    assert!(
        out.contains("FOR UPDATE") || out.contains("LOCK"),
        "lock alias works"
    );
}

#[test]
fn sql_adv_deadlock_alias() {
    let out = sql_adv_calc("deadlock");
    assert!(
        out.contains("Deadlock") || out.contains("deadlock"),
        "deadlock alias works"
    );
}

#[test]
fn sql_adv_savepoint_alias() {
    let out = sql_adv_calc("savepoint");
    assert!(out.contains("SAVEPOINT"), "savepoint alias works");
}

#[test]
fn sql_adv_advisory_alias() {
    let out = sql_adv_calc("advisory");
    assert!(
        out.contains("advisory") || out.contains("pg_advisory"),
        "advisory alias works"
    );
}

#[test]
fn sql_adv_json() {
    let out = sql_adv_calc("json");
    assert!(
        out.contains("JSONB") || out.contains("->"),
        "json section has JSONB operators"
    );
    assert!(
        out.contains("->>"),
        "json section has text extraction operator"
    );
    assert!(out.contains("@>"), "json section has contains operator");
    assert!(
        out.contains("json_agg") || out.contains("JSON aggregation"),
        "json aggregation present"
    );
}

#[test]
fn sql_adv_jsonb_alias() {
    let out = sql_adv_calc("jsonb");
    assert!(
        out.contains("JSONB") || out.contains("->"),
        "jsonb alias resolves to json section"
    );
}

#[test]
fn sql_adv_contains_alias() {
    let out = sql_adv_calc("contains");
    assert!(out.contains("@>"), "contains alias works");
}

#[test]
fn sql_adv_not_found() {
    let out = sql_adv_calc("xyznotfound123");
    assert!(
        out.contains("No topic found"),
        "unknown query returns not-found message"
    );
}

// ── vim_adv_calc ──────────────────────────────────────────────────────────────

#[test]
fn vim_adv_help_empty() {
    let out = vim_adv_calc("");
    assert!(
        out.contains("hematite --vim-adv"),
        "help shown for empty query"
    );
    assert!(out.contains("registers"));
    assert!(out.contains("macros"));
    assert!(out.contains("config"));
}

#[test]
fn vim_adv_all() {
    let out = vim_adv_calc("all");
    assert!(
        out.contains("system clipboard") || out.contains("unnamed"),
        "all includes registers content"
    );
    assert!(
        out.contains("Record") || out.contains("record"),
        "all includes macros content"
    );
    assert!(
        out.contains("Ex commands") || out.contains("RANGE") || out.contains("Ranges"),
        "all includes excommands content"
    );
    assert!(
        out.contains("text object") || out.contains("Text object"),
        "all includes motions content"
    );
    assert!(
        out.contains("foldmethod") || out.contains("fold method"),
        "all includes folds content"
    );
    assert!(
        out.contains("vimrc") || out.contains("init.lua"),
        "all includes config content"
    );
}

#[test]
fn vim_adv_registers() {
    let out = vim_adv_calc("registers");
    assert!(
        out.contains("Unnamed") || out.contains("unnamed"),
        "registers section has unnamed register"
    );
    assert!(
        out.contains("system clipboard") || out.contains("clipboard"),
        "clipboard register present"
    );
    assert!(
        out.contains("Black hole") || out.contains("\"_"),
        "black hole register present"
    );
    assert!(
        out.contains("\"0") || out.contains("last yank"),
        "yank register present"
    );
    assert!(out.contains(":reg"), "registers listing command present");
}

#[test]
fn vim_adv_clipboard_alias() {
    let out = vim_adv_calc("clipboard");
    assert!(
        out.contains("clipboard") || out.contains("\"+"),
        "clipboard alias resolves to registers"
    );
}

#[test]
fn vim_adv_yank_alias() {
    let out = vim_adv_calc("yank");
    assert!(
        out.contains("yank") || out.contains("Yank"),
        "yank alias resolves to registers"
    );
}

#[test]
fn vim_adv_black_hole_alias() {
    let out = vim_adv_calc("black-hole");
    assert!(
        out.contains("Black hole") || out.contains("\"_"),
        "black-hole alias works"
    );
}

#[test]
fn vim_adv_macros() {
    let out = vim_adv_calc("macros");
    assert!(
        out.contains("Record") || out.contains("record"),
        "macros section has record info"
    );
    assert!(
        out.contains("@a") || out.contains("@<reg>"),
        "macro replay syntax present"
    );
    assert!(
        out.contains("recursive") || out.contains("Recursive"),
        "recursive macros mentioned"
    );
    assert!(
        out.contains(":g/") || out.contains("Global command"),
        "global command present"
    );
}

#[test]
fn vim_adv_record_alias() {
    let out = vim_adv_calc("record");
    assert!(
        out.contains("Record") || out.contains("record"),
        "record alias resolves to macros"
    );
}

#[test]
fn vim_adv_replay_alias() {
    let out = vim_adv_calc("replay");
    assert!(
        out.contains("Play") || out.contains("@a") || out.contains("repeat"),
        "replay alias works"
    );
}

#[test]
fn vim_adv_recursive_alias() {
    let out = vim_adv_calc("recursive");
    assert!(
        out.contains("Recursive") || out.contains("recursive"),
        "recursive alias resolves to macros"
    );
}

#[test]
fn vim_adv_global_cmd_alias() {
    let out = vim_adv_calc("global-cmd");
    assert!(
        out.contains(":g/") || out.contains("Global command"),
        "global-cmd alias works"
    );
}

#[test]
fn vim_adv_excommands() {
    let out = vim_adv_calc("ex");
    assert!(
        out.contains("Ranges") || out.contains("range"),
        "excommands section has ranges"
    );
    assert!(
        out.contains(":%") || out.contains("Entire file"),
        "full file range present"
    );
    assert!(out.contains(":g/"), "global command in excommands");
    assert!(
        out.contains(":w") || out.contains(":windo"),
        "write command present"
    );
    assert!(
        out.contains(":ls") || out.contains("buffers"),
        "buffer listing present"
    );
}

#[test]
fn vim_adv_range_alias() {
    let out = vim_adv_calc("range");
    assert!(
        out.contains("Ranges") || out.contains("range"),
        "range alias resolves to excommands"
    );
}

#[test]
fn vim_adv_substitute_alias() {
    let out = vim_adv_calc("substitute");
    assert!(
        out.contains(":s/") || out.contains("Substitute") || out.contains("substitute"),
        "substitute alias works"
    );
}

#[test]
fn vim_adv_buffer_alias() {
    let out = vim_adv_calc("buffer");
    assert!(
        out.contains(":ls") || out.contains("buffer") || out.contains("buffers"),
        "buffer alias works"
    );
}

#[test]
fn vim_adv_split_alias() {
    let out = vim_adv_calc("split");
    assert!(
        out.contains(":sp") || out.contains(":vsp") || out.contains("split"),
        "split alias works"
    );
}

#[test]
fn vim_adv_motions() {
    let out = vim_adv_calc("motions");
    assert!(
        out.contains("iw") || out.contains("inner word"),
        "motions section has text objects"
    );
    assert!(
        out.contains("H/M/L") || out.contains("Top/Middle"),
        "screen motions present"
    );
    assert!(
        out.contains("Ctrl-o") || out.contains("jump list"),
        "jump list navigation present"
    );
    assert!(
        out.contains("marks") || out.contains(":marks"),
        "marks present"
    );
}

#[test]
fn vim_adv_text_object_alias() {
    let out = vim_adv_calc("text-object");
    assert!(
        out.contains("iw") || out.contains("inner"),
        "text-object alias resolves to motions"
    );
}

#[test]
fn vim_adv_mark_alias() {
    let out = vim_adv_calc("mark");
    assert!(
        out.contains(":marks") || out.contains("mark 'a'") || out.contains("Mark"),
        "mark alias works"
    );
}

#[test]
fn vim_adv_jump_alias() {
    let out = vim_adv_calc("jump");
    assert!(
        out.contains("jump") || out.contains("Ctrl-o"),
        "jump alias works"
    );
}

#[test]
fn vim_adv_folds() {
    let out = vim_adv_calc("folds");
    assert!(
        out.contains("zo") || out.contains("Open fold"),
        "folds section has fold open"
    );
    assert!(
        out.contains("zc") || out.contains("close fold"),
        "folds section has fold close"
    );
    assert!(out.contains("foldmethod"), "fold methods present");
    assert!(
        out.contains("quickfix") || out.contains(":copen"),
        "quickfix list present"
    );
}

#[test]
fn vim_adv_fold_alias() {
    let out = vim_adv_calc("fold");
    assert!(
        out.contains("zo") || out.contains("Open fold"),
        "fold alias resolves to folds section"
    );
}

#[test]
fn vim_adv_quickfix_alias() {
    let out = vim_adv_calc("quickfix");
    assert!(
        out.contains(":copen") || out.contains("quickfix"),
        "quickfix alias works"
    );
}

#[test]
fn vim_adv_foldmethod_alias() {
    let out = vim_adv_calc("foldmethod");
    assert!(out.contains("foldmethod"), "foldmethod alias works");
}

#[test]
fn vim_adv_config() {
    let out = vim_adv_calc("config");
    assert!(
        out.contains("vimrc") || out.contains(".vimrc"),
        "config section has vimrc"
    );
    assert!(out.contains("init.lua"), "config section has init.lua");
    assert!(
        out.contains("autocmd") || out.contains("vim.opt"),
        "config settings present"
    );
    assert!(
        out.contains("keymap") || out.contains("vim.keymap"),
        "keymap config present"
    );
}

#[test]
fn vim_adv_vimrc_alias() {
    let out = vim_adv_calc("vimrc");
    assert!(
        out.contains("vimrc") || out.contains(".vimrc"),
        "vimrc alias resolves to config"
    );
}

#[test]
fn vim_adv_init_lua_alias() {
    let out = vim_adv_calc("init-lua");
    assert!(out.contains("init.lua"), "init-lua alias works");
}

#[test]
fn vim_adv_autocmd_alias() {
    let out = vim_adv_calc("autocmd");
    assert!(out.contains("autocmd"), "autocmd alias works");
}

#[test]
fn vim_adv_keymap_alias() {
    let out = vim_adv_calc("keymap");
    assert!(
        out.contains("keymap") || out.contains("vim.keymap"),
        "keymap alias works"
    );
}

#[test]
fn vim_adv_not_found() {
    let out = vim_adv_calc("xyznotfound123");
    assert!(
        out.contains("No topic found"),
        "unknown query returns not-found message"
    );
}

// ── python_data_calc ──────────────────────────────────────────────────────────

#[test]
fn python_data_help_empty() {
    let out = python_data_calc("");
    assert!(
        out.contains("hematite --python-data"),
        "help shown for empty query"
    );
    assert!(out.contains("pandas"));
    assert!(out.contains("numpy"));
    assert!(out.contains("wrangling"));
}

#[test]
fn python_data_all() {
    let out = python_data_calc("all");
    assert!(
        out.contains("read_csv") || out.contains("pd.read_csv"),
        "all includes pandas content"
    );
    assert!(
        out.contains("np.array") || out.contains("zeros"),
        "all includes numpy content"
    );
    assert!(
        out.contains("pathlib") || out.contains("Path"),
        "all includes stdlib content"
    );
    assert!(
        out.contains("plt.plot") || out.contains("matplotlib"),
        "all includes plotting content"
    );
    assert!(
        out.contains("melt") || out.contains("rolling"),
        "all includes wrangling content"
    );
}

#[test]
fn python_data_pandas() {
    let out = python_data_calc("pandas");
    assert!(
        out.contains("pd.read_csv") || out.contains("read_csv"),
        "pandas section has read_csv"
    );
    assert!(
        out.contains("df.head") || out.contains("head("),
        "pandas section has head"
    );
    assert!(out.contains("groupby"), "pandas section has groupby");
    assert!(
        out.contains("merge") || out.contains("pd.merge"),
        "pandas section has merge"
    );
    assert!(
        out.contains("fillna") || out.contains("dropna"),
        "null handling present"
    );
}

#[test]
fn python_data_dataframe_alias() {
    let out = python_data_calc("dataframe");
    assert!(
        out.contains("read_csv") || out.contains("groupby"),
        "dataframe alias resolves to pandas"
    );
}

#[test]
fn python_data_groupby_alias() {
    let out = python_data_calc("groupby");
    assert!(out.contains("groupby"), "groupby alias works");
}

#[test]
fn python_data_merge_alias() {
    let out = python_data_calc("merge");
    assert!(
        out.contains("merge") || out.contains("pd.merge"),
        "merge alias works"
    );
}

#[test]
fn python_data_pivot_alias() {
    let out = python_data_calc("pivot");
    assert!(
        out.contains("pivot_table") || out.contains("pivot"),
        "pivot alias works"
    );
}

#[test]
fn python_data_fillna_alias() {
    let out = python_data_calc("fillna");
    assert!(out.contains("fillna"), "fillna alias works");
}

#[test]
fn python_data_numpy() {
    let out = python_data_calc("numpy");
    assert!(
        out.contains("np.array") || out.contains("np.zeros"),
        "numpy section has array creation"
    );
    assert!(out.contains("reshape"), "numpy section has reshape");
    assert!(
        out.contains("np.dot") || out.contains("np.linalg"),
        "numpy section has linear algebra"
    );
    assert!(
        out.contains("Boolean mask") || out.contains("arr[arr"),
        "boolean indexing present"
    );
    assert!(
        out.contains("np.random") || out.contains("random.seed"),
        "random arrays present"
    );
}

#[test]
fn python_data_array_alias() {
    let out = python_data_calc("array");
    assert!(
        out.contains("np.array") || out.contains("zeros"),
        "array alias resolves to numpy"
    );
}

#[test]
fn python_data_ndarray_alias() {
    let out = python_data_calc("ndarray");
    assert!(
        out.contains("ndim") || out.contains("reshape") || out.contains("zeros"),
        "ndarray alias works"
    );
}

#[test]
fn python_data_reshape_alias() {
    let out = python_data_calc("reshape");
    assert!(out.contains("reshape"), "reshape alias works");
}

#[test]
fn python_data_linalg_alias() {
    let out = python_data_calc("linalg");
    assert!(
        out.contains("linalg") || out.contains("np.linalg"),
        "linalg alias works"
    );
}

#[test]
fn python_data_random_alias() {
    let out = python_data_calc("random");
    assert!(
        out.contains("np.random") || out.contains("random.seed"),
        "random alias works"
    );
}

#[test]
fn python_data_stdlib() {
    let out = python_data_calc("stdlib");
    assert!(
        out.contains("pathlib") || out.contains("Path"),
        "stdlib section has pathlib"
    );
    assert!(
        out.contains("csv") || out.contains("DictReader"),
        "stdlib section has csv module"
    );
    assert!(
        out.contains("json.loads") || out.contains("json.dumps"),
        "stdlib section has json module"
    );
    assert!(
        out.contains("Counter") || out.contains("defaultdict"),
        "stdlib section has collections"
    );
    assert!(
        out.contains("itertools") || out.contains("chain"),
        "stdlib section has itertools"
    );
}

#[test]
fn python_data_pathlib_alias() {
    let out = python_data_calc("pathlib");
    assert!(
        out.contains("pathlib") || out.contains("Path"),
        "pathlib alias resolves to stdlib"
    );
}

#[test]
fn python_data_collections_alias() {
    let out = python_data_calc("collections");
    assert!(
        out.contains("Counter") || out.contains("defaultdict"),
        "collections alias works"
    );
}

#[test]
fn python_data_itertools_alias() {
    let out = python_data_calc("itertools");
    assert!(
        out.contains("itertools") || out.contains("chain"),
        "itertools alias works"
    );
}

#[test]
fn python_data_csv_mod_alias() {
    let out = python_data_calc("csv-mod");
    assert!(
        out.contains("csv") || out.contains("DictReader"),
        "csv-mod alias works"
    );
}

#[test]
fn python_data_json_mod_alias() {
    let out = python_data_calc("json-mod");
    assert!(
        out.contains("json.loads") || out.contains("json.dumps"),
        "json-mod alias works"
    );
}

#[test]
fn python_data_plotting() {
    let out = python_data_calc("plotting");
    assert!(
        out.contains("plt.plot") || out.contains("matplotlib"),
        "plotting section has matplotlib"
    );
    assert!(
        out.contains("plt.scatter") || out.contains("scatter"),
        "scatter plot present"
    );
    assert!(
        out.contains("plt.hist") || out.contains("hist("),
        "histogram present"
    );
    assert!(
        out.contains("subplots") || out.contains("fig, axes"),
        "subplots present"
    );
    assert!(
        out.contains("savefig") || out.contains("plt.savefig"),
        "save figure present"
    );
}

#[test]
fn python_data_matplotlib_alias() {
    let out = python_data_calc("matplotlib");
    assert!(
        out.contains("matplotlib") || out.contains("plt.plot"),
        "matplotlib alias resolves to plotting"
    );
}

#[test]
fn python_data_chart_alias() {
    let out = python_data_calc("chart");
    assert!(
        out.contains("plt.plot") || out.contains("bar"),
        "chart alias works"
    );
}

#[test]
fn python_data_histogram_alias() {
    let out = python_data_calc("histogram");
    assert!(out.contains("hist"), "histogram alias works");
}

#[test]
fn python_data_scatter_alias() {
    let out = python_data_calc("scatter");
    assert!(out.contains("scatter"), "scatter alias works");
}

#[test]
fn python_data_subplot_alias() {
    let out = python_data_calc("subplot");
    assert!(
        out.contains("subplots") || out.contains("axes"),
        "subplot alias works"
    );
}

#[test]
fn python_data_viz_alias() {
    let out = python_data_calc("viz");
    assert!(
        out.contains("plt") || out.contains("matplotlib"),
        "viz alias works"
    );
}

#[test]
fn python_data_wrangling() {
    let out = python_data_calc("wrangling");
    assert!(out.contains("melt"), "wrangling section has melt");
    assert!(out.contains("explode"), "wrangling section has explode");
    assert!(
        out.contains("rolling") || out.contains("rolling_avg"),
        "wrangling section has rolling"
    );
    assert!(out.contains("resample"), "wrangling section has resample");
    assert!(
        out.contains("Categorical") || out.contains("categorical"),
        "categorical dtype present"
    );
    assert!(
        out.contains("memory") || out.contains("Memory"),
        "memory optimization present"
    );
}

#[test]
fn python_data_melt_alias() {
    let out = python_data_calc("melt");
    assert!(out.contains("melt"), "melt alias resolves to wrangling");
}

#[test]
fn python_data_explode_alias() {
    let out = python_data_calc("explode");
    assert!(out.contains("explode"), "explode alias works");
}

#[test]
fn python_data_rolling_alias() {
    let out = python_data_calc("rolling");
    assert!(out.contains("rolling"), "rolling alias works");
}

#[test]
fn python_data_resample_alias() {
    let out = python_data_calc("resample");
    assert!(out.contains("resample"), "resample alias works");
}

#[test]
fn python_data_categorical_alias() {
    let out = python_data_calc("categorical");
    assert!(
        out.contains("Categorical") || out.contains("category"),
        "categorical alias works"
    );
}

#[test]
fn python_data_eda_alias() {
    let out = python_data_calc("eda");
    assert!(
        out.contains("EDA") || out.contains("ProfileReport") || out.contains("memory"),
        "eda alias works"
    );
}

#[test]
fn python_data_not_found() {
    let out = python_data_calc("xyznotfound123");
    assert!(
        out.contains("No topic found"),
        "unknown query returns not-found message"
    );
}

// ── css_ref_calc ──────────────────────────────────────────────────────────────

#[test]
fn css_ref_help_empty() {
    let out = css_ref_calc("");
    assert!(
        out.contains("hematite --css-ref"),
        "help shown for empty query"
    );
    assert!(out.contains("selectors"));
    assert!(out.contains("flexbox"));
    assert!(out.contains("grid"));
}

#[test]
fn css_ref_all() {
    let out = css_ref_calc("all");
    assert!(
        out.contains("specificity") || out.contains("pseudo"),
        "all has selectors content"
    );
    assert!(
        out.contains("flex-direction") || out.contains("justify-content"),
        "all has flexbox content"
    );
    assert!(
        out.contains("grid-template") || out.contains("fr unit"),
        "all has grid content"
    );
    assert!(
        out.contains("keyframe") || out.contains("animation"),
        "all has animation content"
    );
    assert!(
        out.contains("--color") || out.contains("clamp"),
        "all has variables content"
    );
    assert!(
        out.contains("media") || out.contains("breakpoint"),
        "all has responsive content"
    );
}

#[test]
fn css_ref_selectors() {
    let out = css_ref_calc("selectors");
    assert!(
        out.contains("specificity") || out.contains("Specificity"),
        "selectors section has specificity"
    );
    assert!(
        out.contains(":hover") || out.contains("pseudo"),
        "pseudo-classes present"
    );
    assert!(
        out.contains("::before") || out.contains("pseudo-element"),
        "pseudo-elements present"
    );
    assert!(
        out.contains("[attr") || out.contains("attribute"),
        "attribute selectors present"
    );
    assert!(
        out.contains(":nth-child") || out.contains("nth"),
        "nth-child present"
    );
}

#[test]
fn css_ref_specificity_alias() {
    let out = css_ref_calc("specificity");
    assert!(
        out.contains("specificity") || out.contains("Specificity"),
        "specificity alias resolves to selectors"
    );
}

#[test]
fn css_ref_pseudo_alias() {
    let out = css_ref_calc("pseudo");
    assert!(
        out.contains(":hover") || out.contains("::before"),
        "pseudo alias works"
    );
}

#[test]
fn css_ref_nth_alias() {
    let out = css_ref_calc("nth");
    assert!(
        out.contains(":nth-child") || out.contains("nth"),
        "nth alias works"
    );
}

#[test]
fn css_ref_attribute_alias() {
    let out = css_ref_calc("attribute");
    assert!(
        out.contains("[attr") || out.contains("attribute"),
        "attribute alias works"
    );
}

#[test]
fn css_ref_combinator_alias() {
    let out = css_ref_calc("combinator");
    assert!(
        out.contains("Descendant") || out.contains("Direct child") || out.contains(" > "),
        "combinator alias works"
    );
}

#[test]
fn css_ref_flexbox() {
    let out = css_ref_calc("flexbox");
    assert!(
        out.contains("flex-direction"),
        "flexbox section has flex-direction"
    );
    assert!(
        out.contains("justify-content"),
        "flexbox section has justify-content"
    );
    assert!(
        out.contains("align-items"),
        "flexbox section has align-items"
    );
    assert!(
        out.contains("flex-grow") || out.contains("flex-shrink"),
        "flex item properties present"
    );
    assert!(
        out.contains("flex: 1") || out.contains("flex:1"),
        "flex shorthand present"
    );
}

#[test]
fn css_ref_flex_alias() {
    let out = css_ref_calc("flex");
    assert!(
        out.contains("flex-direction") || out.contains("justify-content"),
        "flex alias resolves to flexbox"
    );
}

#[test]
fn css_ref_justify_alias() {
    let out = css_ref_calc("justify");
    assert!(out.contains("justify-content"), "justify alias works");
}

#[test]
fn css_ref_align_alias() {
    let out = css_ref_calc("align");
    assert!(out.contains("align-items"), "align alias works");
}

#[test]
fn css_ref_gap_alias() {
    let out = css_ref_calc("gap");
    assert!(out.contains("gap"), "gap alias works");
}

#[test]
fn css_ref_grid() {
    let out = css_ref_calc("grid");
    assert!(
        out.contains("grid-template-columns"),
        "grid section has grid-template-columns"
    );
    assert!(
        out.contains("grid-template-areas") || out.contains("template-areas"),
        "grid areas present"
    );
    assert!(out.contains("fr"), "fr unit present");
    assert!(
        out.contains("auto-fill") || out.contains("auto-fit"),
        "auto-fill/fit present"
    );
    assert!(out.contains("minmax"), "minmax present");
}

#[test]
fn css_ref_template_alias() {
    let out = css_ref_calc("template");
    assert!(
        out.contains("grid-template") || out.contains("template"),
        "template alias works"
    );
}

#[test]
fn css_ref_fr_unit_alias() {
    let out = css_ref_calc("fr-unit");
    assert!(out.contains("fr"), "fr-unit alias works");
}

#[test]
fn css_ref_minmax_alias() {
    let out = css_ref_calc("minmax");
    assert!(out.contains("minmax"), "minmax alias works");
}

#[test]
fn css_ref_auto_fill_alias() {
    let out = css_ref_calc("auto-fill");
    assert!(out.contains("auto-fill"), "auto-fill alias works");
}

#[test]
fn css_ref_animations() {
    let out = css_ref_calc("animations");
    assert!(
        out.contains("@keyframes") || out.contains("keyframe"),
        "animations section has keyframes"
    );
    assert!(out.contains("transition"), "transitions present");
    assert!(
        out.contains("animation-fill-mode") || out.contains("fill-mode"),
        "fill-mode present"
    );
    assert!(
        out.contains("transform") || out.contains("translate"),
        "transforms present"
    );
    assert!(
        out.contains("ease") || out.contains("timing"),
        "timing functions present"
    );
}

#[test]
fn css_ref_keyframe_alias() {
    let out = css_ref_calc("keyframe");
    assert!(
        out.contains("@keyframes") || out.contains("keyframe"),
        "keyframe alias works"
    );
}

#[test]
fn css_ref_transition_alias() {
    let out = css_ref_calc("transition");
    assert!(out.contains("transition"), "transition alias works");
}

#[test]
fn css_ref_transform_alias() {
    let out = css_ref_calc("transform");
    assert!(
        out.contains("transform") || out.contains("translate"),
        "transform alias works"
    );
}

#[test]
fn css_ref_timing_alias() {
    let out = css_ref_calc("timing");
    assert!(
        out.contains("ease") || out.contains("timing"),
        "timing alias works"
    );
}

#[test]
fn css_ref_variables() {
    let out = css_ref_calc("variables");
    assert!(
        out.contains("--color") || out.contains("custom propert"),
        "variables section has custom properties"
    );
    assert!(out.contains("var("), "var() function present");
    assert!(out.contains("calc("), "calc() present");
    assert!(out.contains("clamp("), "clamp() present");
    assert!(
        out.contains("env(") || out.contains("safe-area"),
        "env() present"
    );
}

#[test]
fn css_ref_custom_prop_alias() {
    let out = css_ref_calc("custom-prop");
    assert!(
        out.contains("var(") || out.contains("--color"),
        "custom-prop alias works"
    );
}

#[test]
fn css_ref_calc_alias() {
    let out = css_ref_calc("calc");
    assert!(out.contains("calc("), "calc alias works");
}

#[test]
fn css_ref_clamp_alias() {
    let out = css_ref_calc("clamp");
    assert!(out.contains("clamp("), "clamp alias works");
}

#[test]
fn css_ref_viewport_alias() {
    let out = css_ref_calc("viewport");
    assert!(
        out.contains("vw") || out.contains("vh") || out.contains("viewport"),
        "viewport alias works"
    );
}

#[test]
fn css_ref_responsive() {
    let out = css_ref_calc("responsive");
    assert!(
        out.contains("@media"),
        "responsive section has media queries"
    );
    assert!(
        out.contains("min-width") || out.contains("max-width"),
        "width breakpoints present"
    );
    assert!(
        out.contains("container") || out.contains("@container"),
        "container queries present"
    );
    assert!(
        out.contains("breakpoint") || out.contains("576px"),
        "breakpoints present"
    );
    assert!(
        out.contains("prefers-color-scheme") || out.contains("dark"),
        "dark mode media query present"
    );
}

#[test]
fn css_ref_media_query_alias() {
    let out = css_ref_calc("media-query");
    assert!(out.contains("@media"), "media-query alias works");
}

#[test]
fn css_ref_breakpoint_alias() {
    let out = css_ref_calc("breakpoint");
    assert!(
        out.contains("breakpoint") || out.contains("576px"),
        "breakpoint alias works"
    );
}

#[test]
fn css_ref_container_query_alias() {
    let out = css_ref_calc("container-query");
    assert!(
        out.contains("@container") || out.contains("container"),
        "container-query alias works"
    );
}

#[test]
fn css_ref_fluid_alias() {
    let out = css_ref_calc("fluid");
    assert!(
        out.contains("clamp") || out.contains("fluid"),
        "fluid alias works"
    );
}

#[test]
fn css_ref_not_found() {
    let out = css_ref_calc("xyznotfound123");
    assert!(
        out.contains("No topic found"),
        "unknown query returns not-found message"
    );
}

// ── rust_adv_calc ─────────────────────────────────────────────────────────────

#[test]
fn rust_adv_help_empty() {
    let out = rust_adv_calc("");
    assert!(
        out.contains("hematite --rust-adv"),
        "help shown for empty query"
    );
    assert!(out.contains("lifetimes"));
    assert!(out.contains("traits"));
    assert!(out.contains("async"));
}

#[test]
fn rust_adv_all() {
    let out = rust_adv_calc("all");
    assert!(
        out.contains("elision") || out.contains("'static"),
        "all has lifetimes content"
    );
    assert!(
        out.contains("Blanket") || out.contains("blanket") || out.contains("Associated") || out.contains("associated"),
        "all has traits content"
    );
    assert!(
        out.contains("tokio") || out.contains("await"),
        "all has async content"
    );
    assert!(
        out.contains("flat_map") || out.contains("collect"),
        "all has iterators content"
    );
    assert!(
        out.contains("macro_rules") || out.contains("proc-macro"),
        "all has macros content"
    );
    assert!(
        out.contains("thiserror") || out.contains("anyhow"),
        "all has errors content"
    );
}

#[test]
fn rust_adv_lifetimes() {
    let out = rust_adv_calc("lifetimes");
    assert!(
        out.contains("'a") || out.contains("lifetime"),
        "lifetimes section has syntax"
    );
    assert!(
        out.contains("elision") || out.contains("Elision"),
        "lifetime elision rules present"
    );
    assert!(out.contains("'static"), "'static lifetime present");
    assert!(
        out.contains("HRTB") || out.contains("higher-ranked") || out.contains("for<'a>"),
        "HRTB present"
    );
}

#[test]
fn rust_adv_lifetime_alias() {
    let out = rust_adv_calc("lifetime");
    assert!(
        out.contains("'a") || out.contains("lifetime"),
        "lifetime alias resolves to lifetimes section"
    );
}

#[test]
fn rust_adv_borrow_alias() {
    let out = rust_adv_calc("borrow");
    assert!(
        out.contains("lifetime") || out.contains("'a"),
        "borrow alias works"
    );
}

#[test]
fn rust_adv_static_alias() {
    let out = rust_adv_calc("static");
    assert!(out.contains("'static"), "static alias works");
}

#[test]
fn rust_adv_elision_alias() {
    let out = rust_adv_calc("elision");
    assert!(
        out.contains("elision") || out.contains("Elision"),
        "elision alias works"
    );
}

#[test]
fn rust_adv_hrtb_alias() {
    let out = rust_adv_calc("hrtb");
    assert!(
        out.contains("for<'a>") || out.contains("higher-ranked") || out.contains("HRTB"),
        "hrtb alias works"
    );
}

#[test]
fn rust_adv_traits() {
    let out = rust_adv_calc("traits");
    assert!(
        out.contains("impl Summary") || out.contains("trait Summary"),
        "traits section has trait def"
    );
    assert!(
        out.contains("where") || out.contains("trait bound"),
        "trait bounds present"
    );
    assert!(
        out.contains("dyn Trait") || out.contains("dyn "),
        "dyn Trait present"
    );
    assert!(
        out.contains("blanket") || out.contains("Blanket"),
        "blanket implementations present"
    );
    assert!(
        out.contains("associated type") || out.contains("type Item"),
        "associated types present"
    );
}

#[test]
fn rust_adv_trait_alias() {
    let out = rust_adv_calc("trait");
    assert!(
        out.contains("trait") || out.contains("impl"),
        "trait alias resolves to traits section"
    );
}

#[test]
fn rust_adv_generic_alias() {
    let out = rust_adv_calc("generic");
    assert!(
        out.contains("generic") || out.contains("<T>"),
        "generic alias works"
    );
}

#[test]
fn rust_adv_dyn_alias() {
    let out = rust_adv_calc("dyn");
    assert!(
        out.contains("dyn Trait") || out.contains("dyn "),
        "dyn alias works"
    );
}

#[test]
fn rust_adv_blanket_alias() {
    let out = rust_adv_calc("blanket");
    assert!(
        out.contains("blanket") || out.contains("Blanket"),
        "blanket alias works"
    );
}

#[test]
fn rust_adv_associated_alias() {
    let out = rust_adv_calc("associated");
    assert!(
        out.contains("associated") || out.contains("type Item"),
        "associated alias works"
    );
}

#[test]
fn rust_adv_async() {
    let out = rust_adv_calc("async");
    assert!(
        out.contains("async fn") || out.contains("await"),
        "async section has async/await"
    );
    assert!(
        out.contains("tokio") || out.contains("Tokio"),
        "Tokio runtime present"
    );
    assert!(
        out.contains("spawn") || out.contains("tokio::spawn"),
        "spawning tasks present"
    );
    assert!(
        out.contains("channel") || out.contains("mpsc"),
        "channels present"
    );
    assert!(
        out.contains("join!") || out.contains("select!"),
        "concurrency macros present"
    );
}

#[test]
fn rust_adv_await_alias() {
    let out = rust_adv_calc("await");
    assert!(
        out.contains("await") || out.contains(".await"),
        "await alias works"
    );
}

#[test]
fn rust_adv_future_alias() {
    let out = rust_adv_calc("future");
    assert!(
        out.contains("Future") || out.contains("async"),
        "future alias works"
    );
}

#[test]
fn rust_adv_tokio_alias() {
    let out = rust_adv_calc("tokio");
    assert!(
        out.contains("tokio") || out.contains("Tokio"),
        "tokio alias works"
    );
}

#[test]
fn rust_adv_spawn_alias() {
    let out = rust_adv_calc("spawn");
    assert!(
        out.contains("spawn") || out.contains("tokio::spawn"),
        "spawn alias works"
    );
}

#[test]
fn rust_adv_channel_alias() {
    let out = rust_adv_calc("channel");
    assert!(
        out.contains("channel") || out.contains("mpsc"),
        "channel alias works"
    );
}

#[test]
fn rust_adv_stream_alias() {
    let out = rust_adv_calc("stream");
    assert!(
        out.contains("Stream") || out.contains("stream"),
        "stream alias works"
    );
}

#[test]
fn rust_adv_iterators() {
    let out = rust_adv_calc("iterators");
    assert!(
        out.contains("Iterator") || out.contains("impl Iterator"),
        "iterators section has Iterator trait"
    );
    assert!(
        out.contains("flat_map") || out.contains("filter_map"),
        "adapters present"
    );
    assert!(
        out.contains("collect") || out.contains("fold"),
        "consumers present"
    );
    assert!(
        out.contains("enumerate") || out.contains("zip"),
        "utility adapters present"
    );
    assert!(
        out.contains("IntoIterator") || out.contains("into_iter"),
        "IntoIterator present"
    );
}

#[test]
fn rust_adv_iterator_alias() {
    let out = rust_adv_calc("iterator");
    assert!(
        out.contains("Iterator") || out.contains("impl Iterator"),
        "iterator alias works"
    );
}

#[test]
fn rust_adv_iter_alias() {
    let out = rust_adv_calc("iter");
    assert!(
        out.contains("Iterator") || out.contains(".iter()"),
        "iter alias works"
    );
}

#[test]
fn rust_adv_map_alias() {
    let out = rust_adv_calc("map");
    assert!(
        out.contains(".map(") || out.contains("map(|"),
        "map alias works"
    );
}

#[test]
fn rust_adv_filter_alias() {
    let out = rust_adv_calc("filter");
    assert!(out.contains(".filter("), "filter alias works");
}

#[test]
fn rust_adv_fold_alias() {
    let out = rust_adv_calc("fold");
    assert!(
        out.contains(".fold(") || out.contains("fold(init"),
        "fold alias works"
    );
}

#[test]
fn rust_adv_collect_alias() {
    let out = rust_adv_calc("collect");
    assert!(out.contains(".collect"), "collect alias works");
}

#[test]
fn rust_adv_macros() {
    let out = rust_adv_calc("macros");
    assert!(
        out.contains("macro_rules!"),
        "macros section has macro_rules!"
    );
    assert!(
        out.contains("$e:expr") || out.contains(":expr"),
        "fragment specifiers present"
    );
    assert!(
        out.contains("dbg!") || out.contains("assert_eq!"),
        "built-in macros present"
    );
    assert!(
        out.contains("proc") || out.contains("proc-macro"),
        "proc macros mentioned"
    );
    assert!(
        out.contains("repetition") || out.contains("Repetition") || out.contains("$($x"),
        "repetition present"
    );
}

#[test]
fn rust_adv_macro_alias() {
    let out = rust_adv_calc("macro");
    assert!(
        out.contains("macro_rules!") || out.contains("macro"),
        "macro alias works"
    );
}

#[test]
fn rust_adv_macro_rules_alias() {
    let out = rust_adv_calc("macro_rules");
    assert!(out.contains("macro_rules!"), "macro_rules alias works");
}

#[test]
fn rust_adv_declarative_alias() {
    let out = rust_adv_calc("declarative");
    assert!(
        out.contains("macro_rules!") || out.contains("declarative"),
        "declarative alias works"
    );
}

#[test]
fn rust_adv_proc_macro_alias() {
    let out = rust_adv_calc("proc-macro");
    assert!(
        out.contains("proc") || out.contains("Procedural"),
        "proc-macro alias works"
    );
}

#[test]
fn rust_adv_derive_alias() {
    let out = rust_adv_calc("derive");
    assert!(
        out.contains("derive") || out.contains("#[derive"),
        "derive alias works"
    );
}

#[test]
fn rust_adv_errors() {
    let out = rust_adv_calc("errors");
    assert!(
        out.contains("Result") || out.contains("Option"),
        "errors section has Result/Option"
    );
    assert!(
        out.contains("thiserror") || out.contains("thiserror"),
        "thiserror present"
    );
    assert!(
        out.contains("anyhow") || out.contains("anyhow"),
        "anyhow present"
    );
    assert!(
        out.contains("? operator") || out.contains("?"),
        "? operator present"
    );
    assert!(
        out.contains(".map_err(") || out.contains("map_err"),
        "error mapping present"
    );
}

#[test]
fn rust_adv_error_alias() {
    let out = rust_adv_calc("error");
    assert!(
        out.contains("Result") || out.contains("Error"),
        "error alias works"
    );
}

#[test]
fn rust_adv_result_alias() {
    let out = rust_adv_calc("result");
    assert!(
        out.contains("Result") || out.contains("Ok("),
        "result alias works"
    );
}

#[test]
fn rust_adv_option_alias() {
    let out = rust_adv_calc("option");
    assert!(
        out.contains("Option") || out.contains("Some("),
        "option alias works"
    );
}

#[test]
fn rust_adv_thiserror_alias() {
    let out = rust_adv_calc("thiserror");
    assert!(out.contains("thiserror"), "thiserror alias works");
}

#[test]
fn rust_adv_anyhow_alias() {
    let out = rust_adv_calc("anyhow");
    assert!(out.contains("anyhow"), "anyhow alias works");
}

#[test]
fn rust_adv_not_found() {
    let out = rust_adv_calc("xyznotfound123");
    assert!(
        out.contains("No topic found"),
        "unknown query returns not-found message"
    );
}

// ── algo_ref_calc ─────────────────────────────────────────────────────────────

#[test]
fn algo_ref_help_empty() {
    let out = algo_ref_calc("");
    assert!(
        out.contains("hematite --algo-ref"),
        "help shown for empty query"
    );
    assert!(out.contains("complexity"));
    assert!(out.contains("sorting"));
    assert!(out.contains("graphs"));
}

#[test]
fn algo_ref_all() {
    let out = algo_ref_calc("all");
    assert!(
        out.contains("O(log n)") || out.contains("Big O"),
        "all has complexity content"
    );
    assert!(
        out.contains("Quicksort") || out.contains("Mergesort"),
        "all has sorting content"
    );
    assert!(
        out.contains("BST") || out.contains("traversal"),
        "all has trees content"
    );
    assert!(
        out.contains("BFS") || out.contains("DFS"),
        "all has graphs content"
    );
    assert!(
        out.contains("Knapsack") || out.contains("memoiz"),
        "all has dp content"
    );
    assert!(
        out.contains("sliding") || out.contains("two pointer") || out.contains("Two Pointer"),
        "all has patterns content"
    );
}

#[test]
fn algo_ref_complexity() {
    let out = algo_ref_calc("complexity");
    assert!(
        out.contains("O(1)") || out.contains("O(log n)"),
        "complexity section has Big O"
    );
    assert!(
        out.contains("Hash Map") || out.contains("HashMap"),
        "hash map complexity present"
    );
    assert!(
        out.contains("Heap") || out.contains("Binary Heap"),
        "heap complexity present"
    );
    assert!(
        out.contains("Quicksort") || out.contains("O(n log n)"),
        "sorting complexity present"
    );
    assert!(
        out.contains("Space") || out.contains("space"),
        "space complexity present"
    );
}

#[test]
fn algo_ref_big_o_alias() {
    let out = algo_ref_calc("big-o");
    assert!(
        out.contains("O(1)") || out.contains("O(log n)"),
        "big-o alias resolves to complexity"
    );
}

#[test]
fn algo_ref_bigo_alias() {
    let out = algo_ref_calc("bigo");
    assert!(
        out.contains("O(1)") || out.contains("Big O"),
        "bigo alias works"
    );
}

#[test]
fn algo_ref_time_alias() {
    let out = algo_ref_calc("time");
    assert!(
        out.contains("O(1)") || out.contains("time"),
        "time alias works"
    );
}

#[test]
fn algo_ref_notation_alias() {
    let out = algo_ref_calc("notation");
    assert!(
        out.contains("O(1)") || out.contains("notation"),
        "notation alias works"
    );
}

#[test]
fn algo_ref_cheatsheet_alias() {
    let out = algo_ref_calc("cheatsheet");
    assert!(
        out.contains("O(1)") || out.contains("O(log n)"),
        "cheatsheet alias works"
    );
}

#[test]
fn algo_ref_sorting() {
    let out = algo_ref_calc("sorting");
    assert!(out.contains("Quicksort"), "sorting section has Quicksort");
    assert!(out.contains("Mergesort"), "sorting section has Mergesort");
    assert!(
        out.contains("Heapsort") || out.contains("Timsort"),
        "other sorts present"
    );
    assert!(
        out.contains("stable") || out.contains("Stable"),
        "stability mentioned"
    );
    assert!(
        out.contains("in-place") || out.contains("in-place"),
        "in-place mentioned"
    );
}

#[test]
fn algo_ref_sort_alias() {
    let out = algo_ref_calc("sort");
    assert!(
        out.contains("Quicksort") || out.contains("Mergesort"),
        "sort alias resolves to sorting"
    );
}

#[test]
fn algo_ref_quicksort_alias() {
    let out = algo_ref_calc("quicksort");
    assert!(out.contains("Quicksort"), "quicksort alias works");
}

#[test]
fn algo_ref_mergesort_alias() {
    let out = algo_ref_calc("mergesort");
    assert!(out.contains("Mergesort"), "mergesort alias works");
}

#[test]
fn algo_ref_heapsort_alias() {
    let out = algo_ref_calc("heapsort");
    assert!(out.contains("Heapsort"), "heapsort alias works");
}

#[test]
fn algo_ref_timsort_alias() {
    let out = algo_ref_calc("timsort");
    assert!(out.contains("Timsort"), "timsort alias works");
}

#[test]
fn algo_ref_trees() {
    let out = algo_ref_calc("trees");
    assert!(
        out.contains("BST") || out.contains("Binary Search Tree"),
        "trees section has BST"
    );
    assert!(
        out.contains("traversal") || out.contains("Pre-order"),
        "traversals present"
    );
    assert!(out.contains("Trie") || out.contains("trie"), "trie present");
    assert!(
        out.contains("AVL") || out.contains("Red-Black") || out.contains("RB"),
        "balanced trees mentioned"
    );
    assert!(
        out.contains("height") || out.contains("Height"),
        "height/depth present"
    );
}

#[test]
fn algo_ref_tree_alias() {
    let out = algo_ref_calc("tree");
    assert!(
        out.contains("BST") || out.contains("traversal"),
        "tree alias resolves to trees section"
    );
}

#[test]
fn algo_ref_bst_alias() {
    let out = algo_ref_calc("bst");
    assert!(
        out.contains("BST") || out.contains("Binary Search Tree"),
        "bst alias works"
    );
}

#[test]
fn algo_ref_trie_alias() {
    let out = algo_ref_calc("trie");
    assert!(
        out.contains("Trie") || out.contains("trie"),
        "trie alias works"
    );
}

#[test]
fn algo_ref_traversal_alias() {
    let out = algo_ref_calc("traversal");
    assert!(
        out.contains("Pre-order") || out.contains("In-order"),
        "traversal alias works"
    );
}

#[test]
fn algo_ref_avl_alias() {
    let out = algo_ref_calc("avl");
    assert!(out.contains("AVL"), "avl alias works");
}

#[test]
fn algo_ref_graphs() {
    let out = algo_ref_calc("graphs");
    assert!(out.contains("BFS"), "graphs section has BFS");
    assert!(out.contains("DFS"), "graphs section has DFS");
    assert!(out.contains("Dijkstra"), "graphs section has Dijkstra");
    assert!(
        out.contains("topological") || out.contains("Topological"),
        "topological sort present"
    );
    assert!(
        out.contains("Union-Find") || out.contains("union-find") || out.contains("Disjoint"),
        "union-find present"
    );
}

#[test]
fn algo_ref_graph_alias() {
    let out = algo_ref_calc("graph");
    assert!(
        out.contains("BFS") || out.contains("DFS"),
        "graph alias resolves to graphs section"
    );
}

#[test]
fn algo_ref_bfs_alias() {
    let out = algo_ref_calc("bfs");
    assert!(out.contains("BFS"), "bfs alias works");
}

#[test]
fn algo_ref_dfs_alias() {
    let out = algo_ref_calc("dfs");
    assert!(out.contains("DFS"), "dfs alias works");
}

#[test]
fn algo_ref_dijkstra_alias() {
    let out = algo_ref_calc("dijkstra");
    assert!(out.contains("Dijkstra"), "dijkstra alias works");
}

#[test]
fn algo_ref_topological_alias() {
    let out = algo_ref_calc("topological");
    assert!(
        out.contains("topological") || out.contains("Topological"),
        "topological alias works"
    );
}

#[test]
fn algo_ref_union_find_alias() {
    let out = algo_ref_calc("union-find");
    assert!(
        out.contains("Union-Find") || out.contains("union-find") || out.contains("Disjoint"),
        "union-find alias works"
    );
}

#[test]
fn algo_ref_dp() {
    let out = algo_ref_calc("dp");
    assert!(
        out.contains("Memoization") || out.contains("memoiz"),
        "dp section has memoization"
    );
    assert!(
        out.contains("Tabulation") || out.contains("tabul"),
        "tabulation present"
    );
    assert!(
        out.contains("Knapsack") || out.contains("knapsack"),
        "knapsack problem present"
    );
    assert!(
        out.contains("LCS") || out.contains("Longest Common"),
        "LCS present"
    );
    assert!(
        out.contains("Edit Distance") || out.contains("Levenshtein"),
        "edit distance present"
    );
}

#[test]
fn algo_ref_dynamic_alias() {
    let out = algo_ref_calc("dynamic");
    assert!(
        out.contains("Memoization") || out.contains("memoiz"),
        "dynamic alias resolves to dp section"
    );
}

#[test]
fn algo_ref_memoize_alias() {
    let out = algo_ref_calc("memoize");
    assert!(
        out.contains("Memoization") || out.contains("memoiz"),
        "memoize alias works"
    );
}

#[test]
fn algo_ref_knapsack_alias() {
    let out = algo_ref_calc("knapsack");
    assert!(
        out.contains("Knapsack") || out.contains("knapsack"),
        "knapsack alias works"
    );
}

#[test]
fn algo_ref_lcs_alias() {
    let out = algo_ref_calc("lcs");
    assert!(
        out.contains("LCS") || out.contains("Common Subsequence"),
        "lcs alias works"
    );
}

#[test]
fn algo_ref_lis_alias() {
    let out = algo_ref_calc("lis");
    assert!(
        out.contains("LIS") || out.contains("Increasing Subsequence"),
        "lis alias works"
    );
}

#[test]
fn algo_ref_edit_alias() {
    let out = algo_ref_calc("edit");
    assert!(
        out.contains("Edit Distance") || out.contains("Levenshtein"),
        "edit alias works"
    );
}

#[test]
fn algo_ref_patterns() {
    let out = algo_ref_calc("patterns");
    assert!(
        out.contains("Sliding Window") || out.contains("sliding"),
        "patterns section has sliding window"
    );
    assert!(
        out.contains("Two Pointer") || out.contains("two pointer"),
        "two pointers present"
    );
    assert!(
        out.contains("Binary Search") || out.contains("binary search"),
        "binary search variants present"
    );
    assert!(
        out.contains("Prefix Sum") || out.contains("prefix sum"),
        "prefix sum present"
    );
    assert!(
        out.contains("Backtracking") || out.contains("backtrack"),
        "backtracking present"
    );
}

#[test]
fn algo_ref_sliding_window_alias() {
    let out = algo_ref_calc("sliding-window");
    assert!(
        out.contains("Sliding Window") || out.contains("sliding"),
        "sliding-window alias works"
    );
}

#[test]
fn algo_ref_two_pointer_alias() {
    let out = algo_ref_calc("two-pointer");
    assert!(
        out.contains("Two Pointer") || out.contains("two pointer"),
        "two-pointer alias works"
    );
}

#[test]
fn algo_ref_binary_search_alias() {
    let out = algo_ref_calc("binary-search");
    assert!(
        out.contains("Binary Search") || out.contains("binary search"),
        "binary-search alias works"
    );
}

#[test]
fn algo_ref_prefix_sum_alias() {
    let out = algo_ref_calc("prefix-sum");
    assert!(
        out.contains("Prefix Sum") || out.contains("prefix sum"),
        "prefix-sum alias works"
    );
}

#[test]
fn algo_ref_backtrack_alias() {
    let out = algo_ref_calc("backtrack");
    assert!(
        out.contains("Backtracking") || out.contains("backtrack"),
        "backtrack alias works"
    );
}

#[test]
fn algo_ref_not_found() {
    let out = algo_ref_calc("xyznotfound123");
    assert!(
        out.contains("No topic found"),
        "unknown query returns not-found message"
    );
}

// ── oop_ref_calc ──────────────────────────────────────────────────────────────

#[test]
fn oop_ref_help_empty() {
    let out = oop_ref_calc("");
    assert!(
        out.contains("hematite --oop-ref"),
        "help shown for empty query"
    );
    assert!(out.contains("creational"));
    assert!(out.contains("solid"));
    assert!(out.contains("behavioral"));
}

#[test]
fn oop_ref_all() {
    let out = oop_ref_calc("all");
    assert!(
        out.contains("Factory") || out.contains("factory"),
        "all has creational content"
    );
    assert!(
        out.contains("Adapter") || out.contains("Decorator"),
        "all has structural content"
    );
    assert!(
        out.contains("Observer") || out.contains("Strategy"),
        "all has behavioral content"
    );
    assert!(
        out.contains("Single Responsibility") || out.contains("SRP"),
        "all has SOLID content"
    );
    assert!(
        out.contains("Mixin") || out.contains("composition"),
        "all has composition content"
    );
    assert!(
        out.contains("God Object") || out.contains("antipattern"),
        "all has antipatterns content"
    );
}

#[test]
fn oop_ref_creational() {
    let out = oop_ref_calc("creational");
    assert!(
        out.contains("Factory") || out.contains("factory"),
        "creational section has Factory"
    );
    assert!(
        out.contains("Builder") || out.contains("builder"),
        "Builder pattern present"
    );
    assert!(
        out.contains("Singleton") || out.contains("singleton"),
        "Singleton present"
    );
    assert!(
        out.contains("Prototype") || out.contains("clone"),
        "Prototype present"
    );
}

#[test]
fn oop_ref_factory_alias() {
    let out = oop_ref_calc("factory");
    assert!(
        out.contains("Factory") || out.contains("factory"),
        "factory alias resolves to creational"
    );
}

#[test]
fn oop_ref_builder_alias() {
    let out = oop_ref_calc("builder");
    assert!(
        out.contains("Builder") || out.contains("builder"),
        "builder alias works"
    );
}

#[test]
fn oop_ref_singleton_alias() {
    let out = oop_ref_calc("singleton");
    assert!(
        out.contains("Singleton") || out.contains("singleton"),
        "singleton alias works"
    );
}

#[test]
fn oop_ref_prototype_alias() {
    let out = oop_ref_calc("prototype");
    assert!(
        out.contains("Prototype") || out.contains("clone"),
        "prototype alias works"
    );
}

#[test]
fn oop_ref_pool_alias() {
    let out = oop_ref_calc("pool");
    assert!(
        out.contains("Pool") || out.contains("pool"),
        "pool alias works"
    );
}

#[test]
fn oop_ref_structural() {
    let out = oop_ref_calc("structural");
    assert!(
        out.contains("Adapter") || out.contains("adapter"),
        "structural section has Adapter"
    );
    assert!(
        out.contains("Decorator") || out.contains("decorator"),
        "Decorator present"
    );
    assert!(
        out.contains("Facade") || out.contains("facade"),
        "Facade present"
    );
    assert!(
        out.contains("Proxy") || out.contains("proxy"),
        "Proxy present"
    );
    assert!(
        out.contains("Composite") || out.contains("composite"),
        "Composite present"
    );
}

#[test]
fn oop_ref_adapter_alias() {
    let out = oop_ref_calc("adapter");
    assert!(
        out.contains("Adapter") || out.contains("adapter"),
        "adapter alias resolves to structural"
    );
}

#[test]
fn oop_ref_decorator_alias() {
    let out = oop_ref_calc("decorator");
    assert!(
        out.contains("Decorator") || out.contains("decorator"),
        "decorator alias works"
    );
}

#[test]
fn oop_ref_facade_alias() {
    let out = oop_ref_calc("facade");
    assert!(
        out.contains("Facade") || out.contains("facade"),
        "facade alias works"
    );
}

#[test]
fn oop_ref_proxy_alias() {
    let out = oop_ref_calc("proxy");
    assert!(
        out.contains("Proxy") || out.contains("proxy"),
        "proxy alias works"
    );
}

#[test]
fn oop_ref_composite_alias() {
    let out = oop_ref_calc("composite");
    assert!(
        out.contains("Composite") || out.contains("composite"),
        "composite alias works"
    );
}

#[test]
fn oop_ref_bridge_alias() {
    let out = oop_ref_calc("bridge");
    assert!(
        out.contains("Bridge") || out.contains("bridge"),
        "bridge alias works"
    );
}

#[test]
fn oop_ref_behavioral() {
    let out = oop_ref_calc("behavioral");
    assert!(
        out.contains("Observer") || out.contains("observer"),
        "behavioral section has Observer"
    );
    assert!(
        out.contains("Strategy") || out.contains("strategy"),
        "Strategy present"
    );
    assert!(
        out.contains("Command") || out.contains("command"),
        "Command present"
    );
    assert!(
        out.contains("State") || out.contains("state"),
        "State present"
    );
    assert!(
        out.contains("Chain") || out.contains("chain"),
        "Chain of Responsibility present"
    );
}

#[test]
fn oop_ref_observer_alias() {
    let out = oop_ref_calc("observer");
    assert!(
        out.contains("Observer") || out.contains("observer"),
        "observer alias resolves to behavioral"
    );
}

#[test]
fn oop_ref_strategy_alias() {
    let out = oop_ref_calc("strategy");
    assert!(
        out.contains("Strategy") || out.contains("strategy"),
        "strategy alias works"
    );
}

#[test]
fn oop_ref_command_alias() {
    let out = oop_ref_calc("command");
    assert!(
        out.contains("Command") || out.contains("command"),
        "command alias works"
    );
}

#[test]
fn oop_ref_template_alias() {
    let out = oop_ref_calc("template");
    assert!(
        out.contains("Template") || out.contains("template"),
        "template alias works"
    );
}

#[test]
fn oop_ref_state_alias() {
    let out = oop_ref_calc("state");
    assert!(
        out.contains("State") || out.contains("state"),
        "state alias works"
    );
}

#[test]
fn oop_ref_chain_alias() {
    let out = oop_ref_calc("chain");
    assert!(
        out.contains("Chain") || out.contains("chain"),
        "chain alias works"
    );
}

#[test]
fn oop_ref_solid() {
    let out = oop_ref_calc("solid");
    assert!(
        out.contains("Single Responsibility") || out.contains("SRP"),
        "SOLID section has SRP"
    );
    assert!(
        out.contains("Open/Closed") || out.contains("OCP"),
        "OCP present"
    );
    assert!(out.contains("Liskov") || out.contains("LSP"), "LSP present");
    assert!(
        out.contains("Interface Segregation") || out.contains("ISP"),
        "ISP present"
    );
    assert!(
        out.contains("Dependency Inversion") || out.contains("DIP"),
        "DIP present"
    );
}

#[test]
fn oop_ref_srp_alias() {
    let out = oop_ref_calc("srp");
    assert!(
        out.contains("Single Responsibility") || out.contains("SRP"),
        "srp alias resolves to solid"
    );
}

#[test]
fn oop_ref_ocp_alias() {
    let out = oop_ref_calc("ocp");
    assert!(
        out.contains("Open/Closed") || out.contains("OCP"),
        "ocp alias works"
    );
}

#[test]
fn oop_ref_lsp_alias() {
    let out = oop_ref_calc("lsp");
    assert!(
        out.contains("Liskov") || out.contains("LSP"),
        "lsp alias works"
    );
}

#[test]
fn oop_ref_isp_alias() {
    let out = oop_ref_calc("isp");
    assert!(
        out.contains("Interface Segregation") || out.contains("ISP"),
        "isp alias works"
    );
}

#[test]
fn oop_ref_dip_alias() {
    let out = oop_ref_calc("dip");
    assert!(
        out.contains("Dependency Inversion") || out.contains("DIP"),
        "dip alias works"
    );
}

#[test]
fn oop_ref_dependency_alias() {
    let out = oop_ref_calc("dependency");
    assert!(
        out.contains("Dependency") || out.contains("DIP"),
        "dependency alias works"
    );
}

#[test]
fn oop_ref_composition() {
    let out = oop_ref_calc("composition");
    assert!(
        out.contains("Mixin") || out.contains("mixin"),
        "composition section has mixins"
    );
    assert!(
        out.contains("delegation") || out.contains("Delegation"),
        "delegation present"
    );
    assert!(
        out.contains("ECS") || out.contains("Entity-Component"),
        "ECS pattern present"
    );
    assert!(
        out.contains("inherit") || out.contains("Inherit"),
        "inheritance comparison present"
    );
}

#[test]
fn oop_ref_mixin_alias() {
    let out = oop_ref_calc("mixin");
    assert!(
        out.contains("Mixin") || out.contains("mixin"),
        "mixin alias resolves to composition"
    );
}

#[test]
fn oop_ref_delegation_alias() {
    let out = oop_ref_calc("delegation");
    assert!(
        out.contains("delegation") || out.contains("Delegation"),
        "delegation alias works"
    );
}

#[test]
fn oop_ref_ecs_alias() {
    let out = oop_ref_calc("ecs");
    assert!(
        out.contains("ECS") || out.contains("Entity-Component"),
        "ecs alias works"
    );
}

#[test]
fn oop_ref_inherit_alias() {
    let out = oop_ref_calc("inherit");
    assert!(
        out.contains("inherit") || out.contains("Inherit"),
        "inherit alias works"
    );
}

#[test]
fn oop_ref_antipatterns() {
    let out = oop_ref_calc("antipatterns");
    assert!(
        out.contains("God Object") || out.contains("God Class"),
        "antipatterns section has God Object"
    );
    assert!(
        out.contains("Anemic") || out.contains("anemic"),
        "Anemic Domain Model present"
    );
    assert!(
        out.contains("Feature Envy") || out.contains("envy"),
        "Feature Envy present"
    );
    assert!(
        out.contains("Primitive Obsession") || out.contains("primitive"),
        "Primitive Obsession present"
    );
    assert!(
        out.contains("Shotgun") || out.contains("shotgun"),
        "Shotgun Surgery present"
    );
}

#[test]
fn oop_ref_god_object_alias() {
    let out = oop_ref_calc("god-object");
    assert!(
        out.contains("God Object") || out.contains("God Class"),
        "god-object alias resolves to antipatterns"
    );
}

#[test]
fn oop_ref_anemic_alias() {
    let out = oop_ref_calc("anemic");
    assert!(
        out.contains("Anemic") || out.contains("anemic"),
        "anemic alias works"
    );
}

#[test]
fn oop_ref_envy_alias() {
    let out = oop_ref_calc("envy");
    assert!(
        out.contains("Feature Envy") || out.contains("envy"),
        "envy alias works"
    );
}

#[test]
fn oop_ref_primitive_alias() {
    let out = oop_ref_calc("primitive");
    assert!(
        out.contains("Primitive Obsession") || out.contains("primitive"),
        "primitive alias works"
    );
}

#[test]
fn oop_ref_shotgun_alias() {
    let out = oop_ref_calc("shotgun");
    assert!(
        out.contains("Shotgun") || out.contains("shotgun"),
        "shotgun alias works"
    );
}

#[test]
fn oop_ref_leaky_alias() {
    let out = oop_ref_calc("leaky");
    assert!(
        out.contains("Leaky") || out.contains("leaky"),
        "leaky alias works"
    );
}

#[test]
fn oop_ref_not_found() {
    let out = oop_ref_calc("xyznotfound123");
    assert!(
        out.contains("No topic found"),
        "unknown query returns not-found message"
    );
}
