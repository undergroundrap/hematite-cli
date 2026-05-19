// ─── Data analysis tools ──────────────────────────────────────────────────────
// Random sampling, correlation matrix, time-series analysis.
// All use the Python code sandbox — no external deps, no model required.

// ── Random data sampling ──────────────────────────────────────────────────────
// Draws N rows (or a fraction) from a CSV/TSV/JSON/SQLite file.
// Optionally splits into train/test sets.

pub async fn sample_data(
    file_path: &str,
    n: usize,
    fraction: f64,
    seed: u64,
    split: f64,
    output: &str,
) -> Result<String, String> {
    let hex_path:   String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_output: String = output.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, random

_path   = bytes.fromhex("{hex_path}").decode().strip()
_outdir = bytes.fromhex("{hex_output}").decode().strip()
_n      = {n}
_frac   = {fraction}
_seed   = {seed}
_split  = {split}

random.seed(_seed)

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r), ext
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        rows = d if isinstance(d, list) else next(iter(d.values()), [])
        return rows, 'json'
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return [], 'csv'
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d[0] for d in cur.description]
        rows2 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows2, 'csv'
    print("Unsupported format: "+ext, file=sys.stderr); sys.exit(1)

rows, ext = _load(_path)
total = len(rows)
if total == 0:
    print("No rows found."); sys.exit(0)

if _frac > 0 and _frac <= 1:
    k = max(1, int(total * _frac))
elif _n > 0:
    k = min(_n, total)
else:
    k = min(100, total)

sample = random.sample(rows, k)

def _write_csv(data, path):
    if not data: return
    fieldnames = list(data[0].keys())
    with open(path, 'w', newline='', encoding='utf-8') as fh:
        w = _csv.DictWriter(fh, fieldnames=fieldnames)
        w.writeheader(); w.writerows(data)

if _split > 0 and _split < 1 and _outdir:
    split_n = int(k * _split)
    train = sample[:split_n]
    test  = sample[split_n:]
    base = os.path.splitext(os.path.basename(_path))[0]
    train_path = os.path.join(_outdir, base + '_train.csv')
    test_path  = os.path.join(_outdir, base + '_test.csv')
    os.makedirs(_outdir, exist_ok=True)
    _write_csv(train, train_path)
    _write_csv(test,  test_path)
    print("Sampled %d rows (seed=%d) → %d%% split" % (k, _seed, int(_split*100)))
    print("Train: %d rows → %s" % (len(train), train_path))
    print("Test:  %d rows → %s" % (len(test), test_path))
elif _outdir:
    base = os.path.splitext(os.path.basename(_path))[0]
    out_path = os.path.join(_outdir, base + '_sample%d.csv' % k)
    os.makedirs(_outdir, exist_ok=True)
    _write_csv(sample, out_path)
    print("Sampled %d / %d rows (seed=%d) → %s" % (k, total, _seed, out_path))
else:
    # Print sample to stdout as CSV
    fieldnames = list(sample[0].keys())
    print(','.join(fieldnames))
    for row in sample:
        print(','.join(str(row.get(f,'')) for f in fieldnames))
    print()
    print("# Sampled %d / %d rows  (seed=%d)" % (k, total, _seed))
    print("# Use --sample-output DIR to save to file, or --split 0.8 for train/test split")
"####,
        hex_path   = hex_path,
        hex_output = hex_output,
        n          = n,
        fraction   = fraction,
        seed       = seed,
        split      = split,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Correlation matrix ────────────────────────────────────────────────────────

pub async fn correlation_matrix(file_path: &str, method: &str) -> Result<String, String> {
    let hex_path:   String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_method: String = method.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math

_path   = bytes.fromhex("{hex_path}").decode().strip()
_method = bytes.fromhex("{hex_method}").decode().strip().lower() or "pearson"

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r)
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        return d if isinstance(d, list) else next(iter(d.values()), [])
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return []
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d[0] for d in cur.description]
        rows2 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows2
    print("Unsupported: "+ext, file=sys.stderr); sys.exit(1)

def _tf(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

rows = _load(_path)
if not rows:
    print("No rows found."); sys.exit(0)

all_cols = list(rows[0].keys())
# Keep columns that are at least 50% numeric
num_cols = [c for c in all_cols
            if sum(1 for r in rows if _tf(r.get(c,'')) is not None) >= len(rows)*0.5]

if len(num_cols) < 2:
    print("Need at least 2 numeric columns. Found: %s" % ', '.join(num_cols or ['(none)']))
    sys.exit(0)

# Build column vectors (paired — both must be non-null for each row)
def _col_vec(c): return [_tf(r.get(c,'')) for r in rows]

vecs = {{c: _col_vec(c) for c in num_cols}}

def _pearson(a, b):
    pairs = [(x,y) for x,y in zip(a,b) if x is not None and y is not None]
    n = len(pairs)
    if n < 3: return float('nan')
    mx = sum(x for x,_ in pairs)/n
    my = sum(y for _,y in pairs)/n
    num = sum((x-mx)*(y-my) for x,y in pairs)
    dx  = math.sqrt(sum((x-mx)**2 for x,_ in pairs))
    dy  = math.sqrt(sum((y-my)**2 for _,y in pairs))
    return num/(dx*dy) if dx*dy else float('nan')

def _spearman(a, b):
    pairs = [(x,y) for x,y in zip(a,b) if x is not None and y is not None]
    n = len(pairs)
    if n < 3: return float('nan')
    def _rank(vs):
        sorted_vs = sorted(enumerate(vs), key=lambda x: x[1])
        ranks = [0.0]*n
        i = 0
        while i < n:
            j = i
            while j < n-1 and sorted_vs[j+1][1] == sorted_vs[i][1]: j+=1
            avg_rank = (i + j)/2 + 1
            for k in range(i,j+1): ranks[sorted_vs[k][0]] = avg_rank
            i = j+1
        return ranks
    ra = _rank([p[0] for p in pairs])
    rb = _rank([p[1] for p in pairs])
    return _pearson(ra, rb)

corr_fn = _spearman if _method.startswith('sp') else _pearson

nc = len(num_cols)
matrix = [[corr_fn(vecs[a], vecs[b]) for b in num_cols] for a in num_cols]

W = 64
print("="*W)
print(" Correlation Matrix (%s)  —  %s" % (_method.capitalize(), os.path.basename(_path)))
print("-"*W)
# Print header
col_w = 8
print("%*s" % (20, ""), end="")
for c in num_cols:
    print("  %*s" % (col_w, c[:col_w]), end="")
print()
print("-"*W)
for i, ra in enumerate(num_cols):
    print("%-20s" % ra[:20], end="")
    for j in range(nc):
        v = matrix[i][j]
        if math.isnan(v): s = "   nan  "
        else: s = " %7.4f" % v
        # Highlight strong correlations
        if i != j and not math.isnan(v) and abs(v) >= 0.7:
            s = s + "*"
        else:
            s = s + " "
        print(" %s" % s[:col_w+1], end="")
    print()
print("="*W)
print("  * |r| >= 0.7  (strong correlation)")
print()
# Report top correlations
pairs_flat = []
for i in range(nc):
    for j in range(i+1, nc):
        v = matrix[i][j]
        if not math.isnan(v):
            pairs_flat.append((abs(v), v, num_cols[i], num_cols[j]))
pairs_flat.sort(reverse=True)
if pairs_flat:
    print("Top correlations:")
    for _abs, v, a, b in pairs_flat[:min(5, len(pairs_flat))]:
        direction = "positive" if v > 0 else "negative"
        strength = "strong" if abs(v)>=0.7 else "moderate" if abs(v)>=0.4 else "weak"
        print("  %s  %-15s  ×  %-15s" % (("r=%+.4f"%v), a[:15], b[:15]))
        print("         (%s %s)" % (strength, direction))
"####,
        hex_path   = hex_path,
        hex_method = hex_method,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Time-series basic analysis ────────────────────────────────────────────────

pub async fn timeseries_analyze(
    file_path: &str,
    date_col: &str,
    value_col: &str,
    window: usize,
) -> Result<String, String> {
    let hex_path:     String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_date_col: String = date_col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_val_col:  String = value_col.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math, re

_path     = bytes.fromhex("{hex_path}").decode().strip()
_date_col = bytes.fromhex("{hex_date_col}").decode().strip()
_val_col  = bytes.fromhex("{hex_val_col}").decode().strip()
_window   = {window}
if _window < 2: _window = 7

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r)
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        return d if isinstance(d, list) else next(iter(d.values()), [])
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return []
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d[0] for d in cur.description]
        rows2 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows2
    print("Unsupported: "+ext, file=sys.stderr); sys.exit(1)

