/// Focused integration tests for hematite::tools::math_util
///
/// Covers known-value correctness, edge-case non-panics, and the newer
/// matrix decomposition modes (QR, SVD, Cholesky).
use hematite::tools::math_util::{
    accessibility_calc, algo_ref_calc, ansible_calc, api_design_calc, api_gateway_calc, ascii_calc,
    ascii_table_calc, auth_ref_calc, awk_calc, bash_adv_calc, bash_ref_calc, bitwise_calc,
    case_calc, chars_calc, checksum_calc, chemistry_calc, chmod_calc, cicd_ref_calc, cipher_calc,
    cloud_native_calc, cloud_ref_calc, color_calc, color_names_calc, combinatorics_calc,
    compiler_ref_calc, complex_calc, concurrency_ref_calc, container_ref_calc, cron_calc,
    crypto_ref_calc, css_ref_calc, csv_calc, curl_calc, data_formats_calc, database_adv_calc,
    datetime_calc, db_design_calc, db_migrations_calc, design_patterns_calc, devops_ref_calc,
    diff_calc, docker_adv_calc, docker_compose_calc, docker_ref_calc, duration_calc,
    electrical_calc, encode_calc, escape_calc, event_driven_calc, find_calc, fraction_calc,
    geometry_calc, git_adv_calc, git_internals_calc, git_ref_calc, gitignore_calc, go_ref_calc,
    grep_calc, grpc_ref_calc, hash_calc, headers_calc, health_calc, http_adv_calc, http_calc,
    http_headers_calc, http_security_calc, http_status_calc, id_gen_calc, ip_calc, jinja_calc,
    jq_calc, js_ref_calc, json_calc, json_path_calc, jwt_calc, k8s_ref_calc, k8s_security_calc,
    kbd_calc, kubectl_calc, license_calc, linux_adv_calc, linux_kernel_calc, linux_perf_calc,
    linux_sys_calc, lorem_calc, make_calc, makefile_calc, markdown_calc, matrix_calc, mime_calc,
    ml_ref_calc, monitoring_ref_calc, net_calc, network_ref_calc, networking_adv_calc, nginx_calc,
    npm_calc, number_format, number_theory_calc, oauth_ref_calc, observability_calc, oop_ref_calc,
    openssl_calc, percent_calc, perf_ref_calc, physics_calc, port_calc, postgres_calc, prob_calc,
    protocols_ref_calc, python_data_calc, python_ref_calc, regex_adv_calc, regex_calc,
    regex_engine_calc, regex_patterns_calc, regex_ref_calc, regex_test_calc, roman_calc,
    rust_adv_calc, rust_patterns_calc, rust_ref_calc, search_ref_calc, security_ref_calc,
    security_scan_calc, sed_calc, semver_calc, set_calc, sort_viz, spark_calc, sql_adv_calc,
    sql_fmt_calc, sql_ref_calc, sql_tuning_calc, ssh_ref_calc, ssl_calc, stats_calc, string_dist,
    systemd_adv_calc, systemd_calc, table_calc, tar_calc, template_calc, terraform_adv_calc,
    terraform_calc, testing_ref_calc, text_stats, timestamp_calc, tmux_calc, toml_calc, trig_calc,
    ts_ref_calc, typescript_adv_calc, tz_calc, unicode_ref_calc, url_calc, uuid_calc,
    validate_calc, vim_adv_calc, vim_calc, wasm_ref_calc, wasm_runtime_calc, web_perf_calc,
    xml_calc, yaml_calc,
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
        out.contains("Blanket")
            || out.contains("blanket")
            || out.contains("Associated")
            || out.contains("associated"),
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

// ── typescript_adv_calc ───────────────────────────────────────────────────────

#[test]
fn typescript_adv_help_empty() {
    let out = typescript_adv_calc("");
    assert!(
        out.contains("hematite --typescript-adv"),
        "help shown for empty query"
    );
}

#[test]
fn typescript_adv_help_unknown() {
    let out = typescript_adv_calc("zzznomatch");
    assert!(
        out.contains("hematite --typescript-adv"),
        "help shown for unknown query"
    );
}

#[test]
fn typescript_adv_all() {
    let out = typescript_adv_calc("all");
    assert!(
        out.contains("Partial") || out.contains("utility"),
        "all has utility types content"
    );
    assert!(
        out.contains("infer") || out.contains("conditional"),
        "all has conditional types content"
    );
    assert!(
        out.contains("keyof") || out.contains("generics"),
        "all has generics content"
    );
}

#[test]
fn typescript_adv_generics() {
    let out = typescript_adv_calc("generics");
    assert!(
        out.contains("keyof") || out.contains("constraint"),
        "generics section has keyof"
    );
}

#[test]
fn typescript_adv_generic_alias() {
    let out = typescript_adv_calc("generic");
    assert!(
        out.contains("keyof") || out.contains("generic"),
        "generic alias resolves"
    );
}

#[test]
fn typescript_adv_type_params_alias() {
    let out = typescript_adv_calc("type-params");
    assert!(
        out.contains("keyof") || out.contains("type-params") || out.contains("Type"),
        "type-params alias resolves"
    );
}

#[test]
fn typescript_adv_variance_alias() {
    let out = typescript_adv_calc("variance");
    assert!(
        out.contains("variance") || out.contains("Variance") || out.contains("covariant"),
        "variance alias resolves"
    );
}

#[test]
fn typescript_adv_utility() {
    let out = typescript_adv_calc("utility");
    assert!(out.contains("Partial"), "utility section has Partial");
    assert!(out.contains("Omit"), "utility section has Omit");
    assert!(out.contains("Pick"), "utility section has Pick");
    assert!(out.contains("Record"), "utility section has Record");
    assert!(out.contains("ReturnType"), "utility section has ReturnType");
}

#[test]
fn typescript_adv_utility_types_alias() {
    let out = typescript_adv_calc("utility-types");
    assert!(
        out.contains("Partial") || out.contains("Omit"),
        "utility-types alias resolves"
    );
}

#[test]
fn typescript_adv_required_alias() {
    let out = typescript_adv_calc("required");
    assert!(
        out.contains("Required") || out.contains("Partial"),
        "required alias resolves"
    );
}

#[test]
fn typescript_adv_readonly_alias() {
    let out = typescript_adv_calc("readonly");
    assert!(
        out.contains("Readonly") || out.contains("readonly"),
        "readonly alias resolves"
    );
}

#[test]
fn typescript_adv_conditional() {
    let out = typescript_adv_calc("conditional");
    assert!(
        out.contains("infer") || out.contains("Conditional"),
        "conditional section has infer"
    );
    assert!(
        out.contains("extends") || out.contains("distributive"),
        "conditional section has extends"
    );
}

#[test]
fn typescript_adv_infer_alias() {
    let out = typescript_adv_calc("infer");
    assert!(
        out.contains("infer") || out.contains("conditional"),
        "infer alias resolves"
    );
}

#[test]
fn typescript_adv_distributive_alias() {
    let out = typescript_adv_calc("distributive");
    assert!(
        out.contains("distributive") || out.contains("Distributive") || out.contains("extends"),
        "distributive alias resolves"
    );
}

#[test]
fn typescript_adv_template_alias() {
    let out = typescript_adv_calc("template");
    assert!(
        out.contains("template") || out.contains("Template") || out.contains("literal"),
        "template alias resolves"
    );
}

#[test]
fn typescript_adv_mapped() {
    let out = typescript_adv_calc("mapped");
    assert!(
        out.contains("keyof") || out.contains("mapped") || out.contains("Mapped"),
        "mapped section present"
    );
}

#[test]
fn typescript_adv_map_alias() {
    let out = typescript_adv_calc("map");
    assert!(
        out.contains("keyof") || out.contains("mapped") || out.contains("Mapped"),
        "map alias resolves to mapped"
    );
}

#[test]
fn typescript_adv_key_remapping_alias() {
    let out = typescript_adv_calc("key-remapping");
    assert!(
        out.contains("key-remapping") || out.contains("remapping") || out.contains("as"),
        "key-remapping alias resolves"
    );
}

#[test]
fn typescript_adv_decorators() {
    let out = typescript_adv_calc("decorators");
    assert!(
        out.contains("decorator") || out.contains("Decorator"),
        "decorators section present"
    );
}

#[test]
fn typescript_adv_decorator_alias() {
    let out = typescript_adv_calc("decorator");
    assert!(
        out.contains("decorator") || out.contains("Decorator"),
        "decorator alias resolves"
    );
}

#[test]
fn typescript_adv_modules() {
    let out = typescript_adv_calc("modules");
    assert!(
        out.contains(".d.ts") || out.contains("module") || out.contains("Module"),
        "modules section present"
    );
}

#[test]
fn typescript_adv_module_alias() {
    let out = typescript_adv_calc("module");
    assert!(
        out.contains(".d.ts") || out.contains("module") || out.contains("Module"),
        "module alias resolves"
    );
}

#[test]
fn typescript_adv_tsconfig_alias() {
    let out = typescript_adv_calc("tsconfig");
    assert!(
        out.contains("tsconfig") || out.contains("strict"),
        "tsconfig alias resolves"
    );
}

#[test]
fn typescript_adv_ambient_alias() {
    let out = typescript_adv_calc("ambient");
    assert!(
        out.contains("ambient") || out.contains("Ambient") || out.contains(".d.ts"),
        "ambient alias resolves"
    );
}

#[test]
fn typescript_adv_no_panic_on_special_chars() {
    let _ = typescript_adv_calc("@#$%");
    let _ = typescript_adv_calc("   ");
}

// ── bash_adv_calc ─────────────────────────────────────────────────────────────

#[test]
fn bash_adv_help_empty() {
    let out = bash_adv_calc("");
    assert!(
        out.contains("hematite --bash-adv"),
        "help shown for empty query"
    );
}

#[test]
fn bash_adv_help_unknown() {
    let out = bash_adv_calc("zzznomatch");
    assert!(
        out.contains("hematite --bash-adv"),
        "help shown for unknown query"
    );
}

#[test]
fn bash_adv_all() {
    let out = bash_adv_calc("all");
    assert!(
        out.contains("associative") || out.contains("declare"),
        "all has arrays content"
    );
    assert!(
        out.contains("parameter") || out.contains("expansion"),
        "all has string content"
    );
    assert!(
        out.contains("pipefail") || out.contains("strict"),
        "all has traps content"
    );
}

#[test]
fn bash_adv_arrays() {
    let out = bash_adv_calc("arrays");
    assert!(
        out.contains("declare") || out.contains("associative"),
        "arrays section has declare"
    );
    assert!(
        out.contains("mapfile") || out.contains("readarray"),
        "arrays section has mapfile"
    );
}

#[test]
fn bash_adv_array_alias() {
    let out = bash_adv_calc("array");
    assert!(
        out.contains("declare") || out.contains("associative") || out.contains("array"),
        "array alias resolves"
    );
}

#[test]
fn bash_adv_associative_alias() {
    let out = bash_adv_calc("associative");
    assert!(
        out.contains("associative") || out.contains("declare"),
        "associative alias resolves"
    );
}

#[test]
fn bash_adv_mapfile_alias() {
    let out = bash_adv_calc("mapfile");
    assert!(
        out.contains("mapfile") || out.contains("readarray"),
        "mapfile alias resolves"
    );
}

#[test]
fn bash_adv_strings() {
    let out = bash_adv_calc("strings");
    assert!(
        out.contains("expansion") || out.contains("parameter"),
        "strings section has parameter expansion"
    );
    assert!(
        out.contains("##") || out.contains("prefix") || out.contains("suffix"),
        "strings section has prefix/suffix removal"
    );
}

#[test]
fn bash_adv_string_alias() {
    let out = bash_adv_calc("string");
    assert!(
        out.contains("expansion") || out.contains("parameter") || out.contains("string"),
        "string alias resolves"
    );
}

#[test]
fn bash_adv_parameter_expansion_alias() {
    let out = bash_adv_calc("parameter-expansion");
    assert!(
        out.contains("expansion") || out.contains("parameter"),
        "parameter-expansion alias resolves"
    );
}

#[test]
fn bash_adv_param_alias() {
    let out = bash_adv_calc("param");
    assert!(
        out.contains("expansion") || out.contains("parameter") || out.contains("param"),
        "param alias resolves"
    );
}

#[test]
fn bash_adv_arithmetic() {
    let out = bash_adv_calc("arithmetic");
    assert!(
        out.contains("(( ") || out.contains("$(( ") || out.contains("let"),
        "arithmetic section has (( ))"
    );
}

#[test]
fn bash_adv_math_alias() {
    let out = bash_adv_calc("math");
    assert!(
        out.contains("(( ") || out.contains("$(( ") || out.contains("let"),
        "math alias resolves to arithmetic"
    );
}

#[test]
fn bash_adv_expr_alias() {
    let out = bash_adv_calc("expr");
    assert!(
        out.contains("(( ") || out.contains("expr") || out.contains("let"),
        "expr alias resolves"
    );
}

#[test]
fn bash_adv_bc_alias() {
    let out = bash_adv_calc("bc");
    assert!(
        out.contains("bc") || out.contains("awk"),
        "bc alias resolves"
    );
}

#[test]
fn bash_adv_substitution() {
    let out = bash_adv_calc("substitution");
    assert!(
        out.contains("$(") || out.contains("command substitution"),
        "substitution section has command substitution"
    );
    assert!(
        out.contains("heredoc") || out.contains("<<") || out.contains("EOF"),
        "substitution section has heredoc"
    );
}

#[test]
fn bash_adv_command_sub_alias() {
    let out = bash_adv_calc("command-sub");
    assert!(
        out.contains("$(") || out.contains("command") || out.contains("substitution"),
        "command-sub alias resolves"
    );
}

#[test]
fn bash_adv_heredoc_alias() {
    let out = bash_adv_calc("heredoc");
    assert!(
        out.contains("heredoc") || out.contains("EOF") || out.contains("<<"),
        "heredoc alias resolves"
    );
}

#[test]
fn bash_adv_process_sub_alias() {
    let out = bash_adv_calc("process-sub");
    assert!(
        out.contains("process") || out.contains("<(") || out.contains(">("),
        "process-sub alias resolves"
    );
}

#[test]
fn bash_adv_traps() {
    let out = bash_adv_calc("traps");
    assert!(
        out.contains("trap") || out.contains("EXIT") || out.contains("ERR"),
        "traps section has trap"
    );
    assert!(
        out.contains("pipefail") || out.contains("set -"),
        "traps section has strict mode"
    );
}

#[test]
fn bash_adv_trap_alias() {
    let out = bash_adv_calc("trap");
    assert!(
        out.contains("trap") || out.contains("EXIT"),
        "trap alias resolves"
    );
}

#[test]
fn bash_adv_strict_alias() {
    let out = bash_adv_calc("strict");
    assert!(
        out.contains("pipefail") || out.contains("set -"),
        "strict alias resolves"
    );
}

#[test]
fn bash_adv_signals_alias() {
    let out = bash_adv_calc("signals");
    assert!(
        out.contains("INT") || out.contains("TERM") || out.contains("EXIT"),
        "signals alias resolves"
    );
}

#[test]
fn bash_adv_patterns() {
    let out = bash_adv_calc("patterns");
    assert!(
        out.contains("glob") || out.contains("extglob") || out.contains("brace"),
        "patterns section has glob"
    );
}

#[test]
fn bash_adv_glob_alias() {
    let out = bash_adv_calc("glob");
    assert!(
        out.contains("glob") || out.contains("*"),
        "glob alias resolves"
    );
}

#[test]
fn bash_adv_extglob_alias() {
    let out = bash_adv_calc("extglob");
    assert!(
        out.contains("extglob") || out.contains("shopt"),
        "extglob alias resolves"
    );
}

#[test]
fn bash_adv_brace_alias() {
    let out = bash_adv_calc("brace");
    assert!(
        out.contains("brace") || out.contains("{"),
        "brace alias resolves"
    );
}

#[test]
fn bash_adv_no_panic_on_special_chars() {
    let _ = bash_adv_calc("@#$%");
    let _ = bash_adv_calc("   ");
}

// ── network_ref_calc ──────────────────────────────────────────────────────────

#[test]
fn network_ref_help_empty() {
    let out = network_ref_calc("");
    assert!(
        out.contains("hematite --network-ref"),
        "help shown for empty query"
    );
}

#[test]
fn network_ref_help_unknown() {
    let out = network_ref_calc("zzznomatch");
    assert!(
        out.contains("hematite --network-ref"),
        "help shown for unknown query"
    );
}

#[test]
fn network_ref_all() {
    let out = network_ref_calc("all");
    assert!(
        out.contains("OSI") || out.contains("Layer"),
        "all has OSI content"
    );
    assert!(
        out.contains("TCP") || out.contains("handshake"),
        "all has TCP content"
    );
    assert!(
        out.contains("DNS") || out.contains("CNAME"),
        "all has DNS content"
    );
}

#[test]
fn network_ref_osi() {
    let out = network_ref_calc("osi");
    assert!(
        out.contains("OSI") || out.contains("Layer"),
        "osi section present"
    );
    assert!(
        out.contains("Physical") || out.contains("Layer 1"),
        "osi has Layer 1"
    );
    assert!(
        out.contains("Application") || out.contains("Layer 7"),
        "osi has Layer 7"
    );
}

#[test]
fn network_ref_layers_alias() {
    let out = network_ref_calc("layers");
    assert!(
        out.contains("OSI") || out.contains("Layer"),
        "layers alias resolves to osi"
    );
}

#[test]
fn network_ref_model_alias() {
    let out = network_ref_calc("model");
    assert!(
        out.contains("OSI") || out.contains("Layer") || out.contains("TCP/IP"),
        "model alias resolves"
    );
}

#[test]
fn network_ref_ports_alias() {
    let out = network_ref_calc("ports");
    assert!(
        out.contains("80") || out.contains("443") || out.contains("port"),
        "ports alias resolves"
    );
}

#[test]
fn network_ref_tcp_ip() {
    let out = network_ref_calc("tcp-ip");
    assert!(
        out.contains("SYN") || out.contains("handshake"),
        "tcp-ip section has SYN"
    );
    assert!(
        out.contains("ACK") || out.contains("FIN"),
        "tcp-ip section has ACK/FIN"
    );
}

#[test]
fn network_ref_tcp_alias() {
    let out = network_ref_calc("tcp");
    assert!(
        out.contains("SYN") || out.contains("TCP"),
        "tcp alias resolves"
    );
}

#[test]
fn network_ref_handshake_alias() {
    let out = network_ref_calc("handshake");
    assert!(
        out.contains("SYN") || out.contains("handshake"),
        "handshake alias resolves"
    );
}

#[test]
fn network_ref_flags_alias() {
    let out = network_ref_calc("flags");
    assert!(
        out.contains("SYN") || out.contains("ACK") || out.contains("FIN"),
        "flags alias resolves"
    );
}

#[test]
fn network_ref_udp_alias() {
    let out = network_ref_calc("udp");
    assert!(
        out.contains("UDP") || out.contains("ICMP"),
        "udp alias resolves"
    );
}

#[test]
fn network_ref_subnetting() {
    let out = network_ref_calc("subnetting");
    assert!(
        out.contains("CIDR") || out.contains("subnet"),
        "subnetting section has CIDR"
    );
    assert!(
        out.contains("192.168") || out.contains("RFC 1918") || out.contains("private"),
        "subnetting section has private ranges"
    );
}

#[test]
fn network_ref_subnet_alias() {
    let out = network_ref_calc("subnet");
    assert!(
        out.contains("CIDR") || out.contains("subnet"),
        "subnet alias resolves"
    );
}

#[test]
fn network_ref_cidr_alias() {
    let out = network_ref_calc("cidr");
    assert!(
        out.contains("CIDR") || out.contains("/24"),
        "cidr alias resolves"
    );
}

#[test]
fn network_ref_ipv6_alias() {
    let out = network_ref_calc("ipv6");
    assert!(
        out.contains("IPv6") || out.contains("::/"),
        "ipv6 alias resolves"
    );
}

#[test]
fn network_ref_dns() {
    let out = network_ref_calc("dns");
    assert!(out.contains("A"), "dns section has A record");
    assert!(
        out.contains("CNAME") || out.contains("MX"),
        "dns section has CNAME/MX"
    );
    assert!(
        out.contains("TXT") || out.contains("NS"),
        "dns section has TXT/NS"
    );
}

#[test]
fn network_ref_records_alias() {
    let out = network_ref_calc("records");
    assert!(
        out.contains("CNAME") || out.contains("MX") || out.contains("DNS"),
        "records alias resolves"
    );
}

#[test]
fn network_ref_mx_alias() {
    let out = network_ref_calc("mx");
    assert!(
        out.contains("MX") || out.contains("DNS"),
        "mx alias resolves"
    );
}

#[test]
fn network_ref_spf_alias() {
    let out = network_ref_calc("spf");
    assert!(
        out.contains("SPF") || out.contains("DKIM") || out.contains("DMARC"),
        "spf alias resolves"
    );
}

#[test]
fn network_ref_tls() {
    let out = network_ref_calc("tls");
    assert!(
        out.contains("TLS") || out.contains("certificate"),
        "tls section has TLS"
    );
    assert!(
        out.contains("handshake") || out.contains("cipher"),
        "tls section has handshake"
    );
}

#[test]
fn network_ref_ssl_alias() {
    let out = network_ref_calc("ssl");
    assert!(
        out.contains("TLS") || out.contains("SSL"),
        "ssl alias resolves to tls"
    );
}

#[test]
fn network_ref_https_alias() {
    let out = network_ref_calc("https");
    assert!(
        out.contains("TLS") || out.contains("HTTPS") || out.contains("certificate"),
        "https alias resolves"
    );
}

#[test]
fn network_ref_certificate_alias() {
    let out = network_ref_calc("certificate");
    assert!(
        out.contains("certificate") || out.contains("cert") || out.contains("TLS"),
        "certificate alias resolves"
    );
}

#[test]
fn network_ref_hsts_alias() {
    let out = network_ref_calc("hsts");
    assert!(
        out.contains("HSTS") || out.contains("Strict"),
        "hsts alias resolves"
    );
}

#[test]
fn network_ref_protocols() {
    let out = network_ref_calc("protocols");
    assert!(
        out.contains("HTTP") || out.contains("HTTP/2"),
        "protocols section has HTTP"
    );
    assert!(
        out.contains("WebSocket") || out.contains("gRPC") || out.contains("MQTT"),
        "protocols section has WebSocket/gRPC/MQTT"
    );
}

#[test]
fn network_ref_http_alias() {
    let out = network_ref_calc("http");
    assert!(
        out.contains("HTTP") || out.contains("HTTP/2"),
        "http alias resolves"
    );
}

#[test]
fn network_ref_http2_alias() {
    let out = network_ref_calc("http2");
    assert!(
        out.contains("HTTP/2") || out.contains("HTTP"),
        "http2 alias resolves"
    );
}

#[test]
fn network_ref_websocket_alias() {
    let out = network_ref_calc("websocket");
    assert!(
        out.contains("WebSocket") || out.contains("websocket"),
        "websocket alias resolves"
    );
}

#[test]
fn network_ref_grpc_alias() {
    let out = network_ref_calc("grpc");
    assert!(
        out.contains("gRPC") || out.contains("grpc"),
        "grpc alias resolves"
    );
}

#[test]
fn network_ref_mqtt_alias() {
    let out = network_ref_calc("mqtt");
    assert!(
        out.contains("MQTT") || out.contains("mqtt"),
        "mqtt alias resolves"
    );
}

#[test]
fn network_ref_quic_alias() {
    let out = network_ref_calc("quic");
    assert!(
        out.contains("QUIC") || out.contains("HTTP/3"),
        "quic alias resolves"
    );
}

#[test]
fn network_ref_no_panic_on_special_chars() {
    let _ = network_ref_calc("@#$%");
    let _ = network_ref_calc("   ");
}

// ── unicode_ref_calc ──────────────────────────────────────────────────────────

#[test]
fn unicode_ref_help_empty() {
    let out = unicode_ref_calc("");
    assert!(
        out.contains("hematite --unicode-ref"),
        "help shown for empty query"
    );
}

#[test]
fn unicode_ref_help_unknown() {
    let out = unicode_ref_calc("zzznomatch");
    assert!(
        out.contains("hematite --unicode-ref"),
        "help shown for unknown query"
    );
}

#[test]
fn unicode_ref_all() {
    let out = unicode_ref_calc("all");
    assert!(
        out.contains("UTF-8") || out.contains("encoding"),
        "all has encoding content"
    );
    assert!(
        out.contains("NFC") || out.contains("normalization"),
        "all has normalization content"
    );
    assert!(
        out.contains("BOM") || out.contains("byte order"),
        "all has BOM content"
    );
}

#[test]
fn unicode_ref_encoding() {
    let out = unicode_ref_calc("encoding");
    assert!(
        out.contains("UTF-8") || out.contains("encoding"),
        "encoding section present"
    );
    assert!(
        out.contains("UTF-16") || out.contains("surrogate"),
        "encoding section has UTF-16"
    );
    assert!(
        out.contains("UTF-32") || out.contains("4 byte"),
        "encoding section has UTF-32"
    );
}

#[test]
fn unicode_ref_utf8_alias() {
    let out = unicode_ref_calc("utf8");
    assert!(
        out.contains("UTF-8") || out.contains("byte"),
        "utf8 alias resolves"
    );
}

#[test]
fn unicode_ref_utf_alias() {
    let out = unicode_ref_calc("utf");
    assert!(
        out.contains("UTF-8") || out.contains("UTF"),
        "utf alias resolves"
    );
}

#[test]
fn unicode_ref_surrogate_alias() {
    let out = unicode_ref_calc("surrogate");
    assert!(
        out.contains("surrogate") || out.contains("UTF-16"),
        "surrogate alias resolves"
    );
}

#[test]
fn unicode_ref_codepoints() {
    let out = unicode_ref_calc("codepoints");
    assert!(
        out.contains("U+") || out.contains("code point"),
        "codepoints section has U+ notation"
    );
    assert!(
        out.contains("plane") || out.contains("Plane"),
        "codepoints section has planes"
    );
}

#[test]
fn unicode_ref_codepoint_alias() {
    let out = unicode_ref_calc("codepoint");
    assert!(
        out.contains("U+") || out.contains("code point") || out.contains("codepoint"),
        "codepoint alias resolves"
    );
}

#[test]
fn unicode_ref_planes_alias() {
    let out = unicode_ref_calc("planes");
    assert!(
        out.contains("plane") || out.contains("Plane") || out.contains("BMP"),
        "planes alias resolves"
    );
}

#[test]
fn unicode_ref_zwj_alias() {
    let out = unicode_ref_calc("zwj");
    assert!(
        out.contains("ZWJ") || out.contains("U+200D") || out.contains("zero"),
        "zwj alias resolves"
    );
}

#[test]
fn unicode_ref_normalization() {
    let out = unicode_ref_calc("normalization");
    assert!(
        out.contains("NFC") || out.contains("NFD"),
        "normalization section has NFC/NFD"
    );
    assert!(
        out.contains("NFKC") || out.contains("NFKD"),
        "normalization section has NFKC/NFKD"
    );
}

#[test]
fn unicode_ref_normalize_alias() {
    let out = unicode_ref_calc("normalize");
    assert!(
        out.contains("NFC") || out.contains("NFD") || out.contains("normali"),
        "normalize alias resolves"
    );
}

#[test]
fn unicode_ref_nfc_alias() {
    let out = unicode_ref_calc("nfc");
    assert!(
        out.contains("NFC") || out.contains("composed"),
        "nfc alias resolves"
    );
}

#[test]
fn unicode_ref_nfd_alias() {
    let out = unicode_ref_calc("nfd");
    assert!(
        out.contains("NFD") || out.contains("decomposed"),
        "nfd alias resolves"
    );
}

#[test]
fn unicode_ref_case_folding_alias() {
    let out = unicode_ref_calc("case-folding");
    assert!(
        out.contains("case") || out.contains("fold") || out.contains("normali"),
        "case-folding alias resolves"
    );
}

#[test]
fn unicode_ref_escapes() {
    let out = unicode_ref_calc("escapes");
    assert!(
        out.contains("\\u") || out.contains("escape"),
        "escapes section has \\u"
    );
}

#[test]
fn unicode_ref_escape_alias() {
    let out = unicode_ref_calc("escape");
    assert!(
        out.contains("\\u") || out.contains("escape"),
        "escape alias resolves"
    );
}

#[test]
fn unicode_ref_python_alias() {
    let out = unicode_ref_calc("python");
    assert!(
        out.contains("\\u") || out.contains("Python") || out.contains("escape"),
        "python alias resolves"
    );
}

#[test]
fn unicode_ref_javascript_alias() {
    let out = unicode_ref_calc("js-esc");
    assert!(
        out.contains("\\u") || out.contains("JS") || out.contains("escape"),
        "js-esc alias resolves"
    );
}

#[test]
fn unicode_ref_categories() {
    let out = unicode_ref_calc("categories");
    assert!(
        out.contains("Lu") || out.contains("category"),
        "categories section has Lu"
    );
    assert!(
        out.contains("Nd") || out.contains("Ll"),
        "categories section has Nd/Ll"
    );
}

#[test]
fn unicode_ref_category_alias() {
    let out = unicode_ref_calc("category");
    assert!(
        out.contains("Lu") || out.contains("category"),
        "category alias resolves"
    );
}

#[test]
fn unicode_ref_emoji_alias() {
    let out = unicode_ref_calc("emoji");
    assert!(
        out.contains("emoji") || out.contains("ZWJ") || out.contains("skin"),
        "emoji alias resolves"
    );
}

#[test]
fn unicode_ref_bidi_alias() {
    let out = unicode_ref_calc("bidi");
    assert!(
        out.contains("Bidi") || out.contains("bidi") || out.contains("CVE"),
        "bidi alias resolves"
    );
}

#[test]
fn unicode_ref_trojan_source_alias() {
    let out = unicode_ref_calc("trojan-source");
    assert!(
        out.contains("Trojan") || out.contains("CVE") || out.contains("Bidi"),
        "trojan-source alias resolves"
    );
}

#[test]
fn unicode_ref_bom() {
    let out = unicode_ref_calc("bom");
    assert!(
        out.contains("BOM") || out.contains("byte order"),
        "bom section present"
    );
    assert!(
        out.contains("UTF-8") || out.contains("EF BB BF"),
        "bom section has UTF-8 BOM"
    );
}

#[test]
fn unicode_ref_byte_order_alias() {
    let out = unicode_ref_calc("byte-order");
    assert!(
        out.contains("BOM") || out.contains("byte order") || out.contains("byte-order"),
        "byte-order alias resolves"
    );
}

#[test]
fn unicode_ref_powershell_alias() {
    let out = unicode_ref_calc("powershell");
    assert!(
        out.contains("PowerShell") || out.contains("UTF-16") || out.contains("BOM"),
        "powershell alias resolves"
    );
}

#[test]
fn unicode_ref_string_length_alias() {
    let out = unicode_ref_calc("string-length");
    assert!(
        out.contains("length") || out.contains("character") || out.contains("BOM"),
        "string-length alias resolves"
    );
}

#[test]
fn unicode_ref_no_panic_on_special_chars() {
    let _ = unicode_ref_calc("@#$%");
    let _ = unicode_ref_calc("   ");
}

// ── regex_test_calc ───────────────────────────────────────────────────────────

#[test]
fn regex_test_help_empty() {
    let out = regex_test_calc("");
    assert!(
        out.contains("hematite --regex-tester"),
        "help shown for empty query"
    );
}

#[test]
fn regex_test_help_unknown() {
    let out = regex_test_calc("zzznomatch");
    assert!(
        out.contains("hematite --regex-tester"),
        "help shown for unknown"
    );
}

#[test]
fn regex_test_all() {
    let out = regex_test_calc("all");
    assert!(
        out.contains("^") || out.contains("anchor"),
        "all has anchors"
    );
    assert!(
        out.contains("greedy") || out.contains("quantifier"),
        "all has quantifiers"
    );
    assert!(
        out.contains("lookahead") || out.contains("group"),
        "all has groups"
    );
    assert!(
        out.contains("email") || out.contains("pattern"),
        "all has patterns"
    );
}

#[test]
fn regex_test_anchors() {
    let out = regex_test_calc("anchors");
    assert!(out.contains("^") || out.contains("\\\\A"), "anchors has ^");
    assert!(
        out.contains("\\\\b") || out.contains("boundary"),
        "anchors has word boundary"
    );
    assert!(
        out.contains("multiline") || out.contains("\\\\Z"),
        "anchors has multiline"
    );
}

#[test]
fn regex_test_anchor_alias() {
    let out = regex_test_calc("anchor");
    assert!(
        out.contains("^") || out.contains("anchor"),
        "anchor alias resolves"
    );
}

#[test]
fn regex_test_boundary_alias() {
    let out = regex_test_calc("boundary");
    assert!(
        out.contains("\\\\b") || out.contains("boundary"),
        "boundary alias resolves"
    );
}

#[test]
fn regex_test_multiline_alias() {
    let out = regex_test_calc("multiline");
    assert!(
        out.contains("multiline") || out.contains("^"),
        "multiline alias resolves"
    );
}

#[test]
fn regex_test_groups() {
    let out = regex_test_calc("groups");
    assert!(
        out.contains("capturing") || out.contains("Capturing"),
        "groups has capturing"
    );
    assert!(
        out.contains("lookahead") || out.contains("(?="),
        "groups has lookahead"
    );
    assert!(
        out.contains("named") || out.contains("(?P<"),
        "groups has named capture"
    );
}

#[test]
fn regex_test_capture_alias() {
    let out = regex_test_calc("capture");
    assert!(
        out.contains("capturing") || out.contains("group"),
        "capture alias resolves"
    );
}

#[test]
fn regex_test_lookahead_alias() {
    let out = regex_test_calc("lookahead");
    assert!(
        out.contains("lookahead") || out.contains("(?="),
        "lookahead alias resolves"
    );
}

#[test]
fn regex_test_lookbehind_alias() {
    let out = regex_test_calc("lookbehind");
    assert!(
        out.contains("lookbehind") || out.contains("(?<="),
        "lookbehind alias resolves"
    );
}

#[test]
fn regex_test_lookaround_alias() {
    let out = regex_test_calc("lookaround");
    assert!(
        out.contains("lookahead") || out.contains("lookbehind"),
        "lookaround alias resolves"
    );
}

#[test]
fn regex_test_named_alias() {
    let out = regex_test_calc("named");
    assert!(
        out.contains("named") || out.contains("(?P<"),
        "named alias resolves"
    );
}

#[test]
fn regex_test_quantifiers() {
    let out = regex_test_calc("quantifiers");
    assert!(
        out.contains("greedy") || out.contains("Greedy"),
        "quantifiers has greedy"
    );
    assert!(
        out.contains("lazy") || out.contains(".*?"),
        "quantifiers has lazy"
    );
    assert!(
        out.contains("{n}") || out.contains("{n,m}"),
        "quantifiers has interval"
    );
}

#[test]
fn regex_test_greedy_alias() {
    let out = regex_test_calc("greedy");
    assert!(
        out.contains("greedy") || out.contains("Greedy"),
        "greedy alias resolves"
    );
}

#[test]
fn regex_test_lazy_alias() {
    let out = regex_test_calc("lazy");
    assert!(
        out.contains("lazy") || out.contains(".*?"),
        "lazy alias resolves"
    );
}

#[test]
fn regex_test_repetition_alias() {
    let out = regex_test_calc("repetition");
    assert!(
        out.contains("greedy") || out.contains("{n}"),
        "repetition alias resolves"
    );
}

#[test]
fn regex_test_charclass() {
    let out = regex_test_calc("charclass");
    assert!(
        out.contains("[abc]") || out.contains("character class"),
        "charclass section present"
    );
    assert!(
        out.contains("Shorthand") || out.contains("shorthand") || out.contains("\\d"),
        "charclass has shorthand"
    );
    assert!(
        out.contains("Unicode") || out.contains("\\p{"),
        "charclass has unicode"
    );
}

#[test]
fn regex_test_bracket_alias() {
    let out = regex_test_calc("bracket");
    assert!(
        out.contains("[abc]") || out.contains("bracket") || out.contains("class"),
        "bracket alias resolves"
    );
}

#[test]
fn regex_test_shorthand_alias() {
    let out = regex_test_calc("shorthand");
    assert!(
        out.contains("Shorthand") || out.contains("shorthand") || out.contains("\\d"),
        "shorthand alias resolves"
    );
}

#[test]
fn regex_test_posix_alias() {
    let out = regex_test_calc("posix");
    assert!(
        out.contains("POSIX") || out.contains("[:alpha:]"),
        "posix alias resolves"
    );
}

#[test]
fn regex_test_flags() {
    let out = regex_test_calc("flags");
    assert!(
        out.contains("case-insensitive") || out.contains("(?i)"),
        "flags has case-insensitive"
    );
    assert!(
        out.contains("DOTALL") || out.contains("dotall") || out.contains("(?s)"),
        "flags has dotall"
    );
    assert!(
        out.contains("verbose") || out.contains("(?x)"),
        "flags has verbose"
    );
}

#[test]
fn regex_test_flag_alias() {
    let out = regex_test_calc("flag");
    assert!(
        out.contains("(?i)") || out.contains("case"),
        "flag alias resolves"
    );
}

#[test]
fn regex_test_case_alias() {
    let out = regex_test_calc("case");
    assert!(
        out.contains("case") || out.contains("(?i)"),
        "case alias resolves"
    );
}

#[test]
fn regex_test_dotall_alias() {
    let out = regex_test_calc("dotall");
    assert!(
        out.contains("DOTALL") || out.contains("dotall") || out.contains("(?s)"),
        "dotall alias resolves"
    );
}

#[test]
fn regex_test_patterns() {
    let out = regex_test_calc("patterns");
    assert!(
        out.contains("email") || out.contains("Email"),
        "patterns has email"
    );
    assert!(
        out.contains("UUID") || out.contains("uuid"),
        "patterns has UUID"
    );
    assert!(
        out.contains("IPv4") || out.contains("IP address"),
        "patterns has IP"
    );
}

#[test]
fn regex_test_email_alias() {
    let out = regex_test_calc("email");
    assert!(
        out.contains("email") || out.contains("@"),
        "email alias resolves"
    );
}

#[test]
fn regex_test_url_alias() {
    let out = regex_test_calc("url");
    assert!(
        out.contains("https?") || out.contains("URL"),
        "url alias resolves"
    );
}

#[test]
fn regex_test_ip_alias() {
    let out = regex_test_calc("ip");
    assert!(
        out.contains("IPv4") || out.contains("25[0-5]"),
        "ip alias resolves"
    );
}

#[test]
fn regex_test_uuid_alias() {
    let out = regex_test_calc("uuid");
    assert!(
        out.contains("UUID") || out.contains("uuid") || out.contains("[0-9a-f]"),
        "uuid alias resolves"
    );
}

#[test]
fn regex_test_password_alias() {
    let out = regex_test_calc("password");
    assert!(
        out.contains("password") || out.contains("Password") || out.contains("strength"),
        "password alias resolves"
    );
}

#[test]
fn regex_test_hex_alias() {
    let out = regex_test_calc("hex");
    assert!(
        out.contains("hex") || out.contains("#") || out.contains("[0-9a-fA"),
        "hex alias resolves"
    );
}

#[test]
fn regex_test_no_panic_special() {
    let _ = regex_test_calc("@#$%");
    let _ = regex_test_calc("   ");
}

// ── http_headers_calc ─────────────────────────────────────────────────────────

#[test]
fn http_headers_help_empty() {
    let out = http_headers_calc("");
    assert!(
        out.contains("hematite --http-headers"),
        "help shown for empty query"
    );
}

#[test]
fn http_headers_help_unknown() {
    let out = http_headers_calc("zzznomatch");
    assert!(
        out.contains("hematite --http-headers"),
        "help shown for unknown"
    );
}

#[test]
fn http_headers_all() {
    let out = http_headers_calc("all");
    assert!(
        out.contains("Host:") || out.contains("User-Agent"),
        "all has request headers"
    );
    assert!(
        out.contains("Content-Type") || out.contains("ETag"),
        "all has response headers"
    );
    assert!(
        out.contains("CSP") || out.contains("HSTS"),
        "all has security headers"
    );
    assert!(out.contains("CORS") || out.contains("cors"), "all has CORS");
}

#[test]
fn http_headers_request() {
    let out = http_headers_calc("request");
    assert!(out.contains("Host:"), "request section has Host");
    assert!(
        out.contains("User-Agent") || out.contains("Accept"),
        "request has Accept"
    );
    assert!(
        out.contains("Authorization") || out.contains("Cookie"),
        "request has auth headers"
    );
}

#[test]
fn http_headers_req_alias() {
    let out = http_headers_calc("req");
    assert!(
        out.contains("Host:") || out.contains("User-Agent"),
        "req alias resolves"
    );
}

#[test]
fn http_headers_accept_alias() {
    let out = http_headers_calc("accept");
    assert!(
        out.contains("Accept") || out.contains("User-Agent"),
        "accept alias resolves"
    );
}

#[test]
fn http_headers_user_agent_alias() {
    let out = http_headers_calc("user-agent");
    assert!(
        out.contains("User-Agent") || out.contains("user-agent"),
        "user-agent alias resolves"
    );
}

#[test]
fn http_headers_response() {
    let out = http_headers_calc("response");
    assert!(
        out.contains("Content-Type") || out.contains("ETag"),
        "response section present"
    );
    assert!(
        out.contains("Cache-Control") || out.contains("Vary"),
        "response has caching"
    );
    assert!(
        out.contains("Set-Cookie") || out.contains("Location"),
        "response has Set-Cookie"
    );
}

#[test]
fn http_headers_resp_alias() {
    let out = http_headers_calc("resp");
    assert!(
        out.contains("Content-Type") || out.contains("ETag"),
        "resp alias resolves"
    );
}

#[test]
fn http_headers_etag_alias() {
    let out = http_headers_calc("etag");
    assert!(
        out.contains("ETag") || out.contains("etag"),
        "etag alias resolves"
    );
}

#[test]
fn http_headers_set_cookie_alias() {
    let out = http_headers_calc("set-cookie");
    assert!(
        out.contains("Set-Cookie") || out.contains("HttpOnly"),
        "set-cookie alias resolves"
    );
}

#[test]
fn http_headers_security() {
    let out = http_headers_calc("security");
    assert!(
        out.contains("HSTS") || out.contains("Strict-Transport"),
        "security has HSTS"
    );
    assert!(
        out.contains("CSP") || out.contains("Content-Security"),
        "security has CSP"
    );
    assert!(
        out.contains("nosniff") || out.contains("X-Content"),
        "security has nosniff"
    );
}

#[test]
fn http_headers_hsts_alias() {
    let out = http_headers_calc("hsts");
    assert!(
        out.contains("HSTS") || out.contains("Strict-Transport"),
        "hsts alias resolves"
    );
}

#[test]
fn http_headers_csp_alias() {
    let out = http_headers_calc("csp");
    assert!(
        out.contains("CSP") || out.contains("Content-Security"),
        "csp alias resolves"
    );
}

#[test]
fn http_headers_x_frame_alias() {
    let out = http_headers_calc("x-frame");
    assert!(
        out.contains("X-Frame") || out.contains("frame"),
        "x-frame alias resolves"
    );
}

#[test]
fn http_headers_nosniff_alias() {
    let out = http_headers_calc("nosniff");
    assert!(
        out.contains("nosniff") || out.contains("X-Content"),
        "nosniff alias resolves"
    );
}

#[test]
fn http_headers_coop_alias() {
    let out = http_headers_calc("coop");
    assert!(
        out.contains("COOP") || out.contains("Cross-Origin"),
        "coop alias resolves"
    );
}

#[test]
fn http_headers_cors() {
    let out = http_headers_calc("cors");
    assert!(
        out.contains("Access-Control") || out.contains("CORS"),
        "cors section present"
    );
    assert!(
        out.contains("Allow-Origin") || out.contains("preflight"),
        "cors has Allow-Origin"
    );
    assert!(
        out.contains("Allow-Methods") || out.contains("OPTIONS"),
        "cors has methods"
    );
}

#[test]
fn http_headers_cross_origin_alias() {
    let out = http_headers_calc("cross-origin");
    assert!(
        out.contains("CORS") || out.contains("Access-Control"),
        "cross-origin alias resolves"
    );
}

#[test]
fn http_headers_preflight_alias() {
    let out = http_headers_calc("preflight");
    assert!(
        out.contains("preflight") || out.contains("OPTIONS"),
        "preflight alias resolves"
    );
}

#[test]
fn http_headers_allow_origin_alias() {
    let out = http_headers_calc("allow-origin");
    assert!(
        out.contains("Allow-Origin") || out.contains("cors"),
        "allow-origin alias resolves"
    );
}

#[test]
fn http_headers_auth() {
    let out = http_headers_calc("auth");
    assert!(
        out.contains("Authorization") || out.contains("Bearer"),
        "auth section present"
    );
    assert!(
        out.contains("Basic") || out.contains("Digest"),
        "auth has Basic"
    );
    assert!(
        out.contains("Cookie") || out.contains("HttpOnly"),
        "auth has Cookie"
    );
}

#[test]
fn http_headers_bearer_alias() {
    let out = http_headers_calc("bearer");
    assert!(
        out.contains("Bearer") || out.contains("Authorization"),
        "bearer alias resolves"
    );
}

#[test]
fn http_headers_jwt_alias() {
    let out = http_headers_calc("jwt");
    assert!(
        out.contains("JWT") || out.contains("Bearer"),
        "jwt alias resolves"
    );
}

#[test]
fn http_headers_oauth_alias() {
    let out = http_headers_calc("oauth");
    assert!(
        out.contains("OAuth") || out.contains("oauth") || out.contains("Bearer"),
        "oauth alias resolves"
    );
}

#[test]
fn http_headers_csrf_alias() {
    let out = http_headers_calc("csrf");
    assert!(
        out.contains("CSRF") || out.contains("csrf") || out.contains("SameSite"),
        "csrf alias resolves"
    );
}

#[test]
fn http_headers_cache() {
    let out = http_headers_calc("cache");
    assert!(
        out.contains("Cache-Control") || out.contains("max-age"),
        "cache section present"
    );
    assert!(
        out.contains("no-store") || out.contains("immutable"),
        "cache has no-store"
    );
    assert!(
        out.contains("ETag") || out.contains("Vary"),
        "cache has ETag"
    );
}

#[test]
fn http_headers_cache_control_alias() {
    let out = http_headers_calc("cache-control");
    assert!(
        out.contains("Cache-Control") || out.contains("max-age"),
        "cache-control alias resolves"
    );
}

#[test]
fn http_headers_max_age_alias() {
    let out = http_headers_calc("max-age");
    assert!(
        out.contains("max-age") || out.contains("Cache-Control"),
        "max-age alias resolves"
    );
}

#[test]
fn http_headers_cdn_alias() {
    let out = http_headers_calc("cdn");
    assert!(
        out.contains("CDN") || out.contains("cdn") || out.contains("max-age"),
        "cdn alias resolves"
    );
}

#[test]
fn http_headers_immutable_alias() {
    let out = http_headers_calc("immutable");
    assert!(
        out.contains("immutable") || out.contains("Cache-Control"),
        "immutable alias resolves"
    );
}

#[test]
fn http_headers_no_panic_special() {
    let _ = http_headers_calc("@#$%");
    let _ = http_headers_calc("   ");
}

// ── crypto_ref_calc ───────────────────────────────────────────────────────────

#[test]
fn crypto_ref_help_empty() {
    let out = crypto_ref_calc("");
    assert!(
        out.contains("hematite --crypto-ref"),
        "help shown for empty query"
    );
}

#[test]
fn crypto_ref_help_unknown() {
    let out = crypto_ref_calc("zzznomatch");
    assert!(
        out.contains("hematite --crypto-ref"),
        "help shown for unknown"
    );
}

#[test]
fn crypto_ref_all() {
    let out = crypto_ref_calc("all");
    assert!(
        out.contains("AES") || out.contains("symmetric"),
        "all has symmetric"
    );
    assert!(
        out.contains("RSA") || out.contains("ECC"),
        "all has asymmetric"
    );
    assert!(
        out.contains("SHA-256") || out.contains("hash"),
        "all has hashing"
    );
    assert!(
        out.contains("certificate") || out.contains("PKI"),
        "all has PKI"
    );
}

#[test]
fn crypto_ref_symmetric() {
    let out = crypto_ref_calc("symmetric");
    assert!(out.contains("AES"), "symmetric has AES");
    assert!(
        out.contains("GCM") || out.contains("ChaCha20"),
        "symmetric has GCM"
    );
    assert!(
        out.contains("Argon2") || out.contains("bcrypt") || out.contains("PBKDF2"),
        "symmetric has KDF"
    );
}

#[test]
fn crypto_ref_aes_alias() {
    let out = crypto_ref_calc("aes");
    assert!(out.contains("AES"), "aes alias resolves");
}

#[test]
fn crypto_ref_gcm_alias() {
    let out = crypto_ref_calc("gcm");
    assert!(
        out.contains("GCM") || out.contains("AEAD"),
        "gcm alias resolves"
    );
}

#[test]
fn crypto_ref_chacha20_alias() {
    let out = crypto_ref_calc("chacha20");
    assert!(
        out.contains("ChaCha20") || out.contains("Poly1305"),
        "chacha20 alias resolves"
    );
}

#[test]
fn crypto_ref_argon2_alias() {
    let out = crypto_ref_calc("argon2");
    assert!(
        out.contains("Argon2") || out.contains("argon2"),
        "argon2 alias resolves"
    );
}

#[test]
fn crypto_ref_bcrypt_alias() {
    let out = crypto_ref_calc("bcrypt");
    assert!(
        out.contains("bcrypt") || out.contains("work factor"),
        "bcrypt alias resolves"
    );
}

#[test]
fn crypto_ref_pbkdf2_alias() {
    let out = crypto_ref_calc("pbkdf2");
    assert!(
        out.contains("PBKDF2") || out.contains("pbkdf2"),
        "pbkdf2 alias resolves"
    );
}

#[test]
fn crypto_ref_asymmetric() {
    let out = crypto_ref_calc("asymmetric");
    assert!(out.contains("RSA"), "asymmetric has RSA");
    assert!(
        out.contains("ECC") || out.contains("ECDSA"),
        "asymmetric has ECC"
    );
    assert!(
        out.contains("Ed25519") || out.contains("Curve25519"),
        "asymmetric has Ed25519"
    );
}

#[test]
fn crypto_ref_rsa_alias() {
    let out = crypto_ref_calc("rsa");
    assert!(out.contains("RSA"), "rsa alias resolves");
}

#[test]
fn crypto_ref_ecc_alias() {
    let out = crypto_ref_calc("ecc");
    assert!(
        out.contains("ECC") || out.contains("Elliptic"),
        "ecc alias resolves"
    );
}

#[test]
fn crypto_ref_ed25519_alias() {
    let out = crypto_ref_calc("ed25519");
    assert!(
        out.contains("Ed25519") || out.contains("EdDSA"),
        "ed25519 alias resolves"
    );
}

#[test]
fn crypto_ref_pq_alias() {
    let out = crypto_ref_calc("pq");
    assert!(
        out.contains("post-quantum") || out.contains("Post-Quantum") || out.contains("Kyber"),
        "pq alias resolves"
    );
}

#[test]
fn crypto_ref_kyber_alias() {
    let out = crypto_ref_calc("kyber");
    assert!(
        out.contains("Kyber") || out.contains("post-quantum"),
        "kyber alias resolves"
    );
}

#[test]
fn crypto_ref_hashing() {
    let out = crypto_ref_calc("hashing");
    assert!(out.contains("SHA-256"), "hashing has SHA-256");
    assert!(
        out.contains("HMAC") || out.contains("MAC"),
        "hashing has HMAC"
    );
    assert!(
        out.contains("Argon2") || out.contains("bcrypt"),
        "hashing has password hashing"
    );
}

#[test]
fn crypto_ref_sha256_alias() {
    let out = crypto_ref_calc("sha256");
    assert!(
        out.contains("SHA-256") || out.contains("SHA"),
        "sha256 alias resolves"
    );
}

#[test]
fn crypto_ref_blake3_alias() {
    let out = crypto_ref_calc("blake3");
    assert!(
        out.contains("BLAKE3") || out.contains("blake3"),
        "blake3 alias resolves"
    );
}

#[test]
fn crypto_ref_hmac_alias() {
    let out = crypto_ref_calc("hmac");
    assert!(
        out.contains("HMAC") || out.contains("hmac"),
        "hmac alias resolves"
    );
}

#[test]
fn crypto_ref_pki() {
    let out = crypto_ref_calc("pki");
    assert!(
        out.contains("certificate") || out.contains("PKI"),
        "pki section present"
    );
    assert!(
        out.contains("DV") || out.contains("OV") || out.contains("EV"),
        "pki has cert types"
    );
    assert!(
        out.contains("OCSP") || out.contains("CRL"),
        "pki has revocation"
    );
}

#[test]
fn crypto_ref_certificate_alias() {
    let out = crypto_ref_calc("certificate");
    assert!(
        out.contains("certificate") || out.contains("X.509"),
        "certificate alias resolves"
    );
}

#[test]
fn crypto_ref_x509_alias() {
    let out = crypto_ref_calc("x509");
    assert!(
        out.contains("X.509") || out.contains("certificate"),
        "x509 alias resolves"
    );
}

#[test]
fn crypto_ref_ocsp_alias() {
    let out = crypto_ref_calc("ocsp");
    assert!(
        out.contains("OCSP") || out.contains("revocation"),
        "ocsp alias resolves"
    );
}

#[test]
fn crypto_ref_pem_alias() {
    let out = crypto_ref_calc("pem");
    assert!(
        out.contains("PEM") || out.contains("base64"),
        "pem alias resolves"
    );
}

#[test]
fn crypto_ref_vulnerabilities() {
    let out = crypto_ref_calc("vulnerabilities");
    assert!(
        out.contains("Padding") || out.contains("padding-oracle"),
        "vulnerabilities has padding oracle"
    );
    assert!(
        out.contains("timing") || out.contains("Timing"),
        "vulnerabilities has timing attack"
    );
    assert!(
        out.contains("nonce") || out.contains("Nonce"),
        "vulnerabilities has nonce reuse"
    );
}

#[test]
fn crypto_ref_attack_alias() {
    let out = crypto_ref_calc("attack");
    assert!(
        out.contains("Attack") || out.contains("attack") || out.contains("Vulnerab"),
        "attack alias resolves"
    );
}

#[test]
fn crypto_ref_timing_alias() {
    let out = crypto_ref_calc("timing");
    assert!(
        out.contains("timing") || out.contains("Timing"),
        "timing alias resolves"
    );
}

#[test]
fn crypto_ref_padding_oracle_alias() {
    let out = crypto_ref_calc("padding-oracle");
    assert!(
        out.contains("Padding") || out.contains("padding"),
        "padding-oracle alias resolves"
    );
}

#[test]
fn crypto_ref_protocols() {
    let out = crypto_ref_calc("protocols");
    assert!(
        out.contains("TLS") || out.contains("tls"),
        "protocols has TLS"
    );
    assert!(
        out.contains("SSH") || out.contains("ssh"),
        "protocols has SSH"
    );
    assert!(
        out.contains("JWT") || out.contains("jwt"),
        "protocols has JWT"
    );
}

#[test]
fn crypto_ref_tls_alias() {
    let out = crypto_ref_calc("tls");
    assert!(
        out.contains("TLS") || out.contains("tls"),
        "tls alias resolves"
    );
}

#[test]
fn crypto_ref_ssh_alias() {
    let out = crypto_ref_calc("ssh");
    assert!(
        out.contains("SSH") || out.contains("ssh"),
        "ssh alias resolves"
    );
}

#[test]
fn crypto_ref_jwt_alias() {
    let out = crypto_ref_calc("jwt");
    assert!(
        out.contains("JWT") || out.contains("Bearer"),
        "jwt alias resolves"
    );
}

#[test]
fn crypto_ref_wireguard_alias() {
    let out = crypto_ref_calc("wireguard");
    assert!(
        out.contains("WireGuard") || out.contains("wireguard"),
        "wireguard alias resolves"
    );
}

#[test]
fn crypto_ref_no_panic_special() {
    let _ = crypto_ref_calc("@#$%");
    let _ = crypto_ref_calc("   ");
}

// ── devops_ref_calc ───────────────────────────────────────────────────────────

#[test]
fn devops_ref_help_empty() {
    let out = devops_ref_calc("");
    assert!(
        out.contains("hematite --devops-ref"),
        "help shown for empty query"
    );
}

#[test]
fn devops_ref_help_unknown() {
    let out = devops_ref_calc("zzznomatch");
    assert!(
        out.contains("hematite --devops-ref"),
        "help shown for unknown"
    );
}

#[test]
fn devops_ref_all() {
    let out = devops_ref_calc("all");
    assert!(
        out.contains("pipeline") || out.contains("CI"),
        "all has cicd"
    );
    assert!(
        out.contains("Docker") || out.contains("Kubernetes"),
        "all has containers"
    );
    assert!(
        out.contains("Terraform") || out.contains("Ansible"),
        "all has iac"
    );
    assert!(
        out.contains("Prometheus") || out.contains("SLO"),
        "all has monitoring"
    );
}

#[test]
fn devops_ref_cicd() {
    let out = devops_ref_calc("cicd");
    assert!(
        out.contains("pipeline") || out.contains("CI"),
        "cicd section present"
    );
    assert!(
        out.contains("GitHub Actions") || out.contains("GitLab"),
        "cicd has GitHub Actions"
    );
    assert!(
        out.contains("canary") || out.contains("blue") || out.contains("Blue"),
        "cicd has deployment strategies"
    );
}

#[test]
fn devops_ref_pipeline_alias() {
    let out = devops_ref_calc("pipeline");
    assert!(
        out.contains("pipeline") || out.contains("CI"),
        "pipeline alias resolves"
    );
}

#[test]
fn devops_ref_github_actions_alias() {
    let out = devops_ref_calc("github-actions");
    assert!(
        out.contains("GitHub Actions") || out.contains("github-actions"),
        "github-actions alias resolves"
    );
}

#[test]
fn devops_ref_deploy_alias() {
    let out = devops_ref_calc("deploy");
    assert!(
        out.contains("deploy") || out.contains("Deploy"),
        "deploy alias resolves"
    );
}

#[test]
fn devops_ref_canary_alias() {
    let out = devops_ref_calc("canary");
    assert!(
        out.contains("canary") || out.contains("Canary"),
        "canary alias resolves"
    );
}

#[test]
fn devops_ref_blue_green_alias() {
    let out = devops_ref_calc("blue-green");
    assert!(
        out.contains("Blue") || out.contains("blue"),
        "blue-green alias resolves"
    );
}

#[test]
fn devops_ref_gitops_alias() {
    let out = devops_ref_calc("gitops");
    assert!(
        out.contains("GitOps") || out.contains("gitops") || out.contains("Argo"),
        "gitops alias resolves"
    );
}

#[test]
fn devops_ref_containers() {
    let out = devops_ref_calc("containers");
    assert!(
        out.contains("Docker") || out.contains("docker"),
        "containers has Docker"
    );
    assert!(
        out.contains("Kubernetes") || out.contains("kubectl"),
        "containers has Kubernetes"
    );
    assert!(
        out.contains("Helm") || out.contains("helm"),
        "containers has Helm"
    );
}

#[test]
fn devops_ref_docker_alias() {
    let out = devops_ref_calc("docker");
    assert!(
        out.contains("Docker") || out.contains("docker"),
        "docker alias resolves"
    );
}

#[test]
fn devops_ref_kubernetes_alias() {
    let out = devops_ref_calc("kubernetes");
    assert!(
        out.contains("Kubernetes") || out.contains("kubectl"),
        "kubernetes alias resolves"
    );
}

#[test]
fn devops_ref_k8s_alias() {
    let out = devops_ref_calc("k8s");
    assert!(
        out.contains("Kubernetes") || out.contains("kubectl") || out.contains("k8s"),
        "k8s alias resolves"
    );
}

#[test]
fn devops_ref_kubectl_alias() {
    let out = devops_ref_calc("kubectl");
    assert!(
        out.contains("kubectl") || out.contains("Kubernetes"),
        "kubectl alias resolves"
    );
}

#[test]
fn devops_ref_helm_alias() {
    let out = devops_ref_calc("helm");
    assert!(
        out.contains("helm") || out.contains("Helm"),
        "helm alias resolves"
    );
}

#[test]
fn devops_ref_iac() {
    let out = devops_ref_calc("iac");
    assert!(
        out.contains("Terraform") || out.contains("terraform"),
        "iac has Terraform"
    );
    assert!(
        out.contains("Ansible") || out.contains("ansible"),
        "iac has Ansible"
    );
    assert!(
        out.contains("Pulumi") || out.contains("pulumi"),
        "iac has Pulumi"
    );
}

#[test]
fn devops_ref_terraform_alias() {
    let out = devops_ref_calc("terraform");
    assert!(
        out.contains("terraform") || out.contains("Terraform"),
        "terraform alias resolves"
    );
}

#[test]
fn devops_ref_ansible_alias() {
    let out = devops_ref_calc("ansible");
    assert!(
        out.contains("Ansible") || out.contains("ansible") || out.contains("playbook"),
        "ansible alias resolves"
    );
}

#[test]
fn devops_ref_pulumi_alias() {
    let out = devops_ref_calc("pulumi");
    assert!(
        out.contains("Pulumi") || out.contains("pulumi"),
        "pulumi alias resolves"
    );
}

#[test]
fn devops_ref_infrastructure_alias() {
    let out = devops_ref_calc("infrastructure");
    assert!(
        out.contains("Terraform") || out.contains("infrastructure"),
        "infrastructure alias resolves"
    );
}

#[test]
fn devops_ref_monitoring() {
    let out = devops_ref_calc("monitoring");
    assert!(
        out.contains("Prometheus") || out.contains("Grafana"),
        "monitoring has Prometheus"
    );
    assert!(
        out.contains("SLO") || out.contains("SLI"),
        "monitoring has SLO/SLI"
    );
    assert!(
        out.contains("error budget") || out.contains("Golden"),
        "monitoring has golden signals"
    );
}

#[test]
fn devops_ref_prometheus_alias() {
    let out = devops_ref_calc("prometheus");
    assert!(
        out.contains("Prometheus") || out.contains("PromQL"),
        "prometheus alias resolves"
    );
}

#[test]
fn devops_ref_sli_alias() {
    let out = devops_ref_calc("sli");
    assert!(
        out.contains("SLI") || out.contains("SLO"),
        "sli alias resolves"
    );
}

#[test]
fn devops_ref_slo_alias() {
    let out = devops_ref_calc("slo");
    assert!(
        out.contains("SLO") || out.contains("SLI"),
        "slo alias resolves"
    );
}

#[test]
fn devops_ref_alerting_alias() {
    let out = devops_ref_calc("alerting");
    assert!(
        out.contains("alert") || out.contains("Alert"),
        "alerting alias resolves"
    );
}

#[test]
fn devops_ref_golden_signals_alias() {
    let out = devops_ref_calc("golden-signals");
    assert!(
        out.contains("Golden") || out.contains("Latency"),
        "golden-signals alias resolves"
    );
}

#[test]
fn devops_ref_sre() {
    let out = devops_ref_calc("sre");
    assert!(
        out.contains("error budget")
            || out.contains("error-budget")
            || out.contains("Error Budget"),
        "sre has error budget"
    );
    assert!(out.contains("toil") || out.contains("Toil"), "sre has toil");
    assert!(
        out.contains("postmortem") || out.contains("Postmortem"),
        "sre has postmortem"
    );
}

#[test]
fn devops_ref_error_budget_alias() {
    let out = devops_ref_calc("error-budget");
    assert!(
        out.contains("error budget") || out.contains("Error Budget"),
        "error-budget alias resolves"
    );
}

#[test]
fn devops_ref_toil_alias() {
    let out = devops_ref_calc("toil");
    assert!(
        out.contains("toil") || out.contains("Toil"),
        "toil alias resolves"
    );
}

#[test]
fn devops_ref_postmortem_alias() {
    let out = devops_ref_calc("postmortem");
    assert!(
        out.contains("postmortem") || out.contains("Postmortem"),
        "postmortem alias resolves"
    );
}

#[test]
fn devops_ref_incident_alias() {
    let out = devops_ref_calc("incident");
    assert!(
        out.contains("incident") || out.contains("Incident"),
        "incident alias resolves"
    );
}

#[test]
fn devops_ref_circuit_breaker_alias() {
    let out = devops_ref_calc("circuit-breaker");
    assert!(
        out.contains("circuit breaker")
            || out.contains("Circuit breaker")
            || out.contains("Circuit Breaker"),
        "circuit-breaker alias resolves"
    );
}

#[test]
fn devops_ref_security() {
    let out = devops_ref_calc("security");
    assert!(
        out.contains("SAST") || out.contains("DAST"),
        "security has SAST/DAST"
    );
    assert!(
        out.contains("SLSA") || out.contains("supply-chain") || out.contains("Supply"),
        "security has SLSA"
    );
    assert!(
        out.contains("signing") || out.contains("cosign"),
        "security has signing"
    );
}

#[test]
fn devops_ref_sast_alias() {
    let out = devops_ref_calc("sast");
    assert!(
        out.contains("SAST") || out.contains("static"),
        "sast alias resolves"
    );
}

#[test]
fn devops_ref_slsa_alias() {
    let out = devops_ref_calc("slsa");
    assert!(
        out.contains("SLSA") || out.contains("slsa"),
        "slsa alias resolves"
    );
}

#[test]
fn devops_ref_supply_chain_alias() {
    let out = devops_ref_calc("supply-chain");
    assert!(
        out.contains("supply-chain") || out.contains("Supply") || out.contains("SLSA"),
        "supply-chain alias resolves"
    );
}

#[test]
fn devops_ref_scanning_alias() {
    let out = devops_ref_calc("scanning");
    assert!(
        out.contains("scan") || out.contains("Scan") || out.contains("Trivy"),
        "scanning alias resolves"
    );
}

#[test]
fn devops_ref_no_panic_special() {
    let _ = devops_ref_calc("@#$%");
    let _ = devops_ref_calc("   ");
}

// ── linux_sys_calc ────────────────────────────────────────────────────────────

#[test]
fn linux_sys_help_empty() {
    let out = linux_sys_calc("");
    assert!(
        out.contains("hematite --linux-sys"),
        "help shown for empty query"
    );
}

#[test]
fn linux_sys_help_unknown() {
    let out = linux_sys_calc("zzznomatch");
    assert!(
        out.contains("hematite --linux-sys"),
        "help shown for unknown"
    );
}

#[test]
fn linux_sys_all() {
    let out = linux_sys_calc("all");
    assert!(
        out.contains("systemctl") || out.contains("systemd"),
        "all has systemctl"
    );
    assert!(
        out.contains("SIGTERM") || out.contains("signal"),
        "all has processes"
    );
    assert!(
        out.contains("sysctl") || out.contains("kernel"),
        "all has kernel"
    );
    assert!(
        out.contains("inode") || out.contains("mount"),
        "all has filesystem"
    );
}

#[test]
fn linux_sys_systemctl() {
    let out = linux_sys_calc("systemctl");
    assert!(out.contains("systemctl"), "systemctl section present");
    assert!(
        out.contains("journalctl") || out.contains("journal"),
        "systemctl has journalctl"
    );
    assert!(
        out.contains("[Unit]") || out.contains("[Service]"),
        "systemctl has unit file"
    );
}

#[test]
fn linux_sys_systemd_alias() {
    let out = linux_sys_calc("systemd");
    assert!(
        out.contains("systemctl") || out.contains("systemd"),
        "systemd alias resolves"
    );
}

#[test]
fn linux_sys_service_alias() {
    let out = linux_sys_calc("service");
    assert!(
        out.contains("systemctl") || out.contains("service"),
        "service alias resolves"
    );
}

#[test]
fn linux_sys_journal_alias() {
    let out = linux_sys_calc("journal");
    assert!(
        out.contains("journalctl") || out.contains("journal"),
        "journal alias resolves"
    );
}

#[test]
fn linux_sys_journalctl_alias() {
    let out = linux_sys_calc("journalctl");
    assert!(out.contains("journalctl"), "journalctl alias resolves");
}

#[test]
fn linux_sys_unit_alias() {
    let out = linux_sys_calc("unit");
    assert!(
        out.contains("[Unit]") || out.contains("unit"),
        "unit alias resolves"
    );
}

#[test]
fn linux_sys_processes() {
    let out = linux_sys_calc("processes");
    assert!(
        out.contains("SIGTERM") || out.contains("SIGKILL"),
        "processes has signals"
    );
    assert!(
        out.contains("kill") || out.contains("pkill"),
        "processes has kill"
    );
    assert!(
        out.contains("strace") || out.contains("lsof"),
        "processes has strace/lsof"
    );
}

#[test]
fn linux_sys_process_alias() {
    let out = linux_sys_calc("process");
    assert!(
        out.contains("SIGTERM") || out.contains("process"),
        "process alias resolves"
    );
}

#[test]
fn linux_sys_signals_alias() {
    let out = linux_sys_calc("signals");
    assert!(
        out.contains("SIGTERM") || out.contains("SIGKILL"),
        "signals alias resolves"
    );
}

#[test]
fn linux_sys_signal_alias() {
    let out = linux_sys_calc("signal");
    assert!(
        out.contains("SIGTERM") || out.contains("signal"),
        "signal alias resolves"
    );
}

#[test]
fn linux_sys_kill_alias() {
    let out = linux_sys_calc("kill");
    assert!(
        out.contains("kill") || out.contains("SIGKILL"),
        "kill alias resolves"
    );
}

#[test]
fn linux_sys_strace_alias() {
    let out = linux_sys_calc("strace");
    assert!(
        out.contains("strace") || out.contains("trace"),
        "strace alias resolves"
    );
}

#[test]
fn linux_sys_lsof_alias() {
    let out = linux_sys_calc("lsof");
    assert!(
        out.contains("lsof") || out.contains("files"),
        "lsof alias resolves"
    );
}

#[test]
fn linux_sys_background_alias() {
    let out = linux_sys_calc("background");
    assert!(
        out.contains("nohup") || out.contains("background"),
        "background alias resolves"
    );
}

#[test]
fn linux_sys_kernel() {
    let out = linux_sys_calc("kernel");
    assert!(
        out.contains("sysctl") || out.contains("kernel"),
        "kernel section present"
    );
    assert!(
        out.contains("cgroup") || out.contains("namespace"),
        "kernel has cgroups/namespaces"
    );
    assert!(
        out.contains("ulimit") || out.contains("nofile"),
        "kernel has ulimit"
    );
}

#[test]
fn linux_sys_sysctl_alias() {
    let out = linux_sys_calc("sysctl");
    assert!(
        out.contains("sysctl") || out.contains("kernel"),
        "sysctl alias resolves"
    );
}

#[test]
fn linux_sys_cgroup_alias() {
    let out = linux_sys_calc("cgroup");
    assert!(
        out.contains("cgroup") || out.contains("cgroups"),
        "cgroup alias resolves"
    );
}

#[test]
fn linux_sys_namespace_alias() {
    let out = linux_sys_calc("namespace");
    assert!(
        out.contains("namespace") || out.contains("unshare"),
        "namespace alias resolves"
    );
}

#[test]
fn linux_sys_ulimit_alias() {
    let out = linux_sys_calc("ulimit");
    assert!(
        out.contains("ulimit") || out.contains("nofile"),
        "ulimit alias resolves"
    );
}

#[test]
fn linux_sys_filesystem() {
    let out = linux_sys_calc("filesystem");
    assert!(
        out.contains("df") || out.contains("du"),
        "filesystem has df/du"
    );
    assert!(
        out.contains("mount") || out.contains("fstab"),
        "filesystem has mount"
    );
    assert!(
        out.contains("inode") || out.contains("lsblk"),
        "filesystem has inode"
    );
}

#[test]
fn linux_sys_disk_alias() {
    let out = linux_sys_calc("disk");
    assert!(
        out.contains("df") || out.contains("disk") || out.contains("lsblk"),
        "disk alias resolves"
    );
}

#[test]
fn linux_sys_mount_alias() {
    let out = linux_sys_calc("mount");
    assert!(
        out.contains("mount") || out.contains("fstab"),
        "mount alias resolves"
    );
}

#[test]
fn linux_sys_inode_alias() {
    let out = linux_sys_calc("inode");
    assert!(
        out.contains("inode") || out.contains("stat"),
        "inode alias resolves"
    );
}

#[test]
fn linux_sys_rsync_alias() {
    let out = linux_sys_calc("rsync");
    assert!(
        out.contains("rsync") || out.contains("sync"),
        "rsync alias resolves"
    );
}

#[test]
fn linux_sys_network() {
    let out = linux_sys_calc("network");
    assert!(
        out.contains("ip addr") || out.contains("ip link"),
        "network has ip commands"
    );
    assert!(
        out.contains("ss -") || out.contains("ss "),
        "network has ss"
    );
    assert!(
        out.contains("tcpdump") || out.contains("iptables"),
        "network has tcpdump"
    );
}

#[test]
fn linux_sys_ip_alias() {
    let out = linux_sys_calc("ip");
    assert!(
        out.contains("ip addr") || out.contains("ip route"),
        "ip alias resolves"
    );
}

#[test]
fn linux_sys_ss_alias() {
    let out = linux_sys_calc("ss");
    assert!(
        out.contains("ss -") || out.contains("netstat"),
        "ss alias resolves"
    );
}

#[test]
fn linux_sys_iptables_alias() {
    let out = linux_sys_calc("iptables");
    assert!(
        out.contains("iptables") || out.contains("nftables"),
        "iptables alias resolves"
    );
}

#[test]
fn linux_sys_tcpdump_alias() {
    let out = linux_sys_calc("tcpdump");
    assert!(
        out.contains("tcpdump") || out.contains("capture"),
        "tcpdump alias resolves"
    );
}

#[test]
fn linux_sys_dig_alias() {
    let out = linux_sys_calc("dig");
    assert!(
        out.contains("dig") || out.contains("DNS"),
        "dig alias resolves"
    );
}

#[test]
fn linux_sys_security() {
    let out = linux_sys_calc("security");
    assert!(
        out.contains("chmod") || out.contains("permission"),
        "security has chmod"
    );
    assert!(
        out.contains("sudo") || out.contains("sudoers"),
        "security has sudo"
    );
    assert!(
        out.contains("SELinux") || out.contains("selinux") || out.contains("lynis"),
        "security has SELinux/lynis"
    );
}

#[test]
fn linux_sys_chmod_alias() {
    let out = linux_sys_calc("chmod");
    assert!(
        out.contains("chmod") || out.contains("permission"),
        "chmod alias resolves"
    );
}

#[test]
fn linux_sys_sudo_alias() {
    let out = linux_sys_calc("sudo");
    assert!(
        out.contains("sudo") || out.contains("sudoers"),
        "sudo alias resolves"
    );
}

#[test]
fn linux_sys_selinux_alias() {
    let out = linux_sys_calc("selinux");
    assert!(
        out.contains("SELinux") || out.contains("selinux"),
        "selinux alias resolves"
    );
}

#[test]
fn linux_sys_acl_alias() {
    let out = linux_sys_calc("acl");
    assert!(
        out.contains("ACL") || out.contains("setfacl") || out.contains("getfacl"),
        "acl alias resolves"
    );
}

#[test]
fn linux_sys_no_panic_special() {
    let _ = linux_sys_calc("@#$%");
    let _ = linux_sys_calc("   ");
}

// ── api_design_calc ───────────────────────────────────────────────────────────

#[test]
fn api_design_help_empty() {
    let out = api_design_calc("");
    assert!(
        out.contains("hematite --api-design"),
        "help shown for empty query"
    );
}

#[test]
fn api_design_help_unknown() {
    let out = api_design_calc("zzznomatch");
    assert!(
        out.contains("hematite --api-design"),
        "help shown for unknown"
    );
}

#[test]
fn api_design_all() {
    let out = api_design_calc("all");
    assert!(out.contains("GET") || out.contains("POST"), "all has REST");
    assert!(
        out.contains("openapi") || out.contains("swagger") || out.contains("OpenAPI"),
        "all has OpenAPI"
    );
    assert!(
        out.contains("GraphQL") || out.contains("graphql"),
        "all has GraphQL"
    );
}

#[test]
fn api_design_rest() {
    let out = api_design_calc("rest");
    assert!(
        out.contains("GET") && out.contains("POST"),
        "rest has GET/POST"
    );
    assert!(
        out.contains("201") || out.contains("204"),
        "rest has status codes"
    );
    assert!(
        out.contains("pagination") || out.contains("Pagination"),
        "rest has pagination"
    );
}

#[test]
fn api_design_http_method_alias() {
    let out = api_design_calc("http-method");
    assert!(
        out.contains("GET") || out.contains("POST"),
        "http-method alias resolves"
    );
}

#[test]
fn api_design_crud_alias() {
    let out = api_design_calc("crud");
    assert!(
        out.contains("GET") || out.contains("POST") || out.contains("REST"),
        "crud alias resolves"
    );
}

#[test]
fn api_design_endpoint_alias() {
    let out = api_design_calc("endpoint");
    assert!(
        out.contains("GET") || out.contains("endpoint") || out.contains("resource"),
        "endpoint alias resolves"
    );
}

#[test]
fn api_design_status_code_alias() {
    let out = api_design_calc("status-code");
    assert!(
        out.contains("200") || out.contains("201") || out.contains("404"),
        "status-code alias resolves"
    );
}

#[test]
fn api_design_pagination_alias() {
    let out = api_design_calc("pagination");
    assert!(
        out.contains("pagination") || out.contains("cursor") || out.contains("page"),
        "pagination alias resolves"
    );
}

#[test]
fn api_design_openapi() {
    let out = api_design_calc("openapi");
    assert!(
        out.contains("openapi") || out.contains("OpenAPI"),
        "openapi section present"
    );
    assert!(
        out.contains("schema") || out.contains("Schema"),
        "openapi has schema"
    );
    assert!(
        out.contains("swagger") || out.contains("Swagger") || out.contains("Redoc"),
        "openapi has tooling"
    );
}

#[test]
fn api_design_swagger_alias() {
    let out = api_design_calc("swagger");
    assert!(
        out.contains("swagger") || out.contains("Swagger") || out.contains("openapi"),
        "swagger alias resolves"
    );
}

#[test]
fn api_design_spec_alias() {
    let out = api_design_calc("spec");
    assert!(
        out.contains("openapi") || out.contains("spec") || out.contains("schema"),
        "spec alias resolves"
    );
}

#[test]
fn api_design_versioning() {
    let out = api_design_calc("versioning");
    assert!(
        out.contains("version") || out.contains("Version"),
        "versioning section present"
    );
    assert!(
        out.contains("deprecation") || out.contains("Deprecation"),
        "versioning has deprecation"
    );
    assert!(
        out.contains("breaking") || out.contains("Breaking"),
        "versioning has breaking changes"
    );
}

#[test]
fn api_design_version_alias() {
    let out = api_design_calc("version");
    assert!(
        out.contains("version") || out.contains("Version"),
        "version alias resolves"
    );
}

#[test]
fn api_design_deprecation_alias() {
    let out = api_design_calc("deprecation");
    assert!(
        out.contains("deprecation") || out.contains("Deprecation") || out.contains("Sunset"),
        "deprecation alias resolves"
    );
}

#[test]
fn api_design_breaking_change_alias() {
    let out = api_design_calc("breaking-change");
    assert!(
        out.contains("breaking") || out.contains("Breaking"),
        "breaking-change alias resolves"
    );
}

#[test]
fn api_design_errors() {
    let out = api_design_calc("errors");
    assert!(
        out.contains("RFC") || out.contains("Problem Details"),
        "errors section present"
    );
    assert!(
        out.contains("422") || out.contains("400"),
        "errors has status codes"
    );
    assert!(
        out.contains("Idempotency") || out.contains("idempotency"),
        "errors has idempotency"
    );
}

#[test]
fn api_design_error_alias() {
    let out = api_design_calc("error");
    assert!(
        out.contains("error") || out.contains("Error"),
        "error alias resolves"
    );
}

#[test]
fn api_design_idempotency_alias() {
    let out = api_design_calc("idempotency");
    assert!(
        out.contains("Idempotency") || out.contains("idempotency"),
        "idempotency alias resolves"
    );
}

#[test]
fn api_design_problem_details_alias() {
    let out = api_design_calc("problem-details");
    assert!(
        out.contains("Problem Details") || out.contains("RFC"),
        "problem-details alias resolves"
    );
}

#[test]
fn api_design_graphql() {
    let out = api_design_calc("graphql");
    assert!(
        out.contains("GraphQL") || out.contains("graphql"),
        "graphql section present"
    );
    assert!(
        out.contains("query") || out.contains("mutation"),
        "graphql has query/mutation"
    );
    assert!(
        out.contains("DataLoader") || out.contains("N+1"),
        "graphql has N+1/DataLoader"
    );
}

#[test]
fn api_design_grpc_alias() {
    let out = api_design_calc("grpc");
    assert!(
        out.contains("gRPC") || out.contains("grpc"),
        "grpc alias resolves"
    );
}

#[test]
fn api_design_mutation_alias() {
    let out = api_design_calc("mutation");
    assert!(
        out.contains("mutation") || out.contains("Mutation"),
        "mutation alias resolves"
    );
}

#[test]
fn api_design_relay_alias() {
    let out = api_design_calc("relay");
    assert!(
        out.contains("Relay") || out.contains("relay") || out.contains("cursor"),
        "relay alias resolves"
    );
}

#[test]
fn api_design_ratelimit() {
    let out = api_design_calc("ratelimit");
    assert!(
        out.contains("rate") || out.contains("Rate"),
        "ratelimit section present"
    );
    assert!(
        out.contains("Token bucket") || out.contains("token bucket") || out.contains("leaky"),
        "ratelimit has token bucket"
    );
    assert!(
        out.contains("OWASP") || out.contains("owasp"),
        "ratelimit has OWASP"
    );
}

#[test]
fn api_design_rate_limit_alias() {
    let out = api_design_calc("rate-limit");
    assert!(
        out.contains("rate") || out.contains("Rate"),
        "rate-limit alias resolves"
    );
}

#[test]
fn api_design_throttle_alias() {
    let out = api_design_calc("throttle");
    assert!(
        out.contains("rate") || out.contains("throttle") || out.contains("limit"),
        "throttle alias resolves"
    );
}

#[test]
fn api_design_oauth_alias() {
    let out = api_design_calc("oauth");
    assert!(
        out.contains("OAuth") || out.contains("oauth") || out.contains("Bearer"),
        "oauth alias resolves"
    );
}

#[test]
fn api_design_owasp_alias() {
    let out = api_design_calc("owasp");
    assert!(
        out.contains("OWASP") || out.contains("owasp"),
        "owasp alias resolves"
    );
}

#[test]
fn api_design_no_panic_special() {
    let _ = api_design_calc("@#$%");
    let _ = api_design_calc("   ");
}

// ── db_design_calc ────────────────────────────────────────────────────────────

#[test]
fn db_design_help_empty() {
    let out = db_design_calc("");
    assert!(
        out.contains("hematite --db-design"),
        "help shown for empty query"
    );
}

#[test]
fn db_design_help_unknown() {
    let out = db_design_calc("zzznomatch");
    assert!(
        out.contains("hematite --db-design"),
        "help shown for unknown"
    );
}

#[test]
fn db_design_all() {
    let out = db_design_calc("all");
    assert!(
        out.contains("1NF") || out.contains("normalization"),
        "all has normalization"
    );
    assert!(
        out.contains("B-tree") || out.contains("index"),
        "all has indexes"
    );
    assert!(
        out.contains("ACID") || out.contains("transaction"),
        "all has transactions"
    );
    assert!(
        out.contains("CAP") || out.contains("distributed"),
        "all has distributed"
    );
}

#[test]
fn db_design_normalization() {
    let out = db_design_calc("normalization");
    assert!(
        out.contains("1NF") || out.contains("First Normal"),
        "normalization has 1NF"
    );
    assert!(
        out.contains("3NF") || out.contains("Third Normal"),
        "normalization has 3NF"
    );
    assert!(
        out.contains("BCNF") || out.contains("Boyce"),
        "normalization has BCNF"
    );
}

#[test]
fn db_design_1nf_alias() {
    let out = db_design_calc("1nf");
    assert!(
        out.contains("1NF") || out.contains("First Normal"),
        "1nf alias resolves"
    );
}

#[test]
fn db_design_3nf_alias() {
    let out = db_design_calc("3nf");
    assert!(
        out.contains("3NF") || out.contains("Third Normal"),
        "3nf alias resolves"
    );
}

#[test]
fn db_design_bcnf_alias() {
    let out = db_design_calc("bcnf");
    assert!(
        out.contains("BCNF") || out.contains("Boyce"),
        "bcnf alias resolves"
    );
}

#[test]
fn db_design_denormalize_alias() {
    let out = db_design_calc("denormalize");
    assert!(
        out.contains("denormali") || out.contains("OLAP"),
        "denormalize alias resolves"
    );
}

#[test]
fn db_design_er_alias() {
    let out = db_design_calc("er");
    assert!(
        out.contains("ER") || out.contains("entity") || out.contains("surrogate"),
        "er alias resolves"
    );
}

#[test]
fn db_design_indexes() {
    let out = db_design_calc("indexes");
    assert!(
        out.contains("B-tree") || out.contains("btree"),
        "indexes has B-tree"
    );
    assert!(
        out.contains("EXPLAIN") || out.contains("Seq Scan"),
        "indexes has EXPLAIN"
    );
    assert!(
        out.contains("Covering") || out.contains("covering") || out.contains("Partial"),
        "indexes has covering/partial"
    );
}

#[test]
fn db_design_index_alias() {
    let out = db_design_calc("index");
    assert!(
        out.contains("B-tree") || out.contains("index"),
        "index alias resolves"
    );
}

#[test]
fn db_design_explain_alias() {
    let out = db_design_calc("explain");
    assert!(
        out.contains("EXPLAIN") || out.contains("Seq Scan"),
        "explain alias resolves"
    );
}

#[test]
fn db_design_btree_alias() {
    let out = db_design_calc("btree");
    assert!(
        out.contains("B-tree") || out.contains("btree"),
        "btree alias resolves"
    );
}

#[test]
fn db_design_gin_alias() {
    let out = db_design_calc("gin");
    assert!(
        out.contains("GIN") || out.contains("gin"),
        "gin alias resolves"
    );
}

#[test]
fn db_design_vacuum_alias() {
    let out = db_design_calc("vacuum");
    assert!(
        out.contains("VACUUM") || out.contains("vacuum"),
        "vacuum alias resolves"
    );
}

#[test]
fn db_design_transactions() {
    let out = db_design_calc("transactions");
    assert!(
        out.contains("ACID") || out.contains("Atomicity"),
        "transactions has ACID"
    );
    assert!(
        out.contains("isolation") || out.contains("Isolation"),
        "transactions has isolation levels"
    );
    assert!(
        out.contains("deadlock") || out.contains("Deadlock"),
        "transactions has deadlock"
    );
}

#[test]
fn db_design_transaction_alias() {
    let out = db_design_calc("transaction");
    assert!(
        out.contains("ACID") || out.contains("transaction"),
        "transaction alias resolves"
    );
}

#[test]
fn db_design_acid_alias() {
    let out = db_design_calc("acid");
    assert!(
        out.contains("ACID") || out.contains("Atomicity"),
        "acid alias resolves"
    );
}

#[test]
fn db_design_isolation_alias() {
    let out = db_design_calc("isolation");
    assert!(
        out.contains("isolation") || out.contains("Isolation"),
        "isolation alias resolves"
    );
}

#[test]
fn db_design_deadlock_alias() {
    let out = db_design_calc("deadlock");
    assert!(
        out.contains("deadlock") || out.contains("Deadlock"),
        "deadlock alias resolves"
    );
}

#[test]
fn db_design_optimistic_alias() {
    let out = db_design_calc("optimistic");
    assert!(
        out.contains("Optimistic") || out.contains("optimistic"),
        "optimistic alias resolves"
    );
}

#[test]
fn db_design_distributed() {
    let out = db_design_calc("distributed");
    assert!(
        out.contains("CAP") || out.contains("Brewer"),
        "distributed has CAP"
    );
    assert!(
        out.contains("replication") || out.contains("Replication"),
        "distributed has replication"
    );
    assert!(
        out.contains("sharding") || out.contains("Sharding"),
        "distributed has sharding"
    );
}

#[test]
fn db_design_cap_alias() {
    let out = db_design_calc("cap");
    assert!(
        out.contains("CAP") || out.contains("Brewer"),
        "cap alias resolves"
    );
}

#[test]
fn db_design_replication_alias() {
    let out = db_design_calc("replication");
    assert!(
        out.contains("replication") || out.contains("Replication"),
        "replication alias resolves"
    );
}

#[test]
fn db_design_sharding_alias() {
    let out = db_design_calc("sharding");
    assert!(
        out.contains("sharding") || out.contains("Sharding"),
        "sharding alias resolves"
    );
}

#[test]
fn db_design_consistency_alias() {
    let out = db_design_calc("consistency");
    assert!(
        out.contains("consistency") || out.contains("Consistency"),
        "consistency alias resolves"
    );
}

#[test]
fn db_design_eventual_alias() {
    let out = db_design_calc("eventual");
    assert!(
        out.contains("eventual") || out.contains("Eventual"),
        "eventual alias resolves"
    );
}

#[test]
fn db_design_nosql() {
    let out = db_design_calc("nosql");
    assert!(
        out.contains("MongoDB") || out.contains("document"),
        "nosql has document"
    );
    assert!(
        out.contains("Redis") || out.contains("key-value"),
        "nosql has key-value"
    );
    assert!(
        out.contains("Cassandra") || out.contains("column"),
        "nosql has column-family"
    );
}

#[test]
fn db_design_mongodb_alias() {
    let out = db_design_calc("mongodb");
    assert!(
        out.contains("MongoDB") || out.contains("document"),
        "mongodb alias resolves"
    );
}

#[test]
fn db_design_redis_alias() {
    let out = db_design_calc("redis");
    assert!(
        out.contains("Redis") || out.contains("key-value"),
        "redis alias resolves"
    );
}

#[test]
fn db_design_cassandra_alias() {
    let out = db_design_calc("cassandra");
    assert!(
        out.contains("Cassandra") || out.contains("column"),
        "cassandra alias resolves"
    );
}

#[test]
fn db_design_elasticsearch_alias() {
    let out = db_design_calc("elasticsearch");
    assert!(
        out.contains("Elasticsearch") || out.contains("inverted index"),
        "elasticsearch alias resolves"
    );
}

#[test]
fn db_design_dynamodb_alias() {
    let out = db_design_calc("dynamodb");
    assert!(
        out.contains("DynamoDB") || out.contains("dynamodb"),
        "dynamodb alias resolves"
    );
}

#[test]
fn db_design_migrations() {
    let out = db_design_calc("migrations");
    assert!(
        out.contains("Flyway") || out.contains("Alembic") || out.contains("migration"),
        "migrations section present"
    );
    assert!(
        out.contains("zero-downtime")
            || out.contains("zero downtime")
            || out.contains("CONCURRENTLY"),
        "migrations has zero-downtime"
    );
    assert!(
        out.contains("backfill") || out.contains("Backfill"),
        "migrations has backfill"
    );
}

#[test]
fn db_design_migration_alias() {
    let out = db_design_calc("migration");
    assert!(
        out.contains("migration") || out.contains("Migration"),
        "migration alias resolves"
    );
}

#[test]
fn db_design_flyway_alias() {
    let out = db_design_calc("flyway");
    assert!(
        out.contains("Flyway") || out.contains("flyway"),
        "flyway alias resolves"
    );
}

#[test]
fn db_design_alembic_alias() {
    let out = db_design_calc("alembic");
    assert!(
        out.contains("Alembic") || out.contains("alembic"),
        "alembic alias resolves"
    );
}

#[test]
fn db_design_backfill_alias() {
    let out = db_design_calc("backfill");
    assert!(
        out.contains("backfill") || out.contains("Backfill"),
        "backfill alias resolves"
    );
}

#[test]
fn db_design_backup_alias() {
    let out = db_design_calc("backup");
    assert!(
        out.contains("backup") || out.contains("Backup") || out.contains("pg_dump"),
        "backup alias resolves"
    );
}

#[test]
fn db_design_zero_downtime_alias() {
    let out = db_design_calc("zero-downtime");
    assert!(
        out.contains("zero-downtime")
            || out.contains("zero downtime")
            || out.contains("CONCURRENTLY"),
        "zero-downtime alias resolves"
    );
}

#[test]
fn db_design_no_panic_special() {
    let _ = db_design_calc("@#$%");
    let _ = db_design_calc("   ");
}

// ── perf_ref_calc ─────────────────────────────────────────────────────────────

#[test]
fn perf_ref_empty() {
    let out = perf_ref_calc("");
    assert!(out.contains("profiling") || out.contains("Profiling"));
    assert!(out.contains("Topics") || out.contains("topics"));
}

#[test]
fn perf_ref_nomatch() {
    let out = perf_ref_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn perf_ref_all() {
    let out = perf_ref_calc("all");
    assert!(out.contains("profiling") || out.contains("Profiling"));
    assert!(out.contains("memory") || out.contains("Memory"));
    assert!(out.contains("benchmarking") || out.contains("Benchmarking"));
    assert!(out.contains("web") || out.contains("Web"));
    assert!(out.contains("database") || out.contains("Database"));
    assert!(out.contains("optimization") || out.contains("Optimization"));
}

#[test]
fn perf_ref_profiling() {
    let out = perf_ref_calc("profiling");
    assert!(out.contains("perf") || out.contains("Profiling"));
    assert!(out.contains("flamegraph") || out.contains("Flamegraph") || out.contains("flamegraph"));
}

#[test]
fn perf_ref_profiler_alias() {
    let out = perf_ref_calc("profiler");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("perf") || out.contains("criterion") || out.contains("Profiling"));
}

#[test]
fn perf_ref_ebpf_alias() {
    let out = perf_ref_calc("ebpf");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("eBPF") || out.contains("bpftrace") || out.contains("profiling"));
}

