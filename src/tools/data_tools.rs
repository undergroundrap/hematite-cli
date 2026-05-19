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

// ── Statistical hypothesis testing ───────────────────────────────────────────
// t-tests, chi-square, ANOVA, Mann-Whitney, Pearson, proportion z-test,
// confidence intervals — all via Python stdlib (statistics, math only).
pub async fn hypothesis_test(
    test_type: &str,
    group1_str: &str,
    group2_str: &str,
    alpha: f64,
    mu: f64,
) -> Result<String, String> {
    let hex_test:   String = test_type.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_g1:     String = group1_str.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_g2:     String = group2_str.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import math, statistics as _st, sys

_test  = bytes.fromhex("{hex_test}").decode().strip().lower()
_g1s   = bytes.fromhex("{hex_g1}").decode().strip()
_g2s   = bytes.fromhex("{hex_g2}").decode().strip()
_alpha = {alpha}
_mu0   = {mu}
W = 60

def parse_nums(s):
    if not s: return []
    try:
        return [float(x.strip()) for x in s.replace(';',',').split(',') if x.strip()]
    except:
        return []

def fmt_p(p):
    if p < 0.001: return "< 0.001"
    return "%.4f" % p

def decision(p, alpha):
    if p < alpha:
        return "REJECT H0  (p %s < a=%.2f)" % (fmt_p(p), alpha)
    return "FAIL TO REJECT H0  (p %s >= a=%.2f)" % (fmt_p(p), alpha)

def t_cdf(t, df):
    x = df / (df + t*t)
    def betainc(a, b, xv):
        if xv<=0: return 0.0
        if xv>=1: return 1.0
        tiny=1e-300; fp_min=tiny
        qab=a+b; qap=a+1; qam=a-1
        c=1.0; d=1.0-qab*xv/qap
        if abs(d)<fp_min: d=fp_min
        d=1.0/d; h=d
        for m in range(1,201):
            m2=2*m; aa=m*(b-m)*xv/((qam+m2)*(a+m2))
            d=1.0+aa*d
            if abs(d)<fp_min: d=fp_min
            c=1.0+aa/c
            if abs(c)<fp_min: c=fp_min
            d=1.0/d; h*=d*c
            aa=-(a+m)*(qab+m)*xv/((a+m2)*(qap+m2))
            d=1.0+aa*d
            if abs(d)<fp_min: d=fp_min
            c=1.0+aa/c
            if abs(c)<fp_min: c=fp_min
            d=1.0/d; delta=d*c; h*=delta
            if abs(delta-1.0)<3e-7: break
        return math.exp(math.lgamma(a+b)-math.lgamma(a)-math.lgamma(b)+
                        a*math.log(xv)+b*math.log(1-xv))*h/a
    a=df/2; b=0.5
    ibeta=betainc(b,a,x)
    return ibeta

def normal_cdf_h(z):
    return 0.5*(1+math.erf(z/math.sqrt(2)))

def chi2_pval(chi2, df):
    a=df/2.0; x2=chi2/2.0
    if x2<=0: return 1.0
    if x2<a+1:
        ap=a; delta=s=1.0/a
        for _ in range(300):
            ap+=1; delta*=x2/ap; s+=delta
            if abs(delta)<abs(s)*1e-9: break
        p=s*math.exp(-x2+a*math.log(x2)-math.lgamma(a))
        return max(0.0,min(1.0,1.0-p))
    else:
        b=x2+1-a; c=1e300; d=1.0/b; h=d
        for i in range(1,301):
            an=-i*(i-a); b+=2
            d=an*d+b
            if abs(d)<1e-300: d=1e-300
            c=b+an/c
            if abs(c)<1e-300: c=1e-300
            d=1.0/d; delta=d*c; h*=delta
            if abs(delta-1.0)<1e-9: break
        q=math.exp(-x2+a*math.log(x2)-math.lgamma(a))*h
        return max(0.0,min(1.0,q))

def f_pval(f, df1, df2):
    x=df2/(df2+df1*f)
    def betainc(a, b, xv):
        if xv<=0: return 0.0
        if xv>=1: return 1.0
        tiny=1e-300; fp_min=tiny
        qab=a+b; qap=a+1; qam=a-1
        c=1.0; d=1.0-qab*xv/qap
        if abs(d)<fp_min: d=fp_min
        d=1.0/d; h=d
        for m in range(1,201):
            m2=2*m; aa=m*(b-m)*xv/((qam+m2)*(a+m2))
            d=1.0+aa*d
            if abs(d)<fp_min: d=fp_min
            c=1.0+aa/c
            if abs(c)<fp_min: c=fp_min
            d=1.0/d; h*=d*c
            aa=-(a+m)*(qab+m)*xv/((a+m2)*(qap+m2))
            d=1.0+aa*d
            if abs(d)<fp_min: d=fp_min
            c=1.0+aa/c
            if abs(c)<fp_min: c=fp_min
            d=1.0/d; delta=d*c; h*=delta
            if abs(delta-1.0)<3e-7: break
        return math.exp(math.lgamma(a+b)-math.lgamma(a)-math.lgamma(b)+
                        a*math.log(xv)+b*math.log(1-xv))*h/a
    return betainc(df2/2,df1/2,x)

print("="*W)
print("  HYPOTHESIS TEST")
print("="*W)

if _test in ("one-t","one_t","onesample","one-sample","t1","t-one"):
    g1=parse_nums(_g1s)
    if len(g1)<2: print("  ERROR: need >=2 values for one-sample t-test"); sys.exit(0)
    n=len(g1); xbar=_st.mean(g1); s=_st.stdev(g1)
    se=s/math.sqrt(n); t=(xbar-_mu0)/se; df=n-1
    p=t_cdf(abs(t),df)
    print("  One-Sample t-Test  (H0: mu = %.4g)" % _mu0)
    print("  n=%-5d  xbar=%.4f  s=%.4f  SE=%.4f" % (n,xbar,s,se))
    print("  t=%.4f  df=%d  p=%s" % (t,df,fmt_p(p)))
    from_t=1.96 if df>30 else (2.093 if df>19 else (2.262 if df>9 else 2.776))
    lo=xbar-from_t*se; hi=xbar+from_t*se
    print("  %.0f%% CI: [%.4f, %.4f]" % ((1-_alpha)*100,lo,hi))
    print(); print("  "+decision(p,_alpha))

elif _test in ("two-t","two_t","twosample","two-sample","welch","t2","t-two"):
    g1=parse_nums(_g1s); g2=parse_nums(_g2s)
    if len(g1)<2 or len(g2)<2: print("  ERROR: need >=2 values in each group"); sys.exit(0)
    n1=len(g1); n2=len(g2)
    x1=_st.mean(g1); x2=_st.mean(g2)
    s1=_st.stdev(g1); s2=_st.stdev(g2)
    se=math.sqrt(s1**2/n1+s2**2/n2)
    t=(x1-x2)/se
    df=(s1**2/n1+s2**2/n2)**2/((s1**2/n1)**2/(n1-1)+(s2**2/n2)**2/(n2-1))
    p=t_cdf(abs(t),df)
    print("  Two-Sample (Welch) t-Test  (H0: mu1 = mu2)")
    print("  G1: n=%d  xbar=%.4f  s=%.4f" % (n1,x1,s1))
    print("  G2: n=%d  xbar=%.4f  s=%.4f" % (n2,x2,s2))
    print("  delta_xbar=%.4f  SE=%.4f  t=%.4f  df=%.1f  p=%s" % (x1-x2,se,t,df,fmt_p(p)))
    print(); print("  "+decision(p,_alpha))

elif _test in ("paired","paired-t","pairedt","t-paired"):
    g1=parse_nums(_g1s); g2=parse_nums(_g2s)
    if len(g1)!=len(g2) or len(g1)<2: print("  ERROR: groups must match in length (>=2)"); sys.exit(0)
    diffs=[a-b for a,b in zip(g1,g2)]
    n=len(diffs); dbar=_st.mean(diffs); sd=_st.stdev(diffs)
    se=sd/math.sqrt(n); t=dbar/se; df=n-1
    p=t_cdf(abs(t),df)
    print("  Paired t-Test  (H0: mu_diff = 0)")
    print("  n=%d  dbar=%.4f  sd=%.4f  SE=%.4f" % (n,dbar,sd,se))
    print("  t=%.4f  df=%d  p=%s" % (t,df,fmt_p(p)))
    print(); print("  "+decision(p,_alpha))

elif _test in ("chi2","chi-square","chisquare","chi-sq","goodness"):
    observed=parse_nums(_g1s)
    if len(observed)<2: print("  ERROR: need >=2 observed counts"); sys.exit(0)
    expected_s=_g2s.strip()
    if expected_s:
        expected=parse_nums(expected_s)
        if len(expected)!=len(observed): print("  ERROR: observed/expected length mismatch"); sys.exit(0)
    else:
        e=sum(observed)/len(observed); expected=[e]*len(observed)
    chi2=sum((o-e)**2/e for o,e in zip(observed,expected) if e>0)
    df=len(observed)-1; p=chi2_pval(chi2,df)
    print("  Chi-Square Goodness-of-Fit  (H0: observed ~ expected)")
    print("  %-12s  %-10s  %-10s  %-8s" % ("Category","Observed","Expected","(O-E)^2/E"))
    for i,(o,e) in enumerate(zip(observed,expected)):
        print("  %-12s  %-10.2f  %-10.2f  %.4f" % ("cat%d"%(i+1),o,e,(o-e)**2/e if e>0 else 0))
    print("  chi2=%.4f  df=%d  p=%s" % (chi2,df,fmt_p(p)))
    print(); print("  "+decision(p,_alpha))

elif _test in ("anova","one-way","oneway","f-test"):
    raw_groups=[g.strip() for g in _g1s.split('|')]
    groups=[parse_nums(g) for g in raw_groups if g]
    groups=[g for g in groups if len(g)>=2]
    if len(groups)<2: print("  ERROR: need >=2 groups separated by | for ANOVA"); sys.exit(0)
    k=len(groups); N=sum(len(g) for g in groups)
    grand=sum(sum(g) for g in groups)/N
    SSB=sum(len(g)*(_st.mean(g)-grand)**2 for g in groups)
    SSW=sum(sum((x-_st.mean(g))**2 for x in g) for g in groups)
    dfB=k-1; dfW=N-k
    F=(SSB/dfB)/(SSW/dfW) if dfW>0 and SSW>0 else float('inf')
    p=f_pval(F,dfB,dfW)
    print("  One-Way ANOVA  (H0: all group means equal)")
    for i,g in enumerate(groups):
        print("  G%d: n=%d  xbar=%.4f  s=%.4f" % (i+1,len(g),_st.mean(g),_st.stdev(g)))
    print("  SSB=%.4f (df=%d)  SSW=%.4f (df=%d)" % (SSB,dfB,SSW,dfW))
    print("  F=%.4f  p=%s" % (F,fmt_p(p)))
    print(); print("  "+decision(p,_alpha))

elif _test in ("mannwhitney","mann-whitney","mwu","wilcoxon-rank","ranksum"):
    g1=parse_nums(_g1s); g2=parse_nums(_g2s)
    if len(g1)<1 or len(g2)<1: print("  ERROR: need values in both groups"); sys.exit(0)
    n1=len(g1); n2=len(g2)
    combined=sorted([(v,'a',i) for i,v in enumerate(g1)]+[(v,'b',i) for i,v in enumerate(g2)])
    ranks={{}}
    i=0
    while i<len(combined):
        j=i
        while j<len(combined)-1 and combined[j][0]==combined[j+1][0]: j+=1
        avg_rank=(i+1+j+1)/2
        for kk in range(i,j+1): ranks[(combined[kk][1],combined[kk][2])]=avg_rank
        i=j+1
    R1=sum(ranks[('a',i)] for i in range(n1))
    U1=R1-n1*(n1+1)/2; U2=n1*n2-U1; U=min(U1,U2)
    mu_U=n1*n2/2; sigma_U=math.sqrt(n1*n2*(n1+n2+1)/12)
    z=(U-mu_U)/sigma_U if sigma_U>0 else 0
    p=2*(1-normal_cdf_h(abs(z)))
    print("  Mann-Whitney U Test  (H0: distributions equal)")
    print("  n1=%d  n2=%d  U=%.1f  z=%.4f  p=%s" % (n1,n2,U,z,fmt_p(p)))
    print(); print("  "+decision(p,_alpha))

elif _test in ("pearson","correlation","corr"):
    g1=parse_nums(_g1s); g2=parse_nums(_g2s)
    if len(g1)!=len(g2) or len(g1)<3: print("  ERROR: need matching vectors of length >=3"); sys.exit(0)
    n=len(g1); x1=_st.mean(g1); x2=_st.mean(g2)
    num=sum((a-x1)*(b-x2) for a,b in zip(g1,g2))
    d1=math.sqrt(sum((a-x1)**2 for a in g1)); d2=math.sqrt(sum((b-x2)**2 for b in g2))
    r=num/(d1*d2) if d1*d2>0 else 0
    t=r*math.sqrt(n-2)/math.sqrt(1-r**2) if abs(r)<1 else float('inf')
    df=n-2; p=t_cdf(abs(t),df)
    print("  Pearson Correlation Test  (H0: rho = 0)")
    print("  n=%d  r=%.4f  t=%.4f  df=%d  p=%s" % (n,r,t,df,fmt_p(p)))
    strength="negligible" if abs(r)<0.1 else "weak" if abs(r)<0.3 else "moderate" if abs(r)<0.5 else "strong"
    print("  Strength: %s (%s)" % (strength,"positive" if r>=0 else "negative"))
    print(); print("  "+decision(p,_alpha))

elif _test in ("proportion","prop","z-prop","zprop","prop1","one-prop"):
    parts=parse_nums(_g1s)
    if len(parts)<2: print("  ERROR: provide 'successes,n' as group1"); sys.exit(0)
    k=int(parts[0]); n=int(parts[1]); p_hat=k/n; p0=_mu0 if _mu0>0 else 0.5
    se=math.sqrt(p0*(1-p0)/n)
    z=(p_hat-p0)/se if se>0 else 0
    p=2*(1-normal_cdf_h(abs(z)))
    ci_se=math.sqrt(p_hat*(1-p_hat)/n); z_crit=1.96
    lo=p_hat-z_crit*ci_se; hi=p_hat+z_crit*ci_se
    print("  One-Proportion z-Test  (H0: p = %.4g)" % p0)
    print("  k=%d  n=%d  p_hat=%.4f  SE=%.4f  z=%.4f  p=%s" % (k,n,p_hat,se,z,fmt_p(p)))
    print("  95%% CI: [%.4f, %.4f]" % (lo,hi))
    print(); print("  "+decision(p,_alpha))

elif _test in ("prop2","two-prop","twoprop","two-proportion"):
    p1=parse_nums(_g1s); p2=parse_nums(_g2s)
    if len(p1)<2 or len(p2)<2: print("  ERROR: each group needs 'successes,n'"); sys.exit(0)
    k1=int(p1[0]); n1=int(p1[1]); k2=int(p2[0]); n2=int(p2[1])
    ph1=k1/n1; ph2=k2/n2; pp=(k1+k2)/(n1+n2)
    se=math.sqrt(pp*(1-pp)*(1/n1+1/n2))
    z=(ph1-ph2)/se if se>0 else 0
    p=2*(1-normal_cdf_h(abs(z)))
    print("  Two-Proportion z-Test  (H0: p1 = p2)")
    print("  G1: %d/%d (p_hat=%.4f)  G2: %d/%d (p_hat=%.4f)" % (k1,n1,ph1,k2,n2,ph2))
    print("  Pooled p_hat=%.4f  SE=%.4f  z=%.4f  p=%s" % (pp,se,z,fmt_p(p)))
    print(); print("  "+decision(p,_alpha))

elif _test in ("ci","confidence","conf-interval","interval"):
    g1=parse_nums(_g1s)
    if len(g1)<2: print("  ERROR: need >=2 values for confidence interval"); sys.exit(0)
    n=len(g1); xbar=_st.mean(g1); s=_st.stdev(g1); se=s/math.sqrt(n)
    z_crit=1.96 if n>30 else (2.093 if n>19 else (2.262 if n>9 else 2.776))
    lo=xbar-z_crit*se; hi=xbar+z_crit*se
    print("  Confidence Interval for Mean")
    print("  n=%d  xbar=%.4f  s=%.4f  SE=%.4f" % (n,xbar,s,se))
    print("  %.0f%% CI: [%.4f, %.4f]  (+-%.4f)" % ((1-_alpha)*100,lo,hi,z_crit*se))
    if _mu0!=0:
        inside=lo<=_mu0<=hi
        print("  H0 value (mu=%.4g) is %s the interval" % (_mu0,"INSIDE" if inside else "OUTSIDE"))

else:
    print("  Available tests:")
    print("  one-t       One-sample t-test:  --hypothesis-mu H0_MEAN")
    print("  two-t       Two-sample (Welch) t-test (--hypothesis-group2 DATA)")
    print("  paired      Paired t-test (--hypothesis-group2 DATA)")
    print("  chi2        Chi-square goodness-of-fit (--hypothesis-group2 EXPECTED)")
    print("  anova       One-way ANOVA (groups separated by | in group1)")
    print("  mannwhitney Mann-Whitney U (--hypothesis-group2 DATA)")
    print("  pearson     Pearson correlation test (--hypothesis-group2 DATA)")
    print("  proportion  One-proportion z-test: 'successes,n' --hypothesis-mu P0")
    print("  prop2       Two-proportion z-test (--hypothesis-group2 'k2,n2')")
    print("  ci          Confidence interval for mean")
    print()
    print("  Data format: comma-separated numbers, e.g.  3.1,2.8,4.0,3.5")
    print("  For ANOVA: groups separated by |  e.g.  2.1,2.3|3.4,3.6|1.9,2.0")

print("="*W)
"####,
        hex_test   = hex_test,
        hex_g1     = hex_g1,
        hex_g2     = hex_g2,
        alpha      = alpha,
        mu         = mu,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Descriptive statistics ─────────────────────────────────────────────────────
// Full per-column stats: mean, median, mode, std, variance, skewness, kurtosis,
// percentiles (P5/P25/P50/P75/P95), IQR, outliers, ASCII histogram.
pub async fn describe_stats(
    file_path: &str,
    cols_str: &str,
    output: &str,
) -> Result<String, String> {
    let hex_path:   String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_cols:   String = cols_str.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_output: String = output.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, math

_path   = bytes.fromhex("{hex_path}").decode().strip()
_cols_s = bytes.fromhex("{hex_cols}").decode().strip()
_output = bytes.fromhex("{hex_output}").decode().strip()
W = 68

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            rows = list(r)
        return rows
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
    return []

rows = _load(_path)
if not rows:
    print("  ERROR: no rows loaded from", _path)
    raise SystemExit(0)

all_cols = list(rows[0].keys())
if _cols_s:
    req = [c.strip() for c in _cols_s.split(',')]
    cols = [c for c in req if c in all_cols]
    if not cols:
        print("  WARNING: none of the specified columns found. Available:", ', '.join(all_cols))
        cols = all_cols
else:
    cols = all_cols

def parse_nums(rows, col):
    vals = []
    for r in rows:
        v = r.get(col, '')
        if v is None: continue
        v = str(v).strip()
        if not v: continue
        try: vals.append(float(v))
        except: pass
    return vals

def percentile(sorted_v, p):
    if not sorted_v: return float('nan')
    n = len(sorted_v)
    idx = (n - 1) * p / 100.0
    lo = int(idx); hi = lo + 1
    if hi >= n: return sorted_v[lo]
    frac = idx - lo
    return sorted_v[lo] * (1 - frac) + sorted_v[hi] * frac

def skewness(vals, mean, std):
    if std == 0 or len(vals) < 3: return float('nan')
    n = len(vals)
    s3 = sum((x - mean)**3 for x in vals)
    return (n / ((n-1)*(n-2))) * s3 / std**3

def kurtosis(vals, mean, std):
    if std == 0 or len(vals) < 4: return float('nan')
    n = len(vals)
    s4 = sum((x - mean)**4 for x in vals)
    k = (n*(n+1)/((n-1)*(n-2)*(n-3))) * s4 / std**4 - 3*(n-1)**2/((n-2)*(n-3))
    return k

def ascii_hist(vals, bins=16, width=36):
    if not vals: return []
    mn = min(vals); mx = max(vals)
    rng = mx - mn
    if rng == 0: return ["  (all values identical: %.4g)" % mn]
    bw = rng / bins
    counts = [0]*bins
    for v in vals:
        idx = min(int((v - mn) / bw), bins - 1)
        counts[idx] += 1
    max_c = max(counts) or 1
    lines = []
    for i, c in enumerate(counts):
        lo = mn + i*bw; hi = lo+bw
        bar = int(c / max_c * width)
        lines.append("  [%8.3g, %8.3g)  %s %d" % (lo, hi, '█'*bar, c))
    return lines

print("=" * W)
print("  DESCRIPTIVE STATISTICS")
print("  File:   %s" % os.path.basename(_path))
print("  Rows:   %d    Cols examined: %d" % (len(rows), len(cols)))
print("=" * W)

results = []
for col in cols:
    vals = parse_nums(rows, col)
    if len(vals) < 2:
        print("\n  %s: too few numeric values (%d)" % (col, len(vals)))
        continue
    s = sorted(vals)
    n = len(vals)
    mean = sum(vals) / n
    var = sum((x-mean)**2 for x in vals) / (n-1) if n > 1 else 0
    std = math.sqrt(var)
    med = percentile(s, 50)
    # mode (simple: most frequent rounded value)
    from collections import Counter
    mode_ctr = Counter(round(v, 4) for v in vals)
    mode_val, mode_cnt = mode_ctr.most_common(1)[0]
    p5  = percentile(s, 5)
    p25 = percentile(s, 25)
    p75 = percentile(s, 75)
    p95 = percentile(s, 95)
    iqr = p75 - p25
    skew = skewness(vals, mean, std)
    kurt = kurtosis(vals, mean, std)
    # Outliers via IQR method
    lo_fence = p25 - 1.5*iqr; hi_fence = p75 + 1.5*iqr
    outliers = [v for v in vals if v < lo_fence or v > hi_fence]
    missing = len(rows) - sum(1 for r in rows if str(r.get(col,'')).strip())

    print("\n  %s" % col)
    print("  " + "-"*50)
    print("  n=%-8d  missing=%-6d  unique=%d" % (n, missing, len(set(round(v,6) for v in vals))))
    print("  mean=%11.6g  std=%11.6g  var=%11.6g" % (mean, std, var))
    print("  min=%12.6g  max=%12.6g  range=%10.6g" % (s[0], s[-1], s[-1]-s[0]))
    print("  P5=%12.6g  P25=%11.6g  median=%9.6g" % (p5, p25, med))
    print("  P75=%11.6g  P95=%11.6g  IQR=%11.6g" % (p75, p95, iqr))
    print("  mode=%11.6g (count=%d)" % (mode_val, mode_cnt))
    if not math.isnan(skew): print("  skewness=%8.4f  kurtosis=%8.4f" % (skew, kurt))
    if outliers: print("  outliers (IQR): %d value(s)  min=%.4g  max=%.4g" % (len(outliers), min(outliers), max(outliers)))
    print()
    for line in ascii_hist(vals): print(line)
    results.append((col, n, mean, std, s[0], s[-1], med))

if _output and results:
    with open(_output, 'w', newline='', encoding='utf-8') as fh:
        w2 = _csv.writer(fh)
        w2.writerow(['column','n','mean','std','min','max','median'])
        for row in results:
            w2.writerow(['%.8g'%v if isinstance(v,float) else v for v in row])
    print("\n  Summary saved to: %s" % _output)
elif _output == '' and results:
    print("\n  (Use --stats-output FILE to save summary CSV)")

print("\n" + "=" * W)
"####,
        hex_path   = hex_path,
        hex_cols   = hex_cols,
        hex_output = hex_output,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Classification (k-NN and Naive Bayes) ────────────────────────────────────
// Trains on labeled CSV data, predicts a class label for new input, and runs
// leave-one-out cross-validation to report accuracy. No external libraries.
pub async fn classify_data(
    file_path: &str,
    label_col: &str,
    feature_cols: &str,
    predict_str: &str,
    k: usize,
    method: &str,
) -> Result<String, String> {
    let hex_path:    String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_label:   String = label_col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_feats:   String = feature_cols.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_predict: String = predict_str.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_method:  String = method.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, math
from collections import Counter, defaultdict

_path    = bytes.fromhex("{hex_path}").decode().strip()
_label   = bytes.fromhex("{hex_label}").decode().strip()
_feats_s = bytes.fromhex("{hex_feats}").decode().strip()
_pred_s  = bytes.fromhex("{hex_predict}").decode().strip()
_k       = {k}
_method  = bytes.fromhex("{hex_method}").decode().strip().lower()
W = 60

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
    return []

rows = _load(_path)
if not rows:
    print("  ERROR: no rows loaded from", _path); raise SystemExit(0)
all_cols = list(rows[0].keys())

# Determine label and feature columns
if not _label:
    _label = all_cols[-1]
    print("  (No --classify-label specified; using last column: %s)" % _label)

if _label not in all_cols:
    print("  ERROR: label column '%s' not found. Available: %s" % (_label, ', '.join(all_cols)))
    raise SystemExit(0)

if _feats_s:
    feat_cols = [c.strip() for c in _feats_s.split(',') if c.strip() in all_cols]
else:
    feat_cols = [c for c in all_cols if c != _label]

if not feat_cols:
    print("  ERROR: no feature columns found."); raise SystemExit(0)

# Extract numeric feature vectors
def row_to_vec(row):
    v = []
    for c in feat_cols:
        try: v.append(float(row.get(c, 0) or 0))
        except: v.append(0.0)
    return v

labeled = [(row_to_vec(r), str(r[_label]).strip()) for r in rows if str(r.get(_label,'')).strip()]
if len(labeled) < 3:
    print("  ERROR: need at least 3 labeled rows."); raise SystemExit(0)

X = [v for v,_ in labeled]
y = [lbl for _,lbl in labeled]
classes = sorted(set(y))

print("="*W)
print("  CLASSIFICATION")
print("  File:    %s" % os.path.basename(_path))
print("  Label:   %s   Features: %s" % (_label, ', '.join(feat_cols)))
print("  Method:  %s   Classes: %s" % (_method, ', '.join(classes)))
print("  Samples: %d" % len(labeled))
print("="*W)

# ── k-NN ──────────────────────────────────────────────────────────────────────
def knn_predict(X_train, y_train, x_q, k):
    dists = [(math.sqrt(sum((a-b)**2 for a,b in zip(x_q, xi))), yi)
             for xi, yi in zip(X_train, y_train)]
    dists.sort(key=lambda d: d[0])
    top = [yi for _, yi in dists[:k]]
    return Counter(top).most_common(1)[0][0]

# ── Gaussian Naive Bayes ───────────────────────────────────────────────────────
def gnb_fit(X_train, y_train):
    classes_t = sorted(set(y_train))
    stats = {{}}
    priors = {{}}
    n = len(y_train)
    for c in classes_t:
        idx = [i for i,yi in enumerate(y_train) if yi == c]
        priors[c] = len(idx) / n
        vecs = [X_train[i] for i in idx]
        m = [sum(vecs[j][f] for j in range(len(vecs)))/len(vecs) for f in range(len(feat_cols))]
        v = [sum((vecs[j][f]-m[f])**2 for j in range(len(vecs)))/max(len(vecs)-1,1) for f in range(len(feat_cols))]
        stats[c] = (m, v)
    return priors, stats

def gnb_predict(priors, stats, x_q):
    best_c = None; best_log = float('-inf')
    for c, (m, v) in stats.items():
        log_p = math.log(priors[c] + 1e-300)
        for xi, mi, vi in zip(x_q, m, v):
            vi = max(vi, 1e-9)
            log_p += -0.5 * math.log(2*math.pi*vi) - (xi-mi)**2/(2*vi)
        if log_p > best_log: best_log = log_p; best_c = c
    return best_c

# ── LOO cross-validation ──────────────────────────────────────────────────────
correct = 0
confusion = defaultdict(lambda: defaultdict(int))
for i in range(len(labeled)):
    Xt = [X[j] for j in range(len(X)) if j != i]
    yt = [y[j] for j in range(len(y)) if j != i]
    x_q = X[i]; true = y[i]
    if _method == 'nb' or _method == 'naive_bayes' or _method == 'gnb':
        p, s = gnb_fit(Xt, yt); pred = gnb_predict(p, s, x_q)
    else:
        pred = knn_predict(Xt, yt, x_q, _k)
    confusion[true][pred] += 1
    if pred == true: correct += 1

acc = correct / len(labeled)
print("\n  Leave-One-Out Cross-Validation")
print("  Accuracy: %d/%d = %.2f%%" % (correct, len(labeled), acc*100))
print()

# Confusion matrix
print("  Confusion Matrix (actual=rows, predicted=cols):")
max_w = max(len(c) for c in classes) + 2
print("  " + " "*(max_w) + "  " + "  ".join(c.ljust(max_w) for c in classes))
for actual in classes:
    row = "  " + actual.ljust(max_w) + "  "
    row += "  ".join(str(confusion[actual].get(pred, 0)).ljust(max_w) for pred in classes)
    print(row)

# Per-class precision/recall
print()
print("  Per-class metrics:")
print("  %-15s  %-10s  %-10s  %-10s" % ("Class","Precision","Recall","F1"))
print("  " + "-"*48)
for c in classes:
    tp = confusion[c].get(c, 0)
    fp = sum(confusion[other].get(c,0) for other in classes if other != c)
    fn = sum(confusion[c].get(other,0) for other in classes if other != c)
    prec = tp/(tp+fp) if tp+fp > 0 else 0
    rec  = tp/(tp+fn) if tp+fn > 0 else 0
    f1   = 2*prec*rec/(prec+rec) if prec+rec > 0 else 0
    print("  %-15s  %-10.3f  %-10.3f  %-10.3f" % (c[:15], prec, rec, f1))

# Predict new sample if provided
if _pred_s:
    print()
    p_vals = [float(v.strip()) for v in _pred_s.split(',') if v.strip()]
    if len(p_vals) != len(feat_cols):
        print("  WARNING: --classify-predict has %d values but %d features expected" % (len(p_vals), len(feat_cols)))
    else:
        if _method in ('nb','naive_bayes','gnb'):
            p2, s2 = gnb_fit(X, y); pred_new = gnb_predict(p2, s2, p_vals)
        else:
            pred_new = knn_predict(X, y, p_vals, _k)
        print("  Prediction for [%s]:" % ', '.join('%.4g'%v for v in p_vals))
        print("  => %s" % pred_new)

print("="*W)
"####,
        hex_path    = hex_path,
        hex_label   = hex_label,
        hex_feats   = hex_feats,
        hex_predict = hex_predict,
        hex_method  = hex_method,
        k           = k,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Regression analysis ───────────────────────────────────────────────────────
pub async fn regression_analysis(
    file_path: &str,
    x_col: &str,
    y_col: &str,
    degree: usize,
    predict_x: &str,
) -> Result<String, String> {
    let hex_path:    String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_x_col:  String = x_col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_y_col:  String = y_col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_predict: String = predict_x.bytes().map(|b| format!("{:02x}", b)).collect();
    let deg = degree.max(1).min(10);

    let script = format!(r####"import csv, sys, math

_path   = bytes.fromhex("{hex_path}").decode().strip()
_x_col  = bytes.fromhex("{hex_x_col}").decode().strip()
_y_col  = bytes.fromhex("{hex_y_col}").decode().strip()
_degree = {deg}
_pred_s = bytes.fromhex("{hex_predict}").decode().strip()
W = 64

# ── load CSV ──────────────────────────────────────────────────────────────────
with open(_path, newline="", encoding="utf-8-sig") as f:
    reader = csv.DictReader(f)
    rows   = list(reader)
    header = reader.fieldnames or []

if not header:
    print("ERROR: empty file or no header"); sys.exit(1)

x_col = _x_col if _x_col else header[0]
y_col = _y_col if _y_col else ([c for c in header if c != x_col] or [header[-1]])[-1]

try:
    xs = [float(r[x_col]) for r in rows]
    ys = [float(r[y_col]) for r in rows]
except (KeyError, ValueError) as e:
    print(f"ERROR: {{e}}"); sys.exit(1)

n = len(xs)
if n < 2:
    print("ERROR: need at least 2 data points"); sys.exit(1)

deg = max(1, min(_degree, 10))

# ── Vandermonde least-squares via Gaussian elimination ────────────────────────
def poly_fit(xs, ys, deg):
    d = deg + 1
    ATA = [[0.0]*d for _ in range(d)]
    ATy = [0.0]*d
    for x, y in zip(xs, ys):
        pows = [x**k for k in range(d)]
        for i in range(d):
            ATy[i] += pows[i] * y
            for j in range(d):
                ATA[i][j] += pows[i] * pows[j]
    mat = [ATA[i][:] + [ATy[i]] for i in range(d)]
    for col in range(d):
        pivot = max(range(col, d), key=lambda r: abs(mat[r][col]))
        mat[col], mat[pivot] = mat[pivot], mat[col]
        if abs(mat[col][col]) < 1e-12:
            continue
        for row in range(col+1, d):
            f = mat[row][col] / mat[col][col]
            for k in range(col, d+1):
                mat[row][k] -= f * mat[col][k]
    coeffs = [0.0]*d
    for row in range(d-1, -1, -1):
        coeffs[row] = mat[row][d]
        for k in range(row+1, d):
            coeffs[row] -= mat[row][k] * coeffs[k]
        if abs(mat[row][row]) > 1e-12:
            coeffs[row] /= mat[row][row]
    return coeffs

def poly_eval(coeffs, x):
    return sum(c * x**k for k, c in enumerate(coeffs))

coeffs = poly_fit(xs, ys, deg)

# ── metrics ───────────────────────────────────────────────────────────────────
y_mean = sum(ys) / n
ss_res = sum((y - poly_eval(coeffs, x))**2 for x, y in zip(xs, ys))
ss_tot = sum((y - y_mean)**2 for y in ys)
r2     = 1.0 - ss_res / ss_tot if ss_tot > 1e-12 else 1.0
rmse   = math.sqrt(ss_res / n)
mae    = sum(abs(y - poly_eval(coeffs, x)) for x, y in zip(xs, ys)) / n

if deg == 1 and n > 1:
    sx  = math.sqrt(sum((x - sum(xs)/n)**2 for x in xs) / (n-1))
    sy  = math.sqrt(sum((y - y_mean)**2 for y in ys) / (n-1))
    sxy = sum((x - sum(xs)/n)*(y - y_mean) for x, y in zip(xs, ys)) / (n-1)
    pearson_r = sxy / (sx * sy) if sx * sy > 1e-12 else 0.0
else:
    pearson_r = None

# ── header ────────────────────────────────────────────────────────────────────
print("=" * W)
label = "LINEAR" if deg == 1 else f"POLYNOMIAL (degree {{deg}})"
print(f"  REGRESSION ANALYSIS — {{label}}")
print(f"  X: {{x_col}}   Y: {{y_col}}   N: {{n}}")
print("=" * W)
terms = []
for k, c in enumerate(coeffs):
    if abs(c) < 1e-12: continue
    if k == 0:   terms.append(f"{{c:.6g}}")
    elif k == 1: terms.append(f"{{c:+.6g}}*x")
    else:        terms.append(f"{{c:+.6g}}*x^{{k}}")
print("  y = " + " ".join(terms) if terms else "  y = 0")
print()
print(f"  R2      : {{r2:.6f}}")
if pearson_r is not None:
    print(f"  Pearson : {{pearson_r:.6f}}")
print(f"  RMSE    : {{rmse:.6g}}")
print(f"  MAE     : {{mae:.6g}}")
if   r2 >= 0.95: qual = "Excellent fit (R2 >= 0.95)"
elif r2 >= 0.80: qual = "Good fit (R2 >= 0.80)"
elif r2 >= 0.60: qual = "Moderate fit (R2 >= 0.60)"
else:            qual = "Weak fit (R2 < 0.60)"
print(f"  Quality : {{qual}}")
print()

# ── ASCII scatter + fit curve ─────────────────────────────────────────────────
ROWS, COLS = 16, W - 6
x_min, x_max = min(xs), max(xs)
cx_list = [x_min + (x_max - x_min)*i/(COLS-1) for i in range(COLS)] if COLS > 1 else [x_min]
cy_list = [poly_eval(coeffs, x) for x in cx_list]
y_min2  = min(list(ys) + cy_list)
y_max2  = max(list(ys) + cy_list)

def to_col(x):
    return int((x - x_min) / (x_max - x_min) * (COLS-1)) if x_max != x_min else 0

def to_row(y):
    return int((y_max2 - y) / (y_max2 - y_min2) * (ROWS-1)) if y_max2 != y_min2 else ROWS//2

grid = [[" "]*COLS for _ in range(ROWS)]
for cx, cy in zip(cx_list, cy_list):
    r, c = to_row(cy), to_col(cx)
    if 0 <= r < ROWS and 0 <= c < COLS and grid[r][c] == " ":
        grid[r][c] = "-"
for x, y in zip(xs, ys):
    r, c = to_row(y), to_col(x)
    if 0 <= r < ROWS and 0 <= c < COLS:
        grid[r][c] = "*"

print("  Scatter (* = data, - = fit curve):")
for i, row in enumerate(grid):
    if i == 0:       lbl = f"{{y_max2:.3g}}"
    elif i == ROWS-1: lbl = f"{{y_min2:.3g}}"
    else:             lbl = ""
    print(f"  {{lbl:>8}} |{{''.join(row)}}")
print(f"  {{' ':>8}} +" + "-"*COLS)
xl, xr = f"{{x_min:.3g}}", f"{{x_max:.3g}}"
pad_w = max(0, COLS - len(xl) - len(xr))
print("  " + " "*9 + xl + " "*pad_w + xr)
print()

# ── residuals ─────────────────────────────────────────────────────────────────
resids = [y - poly_eval(coeffs, x) for x, y in zip(xs, ys)]
r_min, r_max = min(resids), max(resids)
rRs, rCs = 6, W - 6
grid_r = [[" "]*rCs for _ in range(rRs)]
rng    = max(abs(r_min), abs(r_max), 1e-12)
mid_r  = rRs // 2
for c in range(rCs):
    grid_r[mid_r][c] = "."
for x, res in zip(xs, resids):
    c = int((x - x_min)/(x_max - x_min)*(rCs-1)) if x_max != x_min else 0
    r = int((rng - res)/(2*rng)*(rRs-1))
    if 0 <= r < rRs and 0 <= c < rCs:
        grid_r[r][c] = "o"
print("  Residuals (o = data, . = zero line):")
for row in grid_r:
    print("  " + "".join(row))
print("  " + "-"*rCs)
print(f"  Range: [{{r_min:.4g}}, {{r_max:.4g}}]")
print()

# ── predictions ───────────────────────────────────────────────────────────────
if _pred_s:
    try:
        pred_xs = [float(v.strip()) for v in _pred_s.split(",")]
        print("  Predictions:")
        for px in pred_xs:
            py = poly_eval(coeffs, px)
            print(f"    x = {{px:.6g}}  =>  y = {{py:.6g}}")
        print()
    except ValueError:
        print(f"  WARNING: bad --regression-predict value: {{_pred_s}}")
        print()
print("=" * W)
"####,
        hex_path    = hex_path,
        hex_x_col   = hex_x_col,
        hex_y_col   = hex_y_col,
        hex_predict = hex_predict,
        deg         = deg,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}