def _tf(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

# Detect date columns if not specified
def _looks_like_date(v):
    return bool(re.match(r'\d{{4}}[-/]\d{{1,2}}[-/]\d{{1,2}}', str(v)))

rows = _load(_path)
if not rows:
    print("No rows found."); sys.exit(0)

all_cols = list(rows[0].keys())

if not _date_col:
    date_candidates = [c for c in all_cols if sum(1 for r in rows[:50] if _looks_like_date(r.get(c,''))) > 20]
    _date_col = date_candidates[0] if date_candidates else all_cols[0]

num_cols = [c for c in all_cols
            if c != _date_col and sum(1 for r in rows if _tf(r.get(c,'')) is not None) >= len(rows)*0.5]

if not _val_col and num_cols:
    _val_col = num_cols[0]

if not _val_col:
    print("No numeric value column found. Use --ts-value COL to specify one."); sys.exit(0)

# Extract and sort by date string (lexicographic — works for ISO dates)
pairs = []
for r in rows:
    d = str(r.get(_date_col,'')).strip()
    v = _tf(r.get(_val_col,''))
    if d and v is not None:
        pairs.append((d, v))
pairs.sort(key=lambda p: p[0])

if len(pairs) < 3:
    print("Need at least 3 data points. Found: %d" % len(pairs)); sys.exit(0)

dates = [p[0] for p in pairs]
vals  = [p[1] for p in pairs]
n = len(vals)

# Rolling mean
def _roll(vs, w):
    return [sum(vs[max(0,i-w+1):i+1])/len(vs[max(0,i-w+1):i+1]) for i in range(len(vs))]

roll_mean = _roll(vals, _window)

# Linear trend (least squares)
xs = list(range(n))
xm = sum(xs)/n; ym = sum(vals)/n
b  = sum((x-xm)*(y-ym) for x,y in zip(xs,vals)) / sum((x-xm)**2 for x in xs)
a  = ym - b*xm
trend_line = [a + b*x for x in xs]

# Peak/valley detection
peaks   = [i for i in range(1,n-1) if vals[i]>vals[i-1] and vals[i]>vals[i+1]]
valleys = [i for i in range(1,n-1) if vals[i]<vals[i-1] and vals[i]<vals[i+1]]

W = 64
print("="*W)
print(" Time-Series Analysis: %s" % os.path.basename(_path))
print(" Date column:  %s    Value column: %s" % (_date_col, _val_col))
print("-"*W)
print("  Points:  %d   Range: %s → %s" % (n, dates[0][:16], dates[-1][:16]))
print("  Min:     %g  (at %s)" % (min(vals), dates[vals.index(min(vals))][:16]))
print("  Max:     %g  (at %s)" % (max(vals), dates[vals.index(max(vals))][:16]))
print("  Mean:    %.4f   Std: %.4f" % (ym, math.sqrt(sum((v-ym)**2 for v in vals)/n)))
print("  Trend:   %.4f per step  (%s)" % (b, "↑ upward" if b>0 else "↓ downward" if b<0 else "→ flat"))
print("  Peaks:   %d local maxima   Valleys: %d local minima" % (len(peaks), len(valleys)))
print("-"*W)
print("  Rolling mean (window=%d):" % _window)
# Compact sparkline using ASCII
W2 = 50
rng = max(vals) - min(vals) if max(vals) != min(vals) else 1
bar_chars = " ▁▂▃▄▅▆▇█"
spark = ''.join(bar_chars[min(8,int((v-min(vals))/rng*8))] for v in vals)
# Wrap
for i in range(0, len(spark), W2):
    chunk = spark[i:i+W2]
    print("  [%s]  %s–%s" % (chunk, dates[i][:10], dates[min(i+W2-1,n-1)][:10]))
print("-"*W)
# Last few rolling values
print("  Recent rolling mean (%d-period):" % _window)
for i in range(max(0,n-5), n):
    flag = " ← latest" if i==n-1 else ""
    print("    %-16s  value=%g   roll_mean=%.4f%s" % (dates[i][:16], vals[i], roll_mean[i], flag))
print("="*W)
"####,
        hex_path     = hex_path,
        hex_date_col = hex_date_col,
        hex_val_col  = hex_val_col,
        window       = window,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Percentile / quantile report ──────────────────────────────────────────────
// Computes P1 P5 P10 P25 P50 P75 P90 P95 P99 for each numeric column
// (or a specific column if col is non-empty).

pub async fn percentile_report(file_path: &str, col: &str) -> Result<String, String> {
    let hex_path: String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_col:  String = col.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math

_path   = bytes.fromhex("{hex_path}").decode().strip()
_col    = bytes.fromhex("{hex_col}").decode().strip()

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r)
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        return d if isinstance(d, list) else next(iter(d.values()), [])
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return []
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d[0] for d in cur.description]
        rows2 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows2
    print("Unsupported: "+ext, file=sys.stderr); sys.exit(1)