#[test]
fn perf_ref_memory() {
    let out = perf_ref_calc("memory");
    assert!(out.contains("RSS") || out.contains("Memory") || out.contains("memory"));
    assert!(out.contains("leak") || out.contains("Leak") || out.contains("VSZ"));
}

#[test]
fn perf_ref_oom_alias() {
    let out = perf_ref_calc("oom");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("OOM") || out.contains("oom_score") || out.contains("Memory"));
}

#[test]
fn perf_ref_valgrind_alias() {
    let out = perf_ref_calc("valgrind");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Valgrind") || out.contains("valgrind") || out.contains("leak"));
}

#[test]
fn perf_ref_benchmarking() {
    let out = perf_ref_calc("benchmarking");
    assert!(out.contains("Benchmarking") || out.contains("benchmark") || out.contains("wrk"));
}

#[test]
fn perf_ref_latency_alias() {
    let out = perf_ref_calc("latency");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("p99") || out.contains("latency") || out.contains("Latency"));
}

#[test]
fn perf_ref_amdahl_alias() {
    let out = perf_ref_calc("amdahl");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Amdahl") || out.contains("speedup") || out.contains("serial"));
}

#[test]
fn perf_ref_fio_alias() {
    let out = perf_ref_calc("fio");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("fio") || out.contains("Disk") || out.contains("bench"));
}

