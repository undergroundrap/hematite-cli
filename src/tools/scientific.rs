use serde_json::Value;

pub async fn scientific_compute(args: &Value) -> Result<String, String> {
    let mode = args["mode"].as_str().ok_or(
        "Missing 'mode' (symbolic, units, complexity, ledger, dataset, regression, hypothesis, matrix)",
    )?;

    match mode {
        "symbolic" => solve_symbolic(args).await,
        "units" => verify_units(args).await,
        "complexity" => audit_complexity(args).await,
        "ledger" => manage_ledger(args).await,
        "dataset" => calculate_on_dataset(args).await,
        "regression" => run_regression(args).await,
        "hypothesis" => run_hypothesis(args).await,
        "matrix" => run_matrix(args).await,
        _ => Err(format!("Unknown scientific mode: {}", mode)),
    }
}

async fn solve_symbolic(args: &Value) -> Result<String, String> {
    let expr = args["expr"]
        .as_str()
        .ok_or("Missing 'expr' for symbolic mode")?;
    let target = args["target"].as_str().unwrap_or("solve"); // solve, simplify, integrate, diff
    let latex = args["latex"].as_bool().unwrap_or(false);

    let python_script = format!(
        "import sympy\n\
         from sympy import symbols, solve, simplify, integrate, diff, Eq, latex\n\
         # Attempt to find symbols automatically\n\
         import re\n\
         raw_expr = r\"{}\"\n\
         # Extract likely symbols (single letters or words starting with letter)\n\
         sym_names = set(re.findall(r'\\b[a-zA-Z][a-zA-Z0-9]*\\b', raw_expr))\n\
         # Remove common functions\n\
         sym_names -= {{'sin', 'cos', 'tan', 'exp', 'log', 'sqrt', 'pi', 'E', 'oo', 'solve', 'simplify', 'integrate', 'diff'}}\n\
         sym_dict = {{name: symbols(name) for name in sym_names}}\n\
         \n\
         try:\n\
             if \"=\" in raw_expr and \"{}\" == \"solve\":\n\
                 lhs, rhs = raw_expr.split(\"=\")\n\
                 result = solve(Eq(eval(lhs, {{'__builtins__': None}}, sym_dict), eval(rhs, {{'__builtins__': None}}, sym_dict)))\n\
             else:\n\
                 expr_obj = eval(raw_expr, {{'__builtins__': None}}, sym_dict)\n\
                 if \"{}\" == \"simplify\": result = simplify(expr_obj)\n\
                 elif \"{}\" == \"integrate\": result = integrate(expr_obj)\n\
                 elif \"{}\" == \"diff\": result = diff(expr_obj)\n\
                 else: result = solve(expr_obj)\n\
             \n\
             print(f\"RESULT: {{result}}\")\n\
             if {}:\n\
                 print(f\"LATEX: {{latex(result)}}\")\n\
         except Exception as e:\n\
             print(f\"ERROR: {{e}}\")\n",
        expr, target, target, target, target, latex
    );

    execute_in_sandbox(&python_script).await
}

async fn verify_units(args: &Value) -> Result<String, String> {
    let calculation = args["calculation"]
        .as_str()
        .ok_or("Missing 'calculation' for units mode")?;

    let python_script = format!(
        "try:\n\
         # Simple Unit System (SI focus)\n\
         class UnitValue:\n\
             def __init__(self, val, dims):\n\
                 self.val = val\n\
                 self.dims = dims # {{'m': 1, 's': -1, etc}}\n\
             def __add__(self, other):\n\
                 if self.dims != other.dims: raise ValueError(f\"Dimension mismatch: {{self.dims}} vs {{other.dims}}\")\n\
                 return UnitValue(self.val + other.val, self.dims)\n\
             def __mul__(self, other):\n\
                 new_dims = self.dims.copy()\n\
                 for k, v in other.dims.items(): new_dims[k] = new_dims.get(k, 0) + v\n\
                 return UnitValue(self.val * other.val, new_dims)\n\
             def __truediv__(self, other):\n\
                 new_dims = self.dims.copy()\n\
                 for k, v in other.dims.items(): new_dims[k] = new_dims.get(k, 0) - v\n\
                 return UnitValue(self.val / other.val, new_dims)\n\
             def __repr__(self): return f\"{{self.val}} ({{self.dims}})\"\n\
         \n\
         # Helper to parse strings like '10m'\n\
         def u(s):\n\
             m = __import__('re').match(r'([\\d\\.]+)([a-zA-Z]+)', s)\n\
             val = float(m.group(1))\n\
             unit = m.group(2)\n\
             return UnitValue(val, {{unit: 1}})\n\
         \n\
         # Executing the calculation with unit objects\n\
         # User input is expected to use u('10m') etc.\n\
         raw_calc = r\"{}\"\n\
         # Basic auto-wrap for units in the expression if they look like 10m\n\
         wrapped = __import__('re').sub(r'(\\d+)([a-z]+)', r\"u('\\1\\2')\", raw_calc)\n\
         result = eval(wrapped, {{'u': u}})\n\
         print(f\"RESULT: {{result}}\")\n\
         except Exception as e:\n\
         print(f\"ERROR: {{e}}\")\n",
        calculation
    );

    execute_in_sandbox(&python_script).await
}

async fn audit_complexity(args: &Value) -> Result<String, String> {
    let snippet = args["snippet"]
        .as_str()
        .ok_or("Missing 'snippet' for complexity mode")?;

    let python_script = format!(
        "import time\n\
         import math\n\
         def run_target(n):\n\
             {}\n\
         \n\
         samples = [10, 50, 100, 200, 500]\n\
         times = []\n\
         for n in samples:\n\
             start = time.perf_counter()\n\
             run_target(n)\n\
             times.append(time.perf_counter() - start)\n\
         \n\
         # Simplified regression to guess Big-O\n\
         # Compare growth rates: t/n, t/n^2, t/log(n)\n\
         ratios_n = [t/n for t, n in zip(times, samples) if n > 0]\n\
         ratios_n2 = [t/(n**2) for t, n in zip(times, samples) if n > 0]\n\
         \n\
         def variance(data):\n\
             if not data: return 1.0\n\
             avg = sum(data)/len(data)\n\
             return sum((x-avg)**2 for x in data)/len(data)\n\
         \n\
         v_n = variance(ratios_n)\n\
         v_n2 = variance(ratios_n2)\n\
         \n\
         if v_n < v_n2: complexity = \"O(N)\"\n\
         elif v_n2 < v_n: complexity = \"O(N^2)\"\n\
         else: complexity = \"O(Unknown)\"\n\
         \n\
         print(f\"RESULT: Empirically detected {{complexity}}\")\n\
         print(f\"STATS: n={{samples}}, times={{[f'{{t:.6f}}s' for t in times]}}\")\n",
        snippet.replace("\n", "\n    ")
    );

    execute_in_sandbox(&python_script).await
}

/// Headless dataset profiler — loads CSV / TSV / JSON / SQLite and produces a
/// real computed statistical profile without requiring the model or a LIMIT clause.
///
/// The file is read directly inside the Python sandbox (no Rust-side JSON
/// embedding), so even large files stay within the sandbox process limits.
pub async fn analyze_dataset(path_str: &str) -> Result<String, String> {
    if path_str.trim().is_empty() {
        return Err("Missing file path for --analyze.".into());
    }

    // Escape backslashes (Windows paths) and double-quotes so the path can be
    // safely embedded inside a Python double-quoted string literal.
    let safe_path = path_str
        .replace('\\', "\\\\")
        .replace('"', "\\\"");

    let script = format!(
        r####"import os, sys, csv as _csv, sqlite3 as _sql3
from collections import Counter

_path = "{safe_path}"
_ext  = os.path.splitext(_path)[1].lower().lstrip('.')
_data = []
_col_order = None

if _ext in ('csv', 'tsv'):
    _delim = '\t' if _ext == 'tsv' else ','
    try:
        with open(_path, encoding='utf-8-sig', errors='replace', newline='') as _fh:
            _rdr = _csv.DictReader(_fh, delimiter=_delim)
            _col_order = list(_rdr.fieldnames) if _rdr.fieldnames else []
            for _i, _row in enumerate(_rdr):
                if _i >= 5000: break
                _data.append(dict(_row))
    except Exception as _e:
        print("ERROR loading file: " + str(_e))
        sys.exit(1)
elif _ext == 'json':
    try:
        with open(_path, encoding='utf-8') as _fh:
            _raw = json.load(_fh)
        if isinstance(_raw, list):
            _data = _raw[:5000]
        elif isinstance(_raw, dict):
            for _v in _raw.values():
                if isinstance(_v, list):
                    _data = _v[:5000]
                    break
    except Exception as _e:
        print("ERROR loading file: " + str(_e))
        sys.exit(1)
elif _ext in ('db', 'sqlite', 'sqlite3'):
    try:
        with _sql3.connect(_path) as _con:
            _cur = _con.cursor()
            _cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
            _tbl = _cur.fetchone()
            if _tbl:
                _cur.execute("SELECT * FROM [%s] LIMIT 5000" % _tbl[0])
                _col_order = [_d[0] for _d in _cur.description]
                _data = [dict(zip(_col_order, _r)) for _r in _cur.fetchall()]
    except Exception as _e:
        print("ERROR loading file: " + str(_e))
        sys.exit(1)
else:
    print("ERROR: unsupported format '." + _ext + "'. Supported: csv, tsv, json, db/sqlite/sqlite3.")
    sys.exit(1)

if not _data:
    print("No data found in: " + _path)
    sys.exit(0)

columns   = _col_order if _col_order else list(_data[0].keys())
row_count = len(_data)
data      = _data

def _try_num(v):
    if v is None: return None
    try: return float(str(v).replace(',', '').replace('$', '').replace('%', '').strip())
    except (ValueError, TypeError): return None

def _ncol(c):
    return [f for r in data for f in (_try_num(r.get(c)),) if f is not None]

def _quart(vals, q):
    s = sorted(vals)
    n = len(s)
    if n == 0: return float('nan')
    if n == 1: return s[0]
    idx = q * (n - 1)
    lo, hi = int(idx), min(int(idx) + 1, n - 1)
    return s[lo] + (idx - lo) * (s[hi] - s[lo])

num_cols = []
txt_cols = []
for c in columns:
    _nonempty = [r.get(c) for r in data
                 if r.get(c) is not None and str(r.get(c, '')).strip() != '']
    if not _nonempty:
        txt_cols.append(c)
        continue
    _s    = _nonempty[:min(200, len(_nonempty))]
    _hits = sum(1 for v in _s if _try_num(v) is not None)
    (num_cols if _hits >= len(_s) * 0.8 else txt_cols).append(c)

_miss = [(c, sum(1 for r in data
                 if r.get(c) is None or str(r.get(c, '')).strip() == ''))
         for c in columns]
_miss = [(c, n) for c, n in _miss if n > 0]

_sample_note = " (5000-row sample)" if row_count == 5000 else ""
_fname = os.path.basename(_path)
_H2 = "##"
_H3 = "###"
_out = []
_out.append(_H2 + " Dataset Profile: " + _fname)
_out.append("")
_out.append("**File:** " + _path)
_out.append("**Shape:** " + str(row_count) + " rows" + _sample_note + " x " + str(len(columns)) + " columns")
_out.append("**Numeric (%d):** %s" % (len(num_cols), ", ".join(num_cols) if num_cols else "none"))
_out.append("**Text/Mixed (%d):** %s" % (len(txt_cols), ", ".join(txt_cols) if txt_cols else "none"))
_out.append("")

if _miss:
    _total_miss = sum(n for _, n in _miss)
    _out.append("**Missing values:** " + str(_total_miss) + " cell(s) across " + str(len(_miss)) + " column(s)")
    for c, n in _miss:
        _pct = round(n * 100.0 / row_count, 1)
        _out.append("  - " + c + ": " + str(n) + " missing (" + str(_pct) + "%)")
    _out.append("")

if num_cols:
    _out.append(_H3 + " Numeric Column Statistics")
    _out.append("")
    _hdr = "%-22s  %6s  %10s  %10s  %10s  %10s  %10s  %10s  %10s  %8s" % (
        "Column", "N", "Min", "P25", "Median", "P75", "Max", "Mean", "Std Dev", "Outliers")
    _out.append(_hdr)
    _out.append("-" * len(_hdr))
    for c in num_cols:
        _vals = _ncol(c)
        if not _vals:
            _out.append("%-22s  (no numeric values)" % c[:22])
            continue
        _mn, _mx = min(_vals), max(_vals)
        _mean = sum(_vals) / len(_vals)
        _med  = statistics.median(_vals)
        _std  = statistics.stdev(_vals) if len(_vals) >= 2 else 0.0
        _q1   = _quart(_vals, 0.25)
        _q3   = _quart(_vals, 0.75)
        _iqr  = _q3 - _q1
        _otl  = sum(1 for v in _vals if v < _q1 - 1.5 * _iqr or v > _q3 + 1.5 * _iqr)
        _out.append("%-22s  %6d  %10.4g  %10.4g  %10.4g  %10.4g  %10.4g  %10.4g  %10.4g  %8d" % (
            c[:22], len(_vals), _mn, _q1, _med, _q3, _mx, _mean, _std, _otl))
    _out.append("")

if txt_cols:
    _out.append(_H3 + " Text Column Statistics")
    _out.append("")
    for c in txt_cols:
        _vals = [str(r.get(c, '') or '').strip() for r in data
                 if r.get(c) is not None and str(r.get(c, '')).strip() != '']
        if not _vals:
            _out.append("**" + c + "**: (all missing)")
            _out.append("")
            continue
        _uniq = len(set(_vals))
        _card = round(_uniq * 100.0 / len(_vals), 1)
        _out.append("**" + c + "**: " + str(len(_vals)) + " non-null, " +
                    str(_uniq) + " unique (" + str(_card) + "% cardinality)")
        for _v, _n in Counter(_vals).most_common(5):
            _short = (_v[:42] + "...") if len(_v) > 42 else _v
            _vpct  = round(_n * 100.0 / len(_vals), 1)
            _out.append("  - `" + _short + "`: " + str(_n) + " (" + str(_vpct) + "%)")
        _out.append("")

if HAS_NUMPY and len(num_cols) >= 2:
    try:
        import pandas as pd
        _df = pd.DataFrame(data)[num_cols]
        for _c in _df.columns:
            _df[_c] = pd.to_numeric(_df[_c], errors='coerce')
        _corr = _df.corr()
        _out.append(_H3 + " Correlation Matrix")
        _out.append("")
        _heads = [c[:10] for c in num_cols]
        _out.append("            " + "".join("  %10s" % h for h in _heads))
        for _i, c in enumerate(num_cols):
            _rs = "%12s" % _heads[_i]
            for _j in range(len(num_cols)):
                _rs += "  %10.3f" % _corr.iloc[_i, _j]
            _out.append(_rs)
        _out.append("")
    except Exception:
        pass

_out.append(_H3 + " Sample Rows (first 5)")
_out.append("")
_out.append(" | ".join(columns))
_out.append(" | ".join("---" for _ in columns))
for _row in data[:5]:
    _out.append(" | ".join(str(_row.get(c, '') or '')[:20] for c in columns))

print("\n".join(_out))
"####,
        safe_path = safe_path,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

async fn execute_in_sandbox(script: &str) -> Result<String, String> {
    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script
    });

    crate::tools::code_sandbox::execute(&sandbox_args).await
}

