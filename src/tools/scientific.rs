use serde_json::Value;

pub async fn scientific_compute(args: &Value) -> Result<String, String> {
    let mode = args["mode"]
        .as_str()
        .ok_or("Missing 'mode' (symbolic, units, complexity, ledger, dataset, regression)")?;

    match mode {
        "symbolic" => solve_symbolic(args).await,
        "units" => verify_units(args).await,
        "complexity" => audit_complexity(args).await,
        "ledger" => manage_ledger(args).await,
        "dataset" => calculate_on_dataset(args).await,
        "regression" => run_regression(args).await,
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