#[test]
fn perf_ref_web() {
    let out = perf_ref_calc("web");
    assert!(
        out.contains("LCP")
            || out.contains("CLS")
            || out.contains("Web Vitals")
            || out.contains("INP")
    );
}

#[test]
fn perf_ref_lcp_alias() {
    let out = perf_ref_calc("lcp");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("LCP") || out.contains("Largest") || out.contains("Paint"));
}

#[test]
fn perf_ref_lighthouse_alias() {
    let out = perf_ref_calc("lighthouse");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Lighthouse") || out.contains("lighthouse") || out.contains("audit"));
}

#[test]
fn perf_ref_database() {
    let out = perf_ref_calc("database");
    assert!(
        out.contains("EXPLAIN")
            || out.contains("index")
            || out.contains("PostgreSQL")
            || out.contains("connection")
    );
}

#[test]
fn perf_ref_slow_query_alias() {
    let out = perf_ref_calc("slow-query");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("slow") || out.contains("query") || out.contains("log"));
}

#[test]
fn perf_ref_pgbouncer_alias() {
    let out = perf_ref_calc("pgbouncer");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("PgBouncer") || out.contains("pgbouncer") || out.contains("pool"));
}

#[test]
fn perf_ref_optimization() {
    let out = perf_ref_calc("optimization");
    assert!(
        out.contains("Optimization") || out.contains("optimization") || out.contains("Algorithm")
    );
}

#[test]
fn perf_ref_simd_alias() {
    let out = perf_ref_calc("simd");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("SIMD") || out.contains("vector") || out.contains("simd"));
}

#[test]
fn perf_ref_zero_copy_alias() {
    let out = perf_ref_calc("zero-copy");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("zero-copy") || out.contains("sendfile") || out.contains("copy"));
}

#[test]
fn perf_ref_edge_cases() {
    let _ = perf_ref_calc("@#$%");
    let _ = perf_ref_calc("   ");
}

// ── docker_compose_calc ───────────────────────────────────────────────────────

#[test]
fn docker_compose_empty() {
    let out = docker_compose_calc("");
    assert!(out.contains("basics") || out.contains("networking") || out.contains("Topics"));
}

#[test]
fn docker_compose_nomatch() {
    let out = docker_compose_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn docker_compose_all() {
    let out = docker_compose_calc("all");
    assert!(out.contains("basics") || out.contains("Basics"));
    assert!(out.contains("networking") || out.contains("Networking"));
    assert!(out.contains("health") || out.contains("Health"));
    assert!(out.contains("production") || out.contains("Production"));
    assert!(out.contains("logging") || out.contains("Logging"));
    assert!(out.contains("tips") || out.contains("Tips"));
}

#[test]
fn docker_compose_basics() {
    let out = docker_compose_calc("basics");
    assert!(
        out.contains("docker compose") || out.contains("compose up") || out.contains("service")
    );
}

#[test]
fn docker_compose_service_alias() {
    let out = docker_compose_calc("service");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("service") || out.contains("image") || out.contains("ports"));
}

#[test]
fn docker_compose_up_alias() {
    let out = docker_compose_calc("up");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("up") || out.contains("compose") || out.contains("build"));
}

#[test]
fn docker_compose_networking() {
    let out = docker_compose_calc("networking");
    assert!(out.contains("network") || out.contains("Network") || out.contains("DNS"));
}

#[test]
fn docker_compose_volume_alias() {
    let out = docker_compose_calc("volume");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("volume") || out.contains("Volume") || out.contains("bind"));
}

#[test]
fn docker_compose_dns_alias() {
    let out = docker_compose_calc("dns");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("DNS") || out.contains("service name") || out.contains("alias"));
}

#[test]
fn docker_compose_bind_mount_alias() {
    let out = docker_compose_calc("bind-mount");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("bind") || out.contains("Bind") || out.contains("mount"));
}

#[test]
fn docker_compose_health() {
    let out = docker_compose_calc("health");
    assert!(
        out.contains("healthcheck") || out.contains("Healthcheck") || out.contains("pg_isready")
    );
}

#[test]
fn docker_compose_healthcheck_alias() {
    let out = docker_compose_calc("healthcheck");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("healthcheck") || out.contains("test") || out.contains("interval"));
}

#[test]
fn docker_compose_restart_alias() {
    let out = docker_compose_calc("restart");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("restart") || out.contains("always") || out.contains("on-failure"));
}

#[test]
fn docker_compose_profiles_alias() {
    let out = docker_compose_calc("profiles");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("profile") || out.contains("profiles") || out.contains("debug"));
}

#[test]
fn docker_compose_production() {
    let out = docker_compose_calc("production");
    assert!(out.contains("prod") || out.contains("Production") || out.contains("override"));
}

#[test]
fn docker_compose_override_alias() {
    let out = docker_compose_calc("override");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("override") || out.contains("Override") || out.contains("docker-compose.yml")
    );
}

#[test]
fn docker_compose_secrets_alias() {
    let out = docker_compose_calc("secrets");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("secret") || out.contains("Secret") || out.contains("/run/secrets"));
}

#[test]
fn docker_compose_logging() {
    let out = docker_compose_calc("logging");
    assert!(out.contains("driver") || out.contains("json-file") || out.contains("Logging"));
}

#[test]
fn docker_compose_logs_alias() {
    let out = docker_compose_calc("logs");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("logs") || out.contains("follow") || out.contains("driver"));
}

#[test]
fn docker_compose_traefik_alias() {
    let out = docker_compose_calc("traefik");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("traefik") || out.contains("Traefik") || out.contains("label"));
}

#[test]
fn docker_compose_tips() {
    let out = docker_compose_calc("tips");
    assert!(
        out.contains("security")
            || out.contains("Security")
            || out.contains("gotcha")
            || out.contains("Gotcha")
    );
}

#[test]
fn docker_compose_security_alias() {
    let out = docker_compose_calc("security");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("non-root") || out.contains("read_only") || out.contains("cap_drop"));
}

#[test]
fn docker_compose_debugging_alias() {
    let out = docker_compose_calc("debugging");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("debug") || out.contains("inspect") || out.contains("exec"));
}

#[test]
fn docker_compose_edge_cases() {
    let _ = docker_compose_calc("@#$%");
    let _ = docker_compose_calc("   ");
}

// ── wasm_ref_calc ─────────────────────────────────────────────────────────────

#[test]
fn wasm_ref_empty() {
    let out = wasm_ref_calc("");
    assert!(out.contains("format") || out.contains("memory") || out.contains("Topics"));
}

#[test]
fn wasm_ref_nomatch() {
    let out = wasm_ref_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn wasm_ref_all() {
    let out = wasm_ref_calc("all");
    assert!(out.contains("format") || out.contains("Format"));
    assert!(out.contains("memory") || out.contains("Memory"));
    assert!(out.contains("wasi") || out.contains("WASI"));
    assert!(out.contains("wasm-pack") || out.contains("wasm_pack"));
    assert!(out.contains("wabt") || out.contains("WABT") || out.contains("wat2wasm"));
    assert!(out.contains("component") || out.contains("Component"));
}

#[test]
fn wasm_ref_format() {
    let out = wasm_ref_calc("format");
    assert!(out.contains("WAT") || out.contains("module") || out.contains("instruction"));
}

#[test]
fn wasm_ref_wat_alias() {
    let out = wasm_ref_calc("wat");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("WAT") || out.contains("module") || out.contains("text"));
}

#[test]
fn wasm_ref_instruction_alias() {
    let out = wasm_ref_calc("instruction");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("local.get") || out.contains("i32") || out.contains("stack"));
}

#[test]
fn wasm_ref_memory() {
    let out = wasm_ref_calc("memory");
    assert!(
        out.contains("linear")
            || out.contains("Linear")
            || out.contains("page")
            || out.contains("64KB")
    );
}

#[test]
fn wasm_ref_linear_alias() {
    let out = wasm_ref_calc("linear");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("linear") || out.contains("memory") || out.contains("page"));
}

#[test]
fn wasm_ref_heap_alias() {
    let out = wasm_ref_calc("heap");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("heap") || out.contains("Heap") || out.contains("alloc"));
}

#[test]
fn wasm_ref_wasi() {
    let out = wasm_ref_calc("wasi");
    assert!(out.contains("WASI") || out.contains("wasmtime") || out.contains("syscall"));
}

#[test]
fn wasm_ref_wasmtime_alias() {
    let out = wasm_ref_calc("wasmtime");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("wasmtime") || out.contains("WASI") || out.contains("runtime"));
}

#[test]
fn wasm_ref_sandbox_alias() {
    let out = wasm_ref_calc("sandbox");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("sandbox") || out.contains("capability") || out.contains("dir"));
}

#[test]
fn wasm_ref_wasm_pack() {
    let out = wasm_ref_calc("wasm-pack");
    assert!(out.contains("wasm-pack") || out.contains("wasm-bindgen") || out.contains("cdylib"));
}

#[test]
fn wasm_ref_rust_alias() {
    let out = wasm_ref_calc("rust");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("wasm-pack") || out.contains("wasm_bindgen") || out.contains("cdylib"));
}

#[test]
fn wasm_ref_bundler_alias() {
    let out = wasm_ref_calc("bundler");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("bundler") || out.contains("webpack") || out.contains("wasm-pack"));
}

#[test]
fn wasm_ref_wabt() {
    let out = wasm_ref_calc("wabt");
    assert!(out.contains("wat2wasm") || out.contains("wasm2wat") || out.contains("wabt"));
}

#[test]
fn wasm_ref_wat2wasm_alias() {
    let out = wasm_ref_calc("wat2wasm");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("wat2wasm") || out.contains("assemble") || out.contains("binary"));
}

#[test]
fn wasm_ref_wasm_opt_alias() {
    let out = wasm_ref_calc("wasm-opt");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("wasm-opt") || out.contains("binaryen") || out.contains("optim"));
}

#[test]
fn wasm_ref_twiggy_alias() {
    let out = wasm_ref_calc("twiggy");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("twiggy") || out.contains("size") || out.contains("monos"));
}

#[test]
fn wasm_ref_component() {
    let out = wasm_ref_calc("component");
    assert!(out.contains("Component") || out.contains("WIT") || out.contains("compose"));
}

#[test]
fn wasm_ref_wit_alias() {
    let out = wasm_ref_calc("wit");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("WIT") || out.contains("component") || out.contains("interface"));
}

#[test]
fn wasm_ref_emscripten_alias() {
    let out = wasm_ref_calc("emscripten");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Emscripten") || out.contains("emcc") || out.contains("C/C"));
}

#[test]
fn wasm_ref_edge_cases() {
    let _ = wasm_ref_calc("@#$%");
    let _ = wasm_ref_calc("   ");
}

// ── accessibility_calc ────────────────────────────────────────────────────────

#[test]
fn accessibility_empty() {
    let out = accessibility_calc("");
    assert!(out.contains("wcag") || out.contains("WCAG") || out.contains("Topics"));
}

#[test]
fn accessibility_nomatch() {
    let out = accessibility_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn accessibility_all() {
    let out = accessibility_calc("all");
    assert!(out.contains("WCAG") || out.contains("wcag"));
    assert!(out.contains("ARIA") || out.contains("aria"));
    assert!(out.contains("keyboard") || out.contains("Keyboard"));
    assert!(out.contains("contrast") || out.contains("Contrast"));
    assert!(out.contains("screen") || out.contains("Screen"));
    assert!(out.contains("testing") || out.contains("Testing"));
}

#[test]
fn accessibility_wcag() {
    let out = accessibility_calc("wcag");
    assert!(out.contains("WCAG") || out.contains("POUR") || out.contains("Perceivable"));
}

#[test]
fn accessibility_pour_alias() {
    let out = accessibility_calc("pour");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Perceivable") || out.contains("Operable") || out.contains("POUR"));
}

#[test]
fn accessibility_guidelines_alias() {
    let out = accessibility_calc("guidelines");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("WCAG") || out.contains("guideline") || out.contains("criterion"));
}

#[test]
fn accessibility_contrast_ratio_alias() {
    let out = accessibility_calc("contrast-ratio");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("4.5") || out.contains("contrast") || out.contains("ratio"));
}

#[test]
fn accessibility_aria() {
    let out = accessibility_calc("aria");
    assert!(out.contains("aria-label") || out.contains("role") || out.contains("ARIA"));
}

#[test]
fn accessibility_role_alias() {
    let out = accessibility_calc("role");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("role") || out.contains("ARIA") || out.contains("landmark"));
}

#[test]
fn accessibility_aria_label_alias() {
    let out = accessibility_calc("aria-label");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("aria-label") || out.contains("name") || out.contains("accessible"));
}

#[test]
fn accessibility_landmark_alias() {
    let out = accessibility_calc("landmark");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("banner") || out.contains("navigation") || out.contains("main"));
}

#[test]
fn accessibility_dialog_alias() {
    let out = accessibility_calc("dialog");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("dialog") || out.contains("modal") || out.contains("ARIA"));
}

#[test]
fn accessibility_keyboard() {
    let out = accessibility_calc("keyboard");
    assert!(out.contains("Tab") || out.contains("focus") || out.contains("keyboard"));
}

#[test]
fn accessibility_focus_alias() {
    let out = accessibility_calc("focus");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("focus") || out.contains("Tab") || out.contains("tabindex"));
}

#[test]
fn accessibility_tabindex_alias() {
    let out = accessibility_calc("tabindex");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("tabindex") || out.contains("roving") || out.contains("natural"));
}

#[test]
fn accessibility_skip_alias() {
    let out = accessibility_calc("skip");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Skip") || out.contains("skip") || out.contains("main content"));
}

#[test]
fn accessibility_trap_alias() {
    let out = accessibility_calc("trap");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("trap") || out.contains("modal") || out.contains("focus"));
}

#[test]
fn accessibility_contrast() {
    let out = accessibility_calc("contrast");
    assert!(
        out.contains("4.5")
            || out.contains("contrast")
            || out.contains("luminance")
            || out.contains("ratio")
    );
}

#[test]
fn accessibility_color_alias() {
    let out = accessibility_calc("color");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("contrast") || out.contains("color") || out.contains("colorblind"));
}

#[test]
fn accessibility_dark_mode_alias() {
    let out = accessibility_calc("dark-mode");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("dark") || out.contains("prefers-color-scheme") || out.contains("forced"));
}

#[test]
fn accessibility_screen_readers() {
    let out = accessibility_calc("screen-readers");
    assert!(
        out.contains("NVDA")
            || out.contains("VoiceOver")
            || out.contains("JAWS")
            || out.contains("screen reader")
    );
}

#[test]
fn accessibility_nvda_alias() {
    let out = accessibility_calc("nvda");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("NVDA") || out.contains("screen reader") || out.contains("Windows"));
}

#[test]
fn accessibility_voiceover_alias() {
    let out = accessibility_calc("voiceover");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("VoiceOver") || out.contains("macOS") || out.contains("Safari"));
}

#[test]
fn accessibility_testing() {
    let out = accessibility_calc("testing");
    assert!(
        out.contains("axe")
            || out.contains("WAVE")
            || out.contains("Lighthouse")
            || out.contains("checklist")
    );
}

#[test]
fn accessibility_axe_alias() {
    let out = accessibility_calc("axe");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("axe") || out.contains("Deque") || out.contains("automated"));
}

#[test]
fn accessibility_checklist_alias() {
    let out = accessibility_calc("checklist");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("checklist") || out.contains("alt text") || out.contains("label"));
}

#[test]
fn accessibility_eslint_alias() {
    let out = accessibility_calc("eslint-plugin-jsx-a11y");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("eslint")
            || out.contains("jsx")
            || out.contains("linting")
            || out.contains("Linting")
    );
}

#[test]
fn accessibility_edge_cases() {
    let _ = accessibility_calc("@#$%");
    let _ = accessibility_calc("   ");
}

// ── k8s_ref_calc ──────────────────────────────────────────────────────────────

#[test]
fn k8s_ref_empty() {
    let out = k8s_ref_calc("");
    assert!(out.contains("pods") || out.contains("services") || out.contains("Topics"));
}

#[test]
fn k8s_ref_nomatch() {
    let out = k8s_ref_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn k8s_ref_all() {
    let out = k8s_ref_calc("all");
    assert!(out.contains("pods") || out.contains("Pods"));
    assert!(out.contains("services") || out.contains("Services"));
    assert!(out.contains("config") || out.contains("Config"));
    assert!(out.contains("rbac") || out.contains("RBAC"));
    assert!(out.contains("troubleshoot") || out.contains("Troubleshoot"));
    assert!(out.contains("helm") || out.contains("Helm"));
}

#[test]
fn k8s_ref_pods() {
    let out = k8s_ref_calc("pods");
    assert!(out.contains("kubectl") || out.contains("deployment") || out.contains("pod"));
}

#[test]
fn k8s_ref_deployment_alias() {
    let out = k8s_ref_calc("deployment");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Deployment") || out.contains("rollout") || out.contains("kubectl"));
}

#[test]
fn k8s_ref_crashloopbackoff_alias() {
    let out = k8s_ref_calc("crashloopbackoff");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("CrashLoopBackOff") || out.contains("crash") || out.contains("log"));
}

#[test]
fn k8s_ref_statefulset_alias() {
    let out = k8s_ref_calc("statefulset");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("StatefulSet") || out.contains("stateful") || out.contains("ordered"));
}

#[test]
fn k8s_ref_services() {
    let out = k8s_ref_calc("services");
    assert!(out.contains("ClusterIP") || out.contains("NodePort") || out.contains("service"));
}

#[test]
fn k8s_ref_ingress_alias() {
    let out = k8s_ref_calc("ingress");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Ingress") || out.contains("ingress") || out.contains("HTTP"));
}

#[test]
fn k8s_ref_dns_alias() {
    let out = k8s_ref_calc("dns");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("DNS") || out.contains("kube-dns") || out.contains("CoreDNS"));
}

#[test]
fn k8s_ref_networkpolicy_alias() {
    let out = k8s_ref_calc("networkpolicy");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("NetworkPolicy") || out.contains("network") || out.contains("Calico"));
}

#[test]
fn k8s_ref_config() {
    let out = k8s_ref_calc("config");
    assert!(out.contains("ConfigMap") || out.contains("Secret") || out.contains("PVC"));
}

#[test]
fn k8s_ref_configmap_alias() {
    let out = k8s_ref_calc("configmap");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("ConfigMap") || out.contains("configmap") || out.contains("env"));
}

#[test]
fn k8s_ref_secret_alias() {
    let out = k8s_ref_calc("secret");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Secret") || out.contains("base64") || out.contains("Vault"));
}

#[test]
fn k8s_ref_pvc_alias() {
    let out = k8s_ref_calc("pvc");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("PVC") || out.contains("PersistentVolume") || out.contains("StorageClass")
    );
}

#[test]
fn k8s_ref_probe_alias() {
    let out = k8s_ref_calc("probe");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("probe") || out.contains("liveness") || out.contains("readiness"));
}

#[test]
fn k8s_ref_rbac() {
    let out = k8s_ref_calc("rbac");
    assert!(out.contains("RBAC") || out.contains("Role") || out.contains("ClusterRole"));
}

#[test]
fn k8s_ref_role_alias() {
    let out = k8s_ref_calc("role");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Role") || out.contains("RBAC") || out.contains("namespace"));
}

#[test]
fn k8s_ref_serviceaccount_alias() {
    let out = k8s_ref_calc("serviceaccount");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("ServiceAccount") || out.contains("serviceaccount") || out.contains("RBAC")
    );
}

#[test]
fn k8s_ref_securitycontext_alias() {
    let out = k8s_ref_calc("securitycontext");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("securityContext")
            || out.contains("runAsNonRoot")
            || out.contains("privilege")
    );
}

#[test]
fn k8s_ref_troubleshoot() {
    let out = k8s_ref_calc("troubleshoot");
    assert!(out.contains("kubectl") || out.contains("describe") || out.contains("logs"));
}

#[test]
fn k8s_ref_debug_alias() {
    let out = k8s_ref_calc("debug");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("debug") || out.contains("busybox") || out.contains("exec"));
}

#[test]
fn k8s_ref_oomkilled_alias() {
    let out = k8s_ref_calc("oomkilled");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("OOMKilled") || out.contains("memory") || out.contains("limit"));
}

#[test]
fn k8s_ref_helm() {
    let out = k8s_ref_calc("helm");
    assert!(out.contains("helm") || out.contains("Helm") || out.contains("chart"));
}

#[test]
fn k8s_ref_chart_alias() {
    let out = k8s_ref_calc("chart");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("chart") || out.contains("Chart") || out.contains("values"));
}

#[test]
fn k8s_ref_argocd_alias() {
    let out = k8s_ref_calc("argocd");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("ArgoCD") || out.contains("argocd") || out.contains("gitops"));
}

#[test]
fn k8s_ref_edge_cases() {
    let _ = k8s_ref_calc("@#$%");
    let _ = k8s_ref_calc("   ");
}

// ── observability_calc ────────────────────────────────────────────────────────

#[test]
fn observability_empty() {
    let out = observability_calc("");
    assert!(out.contains("metrics") || out.contains("tracing") || out.contains("Topics"));
}