def _tf(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

def _percentile(data, p):
    if not data: return float('nan')
    s = sorted(data)
    n = len(s)
    idx = (p/100.0) * (n-1)
    lo = int(idx); hi = lo + 1
    frac = idx - lo
    if hi >= n: return s[-1]
    return s[lo] + frac*(s[hi]-s[lo])

rows = _load(_path)
if not rows:
    print("No rows found."); sys.exit(0)

all_cols = list(rows[0].keys())
if _col:
    target_cols = [c for c in all_cols if c.lower() == _col.lower()]
    if not target_cols:
        print("Column '%s' not found. Available: %s" % (_col, ', '.join(all_cols)))
        sys.exit(1)
else:
    target_cols = [c for c in all_cols
                   if sum(1 for r in rows if _tf(r.get(c,'')) is not None) >= len(rows)*0.5]
    if not target_cols:
        print("No numeric columns found."); sys.exit(0)

W = 72
print("="*W)
print(" Percentile Report — %s  (%d rows)" % (os.path.basename(_path), len(rows)))
print("-"*W)
hdr = "%-20s %8s %8s %8s %8s %8s %8s %8s" % ("Column", "P25", "P50", "P75", "P90", "P99", "Min", "Max")
print(hdr)
print("-"*W)
for c in target_cols:
    vals = [_tf(r.get(c,'')) for r in rows]
    vals = [v for v in vals if v is not None]
    if not vals: continue
    p25=_percentile(vals,25); p50=_percentile(vals,50); p75=_percentile(vals,75)
    p90=_percentile(vals,90); p99=_percentile(vals,99)
    mn=min(vals); mx=max(vals)
    def _f(v): return "%8g" % v
    print("%-20s %s %s %s %s %s %s %s" % (c[:20], _f(p25), _f(p50), _f(p75), _f(p90), _f(p99), _f(mn), _f(mx)))
print("="*W)
print()
if len(target_cols) == 1:
    c = target_cols[0]
    vals = [_tf(r.get(c,'')) for r in rows if _tf(r.get(c,'')) is not None]
    print("Detailed percentile table for '%s':" % c)
    for p in [1, 5, 10, 25, 50, 75, 90, 95, 99]:
        v = _percentile(vals, p)
        print("  P%-3d  %g" % (p, v))
    mean = sum(vals)/len(vals)
    std  = math.sqrt(sum((x-mean)**2 for x in vals)/len(vals))
    iqr  = _percentile(vals,75) - _percentile(vals,25)
    print()
    print("  Mean: %g   Std: %g   IQR: %g   N: %d" % (mean, std, iqr, len(vals)))
"####,
        hex_path = hex_path,
        hex_col  = hex_col,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Pivot table ───────────────────────────────────────────────────────────────
// Groups rows by row_col × col_col and aggregates value_col.
// Agg: count (default), sum, mean, min, max.

pub async fn pivot_table(
    file_path: &str,
    row_col: &str,
    col_col: &str,
    value_col: &str,
    agg: &str,
) -> Result<String, String> {
    let hex_path:    String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_row_col: String = row_col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_col_col: String = col_col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_val_col: String = value_col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_agg:     String = agg.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys

_path    = bytes.fromhex("{hex_path}").decode().strip()
_row_col = bytes.fromhex("{hex_row_col}").decode().strip()
_col_col = bytes.fromhex("{hex_col_col}").decode().strip()
_val_col = bytes.fromhex("{hex_val_col}").decode().strip()
_agg     = bytes.fromhex("{hex_agg}").decode().strip().lower() or "count"

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r)
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        return d if isinstance(d, list) else next(iter(d.values()), [])
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return []
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d[0] for d in cur.description]
        rows2 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows2
    print("Unsupported: "+ext, file=sys.stderr); sys.exit(1)

def _tf(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

rows = _load(_path)
if not rows:
    print("No rows found."); sys.exit(0)

all_cols = list(rows[0].keys())

if not _row_col:
    cat_cols = [c for c in all_cols if sum(1 for r in rows if _tf(r.get(c,'')) is None) > len(rows)*0.3]
    _row_col = cat_cols[0] if cat_cols else all_cols[0]
if not _col_col:
    cat_cols = [c for c in all_cols if sum(1 for r in rows if _tf(r.get(c,'')) is None) > len(rows)*0.3]
    _col_col = cat_cols[1] if len(cat_cols) > 1 else (all_cols[1] if len(all_cols) > 1 else _row_col)
if not _val_col and _agg != 'count':
    num_cols = [c for c in all_cols if c not in (_row_col, _col_col) and
                sum(1 for r in rows if _tf(r.get(c,'')) is not None) >= len(rows)*0.5]
    _val_col = num_cols[0] if num_cols else ''

data = {{}}
for r in rows:
    rk = str(r.get(_row_col, '')).strip()
    ck = str(r.get(_col_col, '')).strip()
    v  = _tf(r.get(_val_col, '')) if _val_col else 1.0
    if rk not in data: data[rk] = {{}}
    if ck not in data[rk]: data[rk][ck] = []
    if v is not None: data[rk][ck].append(v)

row_keys = sorted(data.keys())
col_keys = sorted({{ck for rv in data.values() for ck in rv}})

def _agg_fn(vals):
    if not vals: return ''
    if _agg == 'count':  return str(len(vals))
    if _agg == 'sum':    return "%.4g" % sum(vals)
    if _agg == 'mean':   return "%.4g" % (sum(vals)/len(vals))
    if _agg == 'min':    return "%.4g" % min(vals)
    if _agg == 'max':    return "%.4g" % max(vals)
    return str(len(vals))

CW = 10
RW = 16
print("Pivot: %s x %s  (%s of %s)  |  rows=%d  cols=%d" % (
    _row_col, _col_col, _agg, _val_col or 'rows', len(row_keys), len(col_keys)))
print()
print("%-*s" % (RW, _row_col[:RW]), end="")
for ck in col_keys: print("  %-*s" % (CW, ck[:CW]), end="")
print()
print("-" * (RW + len(col_keys)*(CW+2)))
for rk in row_keys:
    print("%-*s" % (RW, rk[:RW]), end="")
    for ck in col_keys:
        vals = data.get(rk, {{}}).get(ck, [])
        cell = _agg_fn(vals) if vals else '-'
        print("  %-*s" % (CW, cell[:CW]), end="")
    print()
"####,
        hex_path    = hex_path,
        hex_row_col = hex_row_col,
        hex_col_col = hex_col_col,
        hex_val_col = hex_val_col,
        hex_agg     = hex_agg,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Multivariate linear regression ───────────────────────────────────────────
// OLS via normal equations: β = (XᵀX)⁻¹Xᵀy
// Supports one or more predictor columns. Reports coefficients, R², RMSE,
// and predicted vs actual for first 10 rows.

pub async fn linear_regression(
    file_path: &str,
    predictors: &[&str],
    target: &str,
) -> Result<String, String> {
    let hex_path:   String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_target: String = target.bytes().map(|b| format!("{:02x}", b)).collect();
    let preds_joined = predictors.join("\n");
    let hex_preds:  String = preds_joined.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math

_path   = bytes.fromhex("{hex_path}").decode().strip()
_target = bytes.fromhex("{hex_target}").decode().strip()
_preds_raw = bytes.fromhex("{hex_preds}").decode().strip()
_preds  = [p.strip() for p in _preds_raw.split('\n') if p.strip()] if _preds_raw else []

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r)
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        return d if isinstance(d, list) else next(iter(d.values()), [])
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return []
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d[0] for d in cur.description]
        rows2 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows2
    print("Unsupported: "+ext, file=sys.stderr); sys.exit(1)

