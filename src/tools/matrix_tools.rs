use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => action_info(args),
        "multiply" | "mul" => action_multiply(args),
        "transpose" | "T" => action_transpose(args),
        "determinant" | "det" => action_determinant(args),
        "inverse" | "inv" => action_inverse(args),
        "solve" => action_solve(args),
        "rank" => action_rank(args),
        other => Err(format!(
            "matrix_tools: unknown action '{other}'. Valid: info, multiply, transpose, determinant, inverse, solve, rank"
        )),
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

type Matrix = Vec<Vec<f64>>;

fn parse_matrix(v: &Value) -> Result<Matrix, String> {
    let arr = v
        .as_array()
        .ok_or("matrix must be a JSON array of arrays")?;
    if arr.is_empty() {
        return Err("matrix is empty".into());
    }
    let rows: Result<Vec<Vec<f64>>, String> = arr
        .iter()
        .map(|row| {
            let r = row
                .as_array()
                .ok_or_else(|| "each row must be an array".to_string())?;
            r.iter()
                .map(|x| {
                    x.as_f64()
                        .ok_or_else(|| "matrix values must be numbers".to_string())
                })
                .collect()
        })
        .collect();
    let m = rows?;
    let cols = m[0].len();
    if m.iter().any(|r| r.len() != cols) {
        return Err("all rows must have the same length".into());
    }
    Ok(m)
}