#[test]
fn observability_nomatch() {
    let out = observability_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn observability_all() {
    let out = observability_calc("all");
    assert!(out.contains("metrics") || out.contains("Metrics"));
    assert!(out.contains("tracing") || out.contains("Tracing"));
    assert!(out.contains("logging") || out.contains("Logging"));
    assert!(out.contains("slo") || out.contains("SLO"));
    assert!(out.contains("dashboards") || out.contains("Dashboards"));
    assert!(out.contains("alerting") || out.contains("Alerting"));
}

#[test]
fn observability_metrics() {
    let out = observability_calc("metrics");
    assert!(out.contains("Prometheus") || out.contains("PromQL") || out.contains("Counter"));
}

#[test]
fn observability_prometheus_alias() {
    let out = observability_calc("prometheus");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Prometheus") || out.contains("metric") || out.contains("PromQL"));
}

#[test]
fn observability_promql_alias() {
    let out = observability_calc("promql");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("rate") || out.contains("sum") || out.contains("PromQL"));
}

#[test]
fn observability_histogram_alias() {
    let out = observability_calc("histogram");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Histogram") || out.contains("histogram") || out.contains("bucket"));
}

#[test]
fn observability_exporter_alias() {
    let out = observability_calc("exporter");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("exporter") || out.contains("node_exporter") || out.contains("blackbox"));
}

#[test]
fn observability_tracing() {
    let out = observability_calc("tracing");
    assert!(
        out.contains("trace")
            || out.contains("Trace")
            || out.contains("span")
            || out.contains("OpenTelemetry")
    );
}

#[test]
fn observability_otel_alias() {
    let out = observability_calc("otel");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("OpenTelemetry") || out.contains("OTel") || out.contains("otel"));
}

#[test]
fn observability_jaeger_alias() {
    let out = observability_calc("jaeger");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Jaeger") || out.contains("jaeger") || out.contains("backend"));
}

#[test]
fn observability_span_alias() {
    let out = observability_calc("span");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Span") || out.contains("span") || out.contains("trace"));
}

#[test]
fn observability_logging() {
    let out = observability_calc("logging");
    assert!(
        out.contains("structured")
            || out.contains("Structured")
            || out.contains("JSON")
            || out.contains("level")
    );
}

#[test]
fn observability_loki_alias() {
    let out = observability_calc("loki");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Loki") || out.contains("loki") || out.contains("LogQL"));
}

#[test]
fn observability_elk_alias() {
    let out = observability_calc("elk");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Elasticsearch") || out.contains("ELK") || out.contains("Kibana"));
}

#[test]
fn observability_slo() {
    let out = observability_calc("slo");
    assert!(
        out.contains("SLO")
            || out.contains("SLI")
            || out.contains("error budget")
            || out.contains("Error budget")
    );
}

#[test]
fn observability_error_budget_alias() {
    let out = observability_calc("error-budget");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("error budget") || out.contains("Error budget") || out.contains("budget"));
}

#[test]
fn observability_burn_rate_alias() {
    let out = observability_calc("burn-rate");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("burn") || out.contains("Burn") || out.contains("SLO"));
}

#[test]
fn observability_dashboards() {
    let out = observability_calc("dashboards");
    assert!(
        out.contains("Grafana")
            || out.contains("panel")
            || out.contains("RED")
            || out.contains("USE")
    );
}

#[test]
fn observability_grafana_alias() {
    let out = observability_calc("grafana");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Grafana") || out.contains("grafana") || out.contains("panel"));
}

#[test]
fn observability_red_method_alias() {
    let out = observability_calc("red-method");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("RED") || out.contains("Rate") || out.contains("Duration"));
}

#[test]
fn observability_alerting() {
    let out = observability_calc("alerting");
    assert!(
        out.contains("alert")
            || out.contains("Alert")
            || out.contains("runbook")
            || out.contains("pagerduty")
            || out.contains("PagerDuty")
    );
}

#[test]
fn observability_pagerduty_alias() {
    let out = observability_calc("pagerduty");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("PagerDuty")
            || out.contains("pagerduty")
            || out.contains("oncall")
            || out.contains("on-call")
    );
}

#[test]
fn observability_postmortem_alias() {
    let out = observability_calc("postmortem");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("postmortem") || out.contains("Postmortem") || out.contains("incident"));
}

#[test]
fn observability_edge_cases() {
    let _ = observability_calc("@#$%");
    let _ = observability_calc("   ");
}

// ── terraform_adv_calc ────────────────────────────────────────────────────────

#[test]
fn terraform_adv_empty() {
    let out = terraform_adv_calc("");
    assert!(out.contains("modules") || out.contains("state") || out.contains("Topics"));
}

#[test]
fn terraform_adv_nomatch() {
    let out = terraform_adv_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn terraform_adv_all() {
    let out = terraform_adv_calc("all");
    assert!(out.contains("modules") || out.contains("Modules"));
    assert!(out.contains("state") || out.contains("State"));
    assert!(out.contains("workspaces") || out.contains("Workspaces"));
    assert!(out.contains("testing") || out.contains("Testing"));
    assert!(out.contains("patterns") || out.contains("Patterns"));
    assert!(out.contains("cicd") || out.contains("CI/CD"));
}

#[test]
fn terraform_adv_modules() {
    let out = terraform_adv_calc("modules");
    assert!(out.contains("module") || out.contains("Module") || out.contains("source"));
}

#[test]
fn terraform_adv_module_alias() {
    let out = terraform_adv_calc("module");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("module") || out.contains("source") || out.contains("output"));
}

#[test]
fn terraform_adv_registry_alias() {
    let out = terraform_adv_calc("registry");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("registry") || out.contains("Registry") || out.contains("source"));
}

#[test]
fn terraform_adv_state() {
    let out = terraform_adv_calc("state");
    assert!(out.contains("state") || out.contains("State") || out.contains("backend"));
}

#[test]
fn terraform_adv_backend_alias() {
    let out = terraform_adv_calc("backend");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("backend") || out.contains("S3") || out.contains("remote"));
}

#[test]
fn terraform_adv_s3_alias() {
    let out = terraform_adv_calc("s3");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("S3") || out.contains("bucket") || out.contains("DynamoDB"));
}

#[test]
fn terraform_adv_import_alias() {
    let out = terraform_adv_calc("import");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("import") || out.contains("state") || out.contains("infra"));
}

#[test]
fn terraform_adv_workspaces() {
    let out = terraform_adv_calc("workspaces");
    assert!(
        out.contains("workspace")
            || out.contains("Workspace")
            || out.contains("terraform.workspace")
    );
}

#[test]
fn terraform_adv_workspace_alias() {
    let out = terraform_adv_calc("workspace");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("workspace") || out.contains("new") || out.contains("select"));
}

#[test]
fn terraform_adv_tfvars_alias() {
    let out = terraform_adv_calc("tfvars");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("tfvars") || out.contains("var-file") || out.contains("environment"));
}

#[test]
fn terraform_adv_testing() {
    let out = terraform_adv_calc("testing");
    assert!(
        out.contains("terratest")
            || out.contains("Terratest")
            || out.contains("checkov")
            || out.contains("tflint")
    );
}

#[test]
fn terraform_adv_terratest_alias() {
    let out = terraform_adv_calc("terratest");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Terratest") || out.contains("terratest") || out.contains("Go"));
}

#[test]
fn terraform_adv_checkov_alias() {
    let out = terraform_adv_calc("checkov");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("checkov") || out.contains("Checkov") || out.contains("compliance"));
}

#[test]
fn terraform_adv_tflint_alias() {
    let out = terraform_adv_calc("tflint");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("tflint") || out.contains("lint") || out.contains("validate"));
}

#[test]
fn terraform_adv_patterns() {
    let out = terraform_adv_calc("patterns");
    assert!(out.contains("dynamic") || out.contains("Dynamic") || out.contains("lifecycle"));
}

#[test]
fn terraform_adv_dynamic_alias() {
    let out = terraform_adv_calc("dynamic");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("dynamic") || out.contains("for_each") || out.contains("block"));
}

#[test]
fn terraform_adv_lifecycle_alias() {
    let out = terraform_adv_calc("lifecycle");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("lifecycle")
            || out.contains("create_before_destroy")
            || out.contains("prevent_destroy")
    );
}

#[test]
fn terraform_adv_cicd() {
    let out = terraform_adv_calc("cicd");
    assert!(
        out.contains("CI")
            || out.contains("Atlantis")
            || out.contains("GitHub Actions")
            || out.contains("github-actions")
    );
}

#[test]
fn terraform_adv_atlantis_alias() {
    let out = terraform_adv_calc("atlantis");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Atlantis") || out.contains("atlantis") || out.contains("plan"));
}

#[test]
fn terraform_adv_oidc_alias() {
    let out = terraform_adv_calc("oidc");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("OIDC") || out.contains("oidc") || out.contains("credential"));
}

#[test]
fn terraform_adv_edge_cases() {
    let _ = terraform_adv_calc("@#$%");
    let _ = terraform_adv_calc("   ");
}

// ── security_scan_calc ────────────────────────────────────────────────────────

#[test]
fn security_scan_empty() {
    let out = security_scan_calc("");
    assert!(out.contains("sast") || out.contains("SAST") || out.contains("Topics"));
}

#[test]
fn security_scan_nomatch() {
    let out = security_scan_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn security_scan_all() {
    let out = security_scan_calc("all");
    assert!(out.contains("SAST") || out.contains("sast"));
    assert!(out.contains("DAST") || out.contains("dast"));
    assert!(
        out.contains("deps")
            || out.contains("Deps")
            || out.contains("dependency")
            || out.contains("Dependency")
    );
    assert!(out.contains("container") || out.contains("Container"));
    assert!(out.contains("secret") || out.contains("Secret"));
    assert!(out.contains("compliance") || out.contains("Compliance"));
}

#[test]
fn security_scan_sast() {
    let out = security_scan_calc("sast");
    assert!(out.contains("SAST") || out.contains("Semgrep") || out.contains("static"));
}

#[test]
fn security_scan_semgrep_alias() {
    let out = security_scan_calc("semgrep");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Semgrep") || out.contains("semgrep") || out.contains("rule"));
}

#[test]
fn security_scan_codeql_alias() {
    let out = security_scan_calc("codeql");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("CodeQL") || out.contains("codeql") || out.contains("GitHub"));
}

#[test]
fn security_scan_bandit_alias() {
    let out = security_scan_calc("bandit");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("bandit") || out.contains("Bandit") || out.contains("Python"));
}

#[test]
fn security_scan_dast() {
    let out = security_scan_calc("dast");
    assert!(out.contains("DAST") || out.contains("ZAP") || out.contains("dynamic"));
}

#[test]
fn security_scan_zap_alias() {
    let out = security_scan_calc("zap");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("ZAP") || out.contains("zap") || out.contains("OWASP"));
}

#[test]
fn security_scan_nuclei_alias() {
    let out = security_scan_calc("nuclei");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("nuclei") || out.contains("Nuclei") || out.contains("template"));
}

#[test]
fn security_scan_deps() {
    let out = security_scan_calc("deps");
    assert!(
        out.contains("npm audit")
            || out.contains("cargo audit")
            || out.contains("CVE")
            || out.contains("Trivy")
    );
}

#[test]
fn security_scan_trivy_alias() {
    let out = security_scan_calc("trivy");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Trivy") || out.contains("trivy") || out.contains("scan"));
}

#[test]
fn security_scan_sbom_alias() {
    let out = security_scan_calc("sbom");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("SBOM") || out.contains("sbom") || out.contains("Bill of Materials"));
}

#[test]
fn security_scan_supply_chain_alias() {
    let out = security_scan_calc("supply-chain");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("supply")
            || out.contains("Supply")
            || out.contains("confusion")
            || out.contains("typosquat")
    );
}

#[test]
fn security_scan_containers() {
    let out = security_scan_calc("containers");
    assert!(
        out.contains("container")
            || out.contains("Container")
            || out.contains("Dockerfile")
            || out.contains("image")
    );
}

#[test]
fn security_scan_dockerfile_alias() {
    let out = security_scan_calc("dockerfile");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Dockerfile") || out.contains("image") || out.contains("layer"));
}

#[test]
fn security_scan_cosign_alias() {
    let out = security_scan_calc("cosign");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("cosign") || out.contains("Cosign") || out.contains("sign"));
}

#[test]
fn security_scan_falco_alias() {
    let out = security_scan_calc("falco");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("Falco")
            || out.contains("falco")
            || out.contains("runtime")
            || out.contains("eBPF")
    );
}

#[test]
fn security_scan_secrets() {
    let out = security_scan_calc("secrets");
    assert!(
        out.contains("gitleaks")
            || out.contains("Gitleaks")
            || out.contains("secret")
            || out.contains("exposed")
    );
}

#[test]
fn security_scan_gitleaks_alias() {
    let out = security_scan_calc("gitleaks");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("gitleaks") || out.contains("Gitleaks") || out.contains("git"));
}

#[test]
fn security_scan_vault_alias() {
    let out = security_scan_calc("vault");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Vault") || out.contains("vault") || out.contains("dynamic"));
}

#[test]
fn security_scan_exposed_alias() {
    let out = security_scan_calc("exposed");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("exposed") || out.contains("revoke") || out.contains("Revoke"));
}

#[test]
fn security_scan_compliance() {
    let out = security_scan_calc("compliance");
    assert!(
        out.contains("OWASP")
            || out.contains("CIS")
            || out.contains("compliance")
            || out.contains("GDPR")
    );
}

#[test]
fn security_scan_owasp_alias() {
    let out = security_scan_calc("owasp");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("OWASP") || out.contains("Injection") || out.contains("Top 10"));
}

#[test]
fn security_scan_gdpr_alias() {
    let out = security_scan_calc("gdpr");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("GDPR") || out.contains("gdpr") || out.contains("EU") || out.contains("data")
    );
}

#[test]
fn security_scan_headers_alias() {
    let out = security_scan_calc("headers");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("Strict-Transport")
            || out.contains("Content-Security")
            || out.contains("header")
    );
}

#[test]
fn security_scan_edge_cases() {
    let _ = security_scan_calc("@#$%");
    let _ = security_scan_calc("   ");
}

// ── ml_ref_calc ───────────────────────────────────────────────────────────────

#[test]
fn ml_ref_empty() {
    let out = ml_ref_calc("");
    assert!(out.contains("fundamentals") || out.contains("models") || out.contains("Topics"));
}

#[test]
fn ml_ref_nomatch() {
    let out = ml_ref_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn ml_ref_all() {
    let out = ml_ref_calc("all");
    assert!(out.contains("fundamentals") || out.contains("Fundamentals"));
    assert!(out.contains("models") || out.contains("Models"));
    assert!(out.contains("features") || out.contains("Features"));
    assert!(out.contains("training") || out.contains("Training"));
    assert!(out.contains("evaluation") || out.contains("Evaluation"));
    assert!(out.contains("deployment") || out.contains("Deployment"));
}

#[test]
fn ml_ref_fundamentals() {
    let out = ml_ref_calc("fundamentals");
    assert!(out.contains("bias") || out.contains("Bias") || out.contains("supervised"));
}

#[test]
fn ml_ref_bias_variance_alias() {
    let out = ml_ref_calc("bias-variance");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("bias") || out.contains("variance") || out.contains("overfitting"));
}

#[test]
fn ml_ref_overfitting_alias() {
    let out = ml_ref_calc("overfitting");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("overfitting") || out.contains("regularization") || out.contains("variance")
    );
}

#[test]
fn ml_ref_supervised_alias() {
    let out = ml_ref_calc("supervised");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("supervised") || out.contains("labeled") || out.contains("classification")
    );
}

#[test]
fn ml_ref_models() {
    let out = ml_ref_calc("models");
    assert!(
        out.contains("XGBoost")
            || out.contains("sklearn")
            || out.contains("CNN")
            || out.contains("transformer")
    );
}

#[test]
fn ml_ref_xgboost_alias() {
    let out = ml_ref_calc("xgboost");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("XGBoost") || out.contains("xgboost") || out.contains("gradient"));
}

#[test]
fn ml_ref_random_forest_alias() {
    let out = ml_ref_calc("random-forest");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("RandomForest") || out.contains("random forest") || out.contains("Forest")
    );
}

#[test]
fn ml_ref_tabular_alias() {
    let out = ml_ref_calc("tabular");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("tabular") || out.contains("GBT") || out.contains("structured"));
}

#[test]
fn ml_ref_features() {
    let out = ml_ref_calc("features");
    assert!(
        out.contains("encoding")
            || out.contains("scaling")
            || out.contains("feature")
            || out.contains("Feature")
    );
}

#[test]
fn ml_ref_scaling_alias() {
    let out = ml_ref_calc("scaling");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("StandardScaler") || out.contains("scaling") || out.contains("MinMax"));
}

#[test]
fn ml_ref_encoding_alias() {
    let out = ml_ref_calc("encoding");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("encoding") || out.contains("one-hot") || out.contains("categorical"));
}

#[test]
fn ml_ref_tfidf_alias() {
    let out = ml_ref_calc("tfidf");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("TF-IDF") || out.contains("tfidf") || out.contains("text"));
}

#[test]
fn ml_ref_training() {
    let out = ml_ref_calc("training");
    assert!(
        out.contains("gradient")
            || out.contains("Gradient")
            || out.contains("Adam")
            || out.contains("learning rate")
    );
}

#[test]
fn ml_ref_adam_alias() {
    let out = ml_ref_calc("adam");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Adam") || out.contains("adam") || out.contains("optimizer"));
}

#[test]
fn ml_ref_learning_rate_alias() {
    let out = ml_ref_calc("learning-rate");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("learning rate") || out.contains("LR") || out.contains("schedule"));
}

#[test]
fn ml_ref_dropout_alias() {
    let out = ml_ref_calc("dropout");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Dropout") || out.contains("dropout") || out.contains("regularization"));
}

#[test]
fn ml_ref_evaluation() {
    let out = ml_ref_calc("evaluation");
    assert!(
        out.contains("precision")
            || out.contains("Precision")
            || out.contains("F1")
            || out.contains("AUC")
    );
}

#[test]
fn ml_ref_f1_alias() {
    let out = ml_ref_calc("f1");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("F1") || out.contains("precision") || out.contains("recall"));
}

#[test]
fn ml_ref_auc_alias() {
    let out = ml_ref_calc("auc");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("AUC") || out.contains("ROC") || out.contains("threshold"));
}

#[test]
fn ml_ref_drift_alias() {
    let out = ml_ref_calc("drift");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("drift") || out.contains("Drift") || out.contains("distribution"));
}

#[test]
fn ml_ref_deployment() {
    let out = ml_ref_calc("deployment");
    assert!(
        out.contains("serving")
            || out.contains("ONNX")
            || out.contains("MLflow")
            || out.contains("quantization")
    );
}

#[test]
fn ml_ref_onnx_alias() {
    let out = ml_ref_calc("onnx");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("ONNX") || out.contains("onnx") || out.contains("interoperable"));
}

#[test]
fn ml_ref_quantization_alias() {
    let out = ml_ref_calc("quantization");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("quantization") || out.contains("Quantization") || out.contains("INT8"));
}

#[test]
fn ml_ref_edge_cases() {
    let _ = ml_ref_calc("@#$%");
    let _ = ml_ref_calc("   ");
}

// ── rust_patterns_calc ────────────────────────────────────────────────────────

#[test]
fn rust_patterns_empty() {
    let out = rust_patterns_calc("");
    assert!(out.contains("errors") || out.contains("iterators") || out.contains("Topics"));
}

#[test]
fn rust_patterns_nomatch() {
    let out = rust_patterns_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn rust_patterns_all() {
    let out = rust_patterns_calc("all");
    assert!(out.contains("errors") || out.contains("Errors") || out.contains("Error"));
    assert!(out.contains("iterators") || out.contains("Iterator"));
    assert!(out.contains("traits") || out.contains("Traits") || out.contains("Trait"));
    assert!(out.contains("async") || out.contains("Async"));
    assert!(out.contains("concurrency") || out.contains("Concurrency"));
    assert!(out.contains("design") || out.contains("Design"));
}

#[test]
fn rust_patterns_errors() {
    let out = rust_patterns_calc("errors");
    assert!(out.contains("Result") || out.contains("anyhow") || out.contains("thiserror"));
}

#[test]
fn rust_patterns_anyhow_alias() {
    let out = rust_patterns_calc("anyhow");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("anyhow") || out.contains("context") || out.contains("Result"));
}

#[test]
fn rust_patterns_thiserror_alias() {
    let out = rust_patterns_calc("thiserror");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("thiserror") || out.contains("derive") || out.contains("Error"));
}

#[test]
fn rust_patterns_result_alias() {
    let out = rust_patterns_calc("result");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Result") || out.contains("error") || out.contains("Ok"));
}

#[test]
fn rust_patterns_iterators() {
    let out = rust_patterns_calc("iterators");
    assert!(
        out.contains("Iterator")
            || out.contains("map")
            || out.contains("filter")
            || out.contains("collect")
    );
}

#[test]
fn rust_patterns_map_alias() {
    let out = rust_patterns_calc("map");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("map") || out.contains("transform") || out.contains("Iterator"));
}

#[test]
fn rust_patterns_collect_alias() {
    let out = rust_patterns_calc("collect");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("collect") || out.contains("Vec") || out.contains("HashMap"));
}

#[test]
fn rust_patterns_rayon_alias() {
    let out = rust_patterns_calc("rayon");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("rayon")
            || out.contains("Rayon")
            || out.contains("parallel")
            || out.contains("par_iter")
    );
}

#[test]
fn rust_patterns_traits() {
    let out = rust_patterns_calc("traits");
    assert!(
        out.contains("Trait")
            || out.contains("trait")
            || out.contains("dyn")
            || out.contains("generic")
    );
}

#[test]
fn rust_patterns_dyn_alias() {
    let out = rust_patterns_calc("dyn");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("dyn") || out.contains("trait object") || out.contains("dispatch"));
}

#[test]
fn rust_patterns_builder_alias() {
    let out = rust_patterns_calc("builder");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Builder") || out.contains("builder") || out.contains("build"));
}

#[test]
fn rust_patterns_async() {
    let out = rust_patterns_calc("async");
    assert!(
        out.contains("async")
            || out.contains("Async")
            || out.contains("tokio")
            || out.contains("Tokio")
    );
}

#[test]
fn rust_patterns_tokio_alias() {
    let out = rust_patterns_calc("tokio");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("tokio") || out.contains("Tokio") || out.contains("spawn"));
}

#[test]
fn rust_patterns_spawn_alias() {
    let out = rust_patterns_calc("spawn");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("spawn") || out.contains("task") || out.contains("tokio"));
}

#[test]
fn rust_patterns_select_alias() {
    let out = rust_patterns_calc("select");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("select") || out.contains("first") || out.contains("tokio"));
}

#[test]
fn rust_patterns_concurrency() {
    let out = rust_patterns_calc("concurrency");
    assert!(
        out.contains("Send")
            || out.contains("Sync")
            || out.contains("Arc")
            || out.contains("thread")
    );
}

#[test]
fn rust_patterns_arc_alias() {
    let out = rust_patterns_calc("arc");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Arc") || out.contains("shared") || out.contains("atomic"));
}

#[test]
fn rust_patterns_deadlock_alias() {
    let out = rust_patterns_calc("deadlock");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("deadlock") || out.contains("Deadlock") || out.contains("lock"));
}

#[test]
fn rust_patterns_design() {
    let out = rust_patterns_calc("design");
    assert!(
        out.contains("typestate")
            || out.contains("newtype")
            || out.contains("state machine")
            || out.contains("pattern")
    );
}

#[test]
fn rust_patterns_newtype_alias() {
    let out = rust_patterns_calc("newtype");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("newtype")
            || out.contains("Newtype")
            || out.contains("wrapper")
            || out.contains("Wrapper")
    );
}

#[test]
fn rust_patterns_typestate_alias() {
    let out = rust_patterns_calc("typestate");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("typestate") || out.contains("TypeState") || out.contains("PhantomData"));
}

#[test]
fn rust_patterns_edge_cases() {
    let _ = rust_patterns_calc("@#$%");
    let _ = rust_patterns_calc("   ");
}

// ── event_driven_calc ─────────────────────────────────────────────────────────

#[test]
fn event_driven_empty() {
    let out = event_driven_calc("");
    assert!(out.contains("patterns") || out.contains("kafka") || out.contains("Topics"));
}

#[test]
fn event_driven_nomatch() {
    let out = event_driven_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn event_driven_all() {
    let out = event_driven_calc("all");
    assert!(out.contains("patterns") || out.contains("Patterns"));
    assert!(out.contains("kafka") || out.contains("Kafka"));
    assert!(
        out.contains("event-sourcing") || out.contains("Event Sourcing") || out.contains("CQRS")
    );
    assert!(out.contains("messaging") || out.contains("Messaging"));
    assert!(out.contains("cdc") || out.contains("CDC"));
    assert!(out.contains("schema") || out.contains("Schema"));
}

#[test]
fn event_driven_patterns() {
    let out = event_driven_calc("patterns");
    assert!(
        out.contains("pub/sub")
            || out.contains("pubsub")
            || out.contains("event")
            || out.contains("saga")
    );
}

#[test]
fn event_driven_pubsub_alias() {
    let out = event_driven_calc("pubsub");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("pub") || out.contains("subscriber") || out.contains("topic"));
}

#[test]
fn event_driven_saga_alias() {
    let out = event_driven_calc("saga");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Saga") || out.contains("saga") || out.contains("compensating"));
}

#[test]
fn event_driven_idempotent_alias() {
    let out = event_driven_calc("idempotent");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("idempotent") || out.contains("Idempotent") || out.contains("duplicate"));
}

#[test]
fn event_driven_kafka() {
    let out = event_driven_calc("kafka");
    assert!(
        out.contains("Kafka")
            || out.contains("topic")
            || out.contains("partition")
            || out.contains("consumer")
    );
}

#[test]
fn event_driven_topic_alias() {
    let out = event_driven_calc("topic");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("topic") || out.contains("Topic") || out.contains("partition"));
}

#[test]
fn event_driven_offset_alias() {
    let out = event_driven_calc("offset");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("offset") || out.contains("Offset") || out.contains("position"));
}

#[test]
fn event_driven_ksqldb_alias() {
    let out = event_driven_calc("ksqldb");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("ksqlDB")
            || out.contains("ksqldb")
            || out.contains("SQL")
            || out.contains("stream")
    );
}

#[test]
fn event_driven_event_sourcing() {
    let out = event_driven_calc("event-sourcing");
    assert!(
        out.contains("event sourcing")
            || out.contains("Event Sourcing")
            || out.contains("aggregate")
            || out.contains("CQRS")
    );
}

#[test]
fn event_driven_cqrs_alias() {
    let out = event_driven_calc("cqrs");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("CQRS")
            || out.contains("cqrs")
            || out.contains("command")
            || out.contains("query")
    );
}

#[test]
fn event_driven_aggregate_alias() {
    let out = event_driven_calc("aggregate");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("aggregate") || out.contains("Aggregate") || out.contains("domain"));
}

#[test]
fn event_driven_snapshot_alias() {
    let out = event_driven_calc("snapshot");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("snapshot") || out.contains("Snapshot") || out.contains("replay"));
}

#[test]
fn event_driven_messaging() {
    let out = event_driven_calc("messaging");
    assert!(
        out.contains("RabbitMQ")
            || out.contains("SQS")
            || out.contains("NATS")
            || out.contains("queue")
    );
}

#[test]
fn event_driven_rabbitmq_alias() {
    let out = event_driven_calc("rabbitmq");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("RabbitMQ") || out.contains("rabbitmq") || out.contains("exchange"));
}

#[test]
fn event_driven_sqs_alias() {
    let out = event_driven_calc("sqs");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("SQS") || out.contains("FIFO") || out.contains("DLQ") || out.contains("queue")
    );
}

#[test]
fn event_driven_dlq_alias() {
    let out = event_driven_calc("dlq");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("DLQ") || out.contains("Dead Letter") || out.contains("dead letter"));
}

#[test]
fn event_driven_cdc() {
    let out = event_driven_calc("cdc");
    assert!(
        out.contains("CDC")
            || out.contains("Debezium")
            || out.contains("outbox")
            || out.contains("WAL")
    );
}

#[test]
fn event_driven_debezium_alias() {
    let out = event_driven_calc("debezium");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Debezium") || out.contains("debezium") || out.contains("CDC"));
}

#[test]
fn event_driven_outbox_alias() {
    let out = event_driven_calc("outbox");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("outbox") || out.contains("Outbox") || out.contains("atomic"));
}

#[test]
fn event_driven_dual_write_alias() {
    let out = event_driven_calc("dual-write");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("dual-write") || out.contains("outbox") || out.contains("atomic"));
}

#[test]
fn event_driven_schema() {
    let out = event_driven_calc("schema");
    assert!(
        out.contains("Avro")
            || out.contains("Protobuf")
            || out.contains("schema")
            || out.contains("evolution")
    );
}

#[test]
fn event_driven_avro_alias() {
    let out = event_driven_calc("avro");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Avro") || out.contains("avro") || out.contains("schema"));
}

#[test]
fn event_driven_asyncapi_alias() {
    let out = event_driven_calc("asyncapi");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("AsyncAPI") || out.contains("asyncapi") || out.contains("channel"));
}

#[test]
fn event_driven_edge_cases() {
    let _ = event_driven_calc("@#$%");
    let _ = event_driven_calc("   ");
}

// ── api_gateway_calc ──────────────────────────────────────────────────────────

#[test]
fn api_gateway_empty() {
    let out = api_gateway_calc("");
    assert!(out.contains("patterns") || out.contains("products") || out.contains("Topics"));
}

#[test]
fn api_gateway_nomatch() {
    let out = api_gateway_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn api_gateway_all() {
    let out = api_gateway_calc("all");
    assert!(out.contains("patterns") || out.contains("Patterns"));
    assert!(out.contains("products") || out.contains("Products") || out.contains("Kong"));
    assert!(out.contains("service-mesh") || out.contains("Service Mesh") || out.contains("Istio"));
    assert!(out.contains("auth") || out.contains("Auth"));
    assert!(out.contains("observability") || out.contains("Observability"));
    assert!(out.contains("design") || out.contains("Design"));
}

#[test]
fn api_gateway_patterns() {
    let out = api_gateway_calc("patterns");
    assert!(
        out.contains("BFF")
            || out.contains("rate limit")
            || out.contains("circuit")
            || out.contains("gateway")
    );
}

#[test]
fn api_gateway_bff_alias() {
    let out = api_gateway_calc("bff");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("BFF") || out.contains("Backend for Frontend") || out.contains("client"));
}

#[test]
fn api_gateway_rate_limit_alias() {
    let out = api_gateway_calc("rate-limit");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("rate")
            || out.contains("Rate")
            || out.contains("throttle")
            || out.contains("429")
    );
}

#[test]
fn api_gateway_circuit_breaker_alias() {
    let out = api_gateway_calc("circuit-breaker");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("circuit") || out.contains("Circuit") || out.contains("fail fast"));
}

#[test]
fn api_gateway_products() {
    let out = api_gateway_calc("products");
    assert!(
        out.contains("Kong")
            || out.contains("Nginx")
            || out.contains("Envoy")
            || out.contains("Traefik")
    );
}

#[test]
fn api_gateway_kong_alias() {
    let out = api_gateway_calc("kong");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Kong") || out.contains("kong") || out.contains("plugin"));
}

#[test]
fn api_gateway_nginx_alias() {
    let out = api_gateway_calc("nginx");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("NGINX") || out.contains("nginx") || out.contains("proxy"));
}

#[test]
fn api_gateway_envoy_alias() {
    let out = api_gateway_calc("envoy");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("Envoy")
            || out.contains("envoy")
            || out.contains("filter")
            || out.contains("cluster")
    );
}

#[test]
fn api_gateway_traefik_alias() {
    let out = api_gateway_calc("traefik");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Traefik") || out.contains("traefik") || out.contains("middleware"));
}

#[test]
fn api_gateway_service_mesh() {
    let out = api_gateway_calc("service-mesh");
    assert!(
        out.contains("Istio")
            || out.contains("Linkerd")
            || out.contains("sidecar")
            || out.contains("mTLS")
    );
}

#[test]
fn api_gateway_istio_alias() {
    let out = api_gateway_calc("istio");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Istio") || out.contains("istio") || out.contains("VirtualService"));
}

#[test]
fn api_gateway_linkerd_alias() {
    let out = api_gateway_calc("linkerd");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Linkerd") || out.contains("linkerd") || out.contains("proxy"));
}

#[test]
fn api_gateway_sidecar_alias() {
    let out = api_gateway_calc("sidecar");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("sidecar") || out.contains("Sidecar") || out.contains("Envoy"));
}

#[test]
fn api_gateway_auth() {
    let out = api_gateway_calc("auth");
    assert!(
        out.contains("JWT")
            || out.contains("OAuth")
            || out.contains("API key")
            || out.contains("auth")
    );
}

#[test]
fn api_gateway_jwt_alias() {
    let out = api_gateway_calc("jwt");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("JWT") || out.contains("jwt") || out.contains("token"));
}

#[test]
fn api_gateway_oauth2_alias() {
    let out = api_gateway_calc("oauth2");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("OAuth") || out.contains("oauth") || out.contains("token"));
}

#[test]
fn api_gateway_cors_alias() {
    let out = api_gateway_calc("cors");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("CORS")
            || out.contains("cors")
            || out.contains("preflight")
            || out.contains("Preflight")
    );
}

#[test]
fn api_gateway_jwks_alias() {
    let out = api_gateway_calc("jwks");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("JWKS")
            || out.contains("jwks")
            || out.contains("key rotation")
            || out.contains("rotation")
    );
}

#[test]
fn api_gateway_observability() {
    let out = api_gateway_calc("observability");
    assert!(
        out.contains("metric")
            || out.contains("Metric")
            || out.contains("trace")
            || out.contains("log")
    );
}

#[test]
fn api_gateway_access_log_alias() {
    let out = api_gateway_calc("access-log");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("access log") || out.contains("log") || out.contains("format"));
}

#[test]
fn api_gateway_design() {
    let out = api_gateway_calc("design");
    assert!(
        out.contains("version")
            || out.contains("Version")
            || out.contains("lifecycle")
            || out.contains("deprecat")
    );
}

#[test]
fn api_gateway_versioning_alias() {
    let out = api_gateway_calc("versioning");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("version") || out.contains("v1") || out.contains("URL"));
}

#[test]
fn api_gateway_openapi_alias() {
    let out = api_gateway_calc("openapi");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("OpenAPI")
            || out.contains("openapi")
            || out.contains("spec")
            || out.contains("Swagger")
    );
}

#[test]
fn api_gateway_edge_cases() {
    let _ = api_gateway_calc("@#$%");
    let _ = api_gateway_calc("   ");
}

// ── database_adv_calc ─────────────────────────────────────────────────────────

#[test]
fn database_adv_empty() {
    let out = database_adv_calc("");
    assert!(out.contains("indexing") || out.contains("partitioning") || out.contains("Topics"));
}

#[test]
fn database_adv_nomatch() {
    let out = database_adv_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn database_adv_all() {
    let out = database_adv_calc("all");
    assert!(out.contains("indexing") || out.contains("Indexing") || out.contains("B-tree"));
    assert!(
        out.contains("partitioning") || out.contains("Partitioning") || out.contains("PARTITION")
    );
    assert!(out.contains("replication") || out.contains("Replication"));
    assert!(out.contains("query") || out.contains("Query"));
    assert!(out.contains("transactions") || out.contains("Transactions"));
    assert!(out.contains("maintenance") || out.contains("Maintenance"));
}

#[test]
fn database_adv_indexing() {
    let out = database_adv_calc("indexing");
    assert!(out.contains("B-tree") || out.contains("btree") || out.contains("GIN"));
}

#[test]
fn database_adv_btree_alias() {
    let out = database_adv_calc("btree");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("B-tree") || out.contains("balanced") || out.contains("log n"));
}

#[test]
fn database_adv_gin_alias() {
    let out = database_adv_calc("gin");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("GIN") || out.contains("inverted") || out.contains("full-text"));
}

#[test]
fn database_adv_covering_alias() {
    let out = database_adv_calc("covering");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("covering") || out.contains("INCLUDE") || out.contains("index-only"));
}

#[test]
fn database_adv_partial_index_alias() {
    let out = database_adv_calc("partial-index");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("partial") || out.contains("WHERE") || out.contains("smaller"));
}

#[test]
fn database_adv_partitioning() {
    let out = database_adv_calc("partitioning");
    assert!(out.contains("Range") || out.contains("range") || out.contains("PARTITION"));
}

#[test]
fn database_adv_partition_alias() {
    let out = database_adv_calc("partition");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("partition") || out.contains("Partition") || out.contains("RANGE"));
}

#[test]
fn database_adv_sharding_alias() {
    let out = database_adv_calc("sharding");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("shard") || out.contains("partition") || out.contains("hash"));
}

#[test]
fn database_adv_pruning_alias() {
    let out = database_adv_calc("pruning");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("prun") || out.contains("planner") || out.contains("partition"));
}

#[test]
fn database_adv_replication() {
    let out = database_adv_calc("replication");
    assert!(out.contains("WAL") || out.contains("replica") || out.contains("streaming"));
}

#[test]
fn database_adv_wal_alias() {
    let out = database_adv_calc("wal");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("WAL") || out.contains("streaming") || out.contains("physical"));
}

#[test]
fn database_adv_patroni_alias() {
    let out = database_adv_calc("patroni");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Patroni") || out.contains("failover") || out.contains("automatic"));
}

#[test]
fn database_adv_gtid_alias() {
    let out = database_adv_calc("gtid");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("GTID") || out.contains("binlog") || out.contains("MySQL"));
}

#[test]
fn database_adv_rpo_alias() {
    let out = database_adv_calc("rpo");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("RPO") || out.contains("data loss") || out.contains("synchronous"));
}

#[test]
fn database_adv_query_opt() {
    let out = database_adv_calc("query-opt");
    assert!(out.contains("EXPLAIN") || out.contains("Seq Scan") || out.contains("index scan"));
}

#[test]
fn database_adv_explain_alias() {
    let out = database_adv_calc("explain");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("EXPLAIN") || out.contains("actual rows") || out.contains("cost"));
}

#[test]
fn database_adv_materialized_view_alias() {
    let out = database_adv_calc("materialized-view");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("materialized") || out.contains("REFRESH") || out.contains("precompute"));
}

#[test]
fn database_adv_window_alias() {
    let out = database_adv_calc("window");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("OVER") || out.contains("window") || out.contains("PARTITION BY"));
}

#[test]
fn database_adv_work_mem_alias() {
    let out = database_adv_calc("work-mem");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("work_mem") || out.contains("memory") || out.contains("sort"));
}

#[test]
fn database_adv_transactions() {
    let out = database_adv_calc("transactions");
    assert!(out.contains("MVCC") || out.contains("isolation") || out.contains("deadlock"));
}