async fn manage_ledger(args: &Value) -> Result<String, String> {
    let action = args["action"]
        .as_str()
        .ok_or("Missing 'action' (read, append)")?;
    let ledger_path = std::path::Path::new(".hematite/docs/scientific_ledger.md");

    if let Some(parent) = ledger_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    match action {
        "read" => {
            if !ledger_path.exists() {
                return Ok("Scientific Ledger is currently empty.".to_string());
            }
            std::fs::read_to_string(ledger_path).map_err(|e| e.to_string())
        }
        "append" => {
            let content = args["content"]
                .as_str()
                .ok_or("Missing 'content' to append")?;
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let entry = format!(
                "\n### [{}] Scientific Derivation\n{}\n---\n",
                timestamp, content
            );

            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(ledger_path)
                .map_err(|e| e.to_string())?;

            file.write_all(entry.as_bytes())
                .map_err(|e| e.to_string())?;
            Ok("Derivation successfully persisted to Scientific Ledger (RAG-indexed).".to_string())
        }
        _ => Err(format!("Unknown ledger action: {}", action)),
    }
}

async fn calculate_on_dataset(args: &Value) -> Result<String, String> {
    let path_str = args["path"].as_str().ok_or("Missing 'path' to dataset")?;
    let sql = args["sql"].as_str().unwrap_or("SELECT * FROM data LIMIT 10000");
    let python_op = args["python_op"].as_str().unwrap_or("print(f'{row_count} rows loaded. Columns: {columns}')");

    let path = std::path::PathBuf::from(path_str);
    let data = crate::tools::data_query::query_to_json_helper(&path, sql).await?;
    let data_json = serde_json::to_string(&data).map_err(|e| e.to_string())?;

    // Column-aware data analysis environment:
    // - col(name)        → all values for a named column (including None)
    // - ncol(name)       → numeric-only values for a column (skips blanks/non-numeric)
    // - top(n, by)       → top N rows sorted descending by column name
    // - group_sum(g, v)  → {group_key: sum_of_value_col}
    // - group_count(g)   → {group_key: count}
    // - df / HAS_PANDAS  → pandas DataFrame if pandas is installed
    let python_script = format!(
        r#"import json, math, statistics, datetime, decimal, re
from collections import Counter, defaultdict

data = {data_json}
columns = list(data[0].keys()) if data else []
row_count = len(data)

def col(name):
    """All values for a named column."""
    return [row.get(name) for row in data]

def ncol(name):
    """Numeric-only values for a named column (skips None/blank/non-numeric)."""
    out = []
    for row in data:
        v = row.get(name)
        if v is not None and v != '':
            try:
                out.append(float(v))
            except (ValueError, TypeError):
                pass
    return out

def top(n=10, by=None):
    """Top N rows sorted descending by column name."""
    key = by or (columns[0] if columns else None)
    def _key(r):
        try: return float(r.get(key, 0) or 0)
        except: return 0.0
    return sorted(data, key=_key, reverse=True)[:n]

def group_sum(group_col, value_col):
    """Sum value_col grouped by group_col. Returns dict sorted by value desc."""
    acc = defaultdict(float)
    for row in data:
        k = row.get(group_col, 'unknown') or 'unknown'
        try: acc[k] += float(row.get(value_col, 0) or 0)
        except (ValueError, TypeError): pass
    return dict(sorted(acc.items(), key=lambda x: x[1], reverse=True))

def group_count(group_col):
    """Count rows per unique value in group_col."""
    return dict(Counter(str(row.get(group_col, '')) for row in data).most_common())

def group_mean(group_col, value_col):
    """Mean of value_col grouped by group_col."""
    acc = defaultdict(list)
    for row in data:
        k = row.get(group_col, 'unknown') or 'unknown'
        try: acc[k].append(float(row.get(value_col, 0) or 0))
        except (ValueError, TypeError): pass
    return {{k: statistics.mean(v) for k, v in acc.items() if v}}

def missing(name):
    """Count of missing/None/blank values in a column."""
    return sum(1 for row in data if row.get(name) is None or row.get(name) == '')

try:
    import pandas as pd
    import numpy as np
    df = pd.DataFrame(data)
    for c in df.columns:
        try: df[c] = pd.to_numeric(df[c])
        except (ValueError, TypeError): pass
    HAS_PANDAS = True
except ImportError:
    HAS_PANDAS = False

print(f"Loaded: {{row_count}} rows x {{len(columns)}} columns")
print(f"Columns: {{columns}}")
print(f"Pandas: {{HAS_PANDAS}}")
print()

{python_op}
"#,
        data_json = data_json,
        python_op = python_op
    );

    execute_in_sandbox(&python_script).await
}

