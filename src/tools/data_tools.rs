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