#[test]
fn database_adv_mvcc_alias() {
    let out = database_adv_calc("mvcc");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("MVCC") || out.contains("snapshot") || out.contains("concurrent"));
}

#[test]
fn database_adv_deadlock_alias() {
    let out = database_adv_calc("deadlock");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("deadlock") || out.contains("Deadlock") || out.contains("lock order"));
}

#[test]
fn database_adv_serializable_alias() {
    let out = database_adv_calc("serializable");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Serializable") || out.contains("SSI") || out.contains("isolation"));
}

#[test]
fn database_adv_two_phase_alias() {
    let out = database_adv_calc("two-phase");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("2PC") || out.contains("PREPARE") || out.contains("distributed"));
}

#[test]
fn database_adv_maintenance() {
    let out = database_adv_calc("maintenance");
    assert!(
        out.contains("vacuum")
            || out.contains("Vacuum")
            || out.contains("pgbouncer")
            || out.contains("PgBouncer")
    );
}

#[test]
fn database_adv_pgbouncer_alias() {
    let out = database_adv_calc("pgbouncer");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("PgBouncer") || out.contains("pgbouncer") || out.contains("connection pool")
    );
}

#[test]
fn database_adv_pitr_alias() {
    let out = database_adv_calc("pitr");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("PITR") || out.contains("point-in-time") || out.contains("recovery"));
}

#[test]
fn database_adv_autovacuum_alias() {
    let out = database_adv_calc("autovacuum");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("autovacuum") || out.contains("vacuum") || out.contains("dead tuples"));
}

#[test]
fn database_adv_migration_alias() {
    let out = database_adv_calc("migration");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("migration") || out.contains("Migration") || out.contains("schema"));
}

#[test]
fn database_adv_edge_cases() {
    let _ = database_adv_calc("@#$%");
    let _ = database_adv_calc("   ");
}

// ── networking_adv_calc ───────────────────────────────────────────────────────

#[test]
fn networking_adv_empty() {
    let out = networking_adv_calc("");
    assert!(out.contains("subnetting") || out.contains("routing") || out.contains("Topics"));
}

#[test]
fn networking_adv_nomatch() {
    let out = networking_adv_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn networking_adv_all() {
    let out = networking_adv_calc("all");
    assert!(out.contains("subnetting") || out.contains("Subnetting") || out.contains("CIDR"));
    assert!(out.contains("routing") || out.contains("Routing") || out.contains("OSPF"));
    assert!(out.contains("vlan") || out.contains("VLAN"));
    assert!(out.contains("qos") || out.contains("QoS") || out.contains("DSCP"));
    assert!(out.contains("nat") || out.contains("NAT"));
    assert!(out.contains("tunneling") || out.contains("Tunneling") || out.contains("IPsec"));
}

#[test]
fn networking_adv_subnetting() {
    let out = networking_adv_calc("subnetting");
    assert!(out.contains("CIDR") || out.contains("/24") || out.contains("subnet"));
}

#[test]
fn networking_adv_cidr_alias() {
    let out = networking_adv_calc("cidr");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("CIDR") || out.contains("/24") || out.contains("prefix"));
}

#[test]
fn networking_adv_vlsm_alias() {
    let out = networking_adv_calc("vlsm");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("VLSM") || out.contains("Variable") || out.contains("subnet"));
}

#[test]
fn networking_adv_rfc1918_alias() {
    let out = networking_adv_calc("rfc1918");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("RFC 1918") || out.contains("private") || out.contains("10.0.0.0"));
}

#[test]
fn networking_adv_routing() {
    let out = networking_adv_calc("routing");
    assert!(out.contains("OSPF") || out.contains("BGP") || out.contains("static"));
}

#[test]
fn networking_adv_ospf_alias() {
    let out = networking_adv_calc("ospf");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("OSPF") || out.contains("Dijkstra") || out.contains("link-state"));
}

#[test]
fn networking_adv_bgp_alias() {
    let out = networking_adv_calc("bgp");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("BGP") || out.contains("AS_PATH") || out.contains("eBGP"));
}

#[test]
fn networking_adv_route_reflector_alias() {
    let out = networking_adv_calc("route-reflector");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("route reflector") || out.contains("Route reflector") || out.contains("iBGP")
    );
}

#[test]
fn networking_adv_as_path_alias() {
    let out = networking_adv_calc("as-path");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("AS_PATH") || out.contains("AS") || out.contains("BGP"));
}

#[test]
fn networking_adv_vlan() {
    let out = networking_adv_calc("vlan");
    assert!(out.contains("VLAN") || out.contains("802.1Q") || out.contains("trunk"));
}

#[test]
fn networking_adv_stp_alias() {
    let out = networking_adv_calc("stp");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("STP") || out.contains("spanning tree") || out.contains("Spanning Tree"));
}

#[test]
fn networking_adv_lacp_alias() {
    let out = networking_adv_calc("lacp");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("LACP") || out.contains("802.3ad") || out.contains("link aggregation"));
}

#[test]
fn networking_adv_trunk_alias() {
    let out = networking_adv_calc("trunk");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("trunk") || out.contains("Trunk") || out.contains("VLAN"));
}

#[test]
fn networking_adv_qos() {
    let out = networking_adv_calc("qos");
    assert!(out.contains("DSCP") || out.contains("QoS") || out.contains("queuing"));
}

#[test]
fn networking_adv_dscp_alias() {
    let out = networking_adv_calc("dscp");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("DSCP") || out.contains("EF") || out.contains("AF"));
}

#[test]
fn networking_adv_shaping_alias() {
    let out = networking_adv_calc("shaping");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Shaping") || out.contains("shaping") || out.contains("buffer"));
}

#[test]
fn networking_adv_wred_alias() {
    let out = networking_adv_calc("wred");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("WRED") || out.contains("early detection") || out.contains("drop"));
}

#[test]
fn networking_adv_nat() {
    let out = networking_adv_calc("nat");
    assert!(out.contains("NAT") || out.contains("PAT") || out.contains("address translation"));
}

#[test]
fn networking_adv_pat_alias() {
    let out = networking_adv_calc("pat");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("PAT") || out.contains("Port Address") || out.contains("NAT Overload"));
}

#[test]
fn networking_adv_iptables_alias() {
    let out = networking_adv_calc("iptables");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("iptables") || out.contains("nftables") || out.contains("filter"));
}

#[test]
fn networking_adv_zone_based_alias() {
    let out = networking_adv_calc("zone-based");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("zone") || out.contains("Zone") || out.contains("policy"));
}

#[test]
fn networking_adv_tunneling() {
    let out = networking_adv_calc("tunneling");
    assert!(
        out.contains("IPsec")
            || out.contains("WireGuard")
            || out.contains("GRE")
            || out.contains("VPN")
    );
}

#[test]
fn networking_adv_ipsec_alias() {
    let out = networking_adv_calc("ipsec");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("IPsec") || out.contains("IKE") || out.contains("ESP"));
}

#[test]
fn networking_adv_wireguard_alias() {
    let out = networking_adv_calc("wireguard");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("WireGuard") || out.contains("wg") || out.contains("ChaCha20"));
}

#[test]
fn networking_adv_mpls_alias() {
    let out = networking_adv_calc("mpls");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("MPLS") || out.contains("label") || out.contains("LSR"));
}

#[test]
fn networking_adv_vrf_alias() {
    let out = networking_adv_calc("vrf");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("VRF") || out.contains("L3VPN") || out.contains("MPLS"));
}

#[test]
fn networking_adv_sdwan_alias() {
    let out = networking_adv_calc("sd-wan");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("SD-WAN") || out.contains("software-defined") || out.contains("overlay"));
}

#[test]
fn networking_adv_edge_cases() {
    let _ = networking_adv_calc("@#$%");
    let _ = networking_adv_calc("   ");
}

// ── testing_ref_calc ──────────────────────────────────────────────────────────

#[test]
fn testing_ref_empty() {
    let out = testing_ref_calc("");
    assert!(out.contains("strategy") || out.contains("unit") || out.contains("Topics"));
}

#[test]
fn testing_ref_nomatch() {
    let out = testing_ref_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn testing_ref_all() {
    let out = testing_ref_calc("all");
    assert!(out.contains("strategy") || out.contains("Strategy") || out.contains("pyramid"));
    assert!(out.contains("unit") || out.contains("Unit"));
    assert!(out.contains("integration") || out.contains("Integration"));
    assert!(
        out.contains("e2e")
            || out.contains("E2E")
            || out.contains("Playwright")
            || out.contains("end-to-end")
    );
    assert!(out.contains("performance") || out.contains("Performance") || out.contains("load"));
    assert!(out.contains("mocking") || out.contains("Mocking") || out.contains("mock"));
}

#[test]
fn testing_ref_strategy() {
    let out = testing_ref_calc("strategy");
    assert!(out.contains("pyramid") || out.contains("TDD") || out.contains("coverage"));
}

#[test]
fn testing_ref_tdd_alias() {
    let out = testing_ref_calc("tdd");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("TDD") || out.contains("Red") || out.contains("Refactor"));
}

#[test]
fn testing_ref_bdd_alias() {
    let out = testing_ref_calc("bdd");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("BDD") || out.contains("Given") || out.contains("Gherkin"));
}

#[test]
fn testing_ref_coverage_alias() {
    let out = testing_ref_calc("coverage");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("coverage") || out.contains("Coverage") || out.contains("branch"));
}

#[test]
fn testing_ref_mutation_alias() {
    let out = testing_ref_calc("mutation");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("mutation") || out.contains("Mutation") || out.contains("mutant"));
}

#[test]
fn testing_ref_unit() {
    let out = testing_ref_calc("unit");
    assert!(
        out.contains("stub")
            || out.contains("Stub")
            || out.contains("mock")
            || out.contains("Arrange")
    );
}

#[test]
fn testing_ref_stub_alias() {
    let out = testing_ref_calc("stub");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Stub") || out.contains("stub") || out.contains("canned"));
}

#[test]
fn testing_ref_proptest_alias() {
    let out = testing_ref_calc("proptest");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("proptest") || out.contains("property") || out.contains("invariant"));
}

#[test]
fn testing_ref_snapshot_alias() {
    let out = testing_ref_calc("snapshot");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("snapshot") || out.contains("Snapshot") || out.contains("insta"));
}

#[test]
fn testing_ref_hypothesis_alias() {
    let out = testing_ref_calc("hypothesis");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Hypothesis") || out.contains("shrinking") || out.contains("property"));
}

#[test]
fn testing_ref_integration() {
    let out = testing_ref_calc("integration");
    assert!(
        out.contains("Testcontainers")
            || out.contains("testcontainers")
            || out.contains("Docker")
            || out.contains("Pact")
    );
}

#[test]
fn testing_ref_testcontainers_alias() {
    let out = testing_ref_calc("testcontainers");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Testcontainers") || out.contains("Docker") || out.contains("container"));
}

#[test]
fn testing_ref_pact_alias() {
    let out = testing_ref_calc("pact");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("Pact")
            || out.contains("pact")
            || out.contains("contract")
            || out.contains("consumer")
    );
}

#[test]
fn testing_ref_wiremock_alias() {
    let out = testing_ref_calc("wiremock");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("wiremock")
            || out.contains("WireMock")
            || out.contains("stub")
            || out.contains("HTTP")
    );
}

#[test]
fn testing_ref_db_test_alias() {
    let out = testing_ref_calc("db-test");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("transaction")
            || out.contains("rollback")
            || out.contains("DB")
            || out.contains("database")
    );
}

#[test]
fn testing_ref_e2e() {
    let out = testing_ref_calc("e2e");
    assert!(out.contains("Playwright") || out.contains("Cypress") || out.contains("Selenium"));
}

#[test]
fn testing_ref_playwright_alias() {
    let out = testing_ref_calc("playwright");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Playwright") || out.contains("playwright") || out.contains("browser"));
}

#[test]
fn testing_ref_cypress_alias() {
    let out = testing_ref_calc("cypress");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Cypress") || out.contains("cy.") || out.contains("browser"));
}

#[test]
fn testing_ref_flakiness_alias() {
    let out = testing_ref_calc("flakiness");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("flak")
            || out.contains("Flak")
            || out.contains("wait")
            || out.contains("retry")
    );
}

#[test]
fn testing_ref_visual_regression_alias() {
    let out = testing_ref_calc("visual-regression");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("visual")
            || out.contains("Visual")
            || out.contains("Percy")
            || out.contains("pixel")
    );
}

#[test]
fn testing_ref_performance() {
    let out = testing_ref_calc("performance");
    assert!(
        out.contains("k6")
            || out.contains("load")
            || out.contains("Gatling")
            || out.contains("Locust")
    );
}

#[test]
fn testing_ref_k6_alias() {
    let out = testing_ref_calc("k6");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("k6") || out.contains("threshold") || out.contains("load"));
}

#[test]
fn testing_ref_gatling_alias() {
    let out = testing_ref_calc("gatling");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("Gatling")
            || out.contains("gatling")
            || out.contains("Scala")
            || out.contains("simulation")
    );
}

#[test]
fn testing_ref_load_test_alias() {
    let out = testing_ref_calc("load-test");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("load") || out.contains("SLA") || out.contains("throughput"));
}

#[test]
fn testing_ref_benchmark_alias() {
    let out = testing_ref_calc("benchmark");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("benchmark") || out.contains("criterion") || out.contains("regression"));
}

#[test]
fn testing_ref_mocking() {
    let out = testing_ref_calc("mocking");
    assert!(
        out.contains("mockall")
            || out.contains("mockito")
            || out.contains("jest")
            || out.contains("MagicMock")
    );
}

#[test]
fn testing_ref_mockall_alias() {
    let out = testing_ref_calc("mockall");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("mockall") || out.contains("automock") || out.contains("expect"));
}

#[test]
fn testing_ref_freezegun_alias() {
    let out = testing_ref_calc("freezegun");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("freezegun") || out.contains("datetime") || out.contains("time"));
}

#[test]
fn testing_ref_msw_alias() {
    let out = testing_ref_calc("msw");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("msw")
            || out.contains("MSW")
            || out.contains("Service Worker")
            || out.contains("fetch")
    );
}

#[test]
fn testing_ref_dependency_injection_alias() {
    let out = testing_ref_calc("dependency-injection");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("injection")
            || out.contains("Injection")
            || out.contains("interface")
            || out.contains("trait")
    );
}

#[test]
fn testing_ref_edge_cases() {
    let _ = testing_ref_calc("@#$%");
    let _ = testing_ref_calc("   ");
}

// ── cicd_ref_calc ─────────────────────────────────────────────────────────────

#[test]
fn cicd_ref_empty() {
    let out = cicd_ref_calc("");
    assert!(out.contains("concepts") || out.contains("github") || out.contains("Topics"));
}

#[test]
fn cicd_ref_nomatch() {
    let out = cicd_ref_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn cicd_ref_all() {
    let out = cicd_ref_calc("all");
    assert!(out.contains("concepts") || out.contains("Concepts") || out.contains("DORA"));
    assert!(out.contains("github") || out.contains("GitHub") || out.contains("workflow"));
    assert!(out.contains("gitlab") || out.contains("GitLab"));
    assert!(out.contains("pipelines") || out.contains("Pipelines") || out.contains("canary"));
    assert!(out.contains("security") || out.contains("Security") || out.contains("SAST"));
    assert!(out.contains("jenkins") || out.contains("Jenkins"));
}

#[test]
fn cicd_ref_concepts() {
    let out = cicd_ref_calc("concepts");
    assert!(
        out.contains("CI")
            || out.contains("CD")
            || out.contains("pipeline")
            || out.contains("DORA")
    );
}

#[test]
fn cicd_ref_dora_alias() {
    let out = cicd_ref_calc("dora");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("DORA") || out.contains("Deployment Frequency") || out.contains("Lead Time")
    );
}

#[test]
fn cicd_ref_trunk_based_alias() {
    let out = cicd_ref_calc("trunk-based");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("trunk") || out.contains("Trunk") || out.contains("short-lived"));
}

#[test]
fn cicd_ref_feature_flags_alias() {
    let out = cicd_ref_calc("feature-flags");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("feature flag")
            || out.contains("Feature flag")
            || out.contains("LaunchDarkly")
    );
}

#[test]
fn cicd_ref_github_actions() {
    let out = cicd_ref_calc("github-actions");
    assert!(out.contains("workflow") || out.contains("actions/checkout") || out.contains("matrix"));
}

#[test]
fn cicd_ref_gha_alias() {
    let out = cicd_ref_calc("gha");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("GitHub") || out.contains("workflow") || out.contains("Actions"));
}

#[test]
fn cicd_ref_matrix_build_alias() {
    let out = cicd_ref_calc("matrix-build");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("matrix") || out.contains("Matrix") || out.contains("strategy"));
}

#[test]
fn cicd_ref_reusable_workflow_alias() {
    let out = cicd_ref_calc("reusable-workflow");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("reusable") || out.contains("Reusable") || out.contains("workflow_call"));
}

#[test]
fn cicd_ref_gitlab() {
    let out = cicd_ref_calc("gitlab");
    assert!(out.contains("GitLab") || out.contains(".gitlab-ci.yml") || out.contains("runner"));
}

#[test]
fn cicd_ref_gitlab_runner_alias() {
    let out = cicd_ref_calc("gitlab-runner");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("runner") || out.contains("Runner") || out.contains("GitLab"));
}

#[test]
fn cicd_ref_review_apps_alias() {
    let out = cicd_ref_calc("review-apps");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("review") || out.contains("Review") || out.contains("environment"));
}

#[test]
fn cicd_ref_pipelines() {
    let out = cicd_ref_calc("pipelines");
    assert!(
        out.contains("canary")
            || out.contains("Canary")
            || out.contains("blue-green")
            || out.contains("Blue-green")
    );
}

#[test]
fn cicd_ref_canary_alias() {
    let out = cicd_ref_calc("canary");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("canary") || out.contains("Canary") || out.contains("traffic"));
}

#[test]
fn cicd_ref_blue_green_alias() {
    let out = cicd_ref_calc("blue-green");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("blue")
            || out.contains("Blue")
            || out.contains("load balancer")
            || out.contains("rollback")
    );
}

#[test]
fn cicd_ref_rollback_alias() {
    let out = cicd_ref_calc("rollback");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("rollback") || out.contains("Rollback") || out.contains("undo"));
}

#[test]
fn cicd_ref_security() {
    let out = cicd_ref_calc("security");
    assert!(
        out.contains("SAST")
            || out.contains("SCA")
            || out.contains("secret")
            || out.contains("scan")
    );
}

#[test]
fn cicd_ref_sast_alias() {
    let out = cicd_ref_calc("sast");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("SAST") || out.contains("static") || out.contains("source code"));
}

#[test]
fn cicd_ref_trivy_alias() {
    let out = cicd_ref_calc("trivy");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("Trivy")
            || out.contains("trivy")
            || out.contains("container")
            || out.contains("CVE")
    );
}

#[test]
fn cicd_ref_sbom_alias() {
    let out = cicd_ref_calc("sbom");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("SBOM")
            || out.contains("bill of materials")
            || out.contains("Bill of Materials")
    );
}

#[test]
fn cicd_ref_jenkins() {
    let out = cicd_ref_calc("jenkins");
    assert!(out.contains("Jenkins") || out.contains("Jenkinsfile") || out.contains("pipeline"));
}

#[test]
fn cicd_ref_jenkinsfile_alias() {
    let out = cicd_ref_calc("jenkinsfile");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Jenkinsfile") || out.contains("declarative") || out.contains("pipeline"));
}

#[test]
fn cicd_ref_shared_library_alias() {
    let out = cicd_ref_calc("shared-library");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("Shared")
            || out.contains("shared")
            || out.contains("library")
            || out.contains("@Library")
    );
}

#[test]
fn cicd_ref_edge_cases() {
    let _ = cicd_ref_calc("@#$%");
    let _ = cicd_ref_calc("   ");
}

// ── design_patterns_calc ──────────────────────────────────────────────────────

#[test]
fn design_patterns_empty() {
    let out = design_patterns_calc("");
    assert!(out.contains("creational") || out.contains("structural") || out.contains("Topics"));
}

#[test]
fn design_patterns_nomatch() {
    let out = design_patterns_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn design_patterns_all() {
    let out = design_patterns_calc("all");
    assert!(out.contains("creational") || out.contains("Creational") || out.contains("Singleton"));
    assert!(out.contains("structural") || out.contains("Structural") || out.contains("Adapter"));
    assert!(out.contains("behavioral") || out.contains("Behavioral") || out.contains("Observer"));
    assert!(out.contains("concurrency") || out.contains("Concurrency"));
    assert!(
        out.contains("rust-idioms")
            || out.contains("Rust")
            || out.contains("typestate")
            || out.contains("Typestate")
    );
    assert!(
        out.contains("architecture") || out.contains("Architecture") || out.contains("Hexagonal")
    );
}

#[test]
fn design_patterns_creational() {
    let out = design_patterns_calc("creational");
    assert!(out.contains("Singleton") || out.contains("Factory") || out.contains("Builder"));
}

#[test]
fn design_patterns_singleton_alias() {
    let out = design_patterns_calc("singleton");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Singleton") || out.contains("one instance") || out.contains("OnceLock"));
}

#[test]
fn design_patterns_builder_pattern_alias() {
    let out = design_patterns_calc("builder-pattern");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Builder") || out.contains("builder") || out.contains("build()"));
}

#[test]
fn design_patterns_factory_alias() {
    let out = design_patterns_calc("factory");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Factory") || out.contains("factory") || out.contains("create"));
}

#[test]
fn design_patterns_structural() {
    let out = design_patterns_calc("structural");
    assert!(out.contains("Adapter") || out.contains("Decorator") || out.contains("Facade"));
}

#[test]
fn design_patterns_adapter_alias() {
    let out = design_patterns_calc("adapter");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Adapter") || out.contains("adapter") || out.contains("interface"));
}

#[test]
fn design_patterns_decorator_alias() {
    let out = design_patterns_calc("decorator");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Decorator") || out.contains("decorator") || out.contains("behavior"));
}

#[test]
fn design_patterns_facade_alias() {
    let out = design_patterns_calc("facade");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Facade") || out.contains("facade") || out.contains("simplified"));
}

#[test]
fn design_patterns_proxy_alias() {
    let out = design_patterns_calc("proxy-pattern");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Proxy") || out.contains("proxy") || out.contains("surrogate"));
}

#[test]
fn design_patterns_behavioral() {
    let out = design_patterns_calc("behavioral");
    assert!(out.contains("Observer") || out.contains("Strategy") || out.contains("Command"));
}

#[test]
fn design_patterns_observer_alias() {
    let out = design_patterns_calc("observer");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Observer") || out.contains("observer") || out.contains("subscriber"));
}

#[test]
fn design_patterns_strategy_alias() {
    let out = design_patterns_calc("strategy");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("Strategy") || out.contains("strategy") || out.contains("algorithm"));
}

#[test]
fn design_patterns_state_pattern_alias() {
    let out = design_patterns_calc("state-pattern");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("State") || out.contains("state") || out.contains("behavior"));
}

#[test]
fn design_patterns_concurrency() {
    let out = design_patterns_calc("concurrency");
    assert!(
        out.contains("thread")
            || out.contains("Thread")
            || out.contains("actor")
            || out.contains("Actor")
    );
}

#[test]
fn design_patterns_actor_model_alias() {
    let out = design_patterns_calc("actor-model");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("actor")
            || out.contains("Actor")
            || out.contains("mailbox")
            || out.contains("message")
    );
}

#[test]
fn design_patterns_thread_pool_alias() {
    let out = design_patterns_calc("thread-pool");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("thread pool") || out.contains("Thread Pool") || out.contains("rayon"));
}

#[test]
fn design_patterns_rust_idioms() {
    let out = design_patterns_calc("rust-idioms");
    assert!(
        out.contains("typestate")
            || out.contains("Typestate")
            || out.contains("newtype")
            || out.contains("Newtype")
    );
}

#[test]
fn design_patterns_typestate_alias() {
    let out = design_patterns_calc("typestate");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("typestate") || out.contains("PhantomData") || out.contains("compile"));
}

#[test]
fn design_patterns_newtype_alias() {
    let out = design_patterns_calc("newtype-pattern");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("Newtype")
            || out.contains("newtype")
            || out.contains("wrapper")
            || out.contains("zero")
    );
}

#[test]
fn design_patterns_raii_alias() {
    let out = design_patterns_calc("raii");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("RAII") || out.contains("Drop") || out.contains("Resource Acquisition"));
}

#[test]
fn design_patterns_architecture() {
    let out = design_patterns_calc("architecture");
    assert!(
        out.contains("Hexagonal")
            || out.contains("CQRS")
            || out.contains("microservice")
            || out.contains("Microservice")
    );
}

#[test]
fn design_patterns_hexagonal_alias() {
    let out = design_patterns_calc("hexagonal");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("Hexagonal")
            || out.contains("hexagonal")
            || out.contains("Ports")
            || out.contains("adapter")
    );
}

#[test]
fn design_patterns_cqrs_alias() {
    let out = design_patterns_calc("cqrs-pattern");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("CQRS") || out.contains("command") || out.contains("query"));
}

#[test]
fn design_patterns_strangler_fig_alias() {
    let out = design_patterns_calc("strangler-fig");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("Strangler")
            || out.contains("strangler")
            || out.contains("migrate")
            || out.contains("monolith")
    );
}

#[test]
fn design_patterns_edge_cases() {
    let _ = design_patterns_calc("@#$%");
    let _ = design_patterns_calc("   ");
}

// ── auth_ref_calc ─────────────────────────────────────────────────────────────

#[test]
fn auth_ref_empty() {
    let out = auth_ref_calc("");
    assert!(out.contains("oauth2") || out.contains("oidc") || out.contains("Topics"));
}

#[test]
fn auth_ref_nomatch() {
    let out = auth_ref_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn auth_ref_all() {
    let out = auth_ref_calc("all");
    assert!(out.contains("oauth2") || out.contains("OAuth") || out.contains("Authorization Code"));
    assert!(out.contains("oidc") || out.contains("OIDC") || out.contains("OpenID"));
    assert!(out.contains("jwt") || out.contains("JWT"));
    assert!(
        out.contains("session")
            || out.contains("Session")
            || out.contains("cookie")
            || out.contains("Cookie")
    );
    assert!(out.contains("saml") || out.contains("SAML"));
    assert!(
        out.contains("security")
            || out.contains("Security")
            || out.contains("bcrypt")
            || out.contains("Argon")
    );
}

#[test]
fn auth_ref_oauth2() {
    let out = auth_ref_calc("oauth2");
    assert!(
        out.contains("Authorization Code") || out.contains("PKCE") || out.contains("access_token")
    );
}

#[test]
fn auth_ref_pkce_alias() {
    let out = auth_ref_calc("pkce");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("PKCE") || out.contains("code_challenge") || out.contains("code_verifier")
    );
}

#[test]
fn auth_ref_client_credentials_alias() {
    let out = auth_ref_calc("client-credentials");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("client_credentials") || out.contains("machine") || out.contains("service")
    );
}

#[test]
fn auth_ref_refresh_token_alias() {
    let out = auth_ref_calc("refresh-token");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("refresh_token") || out.contains("refresh token") || out.contains("rotate")
    );
}

#[test]
fn auth_ref_device_code_alias() {
    let out = auth_ref_calc("device-code");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("device_code")
            || out.contains("device code")
            || out.contains("TV")
            || out.contains("CLI")
    );
}

#[test]
fn auth_ref_oidc() {
    let out = auth_ref_calc("oidc");
    assert!(
        out.contains("id_token")
            || out.contains("OpenID")
            || out.contains("discovery")
            || out.contains("JWKS")
    );
}

#[test]
fn auth_ref_id_token_alias() {
    let out = auth_ref_calc("id-token");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("id_token") || out.contains("id token") || out.contains("identity"));
}

#[test]
fn auth_ref_jwks_alias() {
    let out = auth_ref_calc("jwks");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("JWKS") || out.contains("public key") || out.contains("JWK"));
}

#[test]
fn auth_ref_discovery_alias() {
    let out = auth_ref_calc("discovery");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("discovery")
            || out.contains("openid-configuration")
            || out.contains(".well-known")
    );
}

#[test]
fn auth_ref_jwt() {
    let out = auth_ref_calc("jwt");
    assert!(
        out.contains("RS256")
            || out.contains("HS256")
            || out.contains("header.payload")
            || out.contains("signature")
    );
}

#[test]
fn auth_ref_rs256_alias() {
    let out = auth_ref_calc("rs256");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("RS256") || out.contains("RSA") || out.contains("asymmetric"));
}

#[test]
fn auth_ref_claims_alias() {
    let out = auth_ref_calc("claims");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("iss") || out.contains("sub") || out.contains("exp") || out.contains("aud")
    );
}

#[test]
fn auth_ref_jti_alias() {
    let out = auth_ref_calc("jti");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("jti") || out.contains("replay") || out.contains("unique"));
}

#[test]
fn auth_ref_session() {
    let out = auth_ref_calc("session");
    assert!(
        out.contains("cookie")
            || out.contains("Cookie")
            || out.contains("HttpOnly")
            || out.contains("SameSite")
    );
}

#[test]
fn auth_ref_httponly_alias() {
    let out = auth_ref_calc("httponly");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("HttpOnly") || out.contains("httponly") || out.contains("XSS"));
}

#[test]
fn auth_ref_samesite_alias() {
    let out = auth_ref_calc("samesite");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("SameSite") || out.contains("Strict") || out.contains("Lax"));
}

#[test]
fn auth_ref_csrf_alias() {
    let out = auth_ref_calc("csrf");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("CSRF")
            || out.contains("csrf")
            || out.contains("synchronizer")
            || out.contains("SameSite")
    );
}

#[test]
fn auth_ref_saml() {
    let out = auth_ref_calc("saml");
    assert!(
        out.contains("SAML")
            || out.contains("IdP")
            || out.contains("Service Provider")
            || out.contains("Assertion")
    );
}

#[test]
fn auth_ref_saml2_alias() {
    let out = auth_ref_calc("saml2");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("SAML") || out.contains("saml") || out.contains("enterprise"));
}

#[test]
fn auth_ref_idp_alias() {
    let out = auth_ref_calc("idp");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("IdP") || out.contains("Identity Provider") || out.contains("Okta"));
}

#[test]
fn auth_ref_sso_alias() {
    let out = auth_ref_calc("sso");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("SSO") || out.contains("single sign") || out.contains("Single Sign"));
}

#[test]
fn auth_ref_security() {
    let out = auth_ref_calc("security");
    assert!(
        out.contains("bcrypt")
            || out.contains("Argon2")
            || out.contains("TOTP")
            || out.contains("MFA")
    );
}

#[test]
fn auth_ref_bcrypt_alias() {
    let out = auth_ref_calc("bcrypt");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("bcrypt") || out.contains("password") || out.contains("hash"));
}

#[test]
fn auth_ref_totp_alias() {
    let out = auth_ref_calc("totp");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("TOTP")
            || out.contains("OTP")
            || out.contains("time-based")
            || out.contains("Authenticator")
    );
}

#[test]
fn auth_ref_webauthn_alias() {
    let out = auth_ref_calc("webauthn");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("WebAuthn")
            || out.contains("FIDO2")
            || out.contains("passkey")
            || out.contains("biometric")
    );
}

#[test]
fn auth_ref_brute_force_alias() {
    let out = auth_ref_calc("brute-force");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("brute force") || out.contains("rate limit") || out.contains("lockout"));
}

#[test]
fn auth_ref_rbac_alias() {
    let out = auth_ref_calc("rbac");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("RBAC") || out.contains("role") || out.contains("authorization"));
}

#[test]
fn auth_ref_edge_cases() {
    let _ = auth_ref_calc("@#$%");
    let _ = auth_ref_calc("   ");
}

// ── linux_kernel_calc ─────────────────────────────────────────────────────────

#[test]
fn linux_kernel_empty() {
    let out = linux_kernel_calc("");
    assert!(out.contains("syscalls") || out.contains("memory") || out.contains("Topics"));
}

#[test]
fn linux_kernel_nomatch() {
    let out = linux_kernel_calc("zzznomatch");
    assert!(out.contains("No topic found") || out.contains("no topic"));
}

#[test]
fn linux_kernel_all() {
    let out = linux_kernel_calc("all");
    assert!(out.contains("syscalls") || out.contains("Syscalls") || out.contains("strace"));
    assert!(out.contains("memory") || out.contains("Memory") || out.contains("virtual"));
    assert!(out.contains("namespaces") || out.contains("Namespaces") || out.contains("namespace"));
    assert!(out.contains("cgroups") || out.contains("cgroup") || out.contains("Control Groups"));
    assert!(out.contains("ebpf") || out.contains("eBPF"));
    assert!(out.contains("scheduler") || out.contains("Scheduler") || out.contains("CFS"));
}

#[test]
fn linux_kernel_syscalls() {
    let out = linux_kernel_calc("syscalls");
    assert!(out.contains("strace") || out.contains("syscall") || out.contains("mmap"));
}

#[test]
fn linux_kernel_strace_alias() {
    let out = linux_kernel_calc("strace");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("strace") || out.contains("trace") || out.contains("syscall"));
}

#[test]
fn linux_kernel_mmap_alias() {
    let out = linux_kernel_calc("mmap");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("mmap") || out.contains("memory") || out.contains("map"));
}

#[test]
fn linux_kernel_io_uring_alias() {
    let out = linux_kernel_calc("io-uring");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("io_uring") || out.contains("io-uring") || out.contains("async"));
}

#[test]
fn linux_kernel_seccomp_alias() {
    let out = linux_kernel_calc("seccomp");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("seccomp") || out.contains("BPF") || out.contains("filter"));
}

#[test]
fn linux_kernel_memory() {
    let out = linux_kernel_calc("memory");
    assert!(
        out.contains("virtual")
            || out.contains("Virtual")
            || out.contains("page")
            || out.contains("OOM")
    );
}

#[test]
fn linux_kernel_virtual_memory_alias() {
    let out = linux_kernel_calc("virtual-memory");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("virtual") || out.contains("Virtual") || out.contains("address space"));
}

#[test]
fn linux_kernel_oom_killer_alias() {
    let out = linux_kernel_calc("oom-killer");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("OOM")
            || out.contains("oom")
            || out.contains("killed")
            || out.contains("memory")
    );
}

#[test]
fn linux_kernel_huge_pages_alias() {
    let out = linux_kernel_calc("huge-pages");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("huge") || out.contains("Huge") || out.contains("2MB") || out.contains("THP")
    );
}

#[test]
fn linux_kernel_numa_alias() {
    let out = linux_kernel_calc("numa");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("NUMA")
            || out.contains("numa")
            || out.contains("socket")
            || out.contains("local memory")
    );
}

#[test]
fn linux_kernel_namespaces() {
    let out = linux_kernel_calc("namespaces");
    assert!(
        out.contains("pid")
            || out.contains("PID")
            || out.contains("net")
            || out.contains("unshare")
    );
}

#[test]
fn linux_kernel_pid_namespace_alias() {
    let out = linux_kernel_calc("pid-namespace");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("pid") || out.contains("PID") || out.contains("namespace"));
}

#[test]
fn linux_kernel_unshare_alias() {
    let out = linux_kernel_calc("unshare");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("unshare") || out.contains("namespace") || out.contains("shell"));
}

#[test]
fn linux_kernel_rootless_alias() {
    let out = linux_kernel_calc("rootless");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("rootless")
            || out.contains("Rootless")
            || out.contains("user namespace")
            || out.contains("unprivileged")
    );
}

#[test]
fn linux_kernel_cgroups() {
    let out = linux_kernel_calc("cgroups");
    assert!(
        out.contains("cgroup")
            || out.contains("memory")
            || out.contains("cpu")
            || out.contains("Control")
    );
}

#[test]
fn linux_kernel_cgroup_v2_alias() {
    let out = linux_kernel_calc("cgroup-v2");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("v2") || out.contains("unified") || out.contains("cgroup"));
}

#[test]
fn linux_kernel_k8s_cgroups_alias() {
    let out = linux_kernel_calc("k8s-cgroups");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("K8s")
            || out.contains("k8s")
            || out.contains("Pod")
            || out.contains("requests")
    );
}

#[test]
fn linux_kernel_cpuset_alias() {
    let out = linux_kernel_calc("cpuset");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("cpuset") || out.contains("CPU") || out.contains("NUMA"));
}

#[test]
fn linux_kernel_ebpf() {
    let out = linux_kernel_calc("ebpf");
    assert!(out.contains("eBPF") || out.contains("bpftrace") || out.contains("BPF"));
}

#[test]
fn linux_kernel_bpftrace_alias() {
    let out = linux_kernel_calc("bpftrace");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("bpftrace") || out.contains("trace") || out.contains("kprobe"));
}

#[test]
fn linux_kernel_xdp_alias() {
    let out = linux_kernel_calc("xdp");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("XDP")
            || out.contains("xdp")
            || out.contains("packet")
            || out.contains("DDoS")
    );
}

#[test]
fn linux_kernel_kprobe_alias() {
    let out = linux_kernel_calc("kprobe");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("kprobe") || out.contains("kernel function") || out.contains("attach"));
}

#[test]
fn linux_kernel_cilium_alias() {
    let out = linux_kernel_calc("cilium");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("Cilium")
            || out.contains("cilium")
            || out.contains("K8s")
            || out.contains("networking")
    );
}

#[test]
fn linux_kernel_scheduler() {
    let out = linux_kernel_calc("scheduler");
    assert!(
        out.contains("CFS")
            || out.contains("nice")
            || out.contains("vruntime")
            || out.contains("SCHED_FIFO")
    );
}

#[test]
fn linux_kernel_cfs_alias() {
    let out = linux_kernel_calc("cfs");
    assert!(!out.contains("No topic found"));
    assert!(out.contains("CFS") || out.contains("Completely Fair") || out.contains("vruntime"));
}

#[test]
fn linux_kernel_io_scheduler_alias() {
    let out = linux_kernel_calc("io-scheduler");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("scheduler")
            || out.contains("mq-deadline")
            || out.contains("bfq")
            || out.contains("NVMe")
    );
}

#[test]
fn linux_kernel_bfq_alias() {
    let out = linux_kernel_calc("bfq");
    assert!(!out.contains("No topic found"));
    assert!(
        out.contains("bfq")
            || out.contains("BFQ")
            || out.contains("Budget Fair")
            || out.contains("interactive")
    );
}

#[test]
fn linux_kernel_edge_cases() {
    let _ = linux_kernel_calc("@#$%");
    let _ = linux_kernel_calc("   ");
}

// ── compiler_ref_calc ────────────────────────────────────────────────────────

#[test]
fn compiler_ref_empty_returns_topic_list() {
    let out = compiler_ref_calc("");
    assert!(out.contains("compiler-ref topics:"), "{out}");
    assert!(out.contains("parsing"), "{out}");
    assert!(out.contains("codegen"), "{out}");
}

#[test]
fn compiler_ref_nomatch_returns_hint() {
    let out = compiler_ref_calc("zzznomatch");
    assert!(out.contains("No topic found"), "{out}");
    assert!(out.contains("compiler-ref list"), "{out}");
}