async fn run_regression(args: &Value) -> Result<String, String> {
    let path_str = args["path"]
        .as_str()
        .ok_or("Missing 'path' for regression mode")?;
    let y_col = args["y"]
        .as_str()
        .ok_or("Missing 'y' (target column) for regression mode")?;

    let x_cols: Vec<String> = match &args["x"] {
        Value::String(s) => vec![s.clone()],
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => return Err("Missing 'x' (predictor column(s)) for regression mode".into()),
    };
    if x_cols.is_empty() {
        return Err("'x' must specify at least one predictor column".into());
    }

    let reg_type = args["type"].as_str().unwrap_or("linear");
    let degree = args["degree"].as_u64().unwrap_or(2).min(10) as usize;

    let safe_path = path_str.replace('\\', "\\\\").replace('"', "\\\"");
    let safe_y = y_col.replace('"', "\\\"");
    let x_json = serde_json::to_string(&x_cols).unwrap_or_else(|_| "[]".to_string());

    let script = format!(
        r####"import os, sys, csv as _csv, sqlite3 as _sql3, math

_path   = "{safe_path}"
_xcols  = {x_json}
_ycol   = "{safe_y}"
_rtype  = "{reg_type}"
_degree = {degree}
_ext    = os.path.splitext(_path)[1].lower().lstrip('.')
_data   = []

if _ext in ('csv', 'tsv'):
    _delim = '\t' if _ext == 'tsv' else ','
    try:
        with open(_path, encoding='utf-8-sig', errors='replace', newline='') as _fh:
            _rdr = _csv.DictReader(_fh, delimiter=_delim)
            for _i, _row in enumerate(_rdr):
                if _i >= 5000: break
                _data.append(dict(_row))
    except Exception as _e:
        print("ERROR loading file: " + str(_e))
        sys.exit(1)
elif _ext == 'json':
    try:
        with open(_path, encoding='utf-8') as _fh:
            _raw = json.load(_fh)
        if isinstance(_raw, list):
            _data = _raw[:5000]
        elif isinstance(_raw, dict):
            for _v in _raw.values():
                if isinstance(_v, list):
                    _data = _v[:5000]
                    break
    except Exception as _e:
        print("ERROR loading file: " + str(_e))
        sys.exit(1)
elif _ext in ('db', 'sqlite', 'sqlite3'):
    try:
        with _sql3.connect(_path) as _con:
            _cur = _con.cursor()
            _cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
            _tbl = _cur.fetchone()
            if _tbl:
                _cur.execute("SELECT * FROM [%s] LIMIT 5000" % _tbl[0])
                _col_order = [_d[0] for _d in _cur.description]
                _data = [dict(zip(_col_order, _r)) for _r in _cur.fetchall()]
    except Exception as _e:
        print("ERROR loading file: " + str(_e))
        sys.exit(1)
else:
    print("ERROR: unsupported format '." + _ext + "'. Supported: csv, tsv, json, db/sqlite/sqlite3.")
    sys.exit(1)

if not _data:
    print("No data found in: " + _path)
    sys.exit(0)

def _tryf(v):
    if v is None: return None
    try: return float(str(v).replace(',', '').replace('$', '').replace('%', '').strip())
    except: return None

_yx = []
for _row in _data:
    _yv  = _tryf(_row.get(_ycol))
    if _yv is None: continue
    _xvs = [_tryf(_row.get(_xc)) for _xc in _xcols]
    if any(v is None for v in _xvs): continue
    _yx.append((_yv, _xvs))

_n = len(_yx)
if _n < 3:
    print("ERROR: insufficient numeric data (need >=3 valid rows, got %d)" % _n)
    sys.exit(1)

_ys   = [p[0] for p in _yx]
_xmat = [p[1] for p in _yx]
_ym   = sum(_ys) / _n

_out = []
_out.append("## Regression Results")
_out.append("")
_out.append("**File:** " + os.path.basename(_path))
_out.append("**Y (target):** " + _ycol)
_out.append("**X (predictors):** " + ", ".join(_xcols))
_out.append("**N (valid rows):** %d" % _n)
_out.append("")

if len(_xcols) == 1 and _rtype == "linear":
    _xv   = [r[0] for r in _xmat]
    _xm   = sum(_xv) / _n
    _ssxy = sum((_x - _xm) * (_y - _ym) for _x, _y in zip(_xv, _ys))
    _ssx  = sum((_x - _xm)**2 for _x in _xv)
    _ssy  = sum((_y - _ym)**2 for _y in _ys)
    if _ssx == 0:
        print("ERROR: predictor has zero variance.")
        sys.exit(1)
    _slope = _ssxy / _ssx
    _inter = _ym - _slope * _xm
    _preds = [_slope * _x + _inter for _x in _xv]
    _res   = [_y - _p for _y, _p in zip(_ys, _preds)]
    _sse   = sum(r**2 for r in _res)
    _r2    = 1.0 - _sse / _ssy if _ssy > 0 else 0.0
    _rmse  = math.sqrt(_sse / _n)
    _pr    = _ssxy / math.sqrt(_ssx * _ssy) if _ssx > 0 and _ssy > 0 else 0.0
    _rm    = sum(_res) / _n
    _rstd  = math.sqrt(sum((r - _rm)**2 for r in _res) / _n)
    _out.append("**Type:** Simple Linear Regression (pure-Python OLS)")
    _out.append("**Equation:**  y = %+.6g x %+.6g" % (_slope, _inter))
    _out.append("**R-squared:** %.4f" % _r2)
    _out.append("**RMSE:** %.4g" % _rmse)
    _out.append("**Pearson r:** %.4f" % _pr)
    _out.append("**Residuals:**  min=%.4g  max=%.4g  mean=%.4g  std=%.4g" % (
        min(_res), max(_res), _rm, _rstd))
elif HAS_NUMPY:
    import numpy as _np
    if _rtype == "polynomial" and len(_xcols) == 1:
        _xv     = _np.array([r[0] for r in _xmat])
        _ya     = _np.array(_ys)
        _coeffs = _np.polyfit(_xv, _ya, _degree)
        _preds  = _np.polyval(_coeffs, _xv)
        _res    = _ya - _preds
        _sse    = float(_np.sum(_res**2))
        _sst    = float(_np.sum((_ya - _ym)**2))
        _r2     = 1.0 - _sse / _sst if _sst > 0 else 0.0
        _rmse   = float(_np.sqrt(_np.mean(_res**2)))
        _out.append("**Type:** Polynomial Regression  degree=%d  (numpy polyfit)" % _degree)
        _out.append("**Coefficients (highest power first):** " + ", ".join("%.6g" % c for c in _coeffs))
        _out.append("**R-squared:** %.4f" % _r2)
        _out.append("**RMSE:** %.4g" % _rmse)
        _out.append("**Residuals:**  min=%.4g  max=%.4g  mean=%.4g  std=%.4g" % (
            float(_np.min(_res)), float(_np.max(_res)),
            float(_np.mean(_res)), float(_np.std(_res))))
    else:
        _Xm     = _np.column_stack([_np.ones(_n)] + [[r[i] for r in _xmat] for i in range(len(_xcols))])
        _ya     = _np.array(_ys)
        _coeffs, _, _, _ = _np.linalg.lstsq(_Xm, _ya, rcond=None)
        _preds  = _Xm @ _coeffs
        _res    = _ya - _preds
        _sse    = float(_np.sum(_res**2))
        _sst    = float(_np.sum((_ya - _ym)**2))
        _r2     = 1.0 - _sse / _sst if _sst > 0 else 0.0
        _rmse   = float(_np.sqrt(_np.mean(_res**2)))
        _rm     = float(_np.mean(_res))
        _rstd   = float(_np.std(_res))
        _out.append("**Type:** Multiple Linear Regression (numpy lstsq OLS)")
        _out.append("**Intercept:** %.6g" % _coeffs[0])
        for _i, _xc in enumerate(_xcols):
            _out.append("**%s coeff:** %.6g" % (_xc, _coeffs[_i + 1]))
        _out.append("**R-squared:** %.4f" % _r2)
        _out.append("**RMSE:** %.4g" % _rmse)
        _out.append("**Residuals:**  min=%.4g  max=%.4g  mean=%.4g  std=%.4g" % (
            float(_np.min(_res)), float(_np.max(_res)), _rm, _rstd))
else:
    _out.append("**Type:** Multiple/Polynomial Regression requires numpy.")
    _out.append("Use a single predictor with type=linear for pure-Python OLS, or install numpy.")

print("\n".join(_out))
"####,
        safe_path = safe_path,
        x_json = x_json,
        safe_y = safe_y,
        reg_type = reg_type,
        degree = degree,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

/// Zero-model expression evaluator for `hematite --compute`.
/// Evaluates arithmetic, trig, statistics, and common physical constants
/// entirely inside the Python sandbox — no network, no model required.
pub async fn compute_expr(expr: &str) -> Result<String, String> {
    if expr.trim().is_empty() {
        return Err("No expression provided.".into());
    }
    let safe_expr = expr.replace('\\', "\\\\").replace('"', "\\\"");

    let script = format!(
        r####"from math import *
import statistics as _stat, re as _re, sys

# ── Physical & mathematical constants ────────────────────────────────
c_light  = 299_792_458.0          # m/s  — speed of light (exact)
h_planck = 6.62607015e-34         # J·s  — Planck constant (exact)
hbar     = h_planck / (2 * pi)    # J·s  — reduced Planck constant
G_grav   = 6.67430e-11            # m³/(kg·s²) — gravitational constant
k_B      = 1.380649e-23           # J/K  — Boltzmann constant (exact)
N_A      = 6.02214076e23          # /mol — Avogadro's number (exact)
R_gas    = 8.314462618            # J/(mol·K) — molar gas constant
g_std    = 9.80665                # m/s² — standard gravity (exact)
e_q      = 1.602176634e-19        # C    — elementary charge (exact)
m_e      = 9.1093837015e-31       # kg   — electron mass
m_p      = 1.67262192369e-27      # kg   — proton mass
sigma_SB = 5.670374419e-8         # W/(m²·K⁴) — Stefan-Boltzmann
eps_0    = 8.8541878128e-12       # F/m  — vacuum permittivity
mu_0     = 1.25663706212e-6       # H/m  — vacuum permeability
alpha_fs = 7.2973525693e-3        # — fine-structure constant
atm      = 101_325.0              # Pa   — standard atmosphere

# ── Statistics helpers ────────────────────────────────────────────────
mean     = _stat.mean
median   = _stat.median
stdev    = _stat.stdev
variance = _stat.variance
try:    mode = _stat.mode
except Exception: pass

# ── Financial functions ───────────────────────────────────────────────
def pmt(rate, nper, pv, fv=0, when=0):
    """Periodic loan payment. pmt(0.05/12, 360, 300000)"""
    if rate == 0: return -(pv + fv) / nper
    pvif = (1 + rate) ** nper
    r = rate / (pvif - 1) * -(pv * pvif + fv)
    return r / (1 + rate) if when == 1 else r

def fv(rate, nper, pmt_v, pv=0, when=0):
    """Future value. fv(0.06/12, 120, -500)"""
    if rate == 0: return -pv - pmt_v * nper
    pvif = (1 + rate) ** nper
    return -(pv * pvif + pmt_v * (1 + rate * when) * (pvif - 1) / rate)

def pv(rate, nper, pmt_v, fv=0, when=0):
    """Present value. pv(0.05/12, 360, -1500)"""
    if rate == 0: return -fv - pmt_v * nper
    pvif = (1 + rate) ** nper
    return -(fv + pmt_v * (1 + rate * when) * (pvif - 1) / rate) / pvif

def npv(rate, cashflows):
    """Net present value. npv(0.1, [-1000, 200, 300, 400, 500])"""
    return sum(cf / (1 + rate) ** t for t, cf in enumerate(cashflows))

def irr(cashflows, guess=0.1):
    """Internal rate of return (Newton-Raphson). irr([-1000, 300, 400, 500])"""
    r = guess
    for _ in range(200):
        f  = sum(cf / (1 + r) ** t for t, cf in enumerate(cashflows))
        df = sum(-t * cf / (1 + r) ** (t + 1) for t, cf in enumerate(cashflows))
        if df == 0: break
        r2 = r - f / df
        if abs(r2 - r) < 1e-10: return r2
        r = r2
    return r

def compound(principal, rate, n=1, t=1):
    """Compound interest. compound(1000, 0.05, 12, 10)"""
    return principal * (1 + rate / n) ** (n * t)

def cagr(start, end, years):
    """Compound annual growth rate. cagr(1000, 2000, 5) -> 0.1487"""
    return (end / start) ** (1.0 / years) - 1

def roi(gain, cost):
    """Return on investment %. roi(1500, 1000) -> 50.0"""
    return (gain - cost) / cost * 100.0

def breakeven(fixed, price, var_cost):
    """Break-even units. breakeven(10000, 25, 15) -> 1000"""
    return fixed / (price - var_cost)

def _fmt(v):
    if isinstance(v, bool):    return str(v)
    if isinstance(v, int):     return str(v)
    if isinstance(v, float):
        if isnan(v):           return "nan"
        if isinf(v):           return "inf" if v > 0 else "-inf"
        if v == int(v) and abs(v) < 1e15:
            return str(int(v))
        return "%.10g" % v
    if isinstance(v, complex): return str(v)
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(_fmt(x) for x in v) + "]"
    return str(v)

_raw   = "{safe_expr}"
_clean = _raw.strip()
if _clean.endswith('='): _clean = _clean[:-1].strip()
_clean = _clean.replace('^', '**').replace('×', '*').replace('÷', '/')

# "X% of Y" — e.g. "15% of 89.99"
_pm = _re.match(r'^([\d.]+)\s*(?:%%|percent)\s+of\s+([\d,. ]+)$', _clean, _re.I)
if _pm:
    print(_fmt(float(_pm.group(1)) / 100.0 *
               float(_pm.group(2).replace(',','').replace(' ',''))))
    sys.exit(0)

try:
    _r = eval(_clean)
    print(_fmt(_r))
except SyntaxError as _se:
    print("Syntax error: " + str(_se))
    sys.exit(1)
except Exception as _e:
    print("Error: " + str(_e))
    sys.exit(1)
"####,
        safe_expr = safe_expr,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 15
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

async fn run_hypothesis(args: &Value) -> Result<String, String> {
    let test_type = args["test"].as_str().unwrap_or("ttest_ind");
    let alpha = args["alpha"].as_f64().unwrap_or(0.05);
    let mu = args["mu"].as_f64().unwrap_or(0.0);

    let a_json = match &args["a"] {
        Value::Array(arr) => serde_json::to_string(arr).unwrap_or_else(|_| "None".to_string()),
        _ => "None".to_string(),
    };
    let b_json = match &args["b"] {
        Value::Array(arr) => serde_json::to_string(arr).unwrap_or_else(|_| "None".to_string()),
        _ => "None".to_string(),
    };
    let safe_path = args["path"]
        .as_str()
        .unwrap_or("")
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let col_a = args["column_a"].as_str().unwrap_or("a").replace('"', "\\\"");
    let col_b = args["column_b"].as_str().unwrap_or("").replace('"', "\\\"");

    let script = format!(
        r####"import math, sys, os

_test  = "{test_type}"
_alpha = {alpha}
_mu    = {mu}
_a     = {a_json}
_b     = {b_json}
_path  = "{safe_path}"
_col_a = "{col_a}"
_col_b = "{col_b}"

if _a is None and _path:
    import csv as _csv, sqlite3 as _sql3
    _ext  = os.path.splitext(_path)[1].lower().lstrip('.')
    _rows = []
    if _ext in ('csv', 'tsv'):
        _delim = '\t' if _ext == 'tsv' else ','
        with open(_path, encoding='utf-8-sig', errors='replace', newline='') as _fh:
            for _r in _csv.DictReader(_fh, delimiter=_delim):
                _rows.append(_r)
    elif _ext in ('db', 'sqlite', 'sqlite3'):
        with _sql3.connect(_path) as _con:
            _cur = _con.cursor()
            _cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
            _t = _cur.fetchone()
            if _t:
                _cur.execute("SELECT * FROM [%s]" % _t[0])
                _cs = [_d[0] for _d in _cur.description]
                _rows = [dict(zip(_cs, _r)) for _r in _cur.fetchall()]
    def _tryf(v):
        try: return float(str(v or '').replace(',','').strip())
        except: return None
    _a = [_tryf(_r.get(_col_a)) for _r in _rows]
    _a = [v for v in _a if v is not None]
    if _col_b:
        _b = [_tryf(_r.get(_col_b)) for _r in _rows]
        _b = [v for v in _b if v is not None]

if not _a:
    print("ERROR: no numeric data found for group A")
    sys.exit(1)

_na = len(_a)
_nb = len(_b) if _b else 0

try:
    from scipy import stats as _sc
    _HAS_SCI = True
except ImportError:
    _HAS_SCI = False

def _betainc(a, b, x):
    if x <= 0: return 0.0
    if x >= 1: return 1.0
    if x > (a + 1.0) / (a + b + 2.0):
        return 1.0 - _betainc(b, a, 1.0 - x)
    TINY = 1e-30; EPS = 3e-7
    lbeta = math.lgamma(a) + math.lgamma(b) - math.lgamma(a + b)
    front = math.exp(a*math.log(x) + b*math.log(1.0-x) - lbeta) / a
    f = 1.0; C = 1.0
    D = 1.0 - (a+b)*x/(a+1.0)
    if abs(D) < TINY: D = TINY
    D = 1.0/D; f = D
    for m in range(1, 201):
        n1 = m*(b-m)*x/((a+2*m-1)*(a+2*m))
        D = 1.0+n1*D; C = 1.0+n1/C
        if abs(D) < TINY: D = TINY
        if abs(C) < TINY: C = TINY
        D = 1.0/D; f *= D*C
        n2 = -(a+m)*(a+b+m)*x/((a+2*m)*(a+2*m+1))
        D = 1.0+n2*D; C = 1.0+n2/C
        if abs(D) < TINY: D = TINY
        if abs(C) < TINY: C = TINY
        D = 1.0/D; delta = D*C; f *= delta
        if abs(delta-1.0) < EPS: break
    return front * f

def _t2p(t, df):
    return _betainc(df/2.0, 0.5, df/(df + t*t))

def _gammaincc(a, x):
    if x <= 0: return 1.0
    if x < a + 1:
        _ap = a; _s = 1.0/a; _d = 1.0/a
        for _ in range(200):
            _ap += 1; _d *= x/_ap; _s += _d
            if abs(_d) < abs(_s)*3e-7: break
        return 1.0 - _s*math.exp(-x + a*math.log(x) - math.lgamma(a))
    _b2 = x+1-a; _c = 1e30; _d = 1.0/_b2; _h = _d
    for i in range(1, 201):
        _an = -i*(i-a); _b2 += 2
        _d = _an*_d + _b2
        if abs(_d) < 1e-30: _d = 1e-30
        _c = _b2 + _an/_c
        if abs(_c) < 1e-30: _c = 1e-30
        _d = 1.0/_d; _del = _d*_c; _h *= _del
        if abs(_del-1.0) < 3e-7: break
    return math.exp(-x + a*math.log(x) - math.lgamma(a)) * _h

_stat_v = None; _p_val = None; _extra = []; _test_name = ""; _n_info = ""

if _test == "ttest_1samp":
    _test_name = "One-Sample t-Test"
    _ma = sum(_a)/_na
    _sd = math.sqrt(sum((x-_ma)**2 for x in _a)/(_na-1)) if _na>1 else 0.0
    _se = _sd/math.sqrt(_na)
    _stat_v = (_ma - _mu)/_se if _se > 0 else 0.0
    _df = _na - 1
    _n_info = "n=%d  H0: mean=%.6g" % (_na, _mu)
    if _HAS_SCI:
        _res = _sc.ttest_1samp(_a, _mu)
        _stat_v, _p_val = float(_res.statistic), float(_res.pvalue)
    else:
        _p_val = _t2p(abs(_stat_v), _df)
    _extra = ["Sample mean: %.6g" % _ma, "Sample std dev: %.6g" % _sd, "df: %d" % _df]

elif _test == "ttest_ind":
    _test_name = "Independent-Samples t-Test (Welch)"
    if not _b:
        print("ERROR: ttest_ind requires two groups — provide 'a' and 'b'"); sys.exit(1)
    _ma = sum(_a)/_na; _mb = sum(_b)/_nb
    _va = sum((x-_ma)**2 for x in _a)/(_na-1) if _na>1 else 0.0
    _vb = sum((x-_mb)**2 for x in _b)/(_nb-1) if _nb>1 else 0.0
    _se = math.sqrt(_va/_na + _vb/_nb)
    _stat_v = (_ma - _mb)/_se if _se > 0 else 0.0
    _df_n = (_va/_na + _vb/_nb)**2
    _df_d = (_va/_na)**2/(_na-1) + (_vb/_nb)**2/(_nb-1) if _na>1 and _nb>1 else 1
    _df = _df_n/_df_d if _df_d > 0 else 1.0
    _n_info = "n_a=%d  n_b=%d" % (_na, _nb)
    if _HAS_SCI:
        _res = _sc.ttest_ind(_a, _b, equal_var=False)
        _stat_v, _p_val = float(_res.statistic), float(_res.pvalue)
    else:
        _p_val = _t2p(abs(_stat_v), _df)
    _extra = ["Mean A: %.6g" % _ma, "Mean B: %.6g" % _mb,
              "Std Dev A: %.6g" % math.sqrt(_va),
              "Std Dev B: %.6g" % math.sqrt(_vb),
              "df (Welch): %.1f" % _df]

elif _test == "ttest_rel":
    _test_name = "Paired t-Test"
    if not _b:
        print("ERROR: ttest_rel requires two paired groups — provide 'a' and 'b'"); sys.exit(1)
    _np2 = min(_na, _nb)
    _diffs = [_a[i]-_b[i] for i in range(_np2)]
    _md = sum(_diffs)/_np2
    _sd = math.sqrt(sum((d-_md)**2 for d in _diffs)/(_np2-1)) if _np2>1 else 0.0
    _se = _sd/math.sqrt(_np2) if _np2>0 else 0.0
    _stat_v = _md/_se if _se > 0 else 0.0
    _df = _np2 - 1
    _n_info = "n_pairs=%d" % _np2
    if _HAS_SCI:
        _res = _sc.ttest_rel(_a[:_np2], _b[:_np2])
        _stat_v, _p_val = float(_res.statistic), float(_res.pvalue)
    else:
        _p_val = _t2p(abs(_stat_v), _df)
    _extra = ["Mean difference: %.6g" % _md,
              "Std dev of diffs: %.6g" % _sd, "df: %d" % _df]

elif _test == "mannwhitney":
    _test_name = "Mann-Whitney U Test (non-parametric)"
    if not _b:
        print("ERROR: mannwhitney requires two groups — provide 'a' and 'b'"); sys.exit(1)
    _n_info = "n_a=%d  n_b=%d" % (_na, _nb)
    if _HAS_SCI:
        _res = _sc.mannwhitneyu(_a, _b, alternative='two-sided')
        _stat_v, _p_val = float(_res.statistic), float(_res.pvalue)
    else:
        _U = sum(1 if x>y else 0.5 if x==y else 0 for x in _a for y in _b)
        _stat_v = _U
        _mu_U = _na*_nb/2.0
        _sg_U = math.sqrt(_na*_nb*(_na+_nb+1)/12.0)
        _z = (_U - _mu_U)/_sg_U if _sg_U > 0 else 0.0
        _p_val = math.erfc(abs(_z)/math.sqrt(2))
        _extra.append("(Normal approximation — install scipy for exact result)")

elif _test == "chi2":
    _test_name = "Chi-Squared Goodness-of-Fit"
    _n_info = "k=%d bins" % _na
    _expected = list(_b) if _b else [sum(_a)/_na]*_na
    if len(_expected) != _na:
        print("ERROR: 'a' (observed) and 'b' (expected) must have equal length"); sys.exit(1)
    if _HAS_SCI:
        _res = _sc.chisquare(_a, f_exp=_expected)
        _stat_v, _p_val = float(_res.statistic), float(_res.pvalue)
    else:
        _stat_v = sum((o-e)**2/e for o, e in zip(_a, _expected) if e > 0)
        _df2 = _na - 1
        _p_val = _gammaincc(_df2/2.0, _stat_v/2.0)
        _extra.append("df=%d" % _df2)
else:
    print("ERROR: unknown test '%s'. Supported: ttest_1samp, ttest_ind, ttest_rel, mannwhitney, chi2" % _test)
    sys.exit(1)

_H2 = "##"
_out = []
_out.append(_H2 + " Hypothesis Test Results")
_out.append("")
_out.append("**Test:** " + _test_name)
_out.append("**Alpha:** %.3g" % _alpha)
_out.append("**Samples:** " + _n_info)
for _ex in _extra:
    _out.append("  - " + _ex)
_out.append("")
if _stat_v is not None:
    _out.append("**Test Statistic:** %.6g" % _stat_v)
if _p_val is not None:
    _out.append("**p-value:** %.6g" % _p_val)
    _out.append("")
    if _p_val < _alpha:
        _out.append("**Result: REJECT H0**  (p=%.5f < alpha=%.3g)" % (_p_val, _alpha))
        _out.append("Statistically significant — unlikely under the null hypothesis.")
    else:
        _out.append("**Result: FAIL TO REJECT H0**  (p=%.5f >= alpha=%.3g)" % (_p_val, _alpha))
        _out.append("Insufficient evidence to reject the null hypothesis.")
_out.append("")
_out.append("*Engine: %s*" % ("scipy.stats" if _HAS_SCI else "pure-Python (Lentz CF)"))
print("\n".join(_out))
"####,
        test_type = test_type,
        alpha = alpha,
        mu = mu,
        a_json = a_json,
        b_json = b_json,
        safe_path = safe_path,
        col_a = col_a,
        col_b = col_b,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ─── Matrix operations ────────────────────────────────────────────────────────

async fn run_matrix(args: &Value) -> Result<String, String> {
    let operation = args["operation"].as_str().unwrap_or("det");

    let a_json = match &args["a"] {
        Value::Array(arr) => serde_json::to_string(arr).unwrap_or_else(|_| "None".to_string()),
        _ => return Err("Missing 'a' (matrix as nested array) for matrix mode".into()),
    };
    let b_json = match &args["b"] {
        Value::Array(arr) => serde_json::to_string(arr).unwrap_or_else(|_| "None".to_string()),
        _ => "None".to_string(),
    };

    let script = format!(
        r####"import sys, math

_op = "{operation}"
_a  = {a_json}
_b  = {b_json}

try:
    import numpy as _np
    _HAS_NP = True
except ImportError:
    _HAS_NP = False

_A = _np.array(_a, dtype=float) if _HAS_NP else _a
_B = _np.array(_b, dtype=float) if (_HAS_NP and _b is not None) else _b

_H2 = "##"
_out = []

def _fmt_row(row):
    return "  " + "  ".join("%12.6g" % float(x) for x in row)

def _pp(M):
    if _HAS_NP:
        if M.ndim == 1:
            _out.append("  [" + ", ".join("%.6g" % x for x in M) + "]")
        else:
            for row in M: _out.append(_fmt_row(row))
    else:
        if isinstance(M[0], list):
            for row in M: _out.append(_fmt_row(row))
        else:
            _out.append("  [" + ", ".join("%.6g" % x for x in M) + "]")

def _det_py(m):
    n = len(m); m = [list(r) for r in m]; sign = 1
    for i in range(n):
        p = max(range(i, n), key=lambda r: abs(m[r][i]))
        if abs(m[p][i]) < 1e-12: return 0.0
        if p != i: m[i], m[p] = m[p], m[i]; sign *= -1
        for j in range(i+1, n):
            f = m[j][i] / m[i][i]
            for k in range(i, n): m[j][k] -= f * m[i][k]
    d = sign
    for i in range(n): d *= m[i][i]
    return d

def _matmul_py(A, B):
    n, m = len(A), len(A[0])
    if isinstance(B[0], list):
        p = len(B[0])
        return [[sum(A[i][k]*B[k][j] for k in range(m)) for j in range(p)] for i in range(n)]
    return [sum(A[i][k]*B[k] for k in range(m)) for i in range(n)]

if _op == "det":
    _out.append(_H2 + " Determinant")
    _out.append("")
    _d = float(_np.linalg.det(_A)) if _HAS_NP else _det_py(_a)
    _out.append("det(A) = %.10g" % _d)
    if _HAS_NP:
        _out.append("Shape: %dx%d" % (_A.shape[0], _A.shape[1]))

elif _op == "invert":
    if not _HAS_NP:
        print("ERROR: invert requires numpy (pip install numpy)"); sys.exit(1)
    _out.append(_H2 + " Matrix Inverse")
    _out.append("")
    try:
        _R = _np.linalg.inv(_A)
        _pp(_R)
        _out.append("")
        _out.append("Condition number: %.4g" % _np.linalg.cond(_A))
    except _np.linalg.LinAlgError as _e:
        print("ERROR: " + str(_e)); sys.exit(1)

elif _op == "eigenvalues":
    if not _HAS_NP:
        print("ERROR: eigenvalues requires numpy (pip install numpy)"); sys.exit(1)
    _out.append(_H2 + " Eigenvalues & Eigenvectors")
    _out.append("")
    _evals, _evecs = _np.linalg.eig(_A)
    for i, (ev, vec) in enumerate(zip(_evals, _evecs.T)):
        if abs(ev.imag) < 1e-10:
            _out.append("lambda_%d = %.8g" % (i+1, ev.real))
        else:
            _out.append("lambda_%d = %.6g + %.6gi" % (i+1, ev.real, ev.imag))
        _out.append("  eigenvector: [" + ", ".join("%.4f" % x.real for x in vec) + "]")

elif _op == "solve":
    if _b is None:
        print("ERROR: solve requires 'b' (right-hand side vector or matrix)"); sys.exit(1)
    if not _HAS_NP:
        print("ERROR: solve requires numpy (pip install numpy)"); sys.exit(1)
    _out.append(_H2 + " Solution to Ax = b")
    _out.append("")
    try:
        _x = _np.linalg.solve(_A, _B.flatten() if _B.ndim > 1 else _B)
        _out.append("x = [" + ", ".join("%.8g" % v for v in _x) + "]")
        _out.append("")
        _out.append("Residual ||Ax-b||: %.2e" % float(_np.linalg.norm(_A @ _x - _B.flatten())))
    except _np.linalg.LinAlgError as _e:
        print("ERROR: " + str(_e)); sys.exit(1)

elif _op == "transpose":
    _out.append(_H2 + " Transpose")
    _out.append("")
    if _HAS_NP:
        _pp(_A.T)
    else:
        _pp([[_a[j][i] for j in range(len(_a))] for i in range(len(_a[0]))])

elif _op == "multiply":
    if _b is None:
        print("ERROR: multiply requires both 'a' and 'b'"); sys.exit(1)
    _out.append(_H2 + " Matrix Product (A @ B)")
    _out.append("")
    if _HAS_NP:
        _pp(_A @ _B)
    else:
        _pp(_matmul_py(_a, _b))

elif _op == "rank":
    if not _HAS_NP:
        print("ERROR: rank requires numpy (pip install numpy)"); sys.exit(1)
    _out.append(_H2 + " Matrix Rank")
    _out.append("")
    _out.append("rank(A) = %d" % _np.linalg.matrix_rank(_A))
    _out.append("Shape:   %dx%d" % (_A.shape[0], _A.shape[1]))

elif _op == "svd":
    if not _HAS_NP:
        print("ERROR: SVD requires numpy (pip install numpy)"); sys.exit(1)
    _out.append(_H2 + " Singular Value Decomposition")
    _out.append("")
    _U, _S, _Vt = _np.linalg.svd(_A)
    _out.append("Singular values: [" + ", ".join("%.6g" % s for s in _S) + "]")
    _out.append("Rank (numerical): %d" % _np.linalg.matrix_rank(_A))
    _out.append("")
    _out.append("U (%dx%d):" % (_U.shape[0], _U.shape[1]))
    _pp(_U)
    _out.append("Vt (%dx%d):" % (_Vt.shape[0], _Vt.shape[1]))
    _pp(_Vt)

else:
    print("ERROR: unknown operation '%s'. Supported: det, invert, eigenvalues, solve, transpose, multiply, rank, svd" % _op)
    sys.exit(1)

print("\n".join(_out))
"####,
        operation = operation,
        a_json = a_json,
        b_json = b_json,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 20
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ─── Unit conversion ─────────────────────────────────────────────────────────

// The unit table is a `const` so Python dict braces never touch format!().
const UNIT_TABLE_PY: &str = r####"
_U = {}
def _r(names, factor, cat):
    for n in names: _U[n] = (factor, cat)

# Length (SI base: metre)
_r(['m','meter','meters','metre','metres'], 1.0, 'length')
_r(['km','kilometer','kilometers','kilometre','kilometres'], 1e3, 'length')
_r(['cm','centimeter','centimeters'], 1e-2, 'length')
_r(['mm','millimeter','millimeters'], 1e-3, 'length')
_r(['um','micrometer','micron','microns'], 1e-6, 'length')
_r(['nm','nanometer','nanometers'], 1e-9, 'length')
_r(['pm','picometer'], 1e-12, 'length')
_r(['in','inch','inches'], 0.0254, 'length')
_r(['ft','foot','feet'], 0.3048, 'length')
_r(['yd','yard','yards'], 0.9144, 'length')
_r(['mi','mile','miles'], 1609.344, 'length')
_r(['nmi','nautical_mile','nautical_miles'], 1852.0, 'length')
_r(['ly','lightyear','light_year','lightyears'], 9.4607304725808e15, 'length')
_r(['au','astronomical_unit'], 1.495978707e11, 'length')
_r(['pc','parsec','parsecs'], 3.085677581e16, 'length')
_r(['ang','angstrom'], 1e-10, 'length')
_r(['fathom','fathoms'], 1.8288, 'length')
# Mass (SI base: kilogram)
_r(['kg','kilogram','kilograms'], 1.0, 'mass')
_r(['g','gram','grams'], 1e-3, 'mass')
_r(['mg','milligram','milligrams'], 1e-6, 'mass')
_r(['ug','microgram','micrograms'], 1e-9, 'mass')
_r(['t','tonne','metric_ton','metric_tons'], 1e3, 'mass')
_r(['lb','lbs','pound','pounds'], 0.45359237, 'mass')
_r(['oz','ounce','ounces'], 0.028349523125, 'mass')
_r(['ton','short_ton'], 907.18474, 'mass')
_r(['long_ton'], 1016.0469088, 'mass')
_r(['stone','stones'], 6.35029318, 'mass')
_r(['slug','slugs'], 14.593903, 'mass')
_r(['carat','carats','ct'], 2e-4, 'mass')
# Time (SI base: second)
_r(['s','sec','second','seconds'], 1.0, 'time')
_r(['ms','millisecond','milliseconds'], 1e-3, 'time')
_r(['us','microsecond','microseconds'], 1e-6, 'time')
_r(['ns','nanosecond','nanoseconds'], 1e-9, 'time')
_r(['min','minute','minutes'], 60.0, 'time')
_r(['h','hr','hour','hours'], 3600.0, 'time')
_r(['d','day','days'], 86400.0, 'time')
_r(['wk','week','weeks'], 604800.0, 'time')
_r(['month','months'], 2629746.0, 'time')
_r(['yr','year','years'], 31556952.0, 'time')
_r(['decade','decades'], 315569520.0, 'time')
_r(['century','centuries'], 3155695200.0, 'time')
# Speed (SI base: m/s)
_r(['m/s','mps','meters_per_second'], 1.0, 'speed')
_r(['km/h','kph','kmh','kilometers_per_hour'], 1.0/3.6, 'speed')
_r(['mph','miles_per_hour'], 0.44704, 'speed')
_r(['knot','knots','kn'], 0.514444, 'speed')
_r(['ft/s','fps','feet_per_second'], 0.3048, 'speed')
_r(['mach'], 340.29, 'speed')
_r(['c_speed','speed_of_light'], 299792458.0, 'speed')
# Energy (SI base: joule)
_r(['j','joule','joules'], 1.0, 'energy')
_r(['kj','kilojoule','kilojoules'], 1e3, 'energy')
_r(['mj','megajoule','megajoules'], 1e6, 'energy')
_r(['gj','gigajoule','gigajoules'], 1e9, 'energy')
_r(['cal','calorie','calories'], 4.184, 'energy')
_r(['kcal','kilocalorie','kilocalories','cal_food'], 4184.0, 'energy')
_r(['kwh','kw*h','kilowatt_hour','kilowatt_hours'], 3.6e6, 'energy')
_r(['mwh','megawatt_hour'], 3.6e9, 'energy')
_r(['ev','electronvolt','electronvolts'], 1.602176634e-19, 'energy')
_r(['btu','british_thermal_unit'], 1055.06, 'energy')
_r(['erg','ergs'], 1e-7, 'energy')
_r(['therm'], 1.05506e8, 'energy')
# Power (SI base: watt)
_r(['w','watt','watts'], 1.0, 'power')
_r(['kw','kilowatt','kilowatts'], 1e3, 'power')
_r(['mw','megawatt','megawatts'], 1e6, 'power')
_r(['gw','gigawatt','gigawatts'], 1e9, 'power')
_r(['hp','horsepower'], 745.69987, 'power')
_r(['ps','metric_horsepower'], 735.49875, 'power')
_r(['btu/h','btu_per_hour'], 0.293071, 'power')
# Pressure (SI base: pascal)
_r(['pa','pascal','pascals'], 1.0, 'pressure')
_r(['kpa','kilopascal','kilopascals'], 1e3, 'pressure')
_r(['mpa','megapascal','megapascals'], 1e6, 'pressure')
_r(['gpa','gigapascal','gigapascals'], 1e9, 'pressure')
_r(['atm','atmosphere','atmospheres'], 101325.0, 'pressure')
_r(['bar','bars'], 1e5, 'pressure')
_r(['mbar','millibar','millibars'], 100.0, 'pressure')
_r(['psi','pounds_per_square_inch'], 6894.757, 'pressure')
_r(['mmhg','torr'], 133.322, 'pressure')
_r(['inhg','inches_of_mercury'], 3386.39, 'pressure')
_r(['atm_tech','at','technical_atmosphere'], 98066.5, 'pressure')
# Temperature — special (handled separately, marker category)
_r(['c','celsius','degc','deg_c'], ('temp', 'C'), 'temperature')
_r(['f','fahrenheit','degf','deg_f'], ('temp', 'F'), 'temperature')
_r(['k','kelvin','degk','deg_k'], ('temp', 'K'), 'temperature')
_r(['r','rankine','degr','deg_r'], ('temp', 'R'), 'temperature')
# Volume (SI base: litre)
_r(['l','liter','liters','litre','litres'], 1.0, 'volume')
_r(['ml','milliliter','milliliters'], 1e-3, 'volume')
_r(['cl','centiliter','centiliters'], 1e-2, 'volume')
_r(['dl','deciliter','deciliters'], 0.1, 'volume')
_r(['ul','microliter','microliters'], 1e-6, 'volume')
_r(['m3','cubic_meter','cubic_meters'], 1e3, 'volume')
_r(['cm3','cc','cubic_centimeter'], 1e-3, 'volume')
_r(['mm3','cubic_millimeter'], 1e-6, 'volume')
_r(['gal','gallon','gallons','us_gal'], 3.785411784, 'volume')
_r(['qt','quart','quarts'], 0.946352946, 'volume')
_r(['pt','pint','pints'], 0.473176473, 'volume')
_r(['cup','cups'], 0.2365882365, 'volume')
_r(['fl_oz','fluid_ounce','fluid_ounces'], 0.0295735296, 'volume')
_r(['tsp','teaspoon','teaspoons'], 0.00492892, 'volume')
_r(['tbsp','tablespoon','tablespoons'], 0.01478676, 'volume')
_r(['imp_gal','imperial_gallon','imperial_gallons'], 4.54609, 'volume')
_r(['barrel','bbl'], 158.9873, 'volume')
# Area (SI base: square metre)
_r(['m2','sq_m','square_meter','square_meters'], 1.0, 'area')
_r(['km2','square_kilometer','square_kilometers'], 1e6, 'area')
_r(['cm2','square_centimeter'], 1e-4, 'area')
_r(['mm2','square_millimeter'], 1e-6, 'area')
_r(['ft2','sq_ft','square_foot','square_feet'], 0.09290304, 'area')
_r(['in2','sq_in','square_inch','square_inches'], 6.4516e-4, 'area')
_r(['yd2','sq_yd','square_yard','square_yards'], 0.83612736, 'area')
_r(['mi2','square_mile','square_miles'], 2589988.11, 'area')
_r(['acre','acres'], 4046.8564224, 'area')
_r(['ha','hectare','hectares'], 1e4, 'area')
# Digital storage (SI base: byte)
_r(['bit','bits'], 0.125, 'digital')
_r(['b','byte','bytes'], 1.0, 'digital')
_r(['kb','kilobyte','kilobytes'], 1e3, 'digital')
_r(['mb','megabyte','megabytes'], 1e6, 'digital')
_r(['gb','gigabyte','gigabytes'], 1e9, 'digital')
_r(['tb','terabyte','terabytes'], 1e12, 'digital')
_r(['pb','petabyte','petabytes'], 1e15, 'digital')
_r(['kib','kibibyte','kibibytes'], 1024.0, 'digital')
_r(['mib','mebibyte','mebibytes'], 1048576.0, 'digital')
_r(['gib','gibibyte','gibibytes'], 1073741824.0, 'digital')
_r(['tib','tebibyte','tebibytes'], 1099511627776.0, 'digital')
# Force (SI base: newton)
_r(['n','newton','newtons'], 1.0, 'force')
_r(['kn','kilonewton','kilonewtons'], 1e3, 'force')
_r(['mn_force','meganewton'], 1e6, 'force')
_r(['lbf','pound_force','pounds_force'], 4.44822, 'force')
_r(['kgf','kilogram_force'], 9.80665, 'force')
_r(['dyn','dyne','dynes'], 1e-5, 'force')
# Frequency (SI base: Hz)
_r(['hz','hertz'], 1.0, 'frequency')
_r(['khz','kilohertz'], 1e3, 'frequency')
_r(['mhz','megahertz'], 1e6, 'frequency')
_r(['ghz','gigahertz'], 1e9, 'frequency')
_r(['thz','terahertz'], 1e12, 'frequency')
_r(['rpm','rev_per_min','revolutions_per_minute'], 1.0/60, 'frequency')
# Angle (SI base: radian)
_r(['rad','radian','radians'], 1.0, 'angle')
_r(['deg','degree','degrees'], 3.14159265358979/180, 'angle')
_r(['grad','gradian','gradians'], 3.14159265358979/200, 'angle')
_r(['arcmin','arcminute','arcminutes'], 3.14159265358979/10800, 'angle')
_r(['arcsec','arcsecond','arcseconds'], 3.14159265358979/648000, 'angle')
_r(['rev','revolution','revolutions','turn','turns'], 2*3.14159265358979, 'angle')

def _to_celsius(v, scale):
    if scale=='C': return v
    if scale=='F': return (v-32)*5/9
    if scale=='K': return v-273.15
    if scale=='R': return (v-491.67)*5/9
    return None

def _from_celsius(c, scale):
    if scale=='C': return c
    if scale=='F': return c*9/5+32
    if scale=='K': return c+273.15
    if scale=='R': return (c+273.15)*9/5
    return None

def _convert(val, from_u, to_u):
    _fk = from_u.lower().strip().replace(' ','_').replace('/','/')
    _tk = to_u.lower().strip().replace(' ','_').replace('/','/')
    _fi = _U.get(_fk)
    _ti = _U.get(_tk)
    if _fi is None: return None, "Unknown unit: " + from_u
    if _ti is None: return None, "Unknown unit: " + to_u
    if _fi[1] == 'temperature' or _ti[1] == 'temperature':
        if _fi[1] != 'temperature' or _ti[1] != 'temperature':
            return None, "Cannot mix temperature and non-temperature units"
        _c = _to_celsius(val, _fi[0][1])
        return _from_celsius(_c, _ti[0][1]), None
    if _fi[1] != _ti[1]:
        return None, "Dimension mismatch: %s (%s) vs %s (%s)" % (from_u, _fi[1], to_u, _ti[1])
    return val * _fi[0] / _ti[0], None
"####;

pub async fn convert_units(expr: &str) -> Result<String, String> {
    if expr.trim().is_empty() {
        return Err("No expression provided.".into());
    }
    let safe_expr = expr.replace('\\', "\\\\").replace('"', "\\\"");

    let script = format!(
        r####"{unit_table}
import re as _re, sys, math

_raw  = "{safe_expr}"
_expr = _raw.strip()

# ── Number base conversion (prefix check) ────────────────────────────
_bm = _re.match(
    r'^(0x[0-9a-fA-F]+|0b[01]+|0o[0-7]+|\d+)\s+to\s+(hex(?:adecimal)?|dec(?:imal)?|bin(?:ary)?|oct(?:al)?)\s*$',
    _expr, _re.I)
if _bm:
    _bv, _bt = _bm.group(1), _bm.group(2).lower()
    try:
        _n = int(_bv, 0)
        if   _bt.startswith('hex'): _out = hex(_n)
        elif _bt.startswith('bin'): _out = bin(_n)
        elif _bt.startswith('oct'): _out = oct(_n)
        else:                        _out = str(_n)
        print("%s  =  %s" % (_bv, _out))
    except ValueError as _e:
        print("Error: " + str(_e)); sys.exit(1)
    sys.exit(0)

_m = _re.match(
    r'^([\d.,eE+\-]+)\s+(.+?)\s+(?:to|->|=|in)\s+(.+)$', _expr, _re.I)
if not _m:
    print("Format: VALUE UNIT to UNIT")
    print("Examples:  100 mph to km/h  |  72 F to C  |  1 lightyear to km  |  5 kg to lbs")
    sys.exit(1)

_val   = float(_m.group(1).replace(',',''))
_from  = _m.group(2).strip()
_to    = _m.group(3).strip()

_result, _err = _convert(_val, _from, _to)
if _err:
    print("Error: " + _err)
    sys.exit(1)

def _fmtv(v):
    if v == 0: return "0"
    if abs(v) >= 1e12 or (abs(v) < 1e-4 and abs(v) > 0):
        return "%.6e" % v
    if v == int(v) and abs(v) < 1e15: return str(int(v))
    return "%.10g" % v

print("%s %s  =  %s %s" % (_fmtv(_val), _from, _fmtv(_result), _to))
"####,
        unit_table = UNIT_TABLE_PY,
        safe_expr = safe_expr,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 15
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ─── Data visualization ───────────────────────────────────────────────────────

pub async fn plot_dataset(
    path_str: &str,
    plot_type: &str,
    x_col: &str,
    y_col: &str,
    out_path: &str,
) -> Result<String, String> {
    let safe_path = path_str.replace('\\', "\\\\").replace('"', "\\\"");
    let safe_out  = out_path.replace('\\', "\\\\").replace('"', "\\\"");
    let safe_x    = x_col.replace('"', "\\\"");
    let safe_y    = y_col.replace('"', "\\\"");

    let script = format!(
        r####"import os, sys, csv as _csv, sqlite3 as _sql3

os.environ['MPLBACKEND']   = 'Agg'
os.environ['MPLCONFIGDIR'] = os.environ.get('TEMP', os.environ.get('TMP', '/tmp')) + '/hematite_mpl'

_path      = "{safe_path}"
_out_path  = "{safe_out}"
_plot_type = "{plot_type}"
_x_col     = "{safe_x}"
_y_col     = "{safe_y}"
_ext       = os.path.splitext(_path)[1].lower().lstrip('.')
_data      = []

if _ext in ('csv', 'tsv'):
    _delim = '\t' if _ext == 'tsv' else ','
    with open(_path, encoding='utf-8-sig', errors='replace', newline='') as _fh:
        _rdr = _csv.DictReader(_fh, delimiter=_delim)
        for _i, _r in enumerate(_rdr):
            if _i >= 10000: break
            _data.append(_r)
elif _ext == 'json':
    with open(_path, encoding='utf-8') as _fh:
        _raw2 = json.load(_fh)
    _data = _raw2[:10000] if isinstance(_raw2, list) else list(_raw2.values())[0][:10000] if isinstance(_raw2, dict) else []
elif _ext in ('db','sqlite','sqlite3'):
    with _sql3.connect(_path) as _con:
        _cur = _con.cursor()
        _cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        _t = _cur.fetchone()
        if _t:
            _cur.execute("SELECT * FROM [%s] LIMIT 10000" % _t[0])
            _cs = [_d[0] for _d in _cur.description]
            _data = [dict(zip(_cs, _r)) for _r in _cur.fetchall()]
else:
    print("ERROR: unsupported format"); sys.exit(1)

if not _data:
    print("No data found."); sys.exit(1)

_cols = list(_data[0].keys())

def _tryf(v):
    try: return float(str(v or '').replace(',','').strip())
    except: return None

_num_cols = []
for _c in _cols:
    _s = [_tryf(_r.get(_c)) for _r in _data[:200]]
    if sum(1 for x in _s if x is not None) >= len(_s)*0.8: _num_cols.append(_c)

_x_col2 = _x_col or (_num_cols[0] if _num_cols else _cols[0])
_y_col2 = _y_col or (_num_cols[1] if len(_num_cols) > 1 else None)

_x_vals = [_tryf(_r.get(_x_col2)) for _r in _data]
_x_vals = [v for v in _x_vals if v is not None]
_y_vals = []
if _y_col2:
    _y_vals = [_tryf(_r.get(_y_col2)) for _r in _data]
    _y_vals = [v for v in _y_vals if v is not None]

_title = os.path.basename(_path)
if _y_col2:
    _sub = "%s vs %s" % (_x_col2, _y_col2)
else:
    _sub = _x_col2

# ── Attempt matplotlib ────────────────────────────────────────────────
_used_mpl = False
_svg_str   = ""
try:
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as _plt
    _fig, _ax = _plt.subplots(figsize=(8, 5))
    _fig.patch.set_facecolor('#0d0d1a')
    _ax.set_facecolor('#16213e')
    for _sp in _ax.spines.values(): _sp.set_color('#444')
    _ax.tick_params(colors='#999', labelsize=9)
    _ax.xaxis.label.set_color('#bbb')
    _ax.yaxis.label.set_color('#bbb')
    _ax.title.set_color('#7fc3ff')
    _C = '#4a9eff'
    if _plot_type == 'histogram':
        _ax.hist(_x_vals, bins=min(40, max(10, int(len(_x_vals)**0.5)+1)),
                 color=_C, alpha=0.85, edgecolor='#0d0d1a')
        _ax.set_xlabel(_x_col2); _ax.set_ylabel('Count')
        _ax.set_title('Histogram — ' + _x_col2)
    elif _plot_type in ('scatter',''):
        _nx = min(len(_x_vals), len(_y_vals))
        _ax.scatter(_x_vals[:_nx], _y_vals[:_nx], color=_C, alpha=0.6, s=15)
        _ax.set_xlabel(_x_col2); _ax.set_ylabel(_y_col2 or '')
        _ax.set_title('Scatter — ' + _sub)
    elif _plot_type == 'line':
        _pairs = sorted(zip(_x_vals, _y_vals))
        _ax.plot([p[0] for p in _pairs], [p[1] for p in _pairs], color=_C, lw=1.5)
        _ax.set_xlabel(_x_col2); _ax.set_ylabel(_y_col2 or '')
        _ax.set_title('Line — ' + _sub)
    elif _plot_type == 'bar':
        from collections import Counter as _Ctr
        _raw_x = [str(_r.get(_x_col2, '') or '').strip() for _r in _data if _r.get(_x_col2)]
        _ct = _Ctr(_raw_x)
        _lbls = [k for k, _ in _ct.most_common(20)]
        _vals2 = [_ct[k] for k in _lbls]
        _ax.bar(range(len(_lbls)), _vals2, color=_C, alpha=0.85)
        _ax.set_xticks(list(range(len(_lbls))))
        _ax.set_xticklabels(_lbls, rotation=40, ha='right', fontsize=8)
        _ax.set_title('Bar — ' + _x_col2)
    from io import StringIO as _SIO
    _buf = _SIO()
    _fig.tight_layout(pad=1.2)
    _fig.savefig(_buf, format='svg', bbox_inches='tight', facecolor=_fig.get_facecolor())
    _plt.close(_fig)
    _sv = _buf.getvalue()
    _svg_str = _sv[_sv.find('<svg'):]
    _used_mpl = True
except Exception:
    pass

# ── Pure-Python SVG fallback ──────────────────────────────────────────
if not _used_mpl:
    def _hist_svg(vals, lbl, W=640, H=380):
        if not vals: return ""
        mn, mx = min(vals), max(vals)
        if mn == mx: mn -= 0.5; mx += 0.5
        nb = min(30, max(8, int(len(vals)**0.5)+1))
        bw2 = (mx-mn)/nb
        bins = [0]*nb
        for v in vals:
            i = min(int((v-mn)/bw2), nb-1)
            bins[i] += 1
        mc = max(bins) or 1
        P=50; PW=W-2*P; PH=H-2*P
        rects = ''.join(
            '<rect x="%.1f" y="%.1f" width="%.1f" height="%.1f" fill="#4a9eff" opacity=".82"/>' %
            (P+i*PW/nb, P+PH-bins[i]/mc*PH, max(PW/nb-1,1), bins[i]/mc*PH)
            for i in range(nb))
        xt = ''.join('<text x="%.1f" y="%d" text-anchor="middle" font-size="10" fill="#888">%.3g</text>' %
                     (P+k*PW/4, H-8, mn+(mx-mn)*k/4) for k in range(5))
        yt = ''.join('<text x="%d" y="%.1f" text-anchor="end" font-size="10" fill="#888">%d</text>' %
                     (P-4, P+PH-k*PH/4+4, int(mc*k/4)) for k in range(5))
        axs = '<line x1="%d" y1="%d" x2="%d" y2="%d" stroke="#444"/><line x1="%d" y1="%d" x2="%d" y2="%d" stroke="#444"/>'%(P,P,P,P+PH,P,P+PH,P+PW,P+PH)
        ttl = '<text x="%d" y="22" text-anchor="middle" font-size="13" fill="#7fc3ff" font-weight="bold">Histogram — %s</text>'%(W//2,lbl[:50])
        return '<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" style="background:#16213e">%s%s%s%s%s</svg>'%(W,H,ttl,axs,rects,xt,yt)

    def _scatter_svg(xs, ys, xl, yl, W=640, H=400):
        if not xs or not ys: return ""
        xmn,xmx=min(xs),max(xs); ymn,ymx=min(ys),max(ys)
        if xmn==xmx: xmn-=1;xmx+=1
        if ymn==ymx: ymn-=1;ymx+=1
        P=60; PW=W-2*P; PH=H-2*P
        def xp(v): return P+(v-xmn)/(xmx-xmn)*PW
        def yp(v): return P+PH-(v-ymn)/(ymx-ymn)*PH
        dots=''.join('<circle cx="%.1f" cy="%.1f" r="3" fill="#4a9eff" opacity=".65"/>'%(xp(x),yp(y)) for x,y in zip(xs[:3000],ys[:3000]))
        axs='<line x1="%d" y1="%d" x2="%d" y2="%d" stroke="#444"/><line x1="%d" y1="%d" x2="%d" y2="%d" stroke="#444"/>'%(P,P,P,P+PH,P,P+PH,P+PW,P+PH)
        xt=''.join('<text x="%.1f" y="%d" text-anchor="middle" font-size="10" fill="#888">%.3g</text>'%(P+k*PW/4,P+PH+16,xmn+(xmx-xmn)*k/4) for k in range(5))
        yt=''.join('<text x="%d" y="%.1f" text-anchor="end" font-size="10" fill="#888">%.3g</text>'%(P-4,P+PH-k*PH/4+4,ymn+(ymx-ymn)*k/4) for k in range(5))
        xl2='<text x="%d" y="%d" text-anchor="middle" font-size="11" fill="#bbb">%s</text>'%(W//2,H-2,xl[:40])
        ttl='<text x="%d" y="20" text-anchor="middle" font-size="13" fill="#7fc3ff" font-weight="bold">Scatter — %s vs %s</text>'%(W//2,xl[:25],yl[:25])
        return '<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" style="background:#16213e">%s%s%s%s%s%s</svg>'%(W,H,ttl,axs,dots,xt,yt,xl2)

    if _plot_type in ('scatter','line') and _x_vals and _y_vals:
        _nx = min(len(_x_vals), len(_y_vals))
        _svg_str = _scatter_svg(_x_vals[:_nx], _y_vals[:_nx], _x_col2, _y_col2 or '')
    else:
        _svg_str = _hist_svg(_x_vals, _x_col2)

# ── Write HTML ────────────────────────────────────────────────────────
_engine = "matplotlib" if _used_mpl else "pure-Python SVG"
_html = (
    "<!DOCTYPE html><html><head><meta charset='utf-8'><title>" + _title + "</title>"
    "<style>body{{background:#0d0d1a;color:#e0e0e0;font-family:monospace;padding:24px;margin:0}}"
    "h2{{color:#7fc3ff;margin-bottom:4px}}p{{color:#666;font-size:.85em;margin:0 0 20px}}"
    ".chart{{display:block;margin:0 auto;max-width:700px}}</style></head><body>"
    "<h2>" + _title + " &mdash; " + _sub + "</h2>"
    "<p>Generated by Hematite &middot; engine: " + _engine + " &middot; n=" + str(len(_x_vals)) + " rows</p>"
    "<div class='chart'>" + _svg_str + "</div>"
    "</body></html>"
)
os.makedirs(os.path.dirname(_out_path), exist_ok=True)
with open(_out_path, 'w', encoding='utf-8') as _f:
    _f.write(_html)
print(_out_path)
"####,
        safe_path = safe_path,
        safe_out  = safe_out,
        plot_type = plot_type,
        safe_x    = safe_x,
        safe_y    = safe_y,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ─── SQL-on-local-files ────────────────────────────────────────────────────────

pub async fn query_data(file_path: &str, sql: &str) -> Result<String, String> {
    if file_path.trim().is_empty() {
        return Err("No data file specified.".into());
    }
    if sql.trim().is_empty() {
        return Err("No SQL query specified.".into());
    }
    let safe_path = file_path.replace('\\', "\\\\").replace('"', "\\\"");
    // Hex-encode the SQL to eliminate all escaping concerns.
    let sql_hex: String = sql.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(
        r####"import sqlite3 as _sq, csv as _csv, json as _js, sys, os

_path = "{safe_path}"
_sql  = bytes.fromhex("{sql_hex}").decode()
_ext  = os.path.splitext(_path)[1].lower().lstrip('.')
_con  = _sq.connect(':memory:')

def _load_csv(path, delim):
    with open(path, encoding='utf-8-sig', errors='replace', newline='') as _fh:
        _rdr = _csv.DictReader(_fh, delimiter=delim)
        _rows = list(_rdr)
    if not _rows:
        print("No data in file."); sys.exit(1)
    _cols = list(_rows[0].keys())
    _con.execute('CREATE TABLE data (' + ', '.join('"' + c + '"' for c in _cols) + ')')
    _con.executemany(
        'INSERT INTO data VALUES (' + ','.join(['?'] * len(_cols)) + ')',
        [tuple(_r.get(c, '') for c in _cols) for _r in _rows])

def _load_json(path):
    with open(path, encoding='utf-8') as _fh:
        _d = _js.load(_fh)
    _rows = _d if isinstance(_d, list) else next(iter(_d.values()), []) if isinstance(_d, dict) else []
    if not _rows:
        print("No rows found in JSON."); sys.exit(1)
    _cols = list(_rows[0].keys()) if isinstance(_rows[0], dict) else [str(i) for i in range(len(_rows[0]))]
    _con.execute('CREATE TABLE data (' + ', '.join('"' + c + '"' for c in _cols) + ')')
    _con.executemany(
        'INSERT INTO data VALUES (' + ','.join(['?'] * len(_cols)) + ')',
        [tuple(str(_r.get(c, '') if isinstance(_r, dict) else _r[i]) for i, c in enumerate(_cols)) for _r in _rows])

try:
    if _ext == 'csv':                      _load_csv(_path, ',')
    elif _ext == 'tsv':                    _load_csv(_path, '\t')
    elif _ext == 'json':                   _load_json(_path)
    elif _ext in ('db','sqlite','sqlite3'):
        _src = _sq.connect(_path); _src.backup(_con); _src.close()
    else:
        print("Unsupported format: " + _ext + ". Use csv, tsv, json, or sqlite.")
        sys.exit(1)
except Exception as _e:
    print("Load error: " + str(_e), file=sys.stderr); sys.exit(1)

try:
    _cur = _con.execute(_sql)
except Exception as _e:
    print("Query error: " + str(_e), file=sys.stderr); sys.exit(1)

_hdrs  = [_d[0] for _d in _cur.description] if _cur.description else []
_rows2 = _cur.fetchall()
_con.close()

if not _rows2:
    print("(no rows returned)")
    sys.exit(0)

_rs = [[str(c) if c is not None else 'NULL' for c in _r] for _r in _rows2[:2000]]
_ws = [max(len(_h), max((len(_r[_i]) for _r in _rs), default=0))
       for _i, _h in enumerate(_hdrs)]
_sep = '+-' + '-+-'.join('-' * _w for _w in _ws) + '-+'
_hr  = '| ' + ' | '.join(_h.ljust(_ws[_i]) for _i, _h in enumerate(_hdrs)) + ' |'
print(_sep)
print(_hr)
print(_sep)
for _r in _rs:
    print('| ' + ' | '.join(_r[_i].ljust(_ws[_i]) for _i in range(len(_hdrs))) + ' |')
print(_sep)
_total = len(_rows2)
_label = str(_total) + (' rows' if _total != 1 else ' row')
if _total > 2000: _label += ' (showing first 2000)'
print('(' + _label + ')')
"####,
        safe_path = safe_path,
        sql_hex   = sql_hex,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ─── Periodic table ───────────────────────────────────────────────────────────
// Data columns: Z|symbol|name|mass|category|period|group|electronegativity|state
// category: nonmetal/halogen/noble/alkali/alkaline/transition/post-trans/metalloid/lanthanide/actinide
// state: S=solid  L=liquid  G=gas
// en: 0 = not applicable / unknown on Pauling scale

const ELEMENTS_DATA: &str = r#"1|H|Hydrogen|1.008|nonmetal|1|1|2.20|G
2|He|Helium|4.003|noble|1|18|0|G
3|Li|Lithium|6.941|alkali|2|1|0.98|S
4|Be|Beryllium|9.012|alkaline|2|2|1.57|S
5|B|Boron|10.811|metalloid|2|13|2.04|S
6|C|Carbon|12.011|nonmetal|2|14|2.55|S
7|N|Nitrogen|14.007|nonmetal|2|15|3.04|G
8|O|Oxygen|15.999|nonmetal|2|16|3.44|G
9|F|Fluorine|18.998|halogen|2|17|3.98|G
10|Ne|Neon|20.180|noble|2|18|0|G
11|Na|Sodium|22.990|alkali|3|1|0.93|S
12|Mg|Magnesium|24.305|alkaline|3|2|1.31|S
13|Al|Aluminium|26.982|post-trans|3|13|1.61|S
14|Si|Silicon|28.085|metalloid|3|14|1.90|S
15|P|Phosphorus|30.974|nonmetal|3|15|2.19|S
16|S|Sulfur|32.06|nonmetal|3|16|2.58|S
17|Cl|Chlorine|35.45|halogen|3|17|3.16|G
18|Ar|Argon|39.948|noble|3|18|0|G
19|K|Potassium|39.098|alkali|4|1|0.82|S
20|Ca|Calcium|40.078|alkaline|4|2|1.00|S
21|Sc|Scandium|44.956|transition|4|3|1.36|S
22|Ti|Titanium|47.867|transition|4|4|1.54|S
23|V|Vanadium|50.942|transition|4|5|1.63|S
24|Cr|Chromium|51.996|transition|4|6|1.66|S
25|Mn|Manganese|54.938|transition|4|7|1.55|S
26|Fe|Iron|55.845|transition|4|8|1.83|S
27|Co|Cobalt|58.933|transition|4|9|1.88|S
28|Ni|Nickel|58.693|transition|4|10|1.91|S
29|Cu|Copper|63.546|transition|4|11|1.90|S
30|Zn|Zinc|65.38|transition|4|12|1.65|S
31|Ga|Gallium|69.723|post-trans|4|13|1.81|S
32|Ge|Germanium|72.630|metalloid|4|14|2.01|S
33|As|Arsenic|74.922|metalloid|4|15|2.18|S
34|Se|Selenium|78.971|nonmetal|4|16|2.55|S
35|Br|Bromine|79.904|halogen|4|17|2.96|L
36|Kr|Krypton|83.798|noble|4|18|3.00|G
37|Rb|Rubidium|85.468|alkali|5|1|0.82|S
38|Sr|Strontium|87.62|alkaline|5|2|0.95|S
39|Y|Yttrium|88.906|transition|5|3|1.22|S
40|Zr|Zirconium|91.224|transition|5|4|1.33|S
41|Nb|Niobium|92.906|transition|5|5|1.60|S
42|Mo|Molybdenum|95.96|transition|5|6|2.16|S
43|Tc|Technetium|98|transition|5|7|1.90|S
44|Ru|Ruthenium|101.07|transition|5|8|2.20|S
45|Rh|Rhodium|102.906|transition|5|9|2.28|S
46|Pd|Palladium|106.42|transition|5|10|2.20|S
47|Ag|Silver|107.868|transition|5|11|1.93|S
48|Cd|Cadmium|112.414|transition|5|12|1.69|S
49|In|Indium|114.818|post-trans|5|13|1.78|S
50|Sn|Tin|118.710|post-trans|5|14|1.96|S
51|Sb|Antimony|121.760|metalloid|5|15|2.05|S
52|Te|Tellurium|127.60|metalloid|5|16|2.10|S
53|I|Iodine|126.904|halogen|5|17|2.66|S
54|Xe|Xenon|131.293|noble|5|18|2.60|G
55|Cs|Caesium|132.905|alkali|6|1|0.79|S
56|Ba|Barium|137.327|alkaline|6|2|0.89|S
57|La|Lanthanum|138.905|lanthanide|6|0|1.10|S
58|Ce|Cerium|140.116|lanthanide|6|0|1.12|S
59|Pr|Praseodymium|140.908|lanthanide|6|0|1.13|S
60|Nd|Neodymium|144.242|lanthanide|6|0|1.14|S
61|Pm|Promethium|145|lanthanide|6|0|0|S
62|Sm|Samarium|150.36|lanthanide|6|0|1.17|S
63|Eu|Europium|151.964|lanthanide|6|0|0|S
64|Gd|Gadolinium|157.25|lanthanide|6|0|1.20|S
65|Tb|Terbium|158.925|lanthanide|6|0|0|S
66|Dy|Dysprosium|162.500|lanthanide|6|0|1.22|S
67|Ho|Holmium|164.930|lanthanide|6|0|1.23|S
68|Er|Erbium|167.259|lanthanide|6|0|1.24|S
69|Tm|Thulium|168.934|lanthanide|6|0|1.25|S
70|Yb|Ytterbium|173.054|lanthanide|6|0|0|S
71|Lu|Lutetium|174.967|lanthanide|6|0|1.27|S
72|Hf|Hafnium|178.49|transition|6|4|1.30|S
73|Ta|Tantalum|180.948|transition|6|5|1.50|S
74|W|Tungsten|183.84|transition|6|6|2.36|S
75|Re|Rhenium|186.207|transition|6|7|1.90|S
76|Os|Osmium|190.23|transition|6|8|2.20|S
77|Ir|Iridium|192.217|transition|6|9|2.20|S
78|Pt|Platinum|195.084|transition|6|10|2.28|S
79|Au|Gold|196.967|transition|6|11|2.54|S
80|Hg|Mercury|200.592|transition|6|12|2.00|L
81|Tl|Thallium|204.38|post-trans|6|13|1.62|S
82|Pb|Lead|207.2|post-trans|6|14|2.33|S
83|Bi|Bismuth|208.980|post-trans|6|15|2.02|S
84|Po|Polonium|209|metalloid|6|16|2.00|S
85|At|Astatine|210|halogen|6|17|2.20|S
86|Rn|Radon|222|noble|6|18|0|G
87|Fr|Francium|223|alkali|7|1|0.70|S
88|Ra|Radium|226|alkaline|7|2|0.90|S
89|Ac|Actinium|227|actinide|7|0|1.10|S
90|Th|Thorium|232.038|actinide|7|0|1.30|S
91|Pa|Protactinium|231.036|actinide|7|0|1.50|S
92|U|Uranium|238.029|actinide|7|0|1.38|S
93|Np|Neptunium|237|actinide|7|0|1.36|S
94|Pu|Plutonium|244|actinide|7|0|1.28|S
95|Am|Americium|243|actinide|7|0|1.30|S
96|Cm|Curium|247|actinide|7|0|1.30|S
97|Bk|Berkelium|247|actinide|7|0|1.30|S
98|Cf|Californium|251|actinide|7|0|1.30|S
99|Es|Einsteinium|252|actinide|7|0|1.30|S
100|Fm|Fermium|257|actinide|7|0|1.30|S
101|Md|Mendelevium|258|actinide|7|0|1.30|S
102|No|Nobelium|259|actinide|7|0|1.30|S
103|Lr|Lawrencium|266|actinide|7|0|0|S
104|Rf|Rutherfordium|267|transition|7|4|0|S
105|Db|Dubnium|268|transition|7|5|0|S
106|Sg|Seaborgium|271|transition|7|6|0|S
107|Bh|Bohrium|272|transition|7|7|0|S
108|Hs|Hassium|270|transition|7|8|0|S
109|Mt|Meitnerium|276|transition|7|9|0|S
110|Ds|Darmstadtium|281|transition|7|10|0|S
111|Rg|Roentgenium|280|transition|7|11|0|S
112|Cn|Copernicium|285|transition|7|12|0|S
113|Nh|Nihonium|284|post-trans|7|13|0|S
114|Fl|Flerovium|289|post-trans|7|14|0|S
115|Mc|Moscovium|288|post-trans|7|15|0|S
116|Lv|Livermorium|293|post-trans|7|16|0|S
117|Ts|Tennessine|294|halogen|7|17|0|S
118|Og|Oganesson|294|noble|7|18|0|G"#;

pub fn lookup_element(query: &str) -> Result<String, String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("No element specified. Try a symbol (H, Au), name (Gold), or atomic number (79).".into());
    }
    let q_lower = q.to_ascii_lowercase();
    let q_num: Option<u32> = q.parse().ok();

    for line in ELEMENTS_DATA.lines() {
        let f: Vec<&str> = line.splitn(9, '|').collect();
        if f.len() < 9 { continue; }
        let z: u32 = f[0].parse().unwrap_or(0);
        let sym  = f[1];
        let name = f[2];

        let matched = q_num.map_or(false, |n| n == z)
            || sym.eq_ignore_ascii_case(q)
            || name.to_ascii_lowercase().starts_with(&q_lower);
        if !matched { continue; }

        let mass_raw  = f[3];
        let cat_raw   = f[4];
        let period    = f[5];
        let group     = f[6];
        let en_raw    = f[7];
        let state_raw = f[8];

        let category = match cat_raw {
            "alkali"     => "Alkali Metal",
            "alkaline"   => "Alkaline Earth Metal",
            "transition" => "Transition Metal",
            "post-trans" => "Post-Transition Metal",
            "metalloid"  => "Metalloid",
            "nonmetal"   => "Nonmetal",
            "halogen"    => "Halogen",
            "noble"      => "Noble Gas",
            "lanthanide" => "Lanthanide",
            "actinide"   => "Actinide",
            other        => other,
        };
        let group_disp = if group == "0" {
            match cat_raw { "lanthanide" => "La series", "actinide" => "Ac series", _ => "\u{2014}" }
        } else {
            group
        };
        let en_disp = if en_raw == "0" {
            "N/A".to_string()
        } else {
            format!("{} (Pauling)", en_raw)
        };
        let state_disp = match state_raw {
            "S" => "Solid", "L" => "Liquid", "G" => "Gas", _ => "Unknown",
        };
        let mass_disp = if mass_raw.contains('.') {
            format!("{} u", mass_raw)
        } else {
            format!("{} u  (most stable isotope)", mass_raw)
        };

        return Ok(format!(
            "{sym}  {name}  (Z = {z})\n\
             {sep}\n\
             Atomic Mass:         {mass_disp}\n\
             Category:            {category}\n\
             Period / Group:      {period} / {group_disp}\n\
             Electronegativity:   {en_disp}\n\
             State at STP:        {state_disp}",
            sep = "\u{2500}".repeat(42),
        ));
    }

    Err(format!(
        "Element '{}' not found.\nTry: symbol (H, Au, Fe), name (Gold, Iron), or atomic number (79, 26).",
        q
    ))
}

// ─── File / text hash ─────────────────────────────────────────────────────────

pub async fn hash_input(input: &str, algo: &str) -> Result<String, String> {
    let safe_input = input.replace('\\', "\\\\").replace('"', "\\\"");
    let safe_algo  = algo.trim().to_ascii_lowercase().replace('"', "");

    let script = format!(
        r####"import hashlib, os, sys

_target = "{safe_input}"
_algo   = "{safe_algo}"

_is_file = os.path.isfile(_target)
if _is_file:
    with open(_target, 'rb') as _fh:
        _data = _fh.read()
    _sz = len(_data)
    if _sz >= 1_048_576:   _szlbl = "%.2f MB" % (_sz / 1_048_576)
    elif _sz >= 1024:      _szlbl = "%.1f KB" % (_sz / 1024)
    else:                  _szlbl = str(_sz) + " bytes"
    _label = "File: " + _target + "  (" + _szlbl + ")"
else:
    _data  = _target.encode('utf-8')
    _label = 'Text: "' + _target + '"'

_algos = ['md5', 'sha1', 'sha256', 'sha512'] if _algo in ('all', '') else [_algo]
print(_label)
print()
for _a in _algos:
    try:
        _h = hashlib.new(_a)
        _h.update(_data)
        print(_a.upper().ljust(10) + _h.hexdigest())
    except ValueError as _e:
        print(_a + ": " + str(_e), file=sys.stderr); sys.exit(1)
"####,
        safe_input = safe_input,
        safe_algo  = safe_algo,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ─── Encoding utilities ───────────────────────────────────────────────────────

pub async fn encode_decode(text: &str, codec: &str, is_decode: bool) -> Result<String, String> {
    let text_hex: String = text.bytes().map(|b| format!("{:02x}", b)).collect();
    let safe_codec = codec.trim().to_ascii_lowercase().replace('"', "");
    let mode = if is_decode { "decode" } else { "encode" };

    let script = format!(
        r####"import base64 as _b64, binascii as _ba, sys
import urllib.parse as _up

_text  = bytes.fromhex("{text_hex}").decode('utf-8', errors='replace')
_codec = "{safe_codec}"
_mode  = "{mode}"
_CODECS = "base64  hex  url  rot13  html  binary"

try:
    if _mode == "encode":
        if _codec in ("base64", "b64", ""):
            print(_b64.b64encode(_text.encode('utf-8')).decode())
        elif _codec in ("hex", "hexadecimal"):
            print(_ba.hexlify(_text.encode('utf-8')).decode())
        elif _codec in ("url", "urlencode", "percent"):
            print(_up.quote(_text, safe=''))
        elif _codec == "rot13":
            import codecs as _cd; print(_cd.encode(_text, 'rot_13'))
        elif _codec in ("html", "htmlentities"):
            import html as _ht; print(_ht.escape(_text))
        elif _codec in ("binary", "bin"):
            print(' '.join(bin(b)[2:].zfill(8) for b in _text.encode('utf-8')))
        else:
            print("Unknown codec: " + _codec + ".  Supported: " + _CODECS, file=sys.stderr); sys.exit(1)
    else:
        if _codec in ("base64", "b64", ""):
            print(_b64.b64decode(_text.strip() + "==").decode('utf-8', errors='replace'))
        elif _codec in ("hex", "hexadecimal"):
            print(_ba.unhexlify(_text.replace(' ', '')).decode('utf-8', errors='replace'))
        elif _codec in ("url", "urlencode", "percent"):
            print(_up.unquote(_text))
        elif _codec == "rot13":
            import codecs as _cd; print(_cd.decode(_text, 'rot_13'))
        elif _codec in ("html", "htmlentities"):
            import html as _ht; print(_ht.unescape(_text))
        elif _codec in ("binary", "bin"):
            _bytes = bytes(int(b, 2) for b in _text.split() if b)
            print(_bytes.decode('utf-8', errors='replace'))
        else:
            print("Unknown codec: " + _codec + ".  Supported: " + _CODECS, file=sys.stderr); sys.exit(1)
except Exception as _e:
    print("Error: " + str(_e), file=sys.stderr); sys.exit(1)
"####,
        text_hex   = text_hex,
        safe_codec = safe_codec,
        mode       = mode,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 10
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}