fn fmt_matrix(m: &Matrix) -> String {
    if m.is_empty() {
        return "[]".into();
    }
    let cols = m[0].len();
    let mut widths = vec![0usize; cols];
    let strs: Vec<Vec<String>> = m
        .iter()
        .map(|row| {
            row.iter()
                .map(|x| {
                    if x.fract() == 0.0 && x.abs() < 1e12 {
                        format!("{}", *x as i64)
                    } else {
                        format!("{:.4}", x)
                    }
                })
                .collect()
        })
        .collect();
    for row in &strs {
        for (j, s) in row.iter().enumerate() {
            widths[j] = widths[j].max(s.len());
        }
    }
    strs.iter()
        .map(|row| {
            let inner: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(j, s)| format!("{:>w$}", s, w = widths[j]))
                .collect();
            format!("[ {} ]", inner.join("  "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rows(m: &Matrix) -> usize {
    m.len()
}
fn cols(m: &Matrix) -> usize {
    if m.is_empty() {
        0
    } else {
        m[0].len()
    }
}

fn is_square(m: &Matrix) -> bool {
    rows(m) == cols(m) && !m.is_empty()
}

fn transpose(m: &Matrix) -> Matrix {
    let r = rows(m);
    let c = cols(m);
    (0..c).map(|j| (0..r).map(|i| m[i][j]).collect()).collect()
}

fn multiply(a: &Matrix, b: &Matrix) -> Result<Matrix, String> {
    let (ra, ca, rb, cb) = (rows(a), cols(a), rows(b), cols(b));
    if ca != rb {
        return Err(format!("dimension mismatch: ({ra}×{ca}) × ({rb}×{cb})"));
    }
    Ok((0..ra)
        .map(|i| {
            (0..cb)
                .map(|j| (0..ca).map(|k| a[i][k] * b[k][j]).sum())
                .collect()
        })
        .collect())
}

fn lu_decompose(m: &Matrix) -> Result<(Matrix, Vec<usize>, i32), String> {
    let n = rows(m);
    if !is_square(m) {
        return Err("LU requires a square matrix".into());
    }
    let mut a: Matrix = m.clone();
    let mut perm: Vec<usize> = (0..n).collect();
    let mut sign = 1i32;
    for k in 0..n {
        let (mut max_val, mut max_row) = (0.0f64, k);
        for i in k..n {
            if a[i][k].abs() > max_val {
                max_val = a[i][k].abs();
                max_row = i;
            }
        }
        if max_val < 1e-12 {
            return Err("matrix is singular (or nearly so)".into());
        }
        if max_row != k {
            a.swap(k, max_row);
            perm.swap(k, max_row);
            sign = -sign;
        }
        for i in (k + 1)..n {
            let factor = a[i][k] / a[k][k];
            a[i][k] = factor;
            for j in (k + 1)..n {
                let v = a[k][j];
                a[i][j] -= factor * v;
            }
        }
    }
    Ok((a, perm, sign))
}

fn determinant(m: &Matrix) -> Result<f64, String> {
    let n = rows(m);
    let (lu, _, sign) = lu_decompose(m)?;
    let mut det = sign as f64;
    for i in 0..n {
        det *= lu[i][i];
    }
    Ok(det)
}

fn inverse(m: &Matrix) -> Result<Matrix, String> {
    let n = rows(m);
    if !is_square(m) {
        return Err("inverse requires a square matrix".into());
    }
    let (lu, perm, _) = lu_decompose(m)?;
    let mut inv = vec![vec![0.0f64; n]; n];
    for col in 0..n {
        let mut b = vec![0.0f64; n];
        for i in 0..n {
            if perm[i] == col {
                b[i] = 1.0;
            }
        }
        // forward substitution
        for i in 0..n {
            for j in 0..i {
                b[i] -= lu[i][j] * b[j];
            }
        }
        // back substitution
        for i in (0..n).rev() {
            for j in (i + 1)..n {
                b[i] -= lu[i][j] * b[j];
            }
            b[i] /= lu[i][i];
        }
        for i in 0..n {
            inv[i][col] = b[i];
        }
    }
    Ok(inv)
}

fn solve_system(a: &Matrix, b_vec: &[f64]) -> Result<Vec<f64>, String> {
    let n = rows(a);
    if !is_square(a) {
        return Err("solve requires a square coefficient matrix".into());
    }
    if b_vec.len() != n {
        return Err(format!("vector length {}, expected {}", b_vec.len(), n));
    }
    let (lu, perm, _) = lu_decompose(a)?;
    let mut b: Vec<f64> = (0..n)
        .map(|i| {
            // find original index of perm[i]
            b_vec[perm[i]]
        })
        .collect();
    for i in 0..n {
        for j in 0..i {
            b[i] -= lu[i][j] * b[j];
        }
    }
    for i in (0..n).rev() {
        for j in (i + 1)..n {
            b[i] -= lu[i][j] * b[j];
        }
        b[i] /= lu[i][i];
    }
    Ok(b)
}

fn matrix_rank(m: &Matrix) -> usize {
    let mut a = m.clone();
    let r = rows(&a);
    let c = cols(&a);
    let mut rank = 0;
    let mut row = 0;
    for col in 0..c {
        if row >= r {
            break;
        }
        let mut pivot = None;
        for i in row..r {
            if a[i][col].abs() > 1e-10 {
                pivot = Some(i);
                break;
            }
        }
        let p = match pivot {
            None => continue,
            Some(p) => p,
        };
        a.swap(row, p);
        let scale = a[row][col];
        for j in 0..c {
            a[row][j] /= scale;
        }
        for i in 0..r {
            if i != row && a[i][col].abs() > 1e-10 {
                let factor = a[i][col];
                for j in 0..c {
                    let v = a[row][j];
                    a[i][j] -= factor * v;
                }
            }
        }
        rank += 1;
        row += 1;
    }
    rank
}

fn fmt_float(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e12 {
        format!("{}", x as i64)
    } else {
        format!("{:.6}", x)
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_info(args: &Value) -> Result<String, String> {
    let m = parse_matrix(args.get("matrix").ok_or("missing 'matrix' field")?)?;
    let r = rows(&m);
    let c = cols(&m);
    let sq = is_square(&m);
    let rank = matrix_rank(&m);
    let trace: Option<f64> = if sq {
        Some((0..r).map(|i| m[i][i]).sum())
    } else {
        None
    };
    let det: Option<f64> = if sq { determinant(&m).ok() } else { None };
    let flat: Vec<f64> = m.iter().flat_map(|row| row.iter().copied()).collect();
    let min = flat.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = flat.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = flat.iter().sum();
    let mean = sum / flat.len() as f64;

    let mut out = format!("Matrix {}×{}\n\n", r, c);
    out.push_str(&fmt_matrix(&m));
    out.push_str(&format!(
        "\n\nShape:    {}×{} ({})\n",
        r,
        c,
        if sq { "square" } else { "non-square" }
    ));
    out.push_str(&format!("Rank:     {}\n", rank));
    if let Some(t) = trace {
        out.push_str(&format!("Trace:    {}\n", fmt_float(t)));
    }
    if let Some(d) = det {
        out.push_str(&format!("Det:      {}\n", fmt_float(d)));
    }
    out.push_str(&format!("Min:      {}\n", fmt_float(min)));
    out.push_str(&format!("Max:      {}\n", fmt_float(max)));
    out.push_str(&format!("Mean:     {}\n", fmt_float(mean)));
    out.push_str(&format!("Sum:      {}\n", fmt_float(sum)));
    if sq && rank == r {
        out.push_str("Invertible: yes\n");
    } else if sq {
        out.push_str("Invertible: no (singular)\n");
    }
    Ok(out)
}

fn action_multiply(args: &Value) -> Result<String, String> {
    let a = parse_matrix(args.get("a").ok_or("missing 'a' field")?)?;
    let b = parse_matrix(args.get("b").ok_or("missing 'b' field")?)?;
    let c = multiply(&a, &b)?;
    let mut out = format!(
        "A ({}×{})\n{}\n\n× B ({}×{})\n{}\n\n= C ({}×{})\n{}",
        rows(&a),
        cols(&a),
        fmt_matrix(&a),
        rows(&b),
        cols(&b),
        fmt_matrix(&b),
        rows(&c),
        cols(&c),
        fmt_matrix(&c)
    );
    Ok(out)
}

fn action_transpose(args: &Value) -> Result<String, String> {
    let m = parse_matrix(args.get("matrix").ok_or("missing 'matrix' field")?)?;
    let t = transpose(&m);
    let mut out = format!(
        "Original ({}×{})\n{}\n\nTransposed ({}×{})\n{}",
        rows(&m),
        cols(&m),
        fmt_matrix(&m),
        rows(&t),
        cols(&t),
        fmt_matrix(&t)
    );
    Ok(out)
}

fn action_determinant(args: &Value) -> Result<String, String> {
    let m = parse_matrix(args.get("matrix").ok_or("missing 'matrix' field")?)?;
    if !is_square(&m) {
        return Err(format!(
            "determinant requires a square matrix (got {}×{})",
            rows(&m),
            cols(&m)
        ));
    }
    let det = determinant(&m)?;
    let mut out = format!(
        "Matrix ({}×{})\n{}\n\ndet(A) = {}",
        rows(&m),
        cols(&m),
        fmt_matrix(&m),
        fmt_float(det)
    );
    if det.abs() < 1e-10 {
        out.push_str("\n\nMatrix is singular (det ≈ 0)");
    }
    Ok(out)
}

fn action_inverse(args: &Value) -> Result<String, String> {
    let m = parse_matrix(args.get("matrix").ok_or("missing 'matrix' field")?)?;
    if !is_square(&m) {
        return Err(format!(
            "inverse requires a square matrix (got {}×{})",
            rows(&m),
            cols(&m)
        ));
    }
    let inv = inverse(&m)?;
    let check = multiply(&m, &inv).unwrap_or_default();
    let mut out = format!(
        "A ({}×{})\n{}\n\nA⁻¹ ({}×{})\n{}",
        rows(&m),
        cols(&m),
        fmt_matrix(&m),
        rows(&inv),
        cols(&inv),
        fmt_matrix(&inv)
    );
    out.push_str(&format!(
        "\n\nVerification A × A⁻¹ ≈ I\n{}",
        fmt_matrix(&check)
    ));
    Ok(out)
}

fn action_solve(args: &Value) -> Result<String, String> {
    let a = parse_matrix(
        args.get("matrix")
            .ok_or("missing 'matrix' field (coefficient matrix A)")?,
    )?;
    let bv = args
        .get("vector")
        .ok_or("missing 'vector' field (RHS vector b)")?;
    let b: Vec<f64> = bv
        .as_array()
        .ok_or("'vector' must be a JSON array")?
        .iter()
        .map(|x| {
            x.as_f64()
                .ok_or_else(|| "vector values must be numbers".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let x = solve_system(&a, &b)?;
    let n = rows(&a);
    let mut out = format!(
        "A ({}×{})\n{}\n\nb = [ {} ]\n\nSolution x:\n",
        n,
        cols(&a),
        fmt_matrix(&a),
        b.iter()
            .map(|v| fmt_float(*v))
            .collect::<Vec<_>>()
            .join("  ")
    );
    for (i, xi) in x.iter().enumerate() {
        out.push_str(&format!("  x[{}] = {}\n", i, fmt_float(*xi)));
    }
    // verify
    let residuals: Vec<f64> = (0..n)
        .map(|i| b[i] - (0..n).map(|j| a[i][j] * x[j]).sum::<f64>())
        .collect();
    let max_res = residuals.iter().map(|r| r.abs()).fold(0.0f64, f64::max);
    out.push_str(&format!("\nResidual max |Ax - b| = {:.2e}", max_res));
    Ok(out)
}

fn action_rank(args: &Value) -> Result<String, String> {
    let m = parse_matrix(args.get("matrix").ok_or("missing 'matrix' field")?)?;
    let r = matrix_rank(&m);
    let full_row = r == rows(&m);
    let full_col = r == cols(&m);
    let mut out = format!(
        "Matrix ({}×{})\n{}\n\nRank: {}\n",
        rows(&m),
        cols(&m),
        fmt_matrix(&m),
        r
    );
    out.push_str(&format!(
        "Full row rank:    {}\n",
        if full_row {
            "yes"
        } else {
            "no (linearly dependent rows)"
        }
    ));
    out.push_str(&format!(
        "Full column rank: {}\n",
        if full_col {
            "yes"
        } else {
            "no (linearly dependent columns)"
        }
    ));
    let nullity = cols(&m).saturating_sub(r);
    out.push_str(&format!("Nullity:          {}\n", nullity));
    Ok(out)
}