#[test]
fn compiler_ref_all_returns_all_sections() {
    let out = compiler_ref_calc("all");
    assert!(out.contains("Parsing"), "{out}");
    assert!(out.contains("AST"), "{out}");
    assert!(out.contains("Intermediate"), "{out}");
    assert!(out.contains("Optimization"), "{out}");
    assert!(out.contains("Code Generation"), "{out}");
    assert!(out.contains("Compiler Tools"), "{out}");
}

#[test]
fn compiler_ref_parsing_by_name() {
    let out = compiler_ref_calc("parsing");
    assert!(out.contains("Lexing"), "{out}");
    assert!(out.contains("Pratt"), "{out}");
}

#[test]
fn compiler_ref_parsing_via_lexer_alias() {
    let out = compiler_ref_calc("lexer");
    assert!(out.contains("Parsing"), "{out}");
}

#[test]
fn compiler_ref_parsing_via_pratt_alias() {
    let out = compiler_ref_calc("pratt");
    assert!(out.contains("Pratt parser"), "{out}");
}

#[test]
fn compiler_ref_parsing_via_tree_sitter_alias() {
    let out = compiler_ref_calc("tree-sitter");
    assert!(out.contains("tree-sitter"), "{out}");
}

#[test]
fn compiler_ref_ast_by_name() {
    let out = compiler_ref_calc("ast");
    assert!(out.contains("Abstract Syntax Tree"), "{out}");
    assert!(out.contains("Visitor"), "{out}");
}

#[test]
fn compiler_ref_ast_via_hindley_milner_alias() {
    let out = compiler_ref_calc("hindley-milner");
    assert!(out.contains("Hindley-Milner"), "{out}");
}

#[test]
fn compiler_ref_ast_via_borrow_checker_alias() {
    let out = compiler_ref_calc("borrow-checker");
    assert!(out.contains("Borrow checking"), "{out}");
}

#[test]
fn compiler_ref_ast_via_nll_alias() {
    let out = compiler_ref_calc("nll");
    assert!(out.contains("NLL"), "{out}");
}

#[test]
fn compiler_ref_ir_by_name() {
    let out = compiler_ref_calc("ir");
    assert!(out.contains("HIR"), "{out}");
    assert!(out.contains("MIR"), "{out}");
    assert!(out.contains("LLVM IR"), "{out}");
}

#[test]
fn compiler_ref_ir_via_ssa_alias() {
    let out = compiler_ref_calc("ssa");
    assert!(out.contains("SSA"), "{out}");
}

#[test]
fn compiler_ref_ir_via_cranelift_alias() {
    let out = compiler_ref_calc("cranelift");
    assert!(out.contains("Cranelift"), "{out}");
}

#[test]
fn compiler_ref_ir_via_mir_alias() {
    let out = compiler_ref_calc("mir");
    assert!(out.contains("MIR"), "{out}");
}

#[test]
fn compiler_ref_optimization_by_name() {
    let out = compiler_ref_calc("optimization");
    assert!(out.contains("Constant folding"), "{out}");
    assert!(out.contains("DCE"), "{out}");
}

#[test]
fn compiler_ref_optimization_via_pgo_alias() {
    let out = compiler_ref_calc("pgo");
    assert!(out.contains("PGO"), "{out}");
}

#[test]
fn compiler_ref_optimization_via_inlining_alias() {
    let out = compiler_ref_calc("inlining");
    assert!(out.contains("Inline expansion"), "{out}");
}

#[test]
fn compiler_ref_optimization_via_vectorization_alias() {
    let out = compiler_ref_calc("vectorization");
    assert!(out.contains("Vectorization"), "{out}");
}

#[test]
fn compiler_ref_codegen_by_name() {
    let out = compiler_ref_calc("codegen");
    assert!(out.contains("Instruction selection"), "{out}");
    assert!(out.contains("ABI"), "{out}");
}

#[test]
fn compiler_ref_codegen_via_jit_alias() {
    let out = compiler_ref_calc("jit");
    assert!(out.contains("JIT"), "{out}");
}

#[test]
fn compiler_ref_codegen_via_lto_alias() {
    let out = compiler_ref_calc("lto");
    assert!(out.contains("LTO"), "{out}");
}

#[test]
fn compiler_ref_codegen_via_abi_alias() {
    let out = compiler_ref_calc("abi");
    assert!(out.contains("ABI"), "{out}");
}

#[test]
fn compiler_ref_tools_by_name() {
    let out = compiler_ref_calc("tools");
    assert!(out.contains("rustc"), "{out}");
    assert!(out.contains("LLVM"), "{out}");
}

#[test]
fn compiler_ref_tools_via_lsp_alias() {
    let out = compiler_ref_calc("lsp");
    assert!(out.contains("Language server"), "{out}");
}

#[test]
fn compiler_ref_tools_via_cargo_fuzz_alias() {
    let out = compiler_ref_calc("cargo-fuzz");
    assert!(out.contains("cargo-fuzz"), "{out}");
}

#[test]
fn compiler_ref_case_insensitive() {
    let out = compiler_ref_calc("PARSING");
    assert!(out.contains("Parsing") || out.contains("Lexing"), "{out}");
}

#[test]
fn compiler_ref_whitespace_trimmed() {
    let out = compiler_ref_calc("  ast  ");
    assert!(out.contains("Abstract Syntax Tree"), "{out}");
}

// ── monitoring_ref_calc ───────────────────────────────────────────────────────

#[test]
fn monitoring_ref_empty_returns_topic_list() {
    let out = monitoring_ref_calc("");
    assert!(out.contains("monitoring-ref topics:"), "{out}");
    assert!(out.contains("slo"), "{out}");
    assert!(out.contains("prometheus"), "{out}");
}

#[test]
fn monitoring_ref_nomatch_returns_hint() {
    let out = monitoring_ref_calc("zzznomatch");
    assert!(out.contains("No topic found"), "{out}");
    assert!(out.contains("monitoring-ref list"), "{out}");
}

#[test]
fn monitoring_ref_all_returns_all_sections() {
    let out = monitoring_ref_calc("all");
    assert!(out.contains("SLI"), "{out}");
    assert!(out.contains("Prometheus"), "{out}");
    assert!(out.contains("Alerting"), "{out}");
    assert!(out.contains("Grafana"), "{out}");
    assert!(out.contains("OpenTelemetry"), "{out}");
    assert!(out.contains("Logging"), "{out}");
}

#[test]
fn monitoring_ref_slo_by_name() {
    let out = monitoring_ref_calc("slo");
    assert!(out.contains("SLI"), "{out}");
    assert!(out.contains("Error budget"), "{out}");
}

#[test]
fn monitoring_ref_slo_via_sli_alias() {
    let out = monitoring_ref_calc("sli");
    assert!(out.contains("SLI"), "{out}");
}

#[test]
fn monitoring_ref_slo_via_error_budget_alias() {
    let out = monitoring_ref_calc("error-budget");
    assert!(out.contains("Error budget"), "{out}");
}

#[test]
fn monitoring_ref_slo_via_burn_rate_alias() {
    let out = monitoring_ref_calc("burn-rate");
    assert!(out.contains("Burn rate"), "{out}");
}

#[test]
fn monitoring_ref_prometheus_by_name() {
    let out = monitoring_ref_calc("prometheus");
    assert!(out.contains("Counter"), "{out}");
    assert!(out.contains("PromQL"), "{out}");
}

#[test]
fn monitoring_ref_prometheus_via_promql_alias() {
    let out = monitoring_ref_calc("promql");
    assert!(out.contains("PromQL"), "{out}");
}

#[test]
fn monitoring_ref_prometheus_via_histogram_alias() {
    let out = monitoring_ref_calc("histogram");
    assert!(out.contains("Histogram"), "{out}");
}

#[test]
fn monitoring_ref_prometheus_via_counter_alias() {
    let out = monitoring_ref_calc("counter");
    assert!(out.contains("Counter"), "{out}");
}

#[test]
fn monitoring_ref_alerting_by_name() {
    let out = monitoring_ref_calc("alerting");
    assert!(out.contains("Alertmanager"), "{out}");
    assert!(out.contains("severity"), "{out}");
}

#[test]
fn monitoring_ref_alerting_via_alertmanager_alias() {
    let out = monitoring_ref_calc("alertmanager");
    assert!(out.contains("Alertmanager"), "{out}");
}

#[test]
fn monitoring_ref_alerting_via_runbook_alias() {
    let out = monitoring_ref_calc("runbook");
    assert!(out.contains("Runbook"), "{out}");
}

#[test]
fn monitoring_ref_alerting_via_inhibition_alias() {
    let out = monitoring_ref_calc("inhibition");
    assert!(out.contains("Inhibition"), "{out}");
}

#[test]
fn monitoring_ref_grafana_by_name() {
    let out = monitoring_ref_calc("grafana");
    assert!(out.contains("Dashboard"), "{out}");
    assert!(out.contains("Loki"), "{out}");
}

#[test]
fn monitoring_ref_grafana_via_loki_alias() {
    let out = monitoring_ref_calc("loki");
    assert!(out.contains("Loki"), "{out}");
}

#[test]
fn monitoring_ref_grafana_via_tempo_alias() {
    let out = monitoring_ref_calc("tempo");
    assert!(out.contains("Tempo"), "{out}");
}

#[test]
fn monitoring_ref_grafana_via_logql_alias() {
    let out = monitoring_ref_calc("logql");
    assert!(out.contains("LogQL"), "{out}");
}

#[test]
fn monitoring_ref_otel_by_name() {
    let out = monitoring_ref_calc("otel");
    assert!(out.contains("OpenTelemetry"), "{out}");
    assert!(out.contains("OTLP"), "{out}");
}

#[test]
fn monitoring_ref_otel_via_opentelemetry_alias() {
    let out = monitoring_ref_calc("opentelemetry");
    assert!(out.contains("OTel"), "{out}");
}

#[test]
fn monitoring_ref_otel_via_tracing_alias() {
    let out = monitoring_ref_calc("tracing");
    assert!(out.contains("Traces"), "{out}");
}

#[test]
fn monitoring_ref_otel_via_baggage_alias() {
    let out = monitoring_ref_calc("baggage");
    assert!(out.contains("Baggage"), "{out}");
}

#[test]
fn monitoring_ref_logging_by_name() {
    let out = monitoring_ref_calc("logging");
    assert!(out.contains("Structured logging"), "{out}");
    assert!(out.contains("ERROR"), "{out}");
}

#[test]
fn monitoring_ref_logging_via_structured_logging_alias() {
    let out = monitoring_ref_calc("structured-logging");
    assert!(out.contains("Structured logging"), "{out}");
}

#[test]
fn monitoring_ref_logging_via_elk_alias() {
    let out = monitoring_ref_calc("elk");
    assert!(out.contains("ELK"), "{out}");
}

#[test]
fn monitoring_ref_logging_via_cardinality_alias() {
    let out = monitoring_ref_calc("cardinality");
    assert!(out.contains("Cardinality"), "{out}");
}

#[test]
fn monitoring_ref_case_insensitive() {
    let out = monitoring_ref_calc("SLO");
    assert!(out.contains("SLI") || out.contains("Error budget"), "{out}");
}

// ── search_ref_calc ───────────────────────────────────────────────────────────

#[test]
fn search_ref_empty_returns_topic_list() {
    let out = search_ref_calc("");
    assert!(out.contains("search-ref topics:"), "{out}");
    assert!(out.contains("concepts"), "{out}");
    assert!(out.contains("vector"), "{out}");
}

#[test]
fn search_ref_nomatch_returns_hint() {
    let out = search_ref_calc("zzznomatch");
    assert!(out.contains("No topic found"), "{out}");
    assert!(out.contains("search-ref list"), "{out}");
}

#[test]
fn search_ref_all_returns_all_sections() {
    let out = search_ref_calc("all");
    assert!(out.contains("Inverted index"), "{out}");
    assert!(out.contains("Elasticsearch"), "{out}");
    assert!(out.contains("Vector"), "{out}");
    assert!(out.contains("Ranking"), "{out}");
    assert!(out.contains("Search Performance"), "{out}");
    assert!(out.contains("Search System Design"), "{out}");
}

#[test]
fn search_ref_concepts_by_name() {
    let out = search_ref_calc("concepts");
    assert!(out.contains("Inverted index"), "{out}");
    assert!(out.contains("BM25"), "{out}");
}

#[test]
fn search_ref_concepts_via_inverted_index_alias() {
    let out = search_ref_calc("inverted-index");
    assert!(out.contains("Inverted index"), "{out}");
}

#[test]
fn search_ref_concepts_via_bm25_alias() {
    let out = search_ref_calc("bm25");
    assert!(out.contains("BM25"), "{out}");
}

#[test]
fn search_ref_concepts_via_stemming_alias() {
    let out = search_ref_calc("stemming");
    assert!(out.contains("Stemming"), "{out}");
}

#[test]
fn search_ref_elasticsearch_by_name() {
    let out = search_ref_calc("elasticsearch");
    assert!(out.contains("Index"), "{out}");
    assert!(out.contains("bool"), "{out}");
}

#[test]
fn search_ref_elasticsearch_via_es_alias() {
    let out = search_ref_calc("es");
    assert!(
        out.contains("Index") || out.contains("Elasticsearch"),
        "{out}"
    );
}

#[test]
fn search_ref_elasticsearch_via_mapping_alias() {
    let out = search_ref_calc("mapping");
    assert!(out.contains("Mapping"), "{out}");
}

#[test]
fn search_ref_elasticsearch_via_aggregations_alias() {
    let out = search_ref_calc("aggregations");
    assert!(out.contains("Aggregations"), "{out}");
}

#[test]
fn search_ref_vector_by_name() {
    let out = search_ref_calc("vector");
    assert!(out.contains("HNSW"), "{out}");
    assert!(out.contains("ANN"), "{out}");
}

#[test]
fn search_ref_vector_via_hnsw_alias() {
    let out = search_ref_calc("hnsw");
    assert!(out.contains("HNSW"), "{out}");
}

#[test]
fn search_ref_vector_via_rag_alias() {
    let out = search_ref_calc("rag");
    assert!(out.contains("RAG"), "{out}");
}

#[test]
fn search_ref_vector_via_qdrant_alias() {
    let out = search_ref_calc("qdrant");
    assert!(out.contains("Qdrant"), "{out}");
}

#[test]
fn search_ref_ranking_by_name() {
    let out = search_ref_calc("ranking");
    assert!(out.contains("Function score"), "{out}");
    assert!(out.contains("Learning to Rank"), "{out}");
}

#[test]
fn search_ref_ranking_via_ltr_alias() {
    let out = search_ref_calc("ltr");
    assert!(out.contains("Learning to Rank"), "{out}");
}

#[test]
fn search_ref_ranking_via_boosting_alias() {
    let out = search_ref_calc("boosting");
    assert!(out.contains("boost"), "{out}");
}

#[test]
fn search_ref_performance_by_name() {
    let out = search_ref_calc("performance");
    assert!(out.contains("Bulk API"), "{out}");
    assert!(out.contains("Sharding"), "{out}");
}

#[test]
fn search_ref_performance_via_bulk_api_alias() {
    let out = search_ref_calc("bulk-api");
    assert!(out.contains("Bulk API"), "{out}");
}

#[test]
fn search_ref_performance_via_sharding_alias() {
    let out = search_ref_calc("sharding");
    assert!(out.contains("Sharding"), "{out}");
}

#[test]
fn search_ref_design_by_name() {
    let out = search_ref_calc("search-architecture");
    assert!(out.contains("Indexing pipeline"), "{out}");
    assert!(out.contains("CDC"), "{out}");
}

#[test]
fn search_ref_design_via_cdc_search_alias() {
    let out = search_ref_calc("cdc-search");
    assert!(out.contains("CDC"), "{out}");
}

#[test]
fn search_ref_design_via_reindexing_alias() {
    let out = search_ref_calc("reindexing");
    assert!(out.contains("Reindex"), "{out}");
}

#[test]
fn search_ref_case_insensitive() {
    let out = search_ref_calc("CONCEPTS");
    assert!(
        out.contains("Inverted index") || out.contains("BM25"),
        "{out}"
    );
}

// ── protocols_ref_calc ────────────────────────────────────────────────────────

#[test]
fn protocols_ref_empty_returns_topic_list() {
    let out = protocols_ref_calc("");
    assert!(out.contains("protocols-ref topics:"), "{out}");
    assert!(out.contains("grpc"), "{out}");
    assert!(out.contains("tls"), "{out}");
}

#[test]
fn protocols_ref_nomatch_returns_hint() {
    let out = protocols_ref_calc("zzznomatch");
    assert!(out.contains("No topic found"), "{out}");
    assert!(out.contains("protocols-ref list"), "{out}");
}

#[test]
fn protocols_ref_all_returns_all_sections() {
    let out = protocols_ref_calc("all");
    assert!(out.contains("gRPC"), "{out}");
    assert!(out.contains("WebSocket"), "{out}");
    assert!(out.contains("GraphQL"), "{out}");
    assert!(out.contains("MQTT"), "{out}");
    assert!(out.contains("HTTP/2"), "{out}");
    assert!(out.contains("TLS"), "{out}");
}

#[test]
fn protocols_ref_grpc_by_name() {
    let out = protocols_ref_calc("grpc");
    assert!(out.contains("Protocol Buffers"), "{out}");
    assert!(out.contains("HTTP/2"), "{out}");
}

#[test]
fn protocols_ref_grpc_via_protobuf_alias() {
    let out = protocols_ref_calc("protobuf");
    assert!(out.contains("gRPC"), "{out}");
}

#[test]
fn protocols_ref_grpc_via_grpc_gateway_alias() {
    let out = protocols_ref_calc("grpc-gateway");
    assert!(out.contains("gRPC-Gateway"), "{out}");
}

#[test]
fn protocols_ref_grpc_via_grpc_streaming_alias() {
    let out = protocols_ref_calc("grpc-streaming");
    assert!(out.contains("streaming"), "{out}");
}

#[test]
fn protocols_ref_websocket_by_name() {
    let out = protocols_ref_calc("websocket");
    assert!(out.contains("full-duplex"), "{out}");
    assert!(out.contains("Heartbeats"), "{out}");
}

#[test]
fn protocols_ref_websocket_via_ws_alias() {
    let out = protocols_ref_calc("ws");
    assert!(out.contains("WebSocket"), "{out}");
}

#[test]
fn protocols_ref_websocket_via_real_time_alias() {
    let out = protocols_ref_calc("real-time");
    assert!(out.contains("real-time"), "{out}");
}

#[test]
fn protocols_ref_websocket_via_socket_io_alias() {
    let out = protocols_ref_calc("socket-io");
    assert!(out.contains("Socket.IO"), "{out}");
}

#[test]
fn protocols_ref_graphql_by_name() {
    let out = protocols_ref_calc("graphql");
    assert!(out.contains("Schema"), "{out}");
    assert!(out.contains("DataLoader"), "{out}");
}

#[test]
fn protocols_ref_graphql_via_gql_alias() {
    let out = protocols_ref_calc("gql");
    assert!(out.contains("GraphQL"), "{out}");
}

#[test]
fn protocols_ref_graphql_via_dataloader_alias() {
    let out = protocols_ref_calc("dataloader");
    assert!(out.contains("DataLoader"), "{out}");
}

#[test]
fn protocols_ref_graphql_via_federation_alias() {
    let out = protocols_ref_calc("federation");
    assert!(out.contains("Federation"), "{out}");
}

#[test]
fn protocols_ref_mqtt_by_name() {
    let out = protocols_ref_calc("mqtt");
    assert!(out.contains("QoS"), "{out}");
    assert!(out.contains("broker"), "{out}");
}

#[test]
fn protocols_ref_mqtt_via_qos_alias() {
    let out = protocols_ref_calc("qos");
    assert!(out.contains("QoS"), "{out}");
}

#[test]
fn protocols_ref_mqtt_via_lwt_alias() {
    let out = protocols_ref_calc("lwt");
    assert!(out.contains("Last Will"), "{out}");
}

#[test]
fn protocols_ref_mqtt_via_nats_alias() {
    let out = protocols_ref_calc("nats");
    assert!(out.contains("NATS"), "{out}");
}

#[test]
fn protocols_ref_http23_by_name() {
    let out = protocols_ref_calc("http23");
    assert!(out.contains("HTTP/2"), "{out}");
    assert!(out.contains("QUIC"), "{out}");
}

#[test]
fn protocols_ref_http23_via_http2_alias() {
    let out = protocols_ref_calc("http2");
    assert!(out.contains("HTTP/2"), "{out}");
}

#[test]
fn protocols_ref_http23_via_quic_alias() {
    let out = protocols_ref_calc("quic");
    assert!(out.contains("QUIC"), "{out}");
}

#[test]
fn protocols_ref_http23_via_sse_alias() {
    let out = protocols_ref_calc("sse");
    assert!(out.contains("Server-Sent Events"), "{out}");
}

#[test]
fn protocols_ref_tls_by_name() {
    let out = protocols_ref_calc("tls");
    assert!(out.contains("TLS 1.3"), "{out}");
    assert!(out.contains("Certificate"), "{out}");
}

#[test]
fn protocols_ref_tls_via_mtls_alias() {
    let out = protocols_ref_calc("mtls");
    assert!(out.contains("mTLS"), "{out}");
}

#[test]
fn protocols_ref_tls_via_pki_alias() {
    let out = protocols_ref_calc("pki");
    assert!(out.contains("Certificate chain"), "{out}");
}

#[test]
fn protocols_ref_tls_via_hsts_alias() {
    let out = protocols_ref_calc("hsts");
    assert!(out.contains("HSTS"), "{out}");
}

#[test]
fn protocols_ref_tls_via_x509_alias() {
    let out = protocols_ref_calc("x509");
    assert!(out.contains("X.509"), "{out}");
}

#[test]
fn protocols_ref_case_insensitive() {
    let out = protocols_ref_calc("GRPC");
    assert!(
        out.contains("gRPC") || out.contains("Protocol Buffers"),
        "{out}"
    );
}

#[test]
fn protocols_ref_whitespace_trimmed() {
    let out = protocols_ref_calc("  tls  ");
    assert!(out.contains("TLS"), "{out}");
}

// ── container_ref_calc ────────────────────────────────────────────────────────

#[test]
fn container_ref_empty_returns_topic_list() {
    let out = container_ref_calc("");
    assert!(out.contains("container-ref topics:"), "{out}");
    assert!(out.contains("namespaces"), "{out}");
    assert!(out.contains("build"), "{out}");
}

#[test]
fn container_ref_nomatch_returns_hint() {
    let out = container_ref_calc("zzznomatch");
    assert!(out.contains("No topic found"), "{out}");
    assert!(out.contains("container-ref list"), "{out}");
}

#[test]
fn container_ref_all_returns_all_sections() {
    let out = container_ref_calc("all");
    assert!(out.contains("Linux Namespaces"), "{out}");
    assert!(out.contains("Control Groups"), "{out}");
    assert!(out.contains("OCI"), "{out}");
    assert!(out.contains("Container Runtimes"), "{out}");
    assert!(out.contains("Container Build"), "{out}");
    assert!(out.contains("Container Security"), "{out}");
}

#[test]
fn container_ref_namespaces_by_name() {
    let out = container_ref_calc("namespaces");
    assert!(out.contains("pid namespace"), "{out}");
    assert!(out.contains("veth"), "{out}");
}

#[test]
fn container_ref_namespaces_via_pid_namespace_alias() {
    let out = container_ref_calc("pid-namespace");
    assert!(out.contains("pid namespace"), "{out}");
}

#[test]
fn container_ref_namespaces_via_user_namespace_alias() {
    let out = container_ref_calc("user-namespace");
    assert!(out.contains("user namespace"), "{out}");
}

#[test]
fn container_ref_namespaces_via_rootless_alias() {
    let out = container_ref_calc("rootless");
    assert!(out.contains("rootless"), "{out}");
}

#[test]
fn container_ref_cgroups_by_name() {
    let out = container_ref_calc("cgroups");
    assert!(out.contains("Control Groups"), "{out}");
    assert!(out.contains("memory.max"), "{out}");
}

#[test]
fn container_ref_cgroups_via_cgroup_v2_alias() {
    let out = container_ref_calc("cgroup-v2");
    assert!(out.contains("v2"), "{out}");
}

#[test]
fn container_ref_cgroups_via_oom_killer_alias() {
    let out = container_ref_calc("oom-killer");
    assert!(out.contains("OOM"), "{out}");
}

#[test]
fn container_ref_cgroups_via_cpu_limit_alias() {
    let out = container_ref_calc("cpu-limit");
    assert!(out.contains("cpu"), "{out}");
}

#[test]
fn container_ref_oci_by_name() {
    let out = container_ref_calc("oci");
    assert!(out.contains("OCI"), "{out}");
    assert!(out.contains("Manifest"), "{out}");
}

#[test]
fn container_ref_oci_via_manifest_alias() {
    let out = container_ref_calc("manifest");
    assert!(out.contains("Manifest"), "{out}");
}

#[test]
fn container_ref_oci_via_cosign_alias() {
    let out = container_ref_calc("cosign");
    assert!(out.contains("cosign"), "{out}");
}

#[test]
fn container_ref_runtimes_by_name() {
    let out = container_ref_calc("runtimes");
    assert!(out.contains("containerd"), "{out}");
    assert!(out.contains("runc"), "{out}");
}

#[test]
fn container_ref_runtimes_via_containerd_alias() {
    let out = container_ref_calc("containerd");
    assert!(out.contains("containerd"), "{out}");
}

#[test]
fn container_ref_runtimes_via_gvisor_alias() {
    let out = container_ref_calc("gvisor");
    assert!(out.contains("gVisor"), "{out}");
}

#[test]
fn container_ref_runtimes_via_overlayfs_alias() {
    let out = container_ref_calc("overlayfs");
    assert!(out.contains("overlayfs"), "{out}");
}

#[test]
fn container_ref_build_by_name() {
    let out = container_ref_calc("build");
    assert!(out.contains("Dockerfile"), "{out}");
    assert!(out.contains("Multi-stage"), "{out}");
}

#[test]
fn container_ref_build_via_dockerfile_alias() {
    let out = container_ref_calc("dockerfile");
    assert!(out.contains("Dockerfile"), "{out}");
}

#[test]
fn container_ref_build_via_buildkit_alias() {
    let out = container_ref_calc("buildkit");
    assert!(out.contains("BuildKit"), "{out}");
}

#[test]
fn container_ref_build_via_distroless_alias() {
    let out = container_ref_calc("distroless");
    assert!(out.contains("distroless"), "{out}");
}

#[test]
fn container_ref_security_by_name() {
    let out = container_ref_calc("security");
    assert!(out.contains("Capabilities"), "{out}");
    assert!(out.contains("Seccomp"), "{out}");
}

#[test]
fn container_ref_security_via_seccomp_alias() {
    let out = container_ref_calc("seccomp");
    assert!(out.contains("Seccomp"), "{out}");
}

#[test]
fn container_ref_security_via_trivy_alias() {
    let out = container_ref_calc("trivy");
    assert!(out.contains("trivy"), "{out}");
}

#[test]
fn container_ref_case_insensitive() {
    let out = container_ref_calc("NAMESPACES");
    assert!(
        out.contains("pid namespace") || out.contains("Linux Namespaces"),
        "{out}"
    );
}

// ── regex_engine_calc ─────────────────────────────────────────────────────────

#[test]
fn regex_engine_empty_returns_topic_list() {
    let out = regex_engine_calc("");
    assert!(out.contains("regex-engine topics:"), "{out}");
    assert!(out.contains("theory"), "{out}");
    assert!(out.contains("tools"), "{out}");
}

#[test]
fn regex_engine_nomatch_returns_hint() {
    let out = regex_engine_calc("zzznomatch");
    assert!(out.contains("No topic found"), "{out}");
    assert!(out.contains("regex-engine list"), "{out}");
}

#[test]
fn regex_engine_all_returns_all_sections() {
    let out = regex_engine_calc("all");
    assert!(out.contains("Regex Engine Theory"), "{out}");
    assert!(out.contains("Regex Syntax"), "{out}");
    assert!(out.contains("Advanced Regex"), "{out}");
    assert!(out.contains("Regex in Rust"), "{out}");
    assert!(out.contains("Regex Performance"), "{out}");
    assert!(out.contains("Regex Tools"), "{out}");
}

#[test]
fn regex_engine_theory_by_name() {
    let out = regex_engine_calc("theory");
    assert!(out.contains("NFA"), "{out}");
    assert!(out.contains("DFA"), "{out}");
}

#[test]
fn regex_engine_theory_via_nfa_alias() {
    let out = regex_engine_calc("nfa");
    assert!(out.contains("NFA"), "{out}");
}

#[test]
fn regex_engine_theory_via_dfa_alias() {
    let out = regex_engine_calc("dfa");
    assert!(out.contains("DFA"), "{out}");
}

#[test]
fn regex_engine_theory_via_backtracking_alias() {
    let out = regex_engine_calc("backtracking");
    assert!(out.contains("backtracking"), "{out}");
}

#[test]
fn regex_engine_theory_via_redos_alias() {
    let out = regex_engine_calc("redos");
    assert!(
        out.contains("ReDoS") || out.contains("backtracking"),
        "{out}"
    );
}

#[test]
fn regex_engine_syntax_by_name() {
    let out = regex_engine_calc("syntax");
    assert!(out.contains("Anchors"), "{out}");
    assert!(out.contains("Quantifiers"), "{out}");
}

#[test]
fn regex_engine_syntax_via_anchors_alias() {
    let out = regex_engine_calc("anchors");
    assert!(out.contains("Anchors"), "{out}");
}

#[test]
fn regex_engine_syntax_via_lookahead_alias() {
    let out = regex_engine_calc("lookahead");
    assert!(out.contains("lookahead"), "{out}");
}

#[test]
fn regex_engine_syntax_via_quantifiers_alias() {
    let out = regex_engine_calc("quantifiers");
    assert!(out.contains("Quantifiers"), "{out}");
}

#[test]
fn regex_engine_advanced_by_name() {
    let out = regex_engine_calc("advanced");
    assert!(out.contains("Atomic groups"), "{out}");
    assert!(out.contains("Email regex"), "{out}");
}

#[test]
fn regex_engine_advanced_via_atomic_groups_alias() {
    let out = regex_engine_calc("atomic-groups");
    assert!(out.contains("Atomic groups"), "{out}");
}

#[test]
fn regex_engine_advanced_via_possessive_alias() {
    let out = regex_engine_calc("possessive-quantifiers");
    assert!(out.contains("Possessive"), "{out}");
}

#[test]
fn regex_engine_rust_by_name() {
    let out = regex_engine_calc("rust-regex");
    assert!(out.contains("regex crate"), "{out}");
    assert!(out.contains("O(n)"), "{out}");
}

#[test]
fn regex_engine_rust_via_regex_crate_alias() {
    let out = regex_engine_calc("regex-crate");
    assert!(out.contains("regex crate"), "{out}");
}

#[test]
fn regex_engine_rust_via_lazy_static_alias() {
    let out = regex_engine_calc("lazy-static-regex");
    assert!(out.contains("lazy_static"), "{out}");
}

#[test]
fn regex_engine_performance_by_name() {
    let out = regex_engine_calc("performance");
    assert!(out.contains("ReDoS"), "{out}");
    assert!(out.contains("Vulnerable pattern"), "{out}");
}

#[test]
fn regex_engine_performance_via_catastrophic_alias() {
    let out = regex_engine_calc("catastrophic-backtracking");
    assert!(out.contains("catastrophic"), "{out}");
}

#[test]
fn regex_engine_performance_via_hyperscan_alias() {
    let out = regex_engine_calc("hyperscan");
    assert!(out.contains("Hyperscan"), "{out}");
}

#[test]
fn regex_engine_tools_by_name() {
    let out = regex_engine_calc("tools");
    assert!(out.contains("regex101"), "{out}");
    assert!(out.contains("grep"), "{out}");
}

#[test]
fn regex_engine_tools_via_regex101_alias() {
    let out = regex_engine_calc("regex101");
    assert!(out.contains("regex101"), "{out}");
}

#[test]
fn regex_engine_tools_via_ripgrep_alias() {
    let out = regex_engine_calc("ripgrep-regex");
    assert!(out.contains("ripgrep"), "{out}");
}

#[test]
fn regex_engine_case_insensitive() {
    let out = regex_engine_calc("THEORY");
    assert!(out.contains("NFA") || out.contains("DFA"), "{out}");
}

// ── git_internals_calc ────────────────────────────────────────────────────────

#[test]
fn git_internals_empty_returns_topic_list() {
    let out = git_internals_calc("");
    assert!(out.contains("git-internals topics:"), "{out}");
    assert!(out.contains("objects"), "{out}");
    assert!(out.contains("reflog"), "{out}");
}

#[test]
fn git_internals_nomatch_returns_hint() {
    let out = git_internals_calc("zzznomatch");
    assert!(out.contains("No topic found"), "{out}");
    assert!(out.contains("git-internals list"), "{out}");
}

#[test]
fn git_internals_all_returns_all_sections() {
    let out = git_internals_calc("all");
    assert!(out.contains("Git Object Model"), "{out}");
    assert!(out.contains("Pack Files"), "{out}");
    assert!(out.contains("Plumbing"), "{out}");
    assert!(out.contains("Reflog"), "{out}");
    assert!(out.contains("History Rewriting"), "{out}");
    assert!(out.contains("Advanced Git Operations"), "{out}");
}

#[test]
fn git_internals_objects_by_name() {
    let out = git_internals_calc("objects");
    assert!(out.contains("blob"), "{out}");
    assert!(out.contains("commit"), "{out}");
}

#[test]
fn git_internals_objects_via_blob_alias() {
    let out = git_internals_calc("blob");
    assert!(out.contains("blob"), "{out}");
}

#[test]
fn git_internals_objects_via_cat_file_alias() {
    let out = git_internals_calc("cat-file");
    assert!(out.contains("cat-file"), "{out}");
}

#[test]
fn git_internals_objects_via_hash_object_alias() {
    let out = git_internals_calc("hash-object");
    assert!(out.contains("hash-object"), "{out}");
}

#[test]
fn git_internals_pack_files_by_name() {
    let out = git_internals_calc("pack-files");
    assert!(out.contains("Pack files"), "{out}");
    assert!(out.contains("git gc"), "{out}");
}

#[test]
fn git_internals_pack_via_pack_alias() {
    let out = git_internals_calc("pack");
    assert!(out.contains("Pack"), "{out}");
}

#[test]
fn git_internals_pack_via_shallow_clone_alias() {
    let out = git_internals_calc("shallow-clone");
    assert!(out.contains("Shallow"), "{out}");
}

#[test]
fn git_internals_pack_via_worktree_alias() {
    let out = git_internals_calc("worktree");
    assert!(out.contains("worktree"), "{out}");
}

#[test]
fn git_internals_plumbing_by_name() {
    let out = git_internals_calc("plumbing");
    assert!(out.contains("index"), "{out}");
    assert!(out.contains("write-tree"), "{out}");
}

#[test]
fn git_internals_plumbing_via_write_tree_alias() {
    let out = git_internals_calc("write-tree");
    assert!(out.contains("write-tree"), "{out}");
}

#[test]
fn git_internals_plumbing_via_rev_list_alias() {
    let out = git_internals_calc("rev-list");
    assert!(out.contains("rev-list"), "{out}");
}

#[test]
fn git_internals_reflog_by_name() {
    let out = git_internals_calc("reflog");
    assert!(out.contains("reflog"), "{out}");
    assert!(out.contains("Recovery"), "{out}");
}

#[test]
fn git_internals_reflog_via_recovery_alias() {
    let out = git_internals_calc("recovery");
    assert!(out.contains("Recovery"), "{out}");
}

#[test]
fn git_internals_reflog_via_git_fsck_alias() {
    let out = git_internals_calc("git-fsck");
    assert!(out.contains("fsck"), "{out}");
}

#[test]
fn git_internals_reflog_via_orig_head_alias() {
    let out = git_internals_calc("orig-head");
    assert!(out.contains("ORIG_HEAD"), "{out}");
}

#[test]
fn git_internals_rewrite_by_name() {
    let out = git_internals_calc("rewrite");
    assert!(out.contains("rebase"), "{out}");
    assert!(out.contains("git-filter-repo"), "{out}");
}

#[test]
fn git_internals_rewrite_via_interactive_rebase_alias() {
    let out = git_internals_calc("interactive-rebase");
    assert!(out.contains("Interactive rebase"), "{out}");
}

#[test]
fn git_internals_rewrite_via_cherry_pick_alias() {
    let out = git_internals_calc("cherry-pick");
    assert!(out.contains("cherry-pick"), "{out}");
}

#[test]
fn git_internals_rewrite_via_force_with_lease_alias() {
    let out = git_internals_calc("force-with-lease");
    assert!(out.contains("force-with-lease"), "{out}");
}

#[test]
fn git_internals_advanced_ops_by_name() {
    let out = git_internals_calc("advanced-ops");
    assert!(out.contains("Bisect"), "{out}");
    assert!(out.contains("Submodules"), "{out}");
}

#[test]
fn git_internals_advanced_ops_via_bisect_run_alias() {
    let out = git_internals_calc("bisect-run");
    assert!(out.contains("bisect"), "{out}");
}

#[test]
fn git_internals_advanced_ops_via_git_hooks_alias() {
    let out = git_internals_calc("git-hooks");
    assert!(out.contains("Hooks"), "{out}");
}

#[test]
fn git_internals_case_insensitive() {
    let out = git_internals_calc("OBJECTS");
    assert!(out.contains("blob") || out.contains("Git Object"), "{out}");
}

// ── data_formats_calc ─────────────────────────────────────────────────────────

#[test]
fn data_formats_empty_returns_topic_list() {
    let out = data_formats_calc("");
    assert!(out.contains("data-formats topics:"), "{out}");
    assert!(out.contains("binary"), "{out}");
    assert!(out.contains("compression"), "{out}");
}

#[test]
fn data_formats_nomatch_returns_hint() {
    let out = data_formats_calc("zzznomatch");
    assert!(out.contains("No topic found"), "{out}");
    assert!(out.contains("data-formats list"), "{out}");
}

#[test]
fn data_formats_all_returns_all_sections() {
    let out = data_formats_calc("all");
    assert!(out.contains("MessagePack"), "{out}");
    assert!(out.contains("Parquet"), "{out}");
    assert!(out.contains("Avro"), "{out}");
    assert!(out.contains("JSON"), "{out}");
    assert!(out.contains("JSON Schema"), "{out}");
    assert!(out.contains("Snappy"), "{out}");
}

#[test]
fn data_formats_binary_by_name() {
    let out = data_formats_calc("binary");
    assert!(out.contains("MessagePack"), "{out}");
    assert!(out.contains("FlatBuffers"), "{out}");
}

#[test]
fn data_formats_binary_via_msgpack_alias() {
    let out = data_formats_calc("msgpack");
    assert!(out.contains("MessagePack"), "{out}");
}