def _tf(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

rows = _load(_path)
if not rows:
    print("No rows found."); sys.exit(0)

all_cols = list(rows[0].keys())
num_cols = [c for c in all_cols if sum(1 for r in rows if _tf(r.get(c,'')) is not None) >= len(rows)*0.5]

if not _target:
    _target = num_cols[-1] if num_cols else ''
if not _preds:
    _preds = [c for c in num_cols if c != _target]

if not _target:
    print("No target column. Use --regression-target COL"); sys.exit(1)
if not _preds:
    print("No predictor columns. Use --regression-predictors COL1,COL2,..."); sys.exit(1)

valid = []
for r in rows:
    y = _tf(r.get(_target,''))
    xs = [_tf(r.get(pp,'')) for pp in _preds]
    if y is not None and all(x is not None for x in xs):
        valid.append((xs, y))

n = len(valid)
pp = len(_preds)
if n < pp + 2:
    print("Not enough valid rows (%d) for %d predictors." % (n, pp)); sys.exit(1)

X = [[1.0] + row[0] for row in valid]
y = [row[1] for row in valid]

def _mat_mul_sq(A, B):
    ra,ca = len(A),len(A[0]); cb = len(B[0])
    return [[sum(A[i][k]*B[k][j] for k in range(ca)) for j in range(cb)] for i in range(ra)]

def _mat_T(A):
    return [[A[i][j] for i in range(len(A))] for j in range(len(A[0]))]

def _lu_solve(A, b):
    n2 = len(A)
    M = [row[:] + [b[i]] for i,row in enumerate(A)]
    for col in range(n2):
        pivot = max(range(col,n2), key=lambda r2: abs(M[r2][col]))
        M[col],M[pivot] = M[pivot],M[col]
        if abs(M[col][col]) < 1e-12: return None
        for row in range(col+1,n2):
            f = M[row][col]/M[col][col]
            for j in range(col,n2+1): M[row][j] -= f*M[col][j]
    x2 = [0.0]*n2
    for i in range(n2-1,-1,-1):
        x2[i] = (M[i][n2] - sum(M[i][j]*x2[j] for j in range(i+1,n2))) / M[i][i]
    return x2

Xt = _mat_T(X)
XtX_sq = _mat_mul_sq(Xt, X)
Xty = [sum(Xt[i][k]*y[k] for k in range(n)) for i in range(pp+1)]
beta = _lu_solve(XtX_sq, Xty)
if beta is None:
    print("Matrix is singular — check for collinear predictors."); sys.exit(1)

preds_vals = [sum(beta[j]*X[i][j] for j in range(pp+1)) for i in range(n)]
residuals  = [y[i]-preds_vals[i] for i in range(n)]
ss_res = sum(r**2 for r in residuals)
ym = sum(y)/n
ss_tot = sum((v-ym)**2 for v in y)
r2 = 1 - ss_res/ss_tot if ss_tot else 0
rmse = math.sqrt(ss_res/n)
adj_r2 = 1 - (1-r2)*(n-1)/(n-pp-1) if n > pp+1 else float('nan')

W = 64
print("="*W)
print(" Linear Regression — %s" % os.path.basename(_path))
print(" Target: %-20s   N=%d   Predictors=%d" % (_target, n, pp))
print("-"*W)
print("  Coefficients:")
print("    %-20s  %12.6f" % ("(intercept)", beta[0]))
for i2,c2 in enumerate(_preds):
    print("    %-20s  %12.6f" % (c2[:20], beta[i2+1]))
print("-"*W)
print("  R²         = %.6f" % r2)
print("  Adj. R²    = %.6f" % adj_r2)
print("  RMSE       = %.6f" % rmse)
print("  Residuals  min=%.4g  max=%.4g  mean=%.4g" % (min(residuals), max(residuals), sum(residuals)/n))
print("-"*W)
terms = ["%.4g" % beta[0]]
for i2,c2 in enumerate(_preds):
    sign = "+" if beta[i2+1] >= 0 else "-"
    terms.append("%s %.4g*%s" % (sign, abs(beta[i2+1]), c2))
print("  Equation: %s = %s" % (_target, " ".join(terms)))
print("="*W)
print()
print("  First 10 predictions vs actual:")
print("  %-10s  %-10s  %-10s" % ("Actual", "Predicted", "Residual"))
for i3 in range(min(10,n)):
    print("  %-10.4g  %-10.4g  %-10.4g" % (y[i3], preds_vals[i3], residuals[i3]))
"####,
        hex_path   = hex_path,
        hex_target = hex_target,
        hex_preds  = hex_preds,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Outlier detection ─────────────────────────────────────────────────────────
// IQR (1.5× fence) and Z-score (|z|>3) detection.
// Optional: output clean CSV with outliers removed.

pub async fn detect_outliers(
    file_path: &str,
    col: &str,
    output: &str,
) -> Result<String, String> {
    let hex_path:   String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_col:    String = col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_output: String = output.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math

_path   = bytes.fromhex("{hex_path}").decode().strip()
_col    = bytes.fromhex("{hex_col}").decode().strip()
_output = bytes.fromhex("{hex_output}").decode().strip()

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            rd = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            data = list(rd)
            fns = list(rd.fieldnames or [])
            return data, fns
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        rows2 = d if isinstance(d, list) else next(iter(d.values()), [])
        fns2 = list(rows2[0].keys()) if rows2 else []
        return rows2, fns2
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return [], []
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d2[0] for d2 in cur.description]
        rows3 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows3, cols2
    print("Unsupported: "+ext, file=sys.stderr); sys.exit(1)

def _tf(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

def _pct(data, p):
    s = sorted(data); n = len(s)
    idx = (p/100.0)*(n-1); lo = int(idx); hi = lo+1; frac = idx-lo
    return s[-1] if hi >= n else s[lo]+frac*(s[hi]-s[lo])

rows, fieldnames = _load(_path)
if not rows:
    print("No rows found."); sys.exit(0)

all_cols = list(rows[0].keys())
if _col:
    target_cols = [c for c in all_cols if c.lower() == _col.lower()]
    if not target_cols:
        print("Column '%s' not found. Available: %s" % (_col, ', '.join(all_cols))); sys.exit(1)
else:
    target_cols = [c for c in all_cols
                   if sum(1 for r in rows if _tf(r.get(c,'')) is not None) >= len(rows)*0.5]

W = 68
print("="*W)
print(" Outlier Detection — %s  (%d rows)" % (os.path.basename(_path), len(rows)))
print("-"*W)

outlier_row_indices = set()
for c in target_cols:
    valid = [(i, _tf(r.get(c,''))) for i,r in enumerate(rows)]
    valid = [(i,v) for i,v in valid if v is not None]
    if len(valid) < 4: continue
    vs = [v for _,v in valid]
    mean = sum(vs)/len(vs)
    std  = math.sqrt(sum((x-mean)**2 for x in vs)/len(vs))
    q1 = _pct(vs,25); q3 = _pct(vs,75); iqr = q3-q1
    lo_fence = q1 - 1.5*iqr; hi_fence = q3 + 1.5*iqr
    iqr_out = [(i,v) for i,v in valid if v < lo_fence or v > hi_fence]
    z_out   = [(i,v) for i,v in valid if std > 0 and abs((v-mean)/std) > 3]
    print()
    print("  Column: %s  (n=%d  mean=%.4g  std=%.4g)" % (c, len(vs), mean, std))
    print("  IQR fence: [%.4g, %.4g]    IQR outliers: %d" % (lo_fence, hi_fence, len(iqr_out)))
    print("  Z-score |z|>3:  Z outliers: %d" % len(z_out))
    if iqr_out:
        print("  IQR outliers (row, value):")
        for i,v in iqr_out[:10]:
            z = (v-mean)/std if std > 0 else float('nan')
            print("    row %-5d  value=%-12g  z=%.3f" % (i+1, v, z))
            outlier_row_indices.add(i)
        if len(iqr_out) > 10:
            print("    ... and %d more" % (len(iqr_out)-10))
    else:
        print("  No IQR outliers found.")

print()
print("="*W)
print("  Total outlier rows (IQR): %d / %d  (%.1f%%)" % (
    len(outlier_row_indices), len(rows), 100*len(outlier_row_indices)/max(1,len(rows))))

if _output and outlier_row_indices:
    clean = [r for i,r in enumerate(rows) if i not in outlier_row_indices]
    fns2 = fieldnames if fieldnames else (list(clean[0].keys()) if clean else [])
    with open(_output, 'w', newline='', encoding='utf-8') as fh:
        w = _csv.DictWriter(fh, fieldnames=fns2)
        w.writeheader(); w.writerows(clean)
    print("  Clean data (%d rows) saved to: %s" % (len(clean), _output))
elif _output:
    print("  No outliers to remove — output file not written.")
"####,
        hex_path   = hex_path,
        hex_col    = hex_col,
        hex_output = hex_output,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── SVG chart generator ───────────────────────────────────────────────────────
// Produces a self-contained SVG file — no matplotlib, no external deps.
// Chart types: line (default), scatter, bar, histogram.
// Reads CSV/TSV/JSON/SQLite. Auto-opens with --open flag (handled in main.rs).

pub async fn plot_chart(
    file_path: &str,
    x_col: &str,
    y_col: &str,
    chart_type: &str,
    title: &str,
    output: &str,
) -> Result<String, String> {
    let hex_path:  String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_x:     String = x_col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_y:     String = y_col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_type:  String = chart_type.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_title: String = title.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_out:   String = output.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math

_path  = bytes.fromhex("{hex_path}").decode().strip()
_xcol  = bytes.fromhex("{hex_x}").decode().strip()
_ycol  = bytes.fromhex("{hex_y}").decode().strip()
_ctype = bytes.fromhex("{hex_type}").decode().strip().lower() or "line"
_title = bytes.fromhex("{hex_title}").decode().strip()
_out   = bytes.fromhex("{hex_out}").decode().strip()

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r)
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        return d if isinstance(d, list) else next(iter(d.values()), [])
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return []
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d2[0] for d2 in cur.description]
        rows2 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows2
    print("Unsupported: "+ext, file=sys.stderr); sys.exit(1)

def _tf(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

rows = _load(_path)
if not rows:
    print("No rows found."); sys.exit(0)

all_cols = list(rows[0].keys())
num_cols = [c for c in all_cols if sum(1 for r in rows if _tf(r.get(c,'')) is not None) >= len(rows)*0.5]

if not _xcol:
    _xcol = all_cols[0]
if not _ycol:
    _ycol = num_cols[0] if num_cols else (all_cols[1] if len(all_cols)>1 else all_cols[0])

if not _title:
    _title = "%s — %s vs %s" % (os.path.basename(_path), _xcol, _ycol)

if not _out:
    base = os.path.splitext(_path)[0]
    _out = base + "_plot.svg"

# Extract data points
def _to_num_or_str(v): return _tf(v) if _tf(v) is not None else str(v).strip()

raw_pairs = [(_to_num_or_str(r.get(_xcol,'')), _tf(r.get(_ycol,''))) for r in rows]
pairs = [(x,y) for x,y in raw_pairs if y is not None]

if not pairs:
    print("No plottable data in columns '%s' vs '%s'." % (_xcol, _ycol)); sys.exit(0)

# For bar/histogram: bucket string x values
xs_raw = [p[0] for p in pairs]
ys = [p[1] for p in pairs]

# SVG dimensions
W = 800; H = 500; PAD = 70; TW = W-2*PAD; TH = H-2*PAD

def _esc(s): return str(s).replace('&','&amp;').replace('<','&lt;').replace('>','&gt;').replace('"','&quot;')

def _scale(vals, lo, hi, out_lo, out_hi):
    if hi == lo: return [out_lo + (out_hi-out_lo)/2 for _ in vals]
    return [out_lo + (v-lo)/(hi-lo)*(out_hi-out_lo) for v in vals]

svg_parts = []
svg_parts.append('<?xml version="1.0" encoding="UTF-8"?>')
svg_parts.append('<svg xmlns="http://www.w3.org/2000/svg" width="%d" height="%d" style="background:#1e1e2e">' % (W, H))
svg_parts.append('<style>text{{font-family:monospace;fill:#cdd6f4}}line{{stroke:#45475a}}circle{{opacity:0.8}}</style>')
# Title
svg_parts.append('<text x="%d" y="28" font-size="15" text-anchor="middle" font-weight="bold">%s</text>' % (W//2, _esc(_title)))
# Axes
svg_parts.append('<line x1="%d" y1="%d" x2="%d" y2="%d" stroke="#89b4fa" stroke-width="1.5"/>' % (PAD, PAD, PAD, H-PAD))
svg_parts.append('<line x1="%d" y1="%d" x2="%d" y2="%d" stroke="#89b4fa" stroke-width="1.5"/>' % (PAD, H-PAD, W-PAD, H-PAD))
# Axis labels
svg_parts.append('<text x="%d" y="%d" font-size="12" text-anchor="middle">%s</text>' % (W//2, H-10, _esc(_xcol)))
svg_parts.append('<text x="15" y="%d" font-size="12" text-anchor="middle" transform="rotate(-90,15,%d)">%s</text>' % (H//2, H//2, _esc(_ycol)))

if _ctype == 'bar' or (not all(isinstance(x, (int,float)) for x in xs_raw)):
    # Bar chart: group by string x
    from collections import OrderedDict
    groups = OrderedDict()
    for x,y in pairs:
        k = str(x)
        groups.setdefault(k, []).append(y)
    labels = list(groups.keys())[:30]
    bar_vals = [sum(groups[k])/len(groups[k]) for k in labels]
    bw = TW / max(len(labels),1) * 0.8
    x_positions = [PAD + (i+0.5) * TW / max(len(labels),1) for i in range(len(labels))]
    ymin = min(0, min(bar_vals)); ymax = max(bar_vals) if bar_vals else 1
    if ymin == ymax: ymax = ymin + 1
    def _sy(v): return H-PAD - (v-ymin)/(ymax-ymin)*TH
    for i,(lbl,v) in enumerate(zip(labels,bar_vals)):
        x0 = x_positions[i] - bw/2
        y0 = _sy(max(v,0)); y1 = _sy(min(v,0))
        bar_h = abs(y0-y1)
        svg_parts.append('<rect x="%.1f" y="%.1f" width="%.1f" height="%.1f" fill="#89b4fa" rx="2"/>' % (x0, min(y0,y1), bw, max(bar_h,1)))
        if len(labels) <= 15:
            svg_parts.append('<text x="%.1f" y="%d" font-size="10" text-anchor="middle" transform="rotate(-45,%.1f,%d)">%s</text>' % (x_positions[i], H-PAD+14, x_positions[i], H-PAD+14, _esc(lbl[:12])))
    # y-axis ticks
    for tick in [ymin, (ymin+ymax)/2, ymax]:
        sy = _sy(tick)
        svg_parts.append('<line x1="%d" y1="%.1f" x2="%d" y2="%.1f" stroke="#45475a"/>' % (PAD, sy, W-PAD, sy))
        svg_parts.append('<text x="%d" y="%.1f" font-size="10" text-anchor="end">%.3g</text>' % (PAD-4, sy+4, tick))

elif _ctype == 'histogram':
    n_bins = min(30, max(5, int(math.sqrt(len(ys)))))
    ymin_h = min(ys); ymax_h = max(ys)
    if ymin_h == ymax_h: ymax_h = ymin_h + 1
    bin_w = (ymax_h-ymin_h)/n_bins
    counts = [0]*n_bins
    for v in ys:
        idx = min(int((v-ymin_h)/bin_w), n_bins-1)
        counts[idx] += 1
    bar_w = TW/n_bins
    cmax = max(counts) if counts else 1
    for i,c in enumerate(counts):
        x0 = PAD + i*bar_w
        bar_h2 = c/cmax * TH
        y0 = H-PAD-bar_h2
        svg_parts.append('<rect x="%.1f" y="%.1f" width="%.1f" height="%.1f" fill="#a6e3a1" rx="1"/>' % (x0, y0, bar_w-1, bar_h2))
    for i in range(5):
        tick_v = ymin_h + i*(ymax_h-ymin_h)/4
        sx = PAD + (tick_v-ymin_h)/(ymax_h-ymin_h)*TW
        svg_parts.append('<text x="%.1f" y="%d" font-size="10" text-anchor="middle">%.3g</text>' % (sx, H-PAD+14, tick_v))
    for i in range(5):
        tick_c = i*cmax/4
        sy = H-PAD - tick_c/cmax*TH
        svg_parts.append('<text x="%d" y="%.1f" font-size="10" text-anchor="end">%d</text>' % (PAD-4, sy+4, int(tick_c)))

else:
    # Line or scatter: numeric x required
    xs_num = [p[0] if isinstance(p[0],(int,float)) else i for i,p in enumerate(pairs)]
    xmin = min(xs_num); xmax = max(xs_num)
    ymin2 = min(ys); ymax2 = max(ys)
    if xmin == xmax: xmax = xmin+1
    if ymin2 == ymax2: ymax2 = ymin2+1
    def _sx2(v): return PAD + (v-xmin)/(xmax-xmin)*TW
    def _sy2(v): return H-PAD - (v-ymin2)/(ymax2-ymin2)*TH
    # Grid
    for i in range(5):
        gx = PAD + i*TW/4; gy = H-PAD - i*TH/4
        svg_parts.append('<line x1="%.1f" y1="%d" x2="%.1f" y2="%d" stroke="#313244" stroke-dasharray="4"/>' % (gx,PAD,gx,H-PAD))
        svg_parts.append('<line x1="%d" y1="%.1f" x2="%d" y2="%.1f" stroke="#313244" stroke-dasharray="4"/>' % (PAD,gy,W-PAD,gy))
    # x ticks
    for i in range(5):
        tv = xmin + i*(xmax-xmin)/4
        sx2 = _sx2(tv)
        svg_parts.append('<text x="%.1f" y="%d" font-size="10" text-anchor="middle">%.3g</text>' % (sx2, H-PAD+14, tv))
    # y ticks
    for i in range(5):
        tv = ymin2 + i*(ymax2-ymin2)/4
        sy2 = _sy2(tv)
        svg_parts.append('<text x="%d" y="%.1f" font-size="10" text-anchor="end">%.3g</text>' % (PAD-4, sy2+4, tv))
    pts = list(zip(xs_num, ys))
    pts.sort(key=lambda p: p[0])
    sx_list = [_sx2(x) for x,_ in pts]
    sy_list = [_sy2(y) for _,y in pts]
    if _ctype != 'scatter' and len(pts) > 1:
        path_d = "M %.1f %.1f " % (sx_list[0], sy_list[0])
        path_d += " ".join("L %.1f %.1f" % (sx_list[i], sy_list[i]) for i in range(1,len(pts)))
        svg_parts.append('<path d="%s" fill="none" stroke="#89b4fa" stroke-width="2"/>' % path_d)
    for i in range(len(pts)):
        svg_parts.append('<circle cx="%.1f" cy="%.1f" r="3" fill="#cba6f7"/>' % (sx_list[i], sy_list[i]))

svg_parts.append('</svg>')
svg_content = '\n'.join(svg_parts)

with open(_out, 'w', encoding='utf-8') as fh:
    fh.write(svg_content)

print("Chart saved: %s  (%d data points  type=%s)" % (_out, len(pairs), _ctype))
print("Open in any browser to view.")
"####,
        hex_path  = hex_path,
        hex_x     = hex_x,
        hex_y     = hex_y,
        hex_type  = hex_type,
        hex_title = hex_title,
        hex_out   = hex_out,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Discrete Fourier Transform / frequency analysis ───────────────────────────
// Pure-Python DFT. Reads numeric column, reports top-N frequency components.

pub async fn fourier_analysis(
    file_path: &str,
    col: &str,
    top_n: usize,
    sample_rate: f64,
) -> Result<String, String> {
    let hex_path: String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_col:  String = col.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math

_path        = bytes.fromhex("{hex_path}").decode().strip()
_col         = bytes.fromhex("{hex_col}").decode().strip()
_top_n       = {top_n}
_sample_rate = {sample_rate}

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r)
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        return d if isinstance(d, list) else next(iter(d.values()), [])
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return []
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d2[0] for d2 in cur.description]
        rows2 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows2
    print("Unsupported: "+ext, file=sys.stderr); sys.exit(1)

def _tf(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

rows = _load(_path)
if not rows:
    print("No rows found."); sys.exit(0)

all_cols = list(rows[0].keys())
if _col:
    target_col = next((c for c in all_cols if c.lower() == _col.lower()), None)
    if not target_col:
        print("Column '%s' not found. Available: %s" % (_col, ', '.join(all_cols))); sys.exit(1)
else:
    num_cols = [c for c in all_cols if sum(1 for r in rows if _tf(r.get(c,'')) is not None) >= len(rows)*0.5]
    target_col = num_cols[0] if num_cols else None
    if not target_col:
        print("No numeric column found."); sys.exit(0)

vals = [_tf(r.get(target_col,'')) for r in rows]
vals = [v for v in vals if v is not None]
n = len(vals)
if n < 4:
    print("Need at least 4 data points for DFT."); sys.exit(0)

mean = sum(vals)/n
x = [v - mean for v in vals]

if n > 512:
    x = x[:512]; n = 512
    print("Note: DFT computed on first 512 points (large dataset).")

def dft(x2):
    N = len(x2)
    result = []
    for k in range(N//2 + 1):
        re = sum(x2[t]*math.cos(2*math.pi*k*t/N) for t in range(N))
        im = sum(x2[t]*math.sin(2*math.pi*k*t/N) for t in range(N))
        amp = math.sqrt(re**2 + im**2) / N
        phase = math.atan2(-im, re)
        result.append((k, amp, phase))
    return result

spectrum = dft(x)
spectrum_sorted = sorted(spectrum[1:], key=lambda t: -t[1])

sr = _sample_rate if _sample_rate > 0 else 1.0
top = spectrum_sorted[:min(_top_n, len(spectrum_sorted))]

W = 64
print("="*W)
print(" Fourier / Frequency Analysis: %s" % os.path.basename(_path))
print(" Column: %-20s   N=%d   Sample rate: %g Hz" % (target_col, n, sr))
print("-"*W)
print("  DC component (mean offset): %.6f" % spectrum[0][1])
print()
print("  %-5s  %-12s  %-12s  %-10s  %-10s" % ("Rank", "Freq (Hz)", "Period", "Amplitude", "Phase (deg)"))
print("  " + "-"*58)
for i,(k,amp,phase) in enumerate(top):
    freq = k * sr / n
    period = (1.0/freq) if freq > 0 else float('inf')
    period_str = "%.4g" % period if period < 1e10 else "inf"
    print("  %-5d  %-12.6g  %-12s  %-10.6f  %-10.2f" % (
        i+1, freq, period_str, amp, math.degrees(phase)))
total_power = sum(t[1]**2 for t in spectrum[1:])
top_power   = sum(t[1]**2 for t in top)
print()
print("  Top %d components contain %.1f%% of signal power." % (len(top), 100*top_power/max(total_power,1e-30)))
print("="*W)
"####,
        hex_path    = hex_path,
        hex_col     = hex_col,
        top_n       = top_n,
        sample_rate = sample_rate,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 60
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── k-means clustering ────────────────────────────────────────────────────────
// Lloyd's algorithm, pure Python, no sklearn.
// Reports cluster centroids, sizes, inertia, and per-row assignments.

pub async fn cluster_kmeans(
    file_path: &str,
    k: usize,
    cols: &[&str],
    max_iter: usize,
    output: &str,
) -> Result<String, String> {
    let hex_path:   String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let cols_joined = cols.join("\n");
    let hex_cols:   String = cols_joined.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_output: String = output.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math, random

_path    = bytes.fromhex("{hex_path}").decode().strip()
_cols_raw = bytes.fromhex("{hex_cols}").decode().strip()
_cols    = [c.strip() for c in _cols_raw.split('\n') if c.strip()] if _cols_raw else []
_k       = {k}
_max_iter = {max_iter}
_output  = bytes.fromhex("{hex_output}").decode().strip()

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r)
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        return d if isinstance(d, list) else next(iter(d.values()), [])
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return []
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d2[0] for d2 in cur.description]
        rows2 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows2
    print("Unsupported: "+ext, file=sys.stderr); sys.exit(1)

def _tf(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

rows = _load(_path)
if not rows:
    print("No rows found."); sys.exit(0)

all_cols = list(rows[0].keys())
if _cols:
    feature_cols = [c for c in all_cols if c in _cols]
else:
    feature_cols = [c for c in all_cols if sum(1 for r in rows if _tf(r.get(c,'')) is not None) >= len(rows)*0.5]

if len(feature_cols) < 1:
    print("No numeric feature columns found."); sys.exit(1)

valid_rows = [r for r in rows if all(_tf(r.get(c,'')) is not None for c in feature_cols)]
if len(valid_rows) < _k:
    print("Fewer valid rows (%d) than clusters (%d)." % (len(valid_rows), _k)); sys.exit(1)

X = [[_tf(r[c]) for c in feature_cols] for r in valid_rows]
n = len(X); d = len(feature_cols)

def _dist(a, b): return math.sqrt(sum((ai-bi)**2 for ai,bi in zip(a,b)))
def _centroid(pts): return [sum(p[j] for p in pts)/len(pts) for j in range(d)] if pts else [0.0]*d

# k-means++ init
random.seed(42)
centroids = [X[random.randint(0,n-1)]]
for _ in range(_k-1):
    dists = [min(_dist(x, c)**2 for c in centroids) for x in X]
    total = sum(dists)
    r = random.random() * total
    cum = 0
    for i,dd in enumerate(dists):
        cum += dd
        if cum >= r: centroids.append(X[i]); break
    else: centroids.append(X[-1])

labels = [0]*n
for _ in range(_max_iter):
    new_labels = [min(range(_k), key=lambda c: _dist(x, centroids[c])) for x in X]
    if new_labels == labels: break
    labels = new_labels
    for c in range(_k):
        pts = [X[i] for i in range(n) if labels[i]==c]
        if pts: centroids[c] = _centroid(pts)

inertia = sum(_dist(X[i], centroids[labels[i]])**2 for i in range(n))
cluster_sizes = [labels.count(c) for c in range(_k)]

W = 64
print("="*W)
print(" k-Means Clustering: %s  (k=%d)" % (os.path.basename(_path), _k))
print(" Features: %s" % ', '.join(feature_cols))
print(" Rows: %d   Inertia: %.4f" % (n, inertia))
print("-"*W)
for c in range(_k):
    centroid_str = '  '.join("%.4g" % v for v in centroids[c])
    print("  Cluster %d  (%d rows): centroid = [%s]" % (c, cluster_sizes[c], centroid_str))
print("="*W)

if _output:
    with open(_output, 'w', newline='', encoding='utf-8') as fh:
        fns2 = list(valid_rows[0].keys()) + ['cluster']
        w = _csv.DictWriter(fh, fieldnames=fns2)
        w.writeheader()
        for i,r in enumerate(valid_rows):
            r2 = dict(r); r2['cluster'] = labels[i]
            w.writerow(r2)
    print("Labeled data saved to: %s" % _output)
"####,
        hex_path   = hex_path,
        hex_cols   = hex_cols,
        hex_output = hex_output,
        k          = k,
        max_iter   = max_iter,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 60
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Normalize / standardize dataset ──────────────────────────────────────────
// Applies min-max scaling or z-score standardization to numeric columns.
// Outputs a new CSV with scaled values and reports the scaling parameters.

pub async fn normalize_dataset(
    file_path: &str,
    method: &str,
    cols: &[&str],
    output: &str,
) -> Result<String, String> {
    let hex_path:   String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let cols_joined = cols.join("\n");
    let hex_cols:   String = cols_joined.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_method: String = method.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_output: String = output.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math

_path    = bytes.fromhex("{hex_path}").decode().strip()
_cols_raw = bytes.fromhex("{hex_cols}").decode().strip()
_cols    = [c.strip() for c in _cols_raw.split('\n') if c.strip()] if _cols_raw else []
_method  = bytes.fromhex("{hex_method}").decode().strip().lower() or "minmax"
_output  = bytes.fromhex("{hex_output}").decode().strip()

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r), list(r.fieldnames or [])
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        rows2 = d if isinstance(d, list) else next(iter(d.values()), [])
        return rows2, list(rows2[0].keys()) if rows2 else []
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return [], []
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d2[0] for d2 in cur.description]
        rows2 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows2, cols2
    print("Unsupported: "+ext, file=sys.stderr); sys.exit(1)

def _tf(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

rows, fieldnames = _load(_path)
if not rows:
    print("No rows found."); sys.exit(0)

all_cols = list(rows[0].keys())
if _cols:
    target_cols = [c for c in all_cols if c in _cols]
else:
    target_cols = [c for c in all_cols if sum(1 for r in rows if _tf(r.get(c,'')) is not None) >= len(rows)*0.5]

params = {{}}
for c in target_cols:
    vals = [_tf(r.get(c,'')) for r in rows if _tf(r.get(c,'')) is not None]
    if not vals: continue
    mean = sum(vals)/len(vals)
    std  = math.sqrt(sum((v-mean)**2 for v in vals)/len(vals))
    mn   = min(vals); mx = max(vals)
    params[c] = (mean, std, mn, mx)

W = 64
print("="*W)
print(" Dataset Normalization: %s  (method=%s)" % (os.path.basename(_path), _method))
print("-"*W)
print("  %-20s  %-10s  %-10s  %-10s  %-10s" % ("Column", "Min", "Max", "Mean", "Std"))
print("  " + "-"*56)
for c,( mean,std,mn,mx) in params.items():
    print("  %-20s  %-10.4g  %-10.4g  %-10.4g  %-10.4g" % (c[:20], mn, mx, mean, std))
print("="*W)

if _output:
    out_rows = []
    for r in rows:
        out_r = dict(r)
        for c,(mean,std,mn,mx) in params.items():
            v = _tf(r.get(c,''))
            if v is None:
                out_r[c] = ''
                continue
            if _method in ('minmax','min-max','min_max'):
                rng = mx-mn
                out_r[c] = "%.8f" % ((v-mn)/rng if rng else 0.0)
            elif _method in ('zscore','z-score','z_score','standard','standardize'):
                out_r[c] = "%.8f" % ((v-mean)/std if std else 0.0)
            elif _method in ('robust',):
                from functools import reduce
                # Use median and IQR
                vals2 = sorted(_tf(rr.get(c,'')) for rr in rows if _tf(rr.get(c,'')) is not None)
                n2 = len(vals2)
                q1 = vals2[n2//4]; q3 = vals2[3*n2//4]
                iqr = q3-q1
                med = vals2[n2//2]
                out_r[c] = "%.8f" % ((v-med)/iqr if iqr else 0.0)
        out_rows.append(out_r)
    with open(_output, 'w', newline='', encoding='utf-8') as fh:
        fns2 = fieldnames if fieldnames else list(out_rows[0].keys()) if out_rows else []
        w = _csv.DictWriter(fh, fieldnames=fns2)
        w.writeheader(); w.writerows(out_rows)
    print("Normalized data (%d rows) saved to: %s" % (len(out_rows), _output))
else:
    print("  (No --normalize-output specified — use --normalize-output FILE to save scaled CSV)")
"####,
        hex_path   = hex_path,
        hex_cols   = hex_cols,
        hex_method = hex_method,
        hex_output = hex_output,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── PCA — Principal Component Analysis ───────────────────────────────────────
// Pure-Python power-iteration covariance PCA.  No numpy.
// Reports top-N components: eigenvalue, variance explained, loadings bar chart.
// Optionally writes a projected-coordinates CSV.

pub async fn pca_analyze(
    file_path: &str,
    n_components: usize,
    cols: &[&str],
    output: &str,
) -> Result<String, String> {
    let hex_path:   String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_cols:   String = cols.join(",").bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_output: String = output.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, os, sys, math

_path   = bytes.fromhex("{hex_path}").decode().strip()
_cstr   = bytes.fromhex("{hex_cols}").decode().strip()
_output = bytes.fromhex("{hex_output}").decode().strip()
_n_comp = {n_components}

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r)
    raise ValueError("Unsupported file type: " + ext + " (CSV/TSV only for PCA)")

def _tf(v):
    try: return float(v)
    except: return None

rows = _load(_path)
if not rows:
    print("No data found."); sys.exit(0)

all_cols = list(rows[0].keys())
sel = [c.strip() for c in _cstr.split(',') if c.strip()] if _cstr else []
num_cols = sel if sel else [c for c in all_cols if any(_tf(r.get(c,'')) is not None for r in rows[:20])]
num_cols = [c for c in num_cols if c in all_cols]

mat = []
for r in rows:
    vals = [_tf(r.get(c,'')) for c in num_cols]
    if all(v is not None for v in vals):
        mat.append(vals)

n_rows = len(mat); n_cols = len(num_cols)
if n_rows < 2 or n_cols < 2:
    print("Need at least 2 rows and 2 numeric columns for PCA."); sys.exit(0)

means = [sum(mat[i][j] for i in range(n_rows))/n_rows for j in range(n_cols)]
X = [[mat[i][j] - means[j] for j in range(n_cols)] for i in range(n_rows)]

def cov_matrix(X, nc, nr):
    C = [[0.0]*nc for _ in range(nc)]
    for j in range(nc):
        for k in range(j, nc):
            s = sum(X[i][j]*X[i][k] for i in range(nr)) / (nr-1)
            C[j][k] = C[k][j] = s
    return C

C = cov_matrix(X, n_cols, n_rows)

def mat_vec(M, v):
    return [sum(M[i][j]*v[j] for j in range(len(v))) for i in range(len(v))]

def vec_norm(v): return math.sqrt(sum(x*x for x in v))
def vec_scale(v, s): return [x*s for x in v]

n_comp = min(_n_comp, n_cols, n_rows-1)
total_var = sum(C[j][j] for j in range(n_cols))
eigvals = []; eigvecs = []
Cd = [row[:] for row in C]

for ci in range(n_comp):
    v = [1.0 if j == ci % n_cols else 0.1 for j in range(n_cols)]
    nrm = vec_norm(v); v = vec_scale(v, 1.0/nrm)
    for _it in range(300):
        v_new = mat_vec(Cd, v)
        nrm = vec_norm(v_new)
        if nrm < 1e-14: break
        v_new = vec_scale(v_new, 1.0/nrm)
        delta = vec_norm([v_new[j]-v[j] for j in range(n_cols)])
        v = v_new
        if delta < 1e-10: break
    lam = sum(mat_vec(Cd, v)[j]*v[j] for j in range(n_cols))
    if lam < 0: lam = 0.0
    eigvals.append(lam)
    eigvecs.append(v[:])
    for i in range(n_cols):
        for j in range(n_cols):
            Cd[i][j] -= lam * v[i] * v[j]

projected = []
for row_x in X:
    projected.append([sum(row_x[j]*eigvecs[c][j] for j in range(n_cols)) for c in range(n_comp)])

W = 68
print("="*W)
print("  PCA  —  Principal Component Analysis")
print("  File   : %s" % os.path.basename(_path))
print("  Rows   : %d  |  Columns : %d  |  Components: %d" % (n_rows, n_cols, n_comp))
print("  Columns: %s" % ', '.join(num_cols[:6]) + (('  +%d more' % (len(num_cols)-6)) if len(num_cols)>6 else ''))
print("="*W)

cum = 0.0
for ci in range(n_comp):
    pct = (eigvals[ci]/total_var*100) if total_var > 0 else 0.0
    cum += pct
    bar = int(round(pct / 2.5))
    bar_str = "█"*bar + "░"*(40-bar)
    print("\n  PC%d  eigenvalue %.4f  |  var %5.1f%%  |  cumulative %5.1f%%" % (ci+1, eigvals[ci], pct, cum))
    print("  %s" % bar_str)
    loads = sorted(enumerate(eigvecs[ci]), key=lambda x: -abs(x[1]))
    print("  Top loadings:")
    for _idx, (fidx, w) in enumerate(loads[:8]):
        sign = '+' if w >= 0 else '-'
        bar2 = int(abs(w)*20)
        print("    %-22s  %s%.4f  %s" % (num_cols[fidx][:22], sign, abs(w), "▌"*bar2))

print()
print("  Projected sample (first 5 rows):")
print("  " + "".join("  PC%-7d" % (c+1) for c in range(n_comp)))
for row_p in projected[:5]:
    print("  " + "".join("%+-10.4f" % v for v in row_p))
print()
print("="*W)

if _output:
    pc_cols = ["PC%d" % (c+1) for c in range(n_comp)]
    with open(_output, 'w', newline='', encoding='utf-8') as fh:
        w2 = _csv.writer(fh)
        w2.writerow(pc_cols)
        for row_p in projected:
            w2.writerow(["%.8f" % v for v in row_p])
    print("  Projected data (%d rows) saved to: %s" % (len(projected), _output))
else:
    print("  (Use --pca-output FILE to save projected coordinates as CSV)")
"####,
        hex_path   = hex_path,
        hex_cols   = hex_cols,
        hex_output = hex_output,
        n_components = n_components,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 60
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}