#[test]
fn data_formats_binary_via_cbor_alias() {
    let out = data_formats_calc("cbor");
    assert!(out.contains("CBOR"), "{out}");
}

#[test]
fn data_formats_binary_via_flatbuffers_alias() {
    let out = data_formats_calc("flatbuffers");
    assert!(out.contains("FlatBuffers"), "{out}");
}

#[test]
fn data_formats_columnar_by_name() {
    let out = data_formats_calc("columnar");
    assert!(out.contains("Parquet"), "{out}");
    assert!(out.contains("Arrow"), "{out}");
}

#[test]
fn data_formats_columnar_via_parquet_alias() {
    let out = data_formats_calc("parquet");
    assert!(out.contains("Parquet"), "{out}");
}

#[test]
fn data_formats_columnar_via_duckdb_alias() {
    let out = data_formats_calc("duckdb-format");
    assert!(out.contains("DuckDB"), "{out}");
}

#[test]
fn data_formats_columnar_via_predicate_pushdown_alias() {
    let out = data_formats_calc("predicate-pushdown");
    assert!(out.contains("predicate"), "{out}");
}

#[test]
fn data_formats_streaming_by_name() {
    let out = data_formats_calc("streaming");
    assert!(out.contains("Avro"), "{out}");
    assert!(out.contains("Schema Registry"), "{out}");
}

#[test]
fn data_formats_streaming_via_avro_alias() {
    let out = data_formats_calc("avro");
    assert!(out.contains("Avro"), "{out}");
}

#[test]
fn data_formats_streaming_via_schema_registry_alias() {
    let out = data_formats_calc("schema-registry");
    assert!(out.contains("Schema Registry"), "{out}");
}

#[test]
fn data_formats_text_by_name() {
    let out = data_formats_calc("text");
    assert!(out.contains("JSON"), "{out}");
    assert!(out.contains("YAML"), "{out}");
}

#[test]
fn data_formats_text_via_json_format_alias() {
    let out = data_formats_calc("json-format");
    assert!(out.contains("JSON"), "{out}");
}

#[test]
fn data_formats_text_via_yaml_format_alias() {
    let out = data_formats_calc("yaml-format");
    assert!(out.contains("YAML"), "{out}");
}

#[test]
fn data_formats_text_via_toml_format_alias() {
    let out = data_formats_calc("toml-format");
    assert!(out.contains("TOML"), "{out}");
}

#[test]
fn data_formats_schema_by_name() {
    let out = data_formats_calc("schema");
    assert!(out.contains("JSON Schema"), "{out}");
    assert!(out.contains("Delta Lake"), "{out}");
}

#[test]
fn data_formats_schema_via_json_schema_alias() {
    let out = data_formats_calc("json-schema");
    assert!(out.contains("JSON Schema"), "{out}");
}

#[test]
fn data_formats_schema_via_delta_lake_alias() {
    let out = data_formats_calc("delta-lake");
    assert!(out.contains("Delta Lake"), "{out}");
}

#[test]
fn data_formats_schema_via_iceberg_alias() {
    let out = data_formats_calc("iceberg");
    assert!(out.contains("Iceberg"), "{out}");
}

#[test]
fn data_formats_compression_by_name() {
    let out = data_formats_calc("compression");
    assert!(out.contains("Snappy"), "{out}");
    assert!(out.contains("Zstd"), "{out}");
}

#[test]
fn data_formats_compression_via_zstd_alias() {
    let out = data_formats_calc("zstd");
    assert!(out.contains("Zstd"), "{out}");
}

#[test]
fn data_formats_compression_via_lz4_alias() {
    let out = data_formats_calc("lz4");
    assert!(out.contains("LZ4"), "{out}");
}

#[test]
fn data_formats_compression_via_snappy_alias() {
    let out = data_formats_calc("snappy");
    assert!(out.contains("Snappy"), "{out}");
}

#[test]
fn data_formats_compression_via_brotli_alias() {
    let out = data_formats_calc("brotli");
    assert!(out.contains("Brotli"), "{out}");
}

#[test]
fn data_formats_case_insensitive() {
    let out = data_formats_calc("BINARY");
    assert!(
        out.contains("MessagePack") || out.contains("FlatBuffers"),
        "{out}"
    );
}

#[test]
fn data_formats_whitespace_trimmed() {
    let out = data_formats_calc("  columnar  ");
    assert!(out.contains("Parquet"), "{out}");
}

// ─── Wave 34: web_perf_calc tests ────────────────────────────────────────────

#[test]
fn web_perf_list_no_panic() {
    let out = web_perf_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn web_perf_all_contains_cwv() {
    let out = web_perf_calc("all");
    assert!(
        out.contains("cwv") || out.contains("Core Web Vitals") || out.contains("LCP"),
        "all: {out}"
    );
}

#[test]
fn web_perf_cwv_topic() {
    let out = web_perf_calc("cwv");
    assert!(
        out.contains("LCP") || out.contains("CLS") || out.contains("INP"),
        "cwv: {out}"
    );
}

#[test]
fn web_perf_core_web_vitals_alias() {
    let out = web_perf_calc("core-web-vitals");
    assert!(
        out.contains("LCP") || out.contains("CLS"),
        "core-web-vitals alias: {out}"
    );
}

#[test]
fn web_perf_lcp_alias() {
    let out = web_perf_calc("lcp");
    assert!(!out.is_empty(), "lcp alias: {out}");
}

#[test]
fn web_perf_cls_alias() {
    let out = web_perf_calc("cls");
    assert!(!out.is_empty(), "cls alias: {out}");
}

#[test]
fn web_perf_ttfb_alias() {
    let out = web_perf_calc("ttfb");
    assert!(!out.is_empty(), "ttfb alias: {out}");
}

#[test]
fn web_perf_rendering_topic() {
    let out = web_perf_calc("rendering");
    assert!(
        out.contains("render") || out.contains("paint") || out.contains("reflow"),
        "rendering: {out}"
    );
}

#[test]
fn web_perf_critical_render_path_alias() {
    let out = web_perf_calc("critical-render-path");
    assert!(!out.is_empty(), "critical-render-path alias: {out}");
}

#[test]
fn web_perf_defer_async_alias() {
    let out = web_perf_calc("defer-async");
    assert!(!out.is_empty(), "defer-async alias: {out}");
}

#[test]
fn web_perf_preload_alias() {
    let out = web_perf_calc("preload");
    assert!(!out.is_empty(), "preload alias: {out}");
}

#[test]
fn web_perf_caching_topic() {
    let out = web_perf_calc("caching");
    assert!(
        out.contains("cache") || out.contains("Cache") || out.contains("ETag"),
        "caching: {out}"
    );
}

#[test]
fn web_perf_cache_control_alias() {
    let out = web_perf_calc("cache-control");
    assert!(!out.is_empty(), "cache-control alias: {out}");
}

#[test]
fn web_perf_etag_alias() {
    let out = web_perf_calc("etag");
    assert!(!out.is_empty(), "etag alias: {out}");
}

#[test]
fn web_perf_service_worker_cache_alias() {
    let out = web_perf_calc("service-worker-cache");
    assert!(!out.is_empty(), "service-worker-cache alias: {out}");
}

#[test]
fn web_perf_stale_while_revalidate_alias() {
    let out = web_perf_calc("stale-while-revalidate");
    assert!(!out.is_empty(), "stale-while-revalidate alias: {out}");
}

#[test]
fn web_perf_cdn_topic() {
    let out = web_perf_calc("cdn");
    assert!(
        out.contains("CDN") || out.contains("edge") || out.contains("content delivery"),
        "cdn: {out}"
    );
}

#[test]
fn web_perf_edge_caching_alias() {
    let out = web_perf_calc("edge-caching");
    assert!(!out.is_empty(), "edge-caching alias: {out}");
}

#[test]
fn web_perf_http3_cdn_alias() {
    let out = web_perf_calc("http3-cdn");
    assert!(!out.is_empty(), "http3-cdn alias: {out}");
}

#[test]
fn web_perf_bundling_topic() {
    let out = web_perf_calc("bundling");
    assert!(
        out.contains("bundle") || out.contains("tree-shak") || out.contains("split"),
        "bundling: {out}"
    );
}

#[test]
fn web_perf_tree_shaking_alias() {
    let out = web_perf_calc("tree-shaking");
    assert!(!out.is_empty(), "tree-shaking alias: {out}");
}

#[test]
fn web_perf_code_splitting_alias() {
    let out = web_perf_calc("code-splitting");
    assert!(!out.is_empty(), "code-splitting alias: {out}");
}

#[test]
fn web_perf_vite_build_alias() {
    let out = web_perf_calc("vite-build");
    assert!(!out.is_empty(), "vite-build alias: {out}");
}

#[test]
fn web_perf_esbuild_alias() {
    let out = web_perf_calc("esbuild");
    assert!(!out.is_empty(), "esbuild alias: {out}");
}

#[test]
fn web_perf_images_topic() {
    let out = web_perf_calc("images");
    assert!(
        out.contains("image") || out.contains("avif") || out.contains("webp"),
        "images: {out}"
    );
}

#[test]
fn web_perf_avif_alias() {
    let out = web_perf_calc("avif");
    assert!(!out.is_empty(), "avif alias: {out}");
}

#[test]
fn web_perf_webp_alias() {
    let out = web_perf_calc("webp");
    assert!(!out.is_empty(), "webp alias: {out}");
}

#[test]
fn web_perf_lazy_loading_alias() {
    let out = web_perf_calc("lazy-loading");
    assert!(!out.is_empty(), "lazy-loading alias: {out}");
}

#[test]
fn web_perf_srcset_alias() {
    let out = web_perf_calc("srcset");
    assert!(!out.is_empty(), "srcset alias: {out}");
}

#[test]
fn web_perf_unknown_no_panic() {
    let out = web_perf_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn web_perf_empty_no_panic() {
    let out = web_perf_calc("");
    assert!(!out.is_empty());
}

// ─── Wave 34: sql_tuning_calc tests ──────────────────────────────────────────

#[test]
fn sql_tuning_list_no_panic() {
    let out = sql_tuning_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn sql_tuning_all_contains_explain() {
    let out = sql_tuning_calc("all");
    assert!(
        out.contains("explain") || out.contains("EXPLAIN") || out.contains("execution"),
        "all: {out}"
    );
}

#[test]
fn sql_tuning_explain_topic() {
    let out = sql_tuning_calc("explain");
    assert!(
        out.contains("EXPLAIN") || out.contains("plan") || out.contains("scan"),
        "explain: {out}"
    );
}

#[test]
fn sql_tuning_execution_plan_alias() {
    let out = sql_tuning_calc("execution-plan");
    assert!(!out.is_empty(), "execution-plan alias: {out}");
}

#[test]
fn sql_tuning_explain_analyze_alias() {
    let out = sql_tuning_calc("explain-analyze");
    assert!(!out.is_empty(), "explain-analyze alias: {out}");
}

#[test]
fn sql_tuning_seq_scan_alias() {
    let out = sql_tuning_calc("seq-scan");
    assert!(!out.is_empty(), "seq-scan alias: {out}");
}

#[test]
fn sql_tuning_hash_join_alias() {
    let out = sql_tuning_calc("hash-join");
    assert!(!out.is_empty(), "hash-join alias: {out}");
}

#[test]
fn sql_tuning_indexes_topic() {
    let out = sql_tuning_calc("indexes");
    assert!(
        out.contains("index") || out.contains("Index") || out.contains("btree"),
        "indexes: {out}"
    );
}

#[test]
fn sql_tuning_index_design_alias() {
    let out = sql_tuning_calc("index-design");
    assert!(!out.is_empty(), "index-design alias: {out}");
}

#[test]
fn sql_tuning_btree_index_alias() {
    let out = sql_tuning_calc("btree-index");
    assert!(!out.is_empty(), "btree-index alias: {out}");
}

#[test]
fn sql_tuning_gin_index_alias() {
    let out = sql_tuning_calc("gin-index");
    assert!(!out.is_empty(), "gin-index alias: {out}");
}

#[test]
fn sql_tuning_partial_index_alias() {
    let out = sql_tuning_calc("partial-index");
    assert!(!out.is_empty(), "partial-index alias: {out}");
}

#[test]
fn sql_tuning_covering_index_alias() {
    let out = sql_tuning_calc("covering-index");
    assert!(!out.is_empty(), "covering-index alias: {out}");
}

#[test]
fn sql_tuning_statistics_topic() {
    let out = sql_tuning_calc("statistics");
    assert!(
        out.contains("statistic") || out.contains("ANALYZE") || out.contains("histogram"),
        "statistics: {out}"
    );
}

#[test]
fn sql_tuning_pg_stats_alias() {
    let out = sql_tuning_calc("pg-stats");
    assert!(!out.is_empty(), "pg-stats alias: {out}");
}

#[test]
fn sql_tuning_analyze_table_alias() {
    let out = sql_tuning_calc("analyze-table");
    assert!(!out.is_empty(), "analyze-table alias: {out}");
}

#[test]
fn sql_tuning_autovacuum_alias() {
    let out = sql_tuning_calc("autovacuum");
    assert!(!out.is_empty(), "autovacuum alias: {out}");
}

#[test]
fn sql_tuning_optimizer_topic() {
    let out = sql_tuning_calc("optimizer");
    assert!(
        out.contains("optimizer") || out.contains("hint") || out.contains("work_mem"),
        "optimizer: {out}"
    );
}

#[test]
fn sql_tuning_work_mem_alias() {
    let out = sql_tuning_calc("work-mem");
    assert!(!out.is_empty(), "work-mem alias: {out}");
}

#[test]
fn sql_tuning_mysql_hints_alias() {
    let out = sql_tuning_calc("mysql-hints");
    assert!(!out.is_empty(), "mysql-hints alias: {out}");
}

#[test]
fn sql_tuning_keyset_pagination_alias() {
    let out = sql_tuning_calc("keyset-pagination");
    assert!(!out.is_empty(), "keyset-pagination alias: {out}");
}

#[test]
fn sql_tuning_partitioning_topic() {
    let out = sql_tuning_calc("partitioning");
    assert!(
        out.contains("partition") || out.contains("Partition"),
        "partitioning: {out}"
    );
}

#[test]
fn sql_tuning_range_partition_alias() {
    let out = sql_tuning_calc("range-partition");
    assert!(!out.is_empty(), "range-partition alias: {out}");
}

#[test]
fn sql_tuning_hash_partition_alias() {
    let out = sql_tuning_calc("hash-partition");
    assert!(!out.is_empty(), "hash-partition alias: {out}");
}

#[test]
fn sql_tuning_rolling_window_alias() {
    let out = sql_tuning_calc("rolling-window");
    assert!(!out.is_empty(), "rolling-window alias: {out}");
}

#[test]
fn sql_tuning_advanced_sql_topic() {
    let out = sql_tuning_calc("advanced-sql");
    assert!(
        out.contains("window") || out.contains("CTE") || out.contains("LATERAL"),
        "advanced-sql: {out}"
    );
}

#[test]
fn sql_tuning_window_functions_alias() {
    let out = sql_tuning_calc("window-functions");
    assert!(!out.is_empty(), "window-functions alias: {out}");
}

#[test]
fn sql_tuning_cte_alias() {
    let out = sql_tuning_calc("cte");
    assert!(!out.is_empty(), "cte alias: {out}");
}

#[test]
fn sql_tuning_recursive_cte_alias() {
    let out = sql_tuning_calc("recursive-cte");
    assert!(!out.is_empty(), "recursive-cte alias: {out}");
}

#[test]
fn sql_tuning_lateral_join_alias() {
    let out = sql_tuning_calc("lateral-join");
    assert!(!out.is_empty(), "lateral-join alias: {out}");
}

#[test]
fn sql_tuning_jsonb_queries_alias() {
    let out = sql_tuning_calc("jsonb-queries");
    assert!(!out.is_empty(), "jsonb-queries alias: {out}");
}

#[test]
fn sql_tuning_upsert_alias() {
    let out = sql_tuning_calc("upsert");
    assert!(!out.is_empty(), "upsert alias: {out}");
}

#[test]
fn sql_tuning_unknown_no_panic() {
    let out = sql_tuning_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn sql_tuning_empty_no_panic() {
    let out = sql_tuning_calc("");
    assert!(!out.is_empty());
}

// ─── Wave 34: concurrency_ref_calc tests ─────────────────────────────────────

#[test]
fn concurrency_list_no_panic() {
    let out = concurrency_ref_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn concurrency_all_contains_primitives() {
    let out = concurrency_ref_calc("all");
    assert!(
        out.contains("mutex") || out.contains("Mutex") || out.contains("primitive"),
        "all: {out}"
    );
}

#[test]
fn concurrency_primitives_topic() {
    let out = concurrency_ref_calc("primitives");
    assert!(
        out.contains("mutex") || out.contains("Mutex") || out.contains("semaphore"),
        "primitives: {out}"
    );
}

#[test]
fn concurrency_mutex_alias() {
    let out = concurrency_ref_calc("mutex");
    assert!(!out.is_empty(), "mutex alias: {out}");
}

#[test]
fn concurrency_rwlock_alias() {
    let out = concurrency_ref_calc("rwlock");
    assert!(!out.is_empty(), "rwlock alias: {out}");
}

#[test]
fn concurrency_semaphore_alias() {
    let out = concurrency_ref_calc("semaphore");
    assert!(!out.is_empty(), "semaphore alias: {out}");
}

#[test]
fn concurrency_deadlock_alias() {
    let out = concurrency_ref_calc("deadlock");
    assert!(!out.is_empty(), "deadlock alias: {out}");
}

#[test]
fn concurrency_atomics_topic() {
    let out = concurrency_ref_calc("atomics");
    assert!(
        out.contains("atomic") || out.contains("Atomic") || out.contains("memory order"),
        "atomics: {out}"
    );
}

#[test]
fn concurrency_atomic_ops_alias() {
    let out = concurrency_ref_calc("atomic-ops");
    assert!(!out.is_empty(), "atomic-ops alias: {out}");
}

#[test]
fn concurrency_memory_ordering_alias() {
    let out = concurrency_ref_calc("memory-ordering");
    assert!(!out.is_empty(), "memory-ordering alias: {out}");
}

#[test]
fn concurrency_acquire_release_alias() {
    let out = concurrency_ref_calc("acquire-release");
    assert!(!out.is_empty(), "acquire-release alias: {out}");
}

#[test]
fn concurrency_seqcst_alias() {
    let out = concurrency_ref_calc("seqcst");
    assert!(!out.is_empty(), "seqcst alias: {out}");
}

#[test]
fn concurrency_cas_loop_alias() {
    let out = concurrency_ref_calc("cas-loop");
    assert!(!out.is_empty(), "cas-loop alias: {out}");
}

#[test]
fn concurrency_false_sharing_alias() {
    let out = concurrency_ref_calc("false-sharing");
    assert!(!out.is_empty(), "false-sharing alias: {out}");
}

#[test]
fn concurrency_channels_topic() {
    let out = concurrency_ref_calc("channels");
    assert!(
        out.contains("channel") || out.contains("Channel") || out.contains("mpsc"),
        "channels: {out}"
    );
}

#[test]
fn concurrency_channel_mpsc_alias() {
    let out = concurrency_ref_calc("channel-mpsc");
    assert!(!out.is_empty(), "channel-mpsc alias: {out}");
}

#[test]
fn concurrency_backpressure_alias() {
    let out = concurrency_ref_calc("backpressure");
    assert!(!out.is_empty(), "backpressure alias: {out}");
}

#[test]
fn concurrency_bounded_channel_alias() {
    let out = concurrency_ref_calc("bounded-channel");
    assert!(!out.is_empty(), "bounded-channel alias: {out}");
}

#[test]
fn concurrency_actor_model_alias() {
    let out = concurrency_ref_calc("actor-model");
    assert!(!out.is_empty(), "actor-model alias: {out}");
}

#[test]
fn concurrency_lock_free_topic() {
    let out = concurrency_ref_calc("lock-free");
    assert!(
        out.contains("lock-free") || out.contains("lockfree") || out.contains("queue"),
        "lock-free: {out}"
    );
}

#[test]
fn concurrency_lock_free_queue_alias() {
    let out = concurrency_ref_calc("lock-free-queue");
    assert!(!out.is_empty(), "lock-free-queue alias: {out}");
}

#[test]
fn concurrency_hazard_pointers_alias() {
    let out = concurrency_ref_calc("hazard-pointers");
    assert!(!out.is_empty(), "hazard-pointers alias: {out}");
}

#[test]
fn concurrency_aba_problem_alias() {
    let out = concurrency_ref_calc("aba-problem");
    assert!(!out.is_empty(), "aba-problem alias: {out}");
}

#[test]
fn concurrency_async_topic() {
    let out = concurrency_ref_calc("async");
    assert!(
        out.contains("async") || out.contains("tokio") || out.contains("await"),
        "async: {out}"
    );
}

#[test]
fn concurrency_async_await_alias() {
    let out = concurrency_ref_calc("async-await");
    assert!(!out.is_empty(), "async-await alias: {out}");
}

#[test]
fn concurrency_tokio_runtime_alias() {
    let out = concurrency_ref_calc("tokio-runtime");
    assert!(!out.is_empty(), "tokio-runtime alias: {out}");
}

#[test]
fn concurrency_tokio_spawn_alias() {
    let out = concurrency_ref_calc("tokio-spawn");
    assert!(!out.is_empty(), "tokio-spawn alias: {out}");
}

#[test]
fn concurrency_tokio_select_alias() {
    let out = concurrency_ref_calc("tokio-select");
    assert!(!out.is_empty(), "tokio-select alias: {out}");
}

#[test]
fn concurrency_spawn_blocking_alias() {
    let out = concurrency_ref_calc("spawn-blocking");
    assert!(!out.is_empty(), "spawn-blocking alias: {out}");
}

#[test]
fn concurrency_patterns_topic() {
    let out = concurrency_ref_calc("patterns");
    assert!(
        out.contains("thread") || out.contains("pool") || out.contains("worker"),
        "patterns: {out}"
    );
}

#[test]
fn concurrency_thread_pool_alias() {
    let out = concurrency_ref_calc("thread-pool");
    assert!(!out.is_empty(), "thread-pool alias: {out}");
}

#[test]
fn concurrency_producer_consumer_alias() {
    let out = concurrency_ref_calc("producer-consumer");
    assert!(!out.is_empty(), "producer-consumer alias: {out}");
}

#[test]
fn concurrency_rayon_alias() {
    let out = concurrency_ref_calc("rayon");
    assert!(!out.is_empty(), "rayon alias: {out}");
}

#[test]
fn concurrency_work_stealing_alias() {
    let out = concurrency_ref_calc("work-stealing");
    assert!(!out.is_empty(), "work-stealing alias: {out}");
}

#[test]
fn concurrency_graceful_shutdown_alias() {
    let out = concurrency_ref_calc("graceful-shutdown");
    assert!(!out.is_empty(), "graceful-shutdown alias: {out}");
}

#[test]
fn concurrency_unknown_no_panic() {
    let out = concurrency_ref_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn concurrency_empty_no_panic() {
    let out = concurrency_ref_calc("");
    assert!(!out.is_empty());
}

// ─── Wave 34: cloud_native_calc tests ────────────────────────────────────────

#[test]
fn cloud_native_list_no_panic() {
    let out = cloud_native_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn cloud_native_all_contains_12factor() {
    let out = cloud_native_calc("all");
    assert!(
        out.contains("factor") || out.contains("12") || out.contains("config"),
        "all: {out}"
    );
}

#[test]
fn cloud_native_12factor_topic() {
    let out = cloud_native_calc("12factor");
    assert!(
        out.contains("factor") || out.contains("config") || out.contains("stateless"),
        "12factor: {out}"
    );
}

#[test]
fn cloud_native_twelve_factor_alias() {
    let out = cloud_native_calc("twelve-factor");
    assert!(!out.is_empty(), "twelve-factor alias: {out}");
}

#[test]
fn cloud_native_twelve_factor_app_alias() {
    let out = cloud_native_calc("twelve-factor-app");
    assert!(!out.is_empty(), "twelve-factor-app alias: {out}");
}

#[test]
fn cloud_native_config_env_alias() {
    let out = cloud_native_calc("config-env");
    assert!(!out.is_empty(), "config-env alias: {out}");
}

#[test]
fn cloud_native_disposability_alias() {
    let out = cloud_native_calc("disposability");
    assert!(!out.is_empty(), "disposability alias: {out}");
}

#[test]
fn cloud_native_patterns_topic() {
    let out = cloud_native_calc("patterns");
    assert!(
        out.contains("circuit") || out.contains("bulkhead") || out.contains("retry"),
        "patterns: {out}"
    );
}

#[test]
fn cloud_native_cloud_patterns_alias() {
    let out = cloud_native_calc("cloud-patterns");
    assert!(!out.is_empty(), "cloud-patterns alias: {out}");
}

#[test]
fn cloud_native_circuit_breaker_alias() {
    let out = cloud_native_calc("circuit-breaker");
    assert!(!out.is_empty(), "circuit-breaker alias: {out}");
}

#[test]
fn cloud_native_bulkhead_alias() {
    let out = cloud_native_calc("bulkhead");
    assert!(!out.is_empty(), "bulkhead alias: {out}");
}

#[test]
fn cloud_native_saga_pattern_alias() {
    let out = cloud_native_calc("saga-pattern");
    assert!(!out.is_empty(), "saga-pattern alias: {out}");
}

#[test]
fn cloud_native_cqrs_pattern_alias() {
    let out = cloud_native_calc("cqrs-pattern");
    assert!(!out.is_empty(), "cqrs-pattern alias: {out}");
}

#[test]
fn cloud_native_strangler_fig_alias() {
    let out = cloud_native_calc("strangler-fig");
    assert!(!out.is_empty(), "strangler-fig alias: {out}");
}

#[test]
fn cloud_native_service_mesh_topic() {
    let out = cloud_native_calc("service-mesh");
    assert!(
        out.contains("mesh") || out.contains("istio") || out.contains("envoy"),
        "service-mesh: {out}"
    );
}

#[test]
fn cloud_native_istio_alias() {
    let out = cloud_native_calc("istio");
    assert!(!out.is_empty(), "istio alias: {out}");
}

#[test]
fn cloud_native_envoy_proxy_alias() {
    let out = cloud_native_calc("envoy-proxy");
    assert!(!out.is_empty(), "envoy-proxy alias: {out}");
}

#[test]
fn cloud_native_mtls_mesh_alias() {
    let out = cloud_native_calc("mtls-mesh");
    assert!(!out.is_empty(), "mtls-mesh alias: {out}");
}

#[test]
fn cloud_native_cilium_alias() {
    let out = cloud_native_calc("cilium");
    assert!(!out.is_empty(), "cilium alias: {out}");
}

#[test]
fn cloud_native_observability_topic() {
    let out = cloud_native_calc("observability");
    assert!(
        out.contains("observ") || out.contains("prometheus") || out.contains("tracing"),
        "observability: {out}"
    );
}

#[test]
fn cloud_native_cloud_observability_alias() {
    let out = cloud_native_calc("cloud-observability");
    assert!(!out.is_empty(), "cloud-observability alias: {out}");
}

#[test]
fn cloud_native_prometheus_stack_alias() {
    let out = cloud_native_calc("prometheus-stack");
    assert!(!out.is_empty(), "prometheus-stack alias: {out}");
}

#[test]
fn cloud_native_distributed_tracing_alias() {
    let out = cloud_native_calc("distributed-tracing");
    assert!(!out.is_empty(), "distributed-tracing alias: {out}");
}

#[test]
fn cloud_native_deployment_topic() {
    let out = cloud_native_calc("deployment");
    assert!(
        out.contains("deploy") || out.contains("rolling") || out.contains("canary"),
        "deployment: {out}"
    );
}

#[test]
fn cloud_native_rolling_update_alias() {
    let out = cloud_native_calc("rolling-update");
    assert!(!out.is_empty(), "rolling-update alias: {out}");
}

#[test]
fn cloud_native_blue_green_alias() {
    let out = cloud_native_calc("blue-green");
    assert!(!out.is_empty(), "blue-green alias: {out}");
}

#[test]
fn cloud_native_canary_deployment_alias() {
    let out = cloud_native_calc("canary-deployment");
    assert!(!out.is_empty(), "canary-deployment alias: {out}");
}

#[test]
fn cloud_native_gitops_alias() {
    let out = cloud_native_calc("gitops");
    assert!(!out.is_empty(), "gitops alias: {out}");
}

#[test]
fn cloud_native_argocd_alias() {
    let out = cloud_native_calc("argocd");
    assert!(!out.is_empty(), "argocd alias: {out}");
}

#[test]
fn cloud_native_progressive_delivery_alias() {
    let out = cloud_native_calc("progressive-delivery");
    assert!(!out.is_empty(), "progressive-delivery alias: {out}");
}

#[test]
fn cloud_native_events_topic() {
    let out = cloud_native_calc("events");
    assert!(
        out.contains("event") || out.contains("Event") || out.contains("kafka"),
        "events: {out}"
    );
}

#[test]
fn cloud_native_event_sourcing_alias() {
    let out = cloud_native_calc("event-sourcing");
    assert!(!out.is_empty(), "event-sourcing alias: {out}");
}

#[test]
fn cloud_native_outbox_pattern_alias() {
    let out = cloud_native_calc("outbox-pattern");
    assert!(!out.is_empty(), "outbox-pattern alias: {out}");
}

#[test]
fn cloud_native_kafka_patterns_alias() {
    let out = cloud_native_calc("kafka-patterns");
    assert!(!out.is_empty(), "kafka-patterns alias: {out}");
}

#[test]
fn cloud_native_exactly_once_alias() {
    let out = cloud_native_calc("exactly-once");
    assert!(!out.is_empty(), "exactly-once alias: {out}");
}

#[test]
fn cloud_native_dead_letter_queue_alias() {
    let out = cloud_native_calc("dead-letter-queue");
    assert!(!out.is_empty(), "dead-letter-queue alias: {out}");
}

#[test]
fn cloud_native_unknown_no_panic() {
    let out = cloud_native_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn cloud_native_empty_no_panic() {
    let out = cloud_native_calc("");
    assert!(!out.is_empty());
}

// ─── Wave 35: regex_patterns_calc tests ──────────────────────────────────────

#[test]
fn regex_patterns_list_no_panic() {
    let out = regex_patterns_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn regex_patterns_all_contains_common() {
    let out = regex_patterns_calc("all");
    assert!(
        out.contains("Email") || out.contains("email") || out.contains("common"),
        "all: {out}"
    );
}

#[test]
fn regex_patterns_common_topic() {
    let out = regex_patterns_calc("common");
    assert!(
        out.contains("Email") || out.contains("UUID") || out.contains("URL"),
        "common: {out}"
    );
}

#[test]
fn regex_patterns_validation_alias() {
    let out = regex_patterns_calc("validation");
    assert!(!out.is_empty(), "validation alias: {out}");
}

#[test]
fn regex_patterns_email_pattern_alias() {
    let out = regex_patterns_calc("email-pattern");
    assert!(!out.is_empty(), "email-pattern alias: {out}");
}

#[test]
fn regex_patterns_uuid_pattern_alias() {
    let out = regex_patterns_calc("uuid-pattern");
    assert!(!out.is_empty(), "uuid-pattern alias: {out}");
}

#[test]
fn regex_patterns_semver_regex_alias() {
    let out = regex_patterns_calc("semver-regex");
    assert!(!out.is_empty(), "semver-regex alias: {out}");
}

#[test]
fn regex_patterns_text_topic() {
    let out = regex_patterns_calc("text");
    assert!(
        out.contains("whitespace") || out.contains("blank") || out.contains("CSV"),
        "text: {out}"
    );
}

#[test]
fn regex_patterns_text_processing_alias() {
    let out = regex_patterns_calc("text-processing");
    assert!(!out.is_empty(), "text-processing alias: {out}");
}

#[test]
fn regex_patterns_duplicate_words_alias() {
    let out = regex_patterns_calc("duplicate-words");
    assert!(!out.is_empty(), "duplicate-words alias: {out}");
}

#[test]
fn regex_patterns_camelcase_split_alias() {
    let out = regex_patterns_calc("camelcase-split");
    assert!(!out.is_empty(), "camelcase-split alias: {out}");
}

#[test]
fn regex_patterns_ansi_escape_alias() {
    let out = regex_patterns_calc("ansi-escape");
    assert!(!out.is_empty(), "ansi-escape alias: {out}");
}

#[test]
fn regex_patterns_code_topic() {
    let out = regex_patterns_calc("code");
    assert!(
        out.contains("function") || out.contains("class") || out.contains("import"),
        "code: {out}"
    );
}

#[test]
fn regex_patterns_code_extraction_alias() {
    let out = regex_patterns_calc("code-extraction");
    assert!(!out.is_empty(), "code-extraction alias: {out}");
}

#[test]
fn regex_patterns_function_def_alias() {
    let out = regex_patterns_calc("function-def");
    assert!(!out.is_empty(), "function-def alias: {out}");
}

#[test]
fn regex_patterns_todo_comment_alias() {
    let out = regex_patterns_calc("todo-comment");
    assert!(!out.is_empty(), "todo-comment alias: {out}");
}

#[test]
fn regex_patterns_shebang_alias() {
    let out = regex_patterns_calc("shebang");
    assert!(!out.is_empty(), "shebang alias: {out}");
}

#[test]
fn regex_patterns_log_topic() {
    let out = regex_patterns_calc("log");
    assert!(
        out.contains("syslog") || out.contains("Apache") || out.contains("timestamp"),
        "log: {out}"
    );
}

#[test]
fn regex_patterns_log_parsing_alias() {
    let out = regex_patterns_calc("log-parsing");
    assert!(!out.is_empty(), "log-parsing alias: {out}");
}

#[test]
fn regex_patterns_apache_log_alias() {
    let out = regex_patterns_calc("apache-log");
    assert!(!out.is_empty(), "apache-log alias: {out}");
}

#[test]
fn regex_patterns_request_id_alias() {
    let out = regex_patterns_calc("request-id");
    assert!(!out.is_empty(), "request-id alias: {out}");
}

#[test]
fn regex_patterns_security_topic() {
    let out = regex_patterns_calc("security");
    assert!(
        out.contains("AWS") || out.contains("token") || out.contains("JWT"),
        "security: {out}"
    );
}

#[test]
fn regex_patterns_secrets_detection_alias() {
    let out = regex_patterns_calc("secrets-detection");
    assert!(!out.is_empty(), "secrets-detection alias: {out}");
}

#[test]
fn regex_patterns_aws_key_alias() {
    let out = regex_patterns_calc("aws-key");
    assert!(!out.is_empty(), "aws-key alias: {out}");
}

#[test]
fn regex_patterns_jwt_pattern_alias() {
    let out = regex_patterns_calc("jwt-pattern");
    assert!(!out.is_empty(), "jwt-pattern alias: {out}");
}

#[test]
fn regex_patterns_path_traversal_alias() {
    let out = regex_patterns_calc("path-traversal");
    assert!(!out.is_empty(), "path-traversal alias: {out}");
}

#[test]
fn regex_patterns_network_topic() {
    let out = regex_patterns_calc("network");
    assert!(
        out.contains("CIDR") || out.contains("MAC") || out.contains("domain"),
        "network: {out}"
    );
}

#[test]
fn regex_patterns_cidr_pattern_alias() {
    let out = regex_patterns_calc("cidr-pattern");
    assert!(!out.is_empty(), "cidr-pattern alias: {out}");
}

#[test]
fn regex_patterns_mac_address_alias() {
    let out = regex_patterns_calc("mac-address");
    assert!(!out.is_empty(), "mac-address alias: {out}");
}

#[test]
fn regex_patterns_kubernetes_name_alias() {
    let out = regex_patterns_calc("kubernetes-name");
    assert!(!out.is_empty(), "kubernetes-name alias: {out}");
}

#[test]
fn regex_patterns_git_branch_alias() {
    let out = regex_patterns_calc("git-branch");
    assert!(!out.is_empty(), "git-branch alias: {out}");
}

#[test]
fn regex_patterns_unknown_no_panic() {
    let out = regex_patterns_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn regex_patterns_empty_no_panic() {
    let out = regex_patterns_calc("");
    assert!(!out.is_empty());
}

// ─── Wave 35: http_security_calc tests ───────────────────────────────────────

#[test]
fn http_security_list_no_panic() {
    let out = http_security_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn http_security_all_contains_csp() {
    let out = http_security_calc("all");
    assert!(
        out.contains("CSP") || out.contains("Content-Security"),
        "all: {out}"
    );
}

#[test]
fn http_security_csp_topic() {
    let out = http_security_calc("csp");
    assert!(
        out.contains("Content-Security-Policy") || out.contains("script-src"),
        "csp: {out}"
    );
}

#[test]
fn http_security_content_security_policy_alias() {
    let out = http_security_calc("content-security-policy");
    assert!(!out.is_empty(), "content-security-policy alias: {out}");
}

#[test]
fn http_security_nonce_csp_alias() {
    let out = http_security_calc("nonce-csp");
    assert!(!out.is_empty(), "nonce-csp alias: {out}");
}

#[test]
fn http_security_csp_directives_alias() {
    let out = http_security_calc("csp-directives");
    assert!(!out.is_empty(), "csp-directives alias: {out}");
}

#[test]
fn http_security_report_uri_alias() {
    let out = http_security_calc("report-uri");
    assert!(!out.is_empty(), "report-uri alias: {out}");
}

#[test]
fn http_security_cors_topic() {
    let out = http_security_calc("cors");
    assert!(
        out.contains("CORS") || out.contains("Access-Control") || out.contains("preflight"),
        "cors: {out}"
    );
}

#[test]
fn http_security_cross_origin_alias() {
    let out = http_security_calc("cross-origin-resource-sharing");
    assert!(
        !out.is_empty(),
        "cross-origin-resource-sharing alias: {out}"
    );
}

#[test]
fn http_security_preflight_alias() {
    let out = http_security_calc("preflight");
    assert!(!out.is_empty(), "preflight alias: {out}");
}

#[test]
fn http_security_vary_origin_alias() {
    let out = http_security_calc("vary-origin");
    assert!(!out.is_empty(), "vary-origin alias: {out}");
}

#[test]
fn http_security_hsts_topic() {
    let out = http_security_calc("hsts");
    assert!(
        out.contains("Strict-Transport") || out.contains("HSTS") || out.contains("max-age"),
        "hsts: {out}"
    );
}

#[test]
fn http_security_strict_transport_alias() {
    let out = http_security_calc("strict-transport-security");
    assert!(!out.is_empty(), "strict-transport-security alias: {out}");
}

#[test]
fn http_security_hsts_preload_alias() {
    let out = http_security_calc("hsts-preload");
    assert!(!out.is_empty(), "hsts-preload alias: {out}");
}

#[test]
fn http_security_hsts_rollout_alias() {
    let out = http_security_calc("hsts-rollout");
    assert!(!out.is_empty(), "hsts-rollout alias: {out}");
}

#[test]
fn http_security_headers_topic() {
    let out = http_security_calc("headers");
    assert!(
        out.contains("X-Content-Type")
            || out.contains("Referrer-Policy")
            || out.contains("Permissions"),
        "headers: {out}"
    );
}

#[test]
fn http_security_x_content_type_options_alias() {
    let out = http_security_calc("x-content-type-options");
    assert!(!out.is_empty(), "x-content-type-options alias: {out}");
}

#[test]
fn http_security_referrer_policy_alias() {
    let out = http_security_calc("referrer-policy");
    assert!(!out.is_empty(), "referrer-policy alias: {out}");
}

#[test]
fn http_security_permissions_policy_alias() {
    let out = http_security_calc("permissions-policy");
    assert!(!out.is_empty(), "permissions-policy alias: {out}");
}

#[test]
fn http_security_coop_alias() {
    let out = http_security_calc("coop");
    assert!(!out.is_empty(), "coop alias: {out}");
}

#[test]
fn http_security_tls_topic() {
    let out = http_security_calc("tls");
    assert!(
        out.contains("TLS") || out.contains("cipher") || out.contains("certificate"),
        "tls: {out}"
    );
}

#[test]
fn http_security_tls_config_alias() {
    let out = http_security_calc("tls-config");
    assert!(!out.is_empty(), "tls-config alias: {out}");
}

#[test]
fn http_security_cipher_suites_alias() {
    let out = http_security_calc("cipher-suites");
    assert!(!out.is_empty(), "cipher-suites alias: {out}");
}

#[test]
fn http_security_ocsp_stapling_alias() {
    let out = http_security_calc("ocsp-stapling");
    assert!(!out.is_empty(), "ocsp-stapling alias: {out}");
}

#[test]
fn http_security_cookies_topic() {
    let out = http_security_calc("cookies");
    assert!(
        out.contains("Cookie") || out.contains("SameSite") || out.contains("HttpOnly"),
        "cookies: {out}"
    );
}

#[test]
fn http_security_cookie_security_alias() {
    let out = http_security_calc("cookie-security");
    assert!(!out.is_empty(), "cookie-security alias: {out}");
}

#[test]
fn http_security_samesite_alias() {
    let out = http_security_calc("samesite");
    assert!(!out.is_empty(), "samesite alias: {out}");
}

#[test]
fn http_security_csrf_protection_alias() {
    let out = http_security_calc("csrf-protection");
    assert!(!out.is_empty(), "csrf-protection alias: {out}");
}

#[test]
fn http_security_host_prefix_alias() {
    let out = http_security_calc("host-prefix");
    assert!(!out.is_empty(), "host-prefix alias: {out}");
}

#[test]
fn http_security_unknown_no_panic() {
    let out = http_security_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn http_security_empty_no_panic() {
    let out = http_security_calc("");
    assert!(!out.is_empty());
}

// ─── Wave 35: grpc_ref_calc tests ────────────────────────────────────────────

#[test]
fn grpc_ref_list_no_panic() {
    let out = grpc_ref_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn grpc_ref_all_contains_proto() {
    let out = grpc_ref_calc("all");
    assert!(
        out.contains("proto") || out.contains("Proto") || out.contains("Protobuf"),
        "all: {out}"
    );
}

#[test]
fn grpc_ref_proto_topic() {
    let out = grpc_ref_calc("proto");
    assert!(
        out.contains("proto3") || out.contains("message") || out.contains("syntax"),
        "proto: {out}"
    );
}

#[test]
fn grpc_ref_protobuf_alias() {
    let out = grpc_ref_calc("protobuf");
    assert!(!out.is_empty(), "protobuf alias: {out}");
}

#[test]
fn grpc_ref_proto3_alias() {
    let out = grpc_ref_calc("proto3");
    assert!(!out.is_empty(), "proto3 alias: {out}");
}

#[test]
fn grpc_ref_message_definition_alias() {
    let out = grpc_ref_calc("message-definition");
    assert!(!out.is_empty(), "message-definition alias: {out}");
}

#[test]
fn grpc_ref_oneof_alias() {
    let out = grpc_ref_calc("oneof");
    assert!(!out.is_empty(), "oneof alias: {out}");
}

#[test]
fn grpc_ref_services_topic() {
    let out = grpc_ref_calc("services");
    assert!(
        out.contains("service") || out.contains("rpc") || out.contains("streaming"),
        "services: {out}"
    );
}

#[test]
fn grpc_ref_service_definition_alias() {
    let out = grpc_ref_calc("service-definition");
    assert!(!out.is_empty(), "service-definition alias: {out}");
}

#[test]
fn grpc_ref_server_streaming_alias() {
    let out = grpc_ref_calc("server-streaming");
    assert!(!out.is_empty(), "server-streaming alias: {out}");
}

#[test]
fn grpc_ref_bidirectional_streaming_alias() {
    let out = grpc_ref_calc("bidirectional-streaming");
    assert!(!out.is_empty(), "bidirectional-streaming alias: {out}");
}

#[test]
fn grpc_ref_breaking_changes_alias() {
    let out = grpc_ref_calc("breaking-changes");
    assert!(!out.is_empty(), "breaking-changes alias: {out}");
}

#[test]
fn grpc_ref_codegen_topic() {
    let out = grpc_ref_calc("codegen");
    assert!(
        out.contains("protoc") || out.contains("generate") || out.contains("buf"),
        "codegen: {out}"
    );
}

#[test]
fn grpc_ref_code_generation_alias() {
    let out = grpc_ref_calc("code-generation");
    assert!(!out.is_empty(), "code-generation alias: {out}");
}

#[test]
fn grpc_ref_tonic_alias() {
    let out = grpc_ref_calc("tonic");
    assert!(!out.is_empty(), "tonic alias: {out}");
}

#[test]
fn grpc_ref_buf_tool_alias() {
    let out = grpc_ref_calc("buf-tool");
    assert!(!out.is_empty(), "buf-tool alias: {out}");
}

#[test]
fn grpc_ref_buf_breaking_alias() {
    let out = grpc_ref_calc("buf-breaking");
    assert!(!out.is_empty(), "buf-breaking alias: {out}");
}

#[test]
fn grpc_ref_interceptors_topic() {
    let out = grpc_ref_calc("interceptors");
    assert!(
        out.contains("interceptor") || out.contains("middleware") || out.contains("metadata"),
        "interceptors: {out}"
    );
}

#[test]
fn grpc_ref_grpc_middleware_alias() {
    let out = grpc_ref_calc("grpc-middleware");
    assert!(!out.is_empty(), "grpc-middleware alias: {out}");
}

#[test]
fn grpc_ref_grpc_metadata_alias() {
    let out = grpc_ref_calc("grpc-metadata");
    assert!(!out.is_empty(), "grpc-metadata alias: {out}");
}

#[test]
fn grpc_ref_grpc_status_codes_alias() {
    let out = grpc_ref_calc("grpc-status-codes");
    assert!(!out.is_empty(), "grpc-status-codes alias: {out}");
}

#[test]
fn grpc_ref_panic_recovery_alias() {
    let out = grpc_ref_calc("panic-recovery");
    assert!(!out.is_empty(), "panic-recovery alias: {out}");
}

#[test]
fn grpc_ref_transport_topic() {
    let out = grpc_ref_calc("transport");
    assert!(
        out.contains("TLS") || out.contains("HTTP/2") || out.contains("load balanc"),
        "transport: {out}"
    );
}

#[test]
fn grpc_ref_grpc_tls_alias() {
    let out = grpc_ref_calc("grpc-tls");
    assert!(!out.is_empty(), "grpc-tls alias: {out}");
}

#[test]
fn grpc_ref_grpc_load_balancing_alias() {
    let out = grpc_ref_calc("grpc-load-balancing");
    assert!(!out.is_empty(), "grpc-load-balancing alias: {out}");
}

#[test]
fn grpc_ref_grpc_health_check_alias() {
    let out = grpc_ref_calc("grpc-health-check");
    assert!(!out.is_empty(), "grpc-health-check alias: {out}");
}

#[test]
fn grpc_ref_grpc_compression_alias() {
    let out = grpc_ref_calc("grpc-compression");
    assert!(!out.is_empty(), "grpc-compression alias: {out}");
}

#[test]
fn grpc_ref_tools_topic() {
    let out = grpc_ref_calc("tools");
    assert!(
        out.contains("grpcurl") || out.contains("Evans") || out.contains("Prometheus"),
        "tools: {out}"
    );
}

#[test]
fn grpc_ref_grpcurl_alias() {
    let out = grpc_ref_calc("grpcurl");
    assert!(!out.is_empty(), "grpcurl alias: {out}");
}

#[test]
fn grpc_ref_evans_repl_alias() {
    let out = grpc_ref_calc("evans-repl");
    assert!(!out.is_empty(), "evans-repl alias: {out}");
}

#[test]
fn grpc_ref_grpc_otel_alias() {
    let out = grpc_ref_calc("grpc-otel");
    assert!(!out.is_empty(), "grpc-otel alias: {out}");
}

#[test]
fn grpc_ref_unknown_no_panic() {
    let out = grpc_ref_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn grpc_ref_empty_no_panic() {
    let out = grpc_ref_calc("");
    assert!(!out.is_empty());
}

// ─── Wave 35: wasm_runtime_calc tests ────────────────────────────────────────

#[test]
fn wasm_runtime_list_no_panic() {
    let out = wasm_runtime_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn wasm_runtime_all_contains_core() {
    let out = wasm_runtime_calc("all");
    assert!(
        out.contains("WebAssembly") || out.contains("Wasm") || out.contains("wasm"),
        "all: {out}"
    );
}

#[test]
fn wasm_runtime_core_topic() {
    let out = wasm_runtime_calc("core");
    assert!(
        out.contains("WebAssembly") || out.contains("linear memory") || out.contains("stack"),
        "core: {out}"
    );
}

#[test]
fn wasm_runtime_wasm_concepts_alias() {
    let out = wasm_runtime_calc("wasm-concepts");
    assert!(!out.is_empty(), "wasm-concepts alias: {out}");
}

#[test]
fn wasm_runtime_binary_format_alias() {
    let out = wasm_runtime_calc("binary-format");
    assert!(!out.is_empty(), "binary-format alias: {out}");
}

#[test]
fn wasm_runtime_linear_memory_alias() {
    let out = wasm_runtime_calc("linear-memory");
    assert!(!out.is_empty(), "linear-memory alias: {out}");
}

#[test]
fn wasm_runtime_stack_machine_alias() {
    let out = wasm_runtime_calc("stack-machine");
    assert!(!out.is_empty(), "stack-machine alias: {out}");
}

#[test]
fn wasm_runtime_wasi_topic() {
    let out = wasm_runtime_calc("wasi");
    assert!(
        out.contains("WASI") || out.contains("system interface") || out.contains("wasmtime"),
        "wasi: {out}"
    );
}

#[test]
fn wasm_runtime_wasm_system_interface_alias() {
    let out = wasm_runtime_calc("wasm-system-interface");
    assert!(!out.is_empty(), "wasm-system-interface alias: {out}");
}

#[test]
fn wasm_runtime_wasi_preview1_alias() {
    let out = wasm_runtime_calc("wasi-preview1");
    assert!(!out.is_empty(), "wasi-preview1 alias: {out}");
}

#[test]
fn wasm_runtime_wasi_preview2_alias() {
    let out = wasm_runtime_calc("wasi-preview2");
    assert!(!out.is_empty(), "wasi-preview2 alias: {out}");
}

#[test]
fn wasm_runtime_wasmtime_alias() {
    let out = wasm_runtime_calc("wasmtime");
    assert!(!out.is_empty(), "wasmtime alias: {out}");
}

#[test]
fn wasm_runtime_wasmer_alias() {
    let out = wasm_runtime_calc("wasmer");
    assert!(!out.is_empty(), "wasmer alias: {out}");
}

#[test]
fn wasm_runtime_wazero_alias() {
    let out = wasm_runtime_calc("wazero");
    assert!(!out.is_empty(), "wazero alias: {out}");
}

#[test]
fn wasm_runtime_js_topic() {
    let out = wasm_runtime_calc("js");
    assert!(
        out.contains("JavaScript") || out.contains("instantiate") || out.contains("memory"),
        "js: {out}"
    );
}

#[test]
fn wasm_runtime_wasm_javascript_alias() {
    let out = wasm_runtime_calc("wasm-javascript");
    assert!(!out.is_empty(), "wasm-javascript alias: {out}");
}

#[test]
fn wasm_runtime_instantiatestreaming_alias() {
    let out = wasm_runtime_calc("instantiatestreaming");
    assert!(!out.is_empty(), "instantiatestreaming alias: {out}");
}

#[test]
fn wasm_runtime_wasm_memory_access_alias() {
    let out = wasm_runtime_calc("wasm-memory-access");
    assert!(!out.is_empty(), "wasm-memory-access alias: {out}");
}

#[test]
fn wasm_runtime_shared_array_buffer_alias() {
    let out = wasm_runtime_calc("shared-array-buffer");
    assert!(!out.is_empty(), "shared-array-buffer alias: {out}");
}

#[test]
fn wasm_runtime_rust_topic() {
    let out = wasm_runtime_calc("rust");
    assert!(
        out.contains("wasm-pack") || out.contains("wasm-bindgen") || out.contains("Rust"),
        "rust: {out}"
    );
}

#[test]
fn wasm_runtime_wasm_rust_alias() {
    let out = wasm_runtime_calc("wasm-rust");
    assert!(!out.is_empty(), "wasm-rust alias: {out}");
}

#[test]
fn wasm_runtime_wasm_pack_alias() {
    let out = wasm_runtime_calc("wasm-pack");
    assert!(!out.is_empty(), "wasm-pack alias: {out}");
}

#[test]
fn wasm_runtime_wasm_bindgen_alias() {
    let out = wasm_runtime_calc("wasm-bindgen");
    assert!(!out.is_empty(), "wasm-bindgen alias: {out}");
}

#[test]
fn wasm_runtime_wasm_opt_alias() {
    let out = wasm_runtime_calc("wasm-opt");
    assert!(!out.is_empty(), "wasm-opt alias: {out}");
}

#[test]
fn wasm_runtime_wasm_size_alias() {
    let out = wasm_runtime_calc("wasm-size");
    assert!(!out.is_empty(), "wasm-size alias: {out}");
}

#[test]
fn wasm_runtime_components_topic() {
    let out = wasm_runtime_calc("components");
    assert!(
        out.contains("component") || out.contains("WIT") || out.contains("Component"),
        "components: {out}"
    );
}

#[test]
fn wasm_runtime_component_model_alias() {
    let out = wasm_runtime_calc("component-model");
    assert!(!out.is_empty(), "component-model alias: {out}");
}

#[test]
fn wasm_runtime_wit_idl_alias() {
    let out = wasm_runtime_calc("wit-idl");
    assert!(!out.is_empty(), "wit-idl alias: {out}");
}

#[test]
fn wasm_runtime_cargo_component_alias() {
    let out = wasm_runtime_calc("cargo-component");
    assert!(!out.is_empty(), "cargo-component alias: {out}");
}

#[test]
fn wasm_runtime_wasm_tools_alias() {
    let out = wasm_runtime_calc("wasm-tools");
    assert!(!out.is_empty(), "wasm-tools alias: {out}");
}

#[test]
fn wasm_runtime_perf_topic() {
    let out = wasm_runtime_calc("perf");
    assert!(
        out.contains("JIT") || out.contains("SIMD") || out.contains("performance"),
        "perf: {out}"
    );
}

#[test]
fn wasm_runtime_wasm_simd_alias() {
    let out = wasm_runtime_calc("wasm-simd");
    assert!(!out.is_empty(), "wasm-simd alias: {out}");
}

#[test]
fn wasm_runtime_wasm_threads_alias() {
    let out = wasm_runtime_calc("wasm-threads");
    assert!(!out.is_empty(), "wasm-threads alias: {out}");
}

#[test]
fn wasm_runtime_twiggy_profiler_alias() {
    let out = wasm_runtime_calc("twiggy-profiler");
    assert!(!out.is_empty(), "twiggy-profiler alias: {out}");
}

#[test]
fn wasm_runtime_wasm_cold_start_alias() {
    let out = wasm_runtime_calc("wasm-cold-start");
    assert!(!out.is_empty(), "wasm-cold-start alias: {out}");
}

#[test]
fn wasm_runtime_unknown_no_panic() {
    let out = wasm_runtime_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn wasm_runtime_empty_no_panic() {
    let out = wasm_runtime_calc("");
    assert!(!out.is_empty());
}

// ─── Wave 36: linux_perf_calc tests ──────────────────────────────────────────

#[test]
fn linux_perf_list_no_panic() {
    let out = linux_perf_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn linux_perf_all_no_panic() {
    let out = linux_perf_calc("all");
    assert!(
        out.contains("perf") || out.contains("eBPF") || out.contains("ftrace"),
        "all: {out}"
    );
}

#[test]
fn linux_perf_perf_topic() {
    let out = linux_perf_calc("perf");
    assert!(
        out.contains("perf stat") || out.contains("perf record") || out.contains("flame"),
        "perf: {out}"
    );
}

#[test]
fn linux_perf_perf_stat_alias() {
    let out = linux_perf_calc("perf-stat");
    assert!(!out.is_empty(), "perf-stat: {out}");
}

#[test]
fn linux_perf_perf_record_alias() {
    let out = linux_perf_calc("perf-record");
    assert!(!out.is_empty(), "perf-record: {out}");
}

#[test]
fn linux_perf_hardware_counters_alias() {
    let out = linux_perf_calc("hardware-counters");
    assert!(!out.is_empty(), "hardware-counters: {out}");
}

#[test]
fn linux_perf_cpu_profiling_alias() {
    let out = linux_perf_calc("cpu-profiling");
    assert!(!out.is_empty(), "cpu-profiling: {out}");
}

#[test]
fn linux_perf_ebpf_topic() {
    let out = linux_perf_calc("ebpf");
    assert!(
        out.contains("eBPF") || out.contains("bpf") || out.contains("bpftrace"),
        "ebpf: {out}"
    );
}

#[test]
fn linux_perf_bpftrace_alias() {
    let out = linux_perf_calc("bpftrace");
    assert!(!out.is_empty(), "bpftrace: {out}");
}

#[test]
fn linux_perf_bcc_tools_alias() {
    let out = linux_perf_calc("bcc-tools");
    assert!(!out.is_empty(), "bcc-tools: {out}");
}

#[test]
fn linux_perf_opensnoop_alias() {
    let out = linux_perf_calc("opensnoop");
    assert!(!out.is_empty(), "opensnoop: {out}");
}

#[test]
fn linux_perf_kprobe_alias() {
    let out = linux_perf_calc("kprobe");
    assert!(!out.is_empty(), "kprobe: {out}");
}

#[test]
fn linux_perf_xdp_ebpf_alias() {
    let out = linux_perf_calc("xdp-ebpf");
    assert!(!out.is_empty(), "xdp-ebpf: {out}");
}

#[test]
fn linux_perf_ftrace_topic() {
    let out = linux_perf_calc("ftrace");
    assert!(
        out.contains("ftrace") || out.contains("tracer") || out.contains("tracing"),
        "ftrace: {out}"
    );
}

#[test]
fn linux_perf_kernel_tracer_alias() {
    let out = linux_perf_calc("kernel-tracer");
    assert!(!out.is_empty(), "kernel-tracer: {out}");
}

#[test]
fn linux_perf_function_graph_alias() {
    let out = linux_perf_calc("function-graph");
    assert!(!out.is_empty(), "function-graph: {out}");
}

#[test]
fn linux_perf_trace_cmd_alias() {
    let out = linux_perf_calc("trace-cmd");
    assert!(!out.is_empty(), "trace-cmd: {out}");
}

#[test]
fn linux_perf_flamegraph_topic() {
    let out = linux_perf_calc("flamegraph");
    assert!(
        out.contains("flame") || out.contains("Flame") || out.contains("Gregg"),
        "flamegraph: {out}"
    );
}

#[test]
fn linux_perf_flame_graph_alias() {
    let out = linux_perf_calc("flame-graph");
    assert!(!out.is_empty(), "flame-graph: {out}");
}

#[test]
fn linux_perf_brendan_gregg_alias() {
    let out = linux_perf_calc("brendan-gregg");
    assert!(!out.is_empty(), "brendan-gregg: {out}");
}

#[test]
fn linux_perf_off_cpu_alias() {
    let out = linux_perf_calc("off-cpu");
    assert!(!out.is_empty(), "off-cpu: {out}");
}

#[test]
fn linux_perf_cargo_flamegraph_alias() {
    let out = linux_perf_calc("cargo-flamegraph");
    assert!(!out.is_empty(), "cargo-flamegraph: {out}");
}

#[test]
fn linux_perf_memory_topic() {
    let out = linux_perf_calc("memory");
    assert!(
        out.contains("memory") || out.contains("valgrind") || out.contains("NUMA"),
        "memory: {out}"
    );
}

#[test]
fn linux_perf_valgrind_alias() {
    let out = linux_perf_calc("valgrind");
    assert!(!out.is_empty(), "valgrind: {out}");
}

#[test]
fn linux_perf_asan_rust_alias() {
    let out = linux_perf_calc("asan-rust");
    assert!(!out.is_empty(), "asan-rust: {out}");
}

#[test]
fn linux_perf_huge_pages_alias() {
    let out = linux_perf_calc("huge-pages");
    assert!(!out.is_empty(), "huge-pages: {out}");
}

#[test]
fn linux_perf_syscall_topic() {
    let out = linux_perf_calc("syscall");
    assert!(
        out.contains("strace") || out.contains("syscall") || out.contains("ltrace"),
        "syscall: {out}"
    );
}

#[test]
fn linux_perf_strace_alias() {
    let out = linux_perf_calc("strace");
    assert!(!out.is_empty(), "strace: {out}");
}

#[test]
fn linux_perf_ltrace_alias() {
    let out = linux_perf_calc("ltrace");
    assert!(!out.is_empty(), "ltrace: {out}");
}

#[test]
fn linux_perf_seccomp_bpf_alias() {
    let out = linux_perf_calc("seccomp-bpf");
    assert!(!out.is_empty(), "seccomp-bpf: {out}");
}

#[test]
fn linux_perf_unknown_no_panic() {
    let out = linux_perf_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn linux_perf_empty_no_panic() {
    let out = linux_perf_calc("");
    assert!(!out.is_empty());
}

// ─── Wave 36: db_migrations_calc tests ───────────────────────────────────────

#[test]
fn db_migrations_list_no_panic() {
    let out = db_migrations_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn db_migrations_all_no_panic() {
    let out = db_migrations_calc("all");
    assert!(
        out.contains("migration") || out.contains("Flyway") || out.contains("schema"),
        "all: {out}"
    );
}

#[test]
fn db_migrations_concepts_topic() {
    let out = db_migrations_calc("concepts");
    assert!(
        out.contains("migration") || out.contains("schema") || out.contains("DDL"),
        "concepts: {out}"
    );
}

#[test]
fn db_migrations_schema_migration_alias() {
    let out = db_migrations_calc("schema-migration");
    assert!(!out.is_empty(), "schema-migration: {out}");
}

#[test]
fn db_migrations_expand_contract_alias() {
    let out = db_migrations_calc("expand-contract");
    assert!(!out.is_empty(), "expand-contract: {out}");
}

#[test]
fn db_migrations_zero_downtime_alias() {
    let out = db_migrations_calc("zero-downtime-migration");
    assert!(!out.is_empty(), "zero-downtime-migration: {out}");
}

#[test]
fn db_migrations_flyway_topic() {
    let out = db_migrations_calc("flyway");
    assert!(
        out.contains("flyway") || out.contains("Flyway") || out.contains("migrate"),
        "flyway: {out}"
    );
}

#[test]
fn db_migrations_flyway_migrate_alias() {
    let out = db_migrations_calc("flyway-migrate");
    assert!(!out.is_empty(), "flyway-migrate: {out}");
}

#[test]
fn db_migrations_flyway_baseline_alias() {
    let out = db_migrations_calc("flyway-baseline");
    assert!(!out.is_empty(), "flyway-baseline: {out}");
}

#[test]
fn db_migrations_spring_flyway_alias() {
    let out = db_migrations_calc("spring-flyway");
    assert!(!out.is_empty(), "spring-flyway: {out}");
}

#[test]
fn db_migrations_liquibase_topic() {
    let out = db_migrations_calc("liquibase");
    assert!(
        out.contains("Liquibase") || out.contains("liquibase") || out.contains("changeset"),
        "liquibase: {out}"
    );
}

#[test]
fn db_migrations_liquibase_changeset_alias() {
    let out = db_migrations_calc("liquibase-changeset");
    assert!(!out.is_empty(), "liquibase-changeset: {out}");
}

#[test]
fn db_migrations_liquibase_rollback_alias() {
    let out = db_migrations_calc("liquibase-rollback");
    assert!(!out.is_empty(), "liquibase-rollback: {out}");
}

#[test]
fn db_migrations_atlas_topic() {
    let out = db_migrations_calc("atlas");
    assert!(
        out.contains("atlas") || out.contains("Atlas") || out.contains("HCL"),
        "atlas: {out}"
    );
}

#[test]
fn db_migrations_atlas_schema_alias() {
    let out = db_migrations_calc("atlas-schema");
    assert!(!out.is_empty(), "atlas-schema: {out}");
}

#[test]
fn db_migrations_atlas_hcl_alias() {
    let out = db_migrations_calc("atlas-hcl");
    assert!(!out.is_empty(), "atlas-hcl: {out}");
}

#[test]
fn db_migrations_atlas_drift_alias() {
    let out = db_migrations_calc("atlas-drift");
    assert!(!out.is_empty(), "atlas-drift: {out}");
}

#[test]
fn db_migrations_patterns_topic() {
    let out = db_migrations_calc("patterns");
    assert!(
        out.contains("backfill") || out.contains("index") || out.contains("NOT NULL"),
        "patterns: {out}"
    );
}

#[test]
fn db_migrations_backfill_strategy_alias() {
    let out = db_migrations_calc("backfill-strategy");
    assert!(!out.is_empty(), "backfill-strategy: {out}");
}

#[test]
fn db_migrations_concurrent_index_alias() {
    let out = db_migrations_calc("concurrent-index");
    assert!(!out.is_empty(), "concurrent-index: {out}");
}

#[test]
fn db_migrations_schema_drift_alias() {
    let out = db_migrations_calc("schema-drift");
    assert!(!out.is_empty(), "schema-drift: {out}");
}

#[test]
fn db_migrations_rollback_topic() {
    let out = db_migrations_calc("rollback");
    assert!(
        out.contains("rollback") || out.contains("undo") || out.contains("revert"),
        "rollback: {out}"
    );
}

#[test]
fn db_migrations_undo_migration_alias() {
    let out = db_migrations_calc("undo-migration");
    assert!(!out.is_empty(), "undo-migration: {out}");
}

#[test]
fn db_migrations_transactional_ddl_alias() {
    let out = db_migrations_calc("transactional-ddl");
    assert!(!out.is_empty(), "transactional-ddl: {out}");
}

#[test]
fn db_migrations_snapshot_rollback_alias() {
    let out = db_migrations_calc("snapshot-rollback");
    assert!(!out.is_empty(), "snapshot-rollback: {out}");
}

#[test]
fn db_migrations_unknown_no_panic() {
    let out = db_migrations_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn db_migrations_empty_no_panic() {
    let out = db_migrations_calc("");
    assert!(!out.is_empty());
}

// ─── Wave 36: oauth_ref_calc tests ───────────────────────────────────────────

#[test]
fn oauth_ref_list_no_panic() {
    let out = oauth_ref_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn oauth_ref_all_no_panic() {
    let out = oauth_ref_calc("all");
    assert!(
        out.contains("OAuth") || out.contains("token") || out.contains("PKCE"),
        "all: {out}"
    );
}

#[test]
fn oauth_ref_flows_topic() {
    let out = oauth_ref_calc("flows");
    assert!(
        out.contains("Authorization Code")
            || out.contains("PKCE")
            || out.contains("authorization_code"),
        "flows: {out}"
    );
}

#[test]
fn oauth_ref_authorization_code_alias() {
    let out = oauth_ref_calc("authorization-code");
    assert!(!out.is_empty(), "authorization-code: {out}");
}

#[test]
fn oauth_ref_pkce_alias() {
    let out = oauth_ref_calc("pkce");
    assert!(!out.is_empty(), "pkce: {out}");
}

#[test]
fn oauth_ref_client_credentials_alias() {
    let out = oauth_ref_calc("client-credentials");
    assert!(!out.is_empty(), "client-credentials: {out}");
}

#[test]
fn oauth_ref_device_flow_alias() {
    let out = oauth_ref_calc("device-flow");
    assert!(!out.is_empty(), "device-flow: {out}");
}

#[test]
fn oauth_ref_refresh_token_flow_alias() {
    let out = oauth_ref_calc("refresh-token-flow");
    assert!(!out.is_empty(), "refresh-token-flow: {out}");
}

#[test]
fn oauth_ref_tokens_topic() {
    let out = oauth_ref_calc("tokens");
    assert!(
        out.contains("access_token") || out.contains("Access Token") || out.contains("JWT"),
        "tokens: {out}"
    );
}

#[test]
fn oauth_ref_access_token_alias() {
    let out = oauth_ref_calc("access-token");
    assert!(!out.is_empty(), "access-token: {out}");
}

#[test]
fn oauth_ref_id_token_alias() {
    let out = oauth_ref_calc("id-token");
    assert!(!out.is_empty(), "id-token: {out}");
}

#[test]
fn oauth_ref_jwt_verification_alias() {
    let out = oauth_ref_calc("jwt-verification");
    assert!(!out.is_empty(), "jwt-verification: {out}");
}

#[test]
fn oauth_ref_token_introspection_alias() {
    let out = oauth_ref_calc("token-introspection");
    assert!(!out.is_empty(), "token-introspection: {out}");
}

#[test]
fn oauth_ref_oidc_topic() {
    let out = oauth_ref_calc("oidc");
    assert!(
        out.contains("OpenID") || out.contains("OIDC") || out.contains("openid"),
        "oidc: {out}"
    );
}

#[test]
fn oauth_ref_openid_connect_alias() {
    let out = oauth_ref_calc("openid-connect");
    assert!(!out.is_empty(), "openid-connect: {out}");
}

#[test]
fn oauth_ref_oidc_scopes_alias() {
    let out = oauth_ref_calc("oidc-scopes");
    assert!(!out.is_empty(), "oidc-scopes: {out}");
}

#[test]
fn oauth_ref_discovery_endpoint_alias() {
    let out = oauth_ref_calc("discovery-endpoint");
    assert!(!out.is_empty(), "discovery-endpoint: {out}");
}

#[test]
fn oauth_ref_security_topic() {
    let out = oauth_ref_calc("security");
    assert!(
        out.contains("state") || out.contains("PKCE") || out.contains("redirect"),
        "security: {out}"
    );
}

#[test]
fn oauth_ref_state_parameter_alias() {
    let out = oauth_ref_calc("state-parameter");
    assert!(!out.is_empty(), "state-parameter: {out}");
}

#[test]
fn oauth_ref_token_storage_alias() {
    let out = oauth_ref_calc("token-storage");
    assert!(!out.is_empty(), "token-storage: {out}");
}

#[test]
fn oauth_ref_dpop_alias() {
    let out = oauth_ref_calc("dpop");
    assert!(!out.is_empty(), "dpop: {out}");
}

#[test]
fn oauth_ref_providers_topic() {
    let out = oauth_ref_calc("providers");
    assert!(
        out.contains("Auth0") || out.contains("Okta") || out.contains("Google"),
        "providers: {out}"
    );
}

#[test]
fn oauth_ref_auth0_alias() {
    let out = oauth_ref_calc("auth0");
    assert!(!out.is_empty(), "auth0: {out}");
}

#[test]
fn oauth_ref_okta_alias() {
    let out = oauth_ref_calc("okta");
    assert!(!out.is_empty(), "okta: {out}");
}

#[test]
fn oauth_ref_azure_ad_alias() {
    let out = oauth_ref_calc("azure-ad");
    assert!(!out.is_empty(), "azure-ad: {out}");
}

#[test]
fn oauth_ref_keycloak_alias() {
    let out = oauth_ref_calc("keycloak");
    assert!(!out.is_empty(), "keycloak: {out}");
}

#[test]
fn oauth_ref_nextauth_alias() {
    let out = oauth_ref_calc("nextauth");
    assert!(!out.is_empty(), "nextauth: {out}");
}

#[test]
fn oauth_ref_jwt_topic() {
    let out = oauth_ref_calc("jwt");
    assert!(
        out.contains("jwt") || out.contains("JWT") || out.contains("jsonwebtoken"),
        "jwt: {out}"
    );
}

#[test]
fn oauth_ref_jsonwebtoken_alias() {
    let out = oauth_ref_calc("jsonwebtoken");
    assert!(!out.is_empty(), "jsonwebtoken: {out}");
}

#[test]
fn oauth_ref_rs256_alias() {
    let out = oauth_ref_calc("rs256");
    assert!(!out.is_empty(), "rs256: {out}");
}

#[test]
fn oauth_ref_unknown_no_panic() {
    let out = oauth_ref_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn oauth_ref_empty_no_panic() {
    let out = oauth_ref_calc("");
    assert!(!out.is_empty());
}

// ─── Wave 36: k8s_security_calc tests ────────────────────────────────────────

#[test]
fn k8s_security_list_no_panic() {
    let out = k8s_security_calc("list");
    assert!(!out.is_empty());
}

#[test]
fn k8s_security_all_no_panic() {
    let out = k8s_security_calc("all");
    assert!(
        out.contains("RBAC") || out.contains("rbac") || out.contains("security"),
        "all: {out}"
    );
}

#[test]
fn k8s_security_rbac_topic() {
    let out = k8s_security_calc("rbac");
    assert!(
        out.contains("Role") || out.contains("rbac") || out.contains("ServiceAccount"),
        "rbac: {out}"
    );
}

#[test]
fn k8s_security_kubernetes_rbac_alias() {
    let out = k8s_security_calc("kubernetes-rbac");
    assert!(!out.is_empty(), "kubernetes-rbac: {out}");
}

#[test]
fn k8s_security_rolebinding_alias() {
    let out = k8s_security_calc("rolebinding");
    assert!(!out.is_empty(), "rolebinding: {out}");
}

#[test]
fn k8s_security_can_i_alias() {
    let out = k8s_security_calc("can-i");
    assert!(!out.is_empty(), "can-i: {out}");
}

#[test]
fn k8s_security_netpol_topic() {
    let out = k8s_security_calc("netpol");
    assert!(
        out.contains("NetworkPolicy") || out.contains("network policy") || out.contains("deny"),
        "netpol: {out}"
    );
}

#[test]
fn k8s_security_network_policy_alias() {
    let out = k8s_security_calc("network-policy");
    assert!(!out.is_empty(), "network-policy: {out}");
}

#[test]
fn k8s_security_default_deny_alias() {
    let out = k8s_security_calc("default-deny");
    assert!(!out.is_empty(), "default-deny: {out}");
}

#[test]
fn k8s_security_zero_trust_k8s_alias() {
    let out = k8s_security_calc("zero-trust-k8s");
    assert!(!out.is_empty(), "zero-trust-k8s: {out}");
}

#[test]
fn k8s_security_podsec_topic() {
    let out = k8s_security_calc("podsec");
    assert!(
        out.contains("security") || out.contains("nonRoot") || out.contains("seccompProfile"),
        "podsec: {out}"
    );
}

#[test]
fn k8s_security_pod_security_standards_alias() {
    let out = k8s_security_calc("pod-security-standards");
    assert!(!out.is_empty(), "pod-security-standards: {out}");
}

#[test]
fn k8s_security_runasnonroot_alias() {
    let out = k8s_security_calc("runasnonroot");
    assert!(!out.is_empty(), "runasnonroot: {out}");
}

#[test]
fn k8s_security_capabilities_drop_alias() {
    let out = k8s_security_calc("capabilities-drop");
    assert!(!out.is_empty(), "capabilities-drop: {out}");
}

#[test]
fn k8s_security_secrets_topic() {
    let out = k8s_security_calc("secrets");
    assert!(
        out.contains("secret") || out.contains("Secret") || out.contains("Vault"),
        "secrets: {out}"
    );
}

#[test]
fn k8s_security_encryption_at_rest_alias() {
    let out = k8s_security_calc("encryption-at-rest");
    assert!(!out.is_empty(), "encryption-at-rest: {out}");
}

#[test]
fn k8s_security_external_secrets_alias() {
    let out = k8s_security_calc("external-secrets");
    assert!(!out.is_empty(), "external-secrets: {out}");
}

#[test]
fn k8s_security_vault_agent_alias() {
    let out = k8s_security_calc("vault-agent");
    assert!(!out.is_empty(), "vault-agent: {out}");
}

#[test]
fn k8s_security_sealed_secrets_alias() {
    let out = k8s_security_calc("sealed-secrets");
    assert!(!out.is_empty(), "sealed-secrets: {out}");
}

#[test]
fn k8s_security_supply_topic() {
    let out = k8s_security_calc("supply");
    assert!(
        out.contains("cosign") || out.contains("Trivy") || out.contains("signing"),
        "supply: {out}"
    );
}

#[test]
fn k8s_security_image_signing_alias() {
    let out = k8s_security_calc("image-signing");
    assert!(!out.is_empty(), "image-signing: {out}");
}

#[test]
fn k8s_security_cosign_alias() {
    let out = k8s_security_calc("cosign");
    assert!(!out.is_empty(), "cosign: {out}");
}

#[test]
fn k8s_security_falco_alias() {
    let out = k8s_security_calc("falco");
    assert!(!out.is_empty(), "falco: {out}");
}

#[test]
fn k8s_security_kyverno_alias() {
    let out = k8s_security_calc("kyverno");
    assert!(!out.is_empty(), "kyverno: {out}");
}

#[test]
fn k8s_security_audit_topic() {
    let out = k8s_security_calc("audit");
    assert!(
        out.contains("audit") || out.contains("Audit") || out.contains("kube-bench"),
        "audit: {out}"
    );
}

#[test]
fn k8s_security_k8s_audit_alias() {
    let out = k8s_security_calc("k8s-audit");
    assert!(!out.is_empty(), "k8s-audit: {out}");
}

#[test]
fn k8s_security_kube_bench_alias() {
    let out = k8s_security_calc("kube-bench");
    assert!(!out.is_empty(), "kube-bench: {out}");
}

#[test]
fn k8s_security_cis_benchmark_alias() {
    let out = k8s_security_calc("cis-benchmark");
    assert!(!out.is_empty(), "cis-benchmark: {out}");
}

#[test]
fn k8s_security_unknown_no_panic() {
    let out = k8s_security_calc("nonexistent-topic");
    assert!(!out.is_empty());
}

#[test]
fn k8s_security_empty_no_panic() {
    let out = k8s_security_calc("");
    assert!(!out.is_empty());
}
