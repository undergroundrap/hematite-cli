// ─── Formula library, random generation, and data diff ───────────────────────
// Compiled into scientific.rs via `mod scientific_ext` in tools/mod.rs — kept
// in a separate file purely to avoid making scientific.rs even larger.

// ── Formula library ───────────────────────────────────────────────────────────
// Tuple layout: (name, formula, category, variables)

const FORMULAS: &[(&str, &str, &str, &str)] = &[
    // Mechanics
    ("Newton's Second Law",        "F = m \u{00D7} a",                         "mechanics",      "F=force(N)  m=mass(kg)  a=acceleration(m/s\u{00B2})"),
    ("Kinematic: velocity",        "v = u + a\u{00D7}t",                        "mechanics",      "v=final velocity  u=initial velocity  a=acceleration  t=time"),
    ("Kinematic: displacement",    "s = u\u{00D7}t + \u{00BD}\u{00D7}a\u{00D7}t\u{00B2}",  "mechanics", "s=displacement  u=initial velocity  a=acceleration  t=time"),
    ("Kinematic: v\u{00B2} relation", "v\u{00B2} = u\u{00B2} + 2\u{00D7}a\u{00D7}s",   "mechanics", "v=final velocity  u=initial velocity  a=acceleration  s=displacement"),
    ("Kinetic Energy",             "KE = \u{00BD} \u{00D7} m \u{00D7} v\u{00B2}",  "mechanics",  "KE=kinetic energy(J)  m=mass(kg)  v=velocity(m/s)"),
    ("Gravitational Potential Energy", "PE = m \u{00D7} g \u{00D7} h",         "mechanics",      "PE=potential energy(J)  m=mass(kg)  g=9.80665 m/s\u{00B2}  h=height(m)"),
    ("Momentum",                   "p = m \u{00D7} v",                          "mechanics",      "p=momentum(kg\u{00B7}m/s)  m=mass(kg)  v=velocity(m/s)"),
    ("Impulse-Momentum Theorem",   "J = F \u{00D7} \u{0394}t = \u{0394}p",     "mechanics",      "J=impulse(N\u{00B7}s)  F=force(N)  \u{0394}t=time interval  \u{0394}p=change in momentum"),
    ("Work",                       "W = F \u{00D7} d \u{00D7} cos(\u{03B8})",   "mechanics",      "W=work(J)  F=force(N)  d=distance(m)  \u{03B8}=angle between F and d"),
    ("Power",                      "P = W / t = F \u{00D7} v",                  "mechanics",      "P=power(W)  W=work(J)  t=time(s)  F=force(N)  v=velocity(m/s)"),
    ("Torque",                     "\u{03C4} = r \u{00D7} F \u{00D7} sin(\u{03B8})", "mechanics", "\u{03C4}=torque(N\u{00B7}m)  r=moment arm(m)  F=force(N)  \u{03B8}=angle"),
    ("Universal Gravitation",      "F = G \u{00D7} m\u{2081} \u{00D7} m\u{2082} / r\u{00B2}", "mechanics", "G=6.674\u{00D7}10\u{207B}\u{00B9}\u{00B9} N\u{00B7}m\u{00B2}/kg\u{00B2}  m\u{2081},m\u{2082}=masses(kg)  r=distance(m)"),
    ("Centripetal Acceleration",   "a_c = v\u{00B2} / r = \u{03C9}\u{00B2} \u{00D7} r", "mechanics", "a_c=centripetal acceleration(m/s\u{00B2})  v=speed(m/s)  r=radius(m)  \u{03C9}=angular velocity(rad/s)"),

    // Waves & Optics
    ("Wave Speed",                 "v = f \u{00D7} \u{03BB}",                   "waves",          "v=wave speed(m/s)  f=frequency(Hz)  \u{03BB}=wavelength(m)"),
    ("Photon Energy",              "E = h \u{00D7} f = h\u{00D7}c / \u{03BB}",  "waves",          "h=6.626\u{00D7}10\u{207B}\u{00B3}\u{2074} J\u{00B7}s  f=frequency(Hz)  c=3\u{00D7}10\u{2078} m/s  \u{03BB}=wavelength(m)"),
    ("Doppler Effect",             "f\u{2019} = f \u{00D7} (v + v_o) / (v \u{2212} v_s)", "waves", "f\u{2019}=observed freq  f=source freq  v=wave speed  v_o=observer speed  v_s=source speed"),
    ("Snell's Law",                "n\u{2081} \u{00D7} sin(\u{03B8}\u{2081}) = n\u{2082} \u{00D7} sin(\u{03B8}\u{2082})", "waves", "n=refractive index  \u{03B8}=angle to normal"),

    // Thermodynamics
    ("Ideal Gas Law",              "P \u{00D7} V = n \u{00D7} R \u{00D7} T",   "thermodynamics", "P=pressure(Pa)  V=volume(m\u{00B3})  n=moles  R=8.314 J/(mol\u{00B7}K)  T=temperature(K)"),
    ("Heat Transfer",              "Q = m \u{00D7} c \u{00D7} \u{0394}T",       "thermodynamics", "Q=heat(J)  m=mass(kg)  c=specific heat capacity  \u{0394}T=temperature change(K)"),
    ("Carnot Efficiency",          "\u{03B7} = 1 \u{2212} T_c / T_h",           "thermodynamics", "\u{03B7}=max efficiency(0\u{2013}1)  T_c=cold reservoir(K)  T_h=hot reservoir(K)"),
    ("Stefan-Boltzmann Law",       "P = \u{03C3} \u{00D7} A \u{00D7} T\u{2074}", "thermodynamics", "\u{03C3}=5.67\u{00D7}10\u{207B}\u{2078} W/(m\u{00B2}\u{00B7}K\u{2074})  A=area(m\u{00B2})  T=temperature(K)"),

    // Electricity & Magnetism
    ("Ohm's Law",                  "V = I \u{00D7} R",                          "electricity",    "V=voltage(V)  I=current(A)  R=resistance(\u{03A9})"),
    ("Electrical Power",           "P = I \u{00D7} V = I\u{00B2} \u{00D7} R = V\u{00B2} / R", "electricity", "P=power(W)  I=current(A)  V=voltage(V)  R=resistance(\u{03A9})"),
    ("Coulomb's Law",              "F = k \u{00D7} q\u{2081} \u{00D7} q\u{2082} / r\u{00B2}", "electricity", "k=8.99\u{00D7}10\u{2079} N\u{00B7}m\u{00B2}/C\u{00B2}  q=charges(C)  r=distance(m)"),
    ("Capacitance",                "C = Q / V",                                 "electricity",    "C=capacitance(F)  Q=charge(C)  V=voltage(V)"),
    ("Energy in Capacitor",        "E = \u{00BD} \u{00D7} C \u{00D7} V\u{00B2}", "electricity",  "E=energy(J)  C=capacitance(F)  V=voltage(V)"),
    ("LC Resonant Frequency",      "f = 1 / (2\u{03C0} \u{00D7} \u{221A}(L\u{00D7}C))", "electricity", "f=frequency(Hz)  L=inductance(H)  C=capacitance(F)"),
    ("Faraday's Law",              "EMF = \u{2212}N \u{00D7} \u{0394}\u{03A6} / \u{0394}t", "electricity", "N=turns  \u{0394}\u{03A6}=flux change(Wb)  \u{0394}t=time(s)"),
    ("Magnetic Force on Current",  "F = I \u{00D7} L \u{00D7} B \u{00D7} sin(\u{03B8})", "electricity", "I=current(A)  L=length(m)  B=magnetic field(T)  \u{03B8}=angle"),

    // Special Relativity
    ("Mass-Energy Equivalence",    "E = m \u{00D7} c\u{00B2}",                 "relativity",     "E=energy(J)  m=mass(kg)  c=2.998\u{00D7}10\u{2078} m/s"),
    ("Lorentz Factor",             "\u{03B3} = 1 / \u{221A}(1 \u{2212} v\u{00B2}/c\u{00B2})", "relativity", "\u{03B3}=Lorentz factor  v=velocity  c=speed of light"),
    ("Time Dilation",              "t = t\u{2080} \u{00D7} \u{03B3}",          "relativity",     "t=dilated time  t\u{2080}=proper time  \u{03B3}=Lorentz factor"),
    ("Length Contraction",         "L = L\u{2080} / \u{03B3}",                 "relativity",     "L=contracted length  L\u{2080}=proper length  \u{03B3}=Lorentz factor"),

    // Chemistry
    ("pH Definition",              "pH = \u{2212}log\u{2081}\u{2080}[H\u{207A}]", "chemistry",  "[H\u{207A}]=hydrogen ion concentration(mol/L)"),
    ("Henderson-Hasselbalch",      "pH = pKa + log([A\u{207B}]/[HA])",         "chemistry",      "pKa=acid dissociation constant  [A\u{207B}]=conjugate base  [HA]=acid concentration"),
    ("Arrhenius Equation",         "k = A \u{00D7} e^(\u{2212}Ea/(R\u{00D7}T))", "chemistry",  "A=pre-exponential factor  Ea=activation energy(J/mol)  R=8.314  T=temperature(K)"),
    ("Nernst Equation",            "E = E\u{00B0} \u{2212} (RT)/(nF) \u{00D7} ln(Q)", "chemistry", "E\u{00B0}=standard potential  n=electrons  F=96485 C/mol  Q=reaction quotient"),
    ("Beer-Lambert Law",           "A = \u{03B5} \u{00D7} c \u{00D7} l",       "chemistry",      "A=absorbance  \u{03B5}=molar absorptivity(L/(mol\u{00B7}cm))  c=concentration(mol/L)  l=path length(cm)"),

    // Mathematics
    ("Quadratic Formula",          "x = (\u{2212}b \u{00B1} \u{221A}(b\u{00B2}\u{2212}4ac)) / (2a)", "mathematics", "coefficients of ax\u{00B2}+bx+c=0"),
    ("Pythagorean Theorem",        "a\u{00B2} + b\u{00B2} = c\u{00B2}",        "mathematics",    "a,b=legs of right triangle  c=hypotenuse"),
    ("Circle Area",                "A = \u{03C0} \u{00D7} r\u{00B2}",          "mathematics",    "A=area  r=radius"),
    ("Circle Circumference",       "C = 2 \u{00D7} \u{03C0} \u{00D7} r",      "mathematics",    "C=circumference  r=radius"),
    ("Sphere Volume",              "V = (4/3) \u{00D7} \u{03C0} \u{00D7} r\u{00B3}", "mathematics", "V=volume  r=radius"),
    ("Sphere Surface Area",        "A = 4 \u{00D7} \u{03C0} \u{00D7} r\u{00B2}", "mathematics", "A=surface area  r=radius"),
    ("Cylinder Volume",            "V = \u{03C0} \u{00D7} r\u{00B2} \u{00D7} h", "mathematics", "V=volume  r=radius  h=height"),
    ("Euler's Formula",            "e^(i\u{03B8}) = cos(\u{03B8}) + i\u{00D7}sin(\u{03B8})", "mathematics", "\u{03B8}=angle(radians)  i=imaginary unit"),
    ("Bayes' Theorem",             "P(A|B) = P(B|A) \u{00D7} P(A) / P(B)",    "mathematics",    "P(A|B)=probability of A given B"),
    ("Normal Distribution PDF",    "f(x) = (1/(\u{03C3}\u{221A}(2\u{03C0}))) \u{00D7} e^(\u{2212}(x\u{2212}\u{03BC})\u{00B2}/(2\u{03C3}\u{00B2}))", "mathematics", "\u{03BC}=mean  \u{03C3}=standard deviation"),

    // Finance
    ("Compound Interest",          "A = P \u{00D7} (1 + r/n)^(n\u{00D7}t)",   "finance",        "A=final amount  P=principal  r=annual rate(decimal)  n=compounds/year  t=years"),
    ("Simple Interest",            "I = P \u{00D7} r \u{00D7} t",              "finance",        "I=interest  P=principal  r=annual rate(decimal)  t=years"),
    ("Present Value",              "PV = FV / (1 + r)^n",                       "finance",        "PV=present value  FV=future value  r=rate per period  n=periods"),
];

pub fn search_formulas(query: &str) -> String {
    let q = query.trim();

    if q.is_empty() || q.eq_ignore_ascii_case("list") || q.eq_ignore_ascii_case("all") {
        let mut cats: std::collections::BTreeMap<&str, Vec<&&str>> = Default::default();
        for (name, _, cat, _) in FORMULAS {
            cats.entry(cat).or_default().push(name);
        }
        let mut out = format!("Formula library — {} entries\n\n", FORMULAS.len());
        for (cat, names) in &cats {
            out.push_str(&format!("{}  ({} formulas)\n", cat.to_uppercase(), names.len()));
            for n in names {
                out.push_str(&format!("  {n}\n"));
            }
            out.push('\n');
        }
        out.push_str("Search: hematite --formula \"kinetic energy\"  or  hematite --formula ohms");
        return out;
    }

    let q_lower = q.to_ascii_lowercase();
    let hits: Vec<_> = FORMULAS.iter()
        .filter(|(name, formula, cat, vars)| {
            name.to_ascii_lowercase().contains(&q_lower)
                || cat.to_ascii_lowercase().contains(&q_lower)
                || formula.to_ascii_lowercase().contains(&q_lower)
                || vars.to_ascii_lowercase().contains(&q_lower)
        })
        .collect();

    if hits.is_empty() {
        return format!(
            "No formulas found for '{q}'.\nRun  hematite --formula list  to browse all {} entries.",
            FORMULAS.len()
        );
    }

    let sep = "\u{2500}".repeat(50);
    let mut out = String::new();
    for (name, formula, cat, vars) in &hits {
        out.push_str(&format!(
            "{name}\n{sep}\n  Formula:    {formula}\n  Variables:  {vars}\n  Category:   {cat}\n\n"
        ));
    }
    out.trim_end().to_string()
}

// ── Cryptographically secure random generation ────────────────────────────────

pub async fn generate_random(
    kind: &str,
    length: usize,
    count: usize,
    extra: &str,
) -> Result<String, String> {
    let safe_kind  = kind.trim().to_ascii_lowercase().replace('"', "");
    let safe_extra: String = extra.bytes().map(|b| format!("{:02x}", b)).collect();
    let eff_count  = count.max(1).min(1000);
    let eff_length = length.max(1).min(4096);

    let script = format!(
        r####"import secrets, uuid as _uuid, sys, string, re as _re

_kind  = "{safe_kind}"
_len   = {eff_length}
_count = {eff_count}
_extra = bytes.fromhex("{safe_extra}").decode('utf-8', errors='replace').strip()

_TYPES = "uuid  password  token  hex  urlsafe  pin  bytes  int  dice"

for _i in range(_count):
    try:
        if _kind == "uuid":
            print(str(_uuid.uuid4()))
        elif _kind in ("password", "pwd", "pass"):
            _cs = _extra if _extra else string.ascii_letters + string.digits + "!@#$%^&*-_=+"
            print(''.join(secrets.choice(_cs) for _ in range(_len)))
        elif _kind in ("token", "hex"):
            nb = max(1, _len // 2)
            print(secrets.token_hex(nb))
        elif _kind in ("urlsafe", "url"):
            print(secrets.token_urlsafe(_len))
        elif _kind == "pin":
            print(''.join(secrets.choice(string.digits) for _ in range(_len)))
        elif _kind == "bytes":
            print(secrets.token_bytes(_len).hex())
        elif _kind in ("int", "integer", "number"):
            _parts = _extra.split()
            _lo = int(_parts[0]) if len(_parts) > 0 else 1
            _hi = int(_parts[1]) if len(_parts) > 1 else 100
            if _lo > _hi: _lo, _hi = _hi, _lo
            print(secrets.randbelow(_hi - _lo + 1) + _lo)
        elif _kind == "dice":
            _notation = _extra or "1d6"
            _dm = _re.match(r'^(\d*)d(\d+)([+-]\d+)?$', _notation, _re.I)
            if not _dm:
                print("Dice format: NdN or NdN+M (e.g. 2d6  d20  3d8+2)", file=sys.stderr)
                sys.exit(1)
            _nd  = int(_dm.group(1) or 1)
            _ds  = int(_dm.group(2))
            _mod = int(_dm.group(3) or 0)
            _rolls = [secrets.randbelow(_ds) + 1 for _ in range(_nd)]
            _total = sum(_rolls) + _mod
            if _nd > 1 or _mod:
                _breakdown = " + ".join(str(r) for r in _rolls)
                _suffix = ("  +" + str(_mod)) if _mod > 0 else (("  " + str(_mod)) if _mod < 0 else "")
                print(str(_total) + "  [" + _breakdown + _suffix + "]")
            else:
                print(str(_total))
        else:
            print("Unknown type: " + _kind + ".  Supported: " + _TYPES, file=sys.stderr)
            sys.exit(1)
    except Exception as _e:
        print("Error: " + str(_e), file=sys.stderr); sys.exit(1)
"####,
        safe_kind  = safe_kind,
        safe_extra = safe_extra,
        eff_length = eff_length,
        eff_count  = eff_count,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 10
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Row-level data diff ───────────────────────────────────────────────────────

pub async fn diff_data(path_a: &str, path_b: &str, key_col: &str) -> Result<String, String> {
    let hex_a:   String = path_a.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_b:   String = path_b.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_key: String = key_col.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(
        r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys

_pa  = bytes.fromhex("{hex_a}").decode()
_pb  = bytes.fromhex("{hex_b}").decode()
_key = bytes.fromhex("{hex_key}").decode().strip()

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv', 'tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as _fh:
            _r = _csv.DictReader(_fh, delimiter='\t' if ext == 'tsv' else ',')
            return list(_r)
    elif ext == 'json':
        with open(path, encoding='utf-8') as _fh:
            _d = _js.load(_fh)
        return _d if isinstance(_d, list) else next(iter(_d.values()), [])
    elif ext in ('db', 'sqlite', 'sqlite3'):
        _con = _sq.connect(path)
        _cur = _con.cursor()
        _cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        _t = _cur.fetchone()
        if not _t: return []
        _cur.execute("SELECT * FROM [%s]" % _t[0])
        _cols2 = [_d[0] for _d in _cur.description]
        _rows2 = [dict(zip(_cols2, _r)) for _r in _cur.fetchall()]
        _con.close()
        return _rows2
    else:
        print("Unsupported format: " + ext, file=sys.stderr); sys.exit(1)

_ra = _load(_pa)
_rb = _load(_pb)

if not _ra and not _rb:
    print("Both files are empty."); sys.exit(0)

_cols = list((_ra[0] if _ra else _rb[0]).keys())
_kc   = _key if _key else _cols[0]

def _idx(rows):
    d = {{}}
    for r in rows:
        k = str(r.get(_kc, ''))
        d.setdefault(k, []).append(r)
    return d

_da = _idx(_ra)
_db = _idx(_rb)
_ka = set(_da); _kb = set(_db)

_added    = sorted(_kb - _ka)
_removed  = sorted(_ka - _kb)
_common   = sorted(_ka & _kb)

_modified = []
for _k in _common:
    _r1 = _da[_k][0]; _r2 = _db[_k][0]
    _diffs = {{c: (str(_r1.get(c,'')), str(_r2.get(c,'')))
              for c in _cols if str(_r1.get(c,'')) != str(_r2.get(c,'')) and c != _kc}}
    if _diffs:
        _modified.append((_k, _diffs))

_sep = "─" * 52
print(_sep)
print("Data diff:  A = %s" % os.path.basename(_pa))
print("            B = %s" % os.path.basename(_pb))
print("Key column: %s  |  A: %d rows  B: %d rows" % (_kc, len(_ra), len(_rb)))
print(_sep)
print()

if not _added and not _removed and not _modified:
    print("✓ No differences found.  Files are identical on key column '%s'." % _kc)
    sys.exit(0)

def _preview(row, n=4):
    items = list(row.items())[:n]
    return "  ".join(("%s=%s" % (k, str(v)[:20])) for k, v in items if k != _kc)

if _added:
    print("+ Added (%d rows in B not in A):" % len(_added))
    for _k in _added[:25]:
        print("  + %s  %s" % (_k, _preview(_db[_k][0])))
    if len(_added) > 25: print("  ... and %d more" % (len(_added)-25))
    print()

if _removed:
    print("- Removed (%d rows in A not in B):" % len(_removed))
    for _k in _removed[:25]:
        print("  - %s  %s" % (_k, _preview(_da[_k][0])))
    if len(_removed) > 25: print("  ... and %d more" % (len(_removed)-25))
    print()

if _modified:
    print("~ Modified (%d rows with changed values):" % len(_modified))
    for _k, _diffs in _modified[:25]:
        print("  ~ %s" % _k)
        for _c, (_va, _vb) in _diffs.items():
            print("      %-20s %s  →  %s" % (_c + ":", _va[:40], _vb[:40]))
    if len(_modified) > 25: print("  ... and %d more" % (len(_modified)-25))
    print()

print("Summary:  +%d added  -%d removed  ~%d modified" % (len(_added), len(_removed), len(_modified)))
"####,
        hex_a   = hex_a,
        hex_b   = hex_b,
        hex_key = hex_key,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Column statistics ─────────────────────────────────────────────────────────

pub async fn column_stats(file_path: &str, column: &str) -> Result<String, String> {
    let hex_path: String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_col:  String = column.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math

_path = bytes.fromhex("{hex_path}").decode()
_col  = bytes.fromhex("{hex_col}").decode().strip()

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as _fh:
            _r = _csv.DictReader(_fh, delimiter='\t' if ext=='tsv' else ',')
            return list(_r), None
    elif ext == 'json':
        with open(path, encoding='utf-8') as _fh: _d = _js.load(_fh)
        rows = _d if isinstance(_d, list) else next(iter(_d.values()), [])
        return rows, None
    elif ext in ('db','sqlite','sqlite3'):
        _con = _sq.connect(path)
        _cur = _con.cursor()
        _cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        _t = _cur.fetchone()
        if not _t: return [], None
        _cur.execute("SELECT * FROM [%s]" % _t[0])
        _cols2 = [_d[0] for _d in _cur.description]
        _rows2 = [dict(zip(_cols2, _r)) for _r in _cur.fetchall()]
        _con.close()
        return _rows2, None
    else:
        return None, "Unsupported format: " + ext

def _try_float(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

def _stats_for(nums, label):
    n = len(nums)
    if n == 0:
        print("  %s: no numeric values found" % label); return
    nums_s = sorted(nums)
    mean   = sum(nums_s) / n
    def _pct(p):
        idx = p * (n - 1) / 100
        lo = int(idx); hi = min(lo+1, n-1)
        return nums_s[lo] + (idx - lo) * (nums_s[hi] - nums_s[lo])
    median = _pct(50)
    q1     = _pct(25)
    q3     = _pct(75)
    iqr    = q3 - q1
    var    = sum((x - mean)**2 for x in nums_s) / n
    std    = math.sqrt(var)
    # Population skewness
    if std > 0:
        skew = (sum((x-mean)**3 for x in nums_s)/n) / (std**3)
    else:
        skew = 0.0
    # Mode (most common value, rounded to 4 sig figs)
    from collections import Counter
    rounded = [round(x, 4) for x in nums_s]
    mode_val, mode_cnt = Counter(rounded).most_common(1)[0]

    W = 52
    print("=" * W)
    print(" Statistics: %s  (n=%d)" % (label, n))
    print("-" * W)
    print("  Min       %g" % nums_s[0])
    print("  Max       %g" % nums_s[-1])
    print("  Range     %g" % (nums_s[-1] - nums_s[0]))
    print("  Mean      %g" % mean)
    print("  Median    %g" % median)
    print("  Mode      %g  (count: %d)" % (mode_val, mode_cnt))
    print("  Std Dev   %g" % std)
    print("  Variance  %g" % var)
    print("  Q1        %g" % q1)
    print("  Q3        %g" % q3)
    print("  IQR       %g" % iqr)
    print("  Skewness  %.4f" % skew)
    # ASCII histogram (10 bins)
    bins = 10
    lo = nums_s[0]; hi_v = nums_s[-1]
    if lo == hi_v: hi_v = lo + 1
    step = (hi_v - lo) / bins
    counts = [0]*bins
    for x in nums_s:
        idx = min(int((x-lo)/step), bins-1)
        counts[idx] += 1
    max_c = max(counts) if max(counts) > 0 else 1
    bar_w = 30
    print("-" * W)
    print("  Histogram (%d bins):" % bins)
    for i in range(bins):
        lo_b = lo + i*step; hi_b = lo_b + step
        bar = "#" * int(counts[i] / max_c * bar_w)
        print("  [%8.3g,%8.3g)  %-30s %d" % (lo_b, hi_b, bar, counts[i]))
    print("=" * W)

rows, err = _load(_path)
if err:
    print("Error:", err, file=sys.stderr); sys.exit(1)
if not rows:
    print("No rows found in %s" % _path); sys.exit(0)

all_cols = list(rows[0].keys())
if _col:
    if _col not in all_cols:
        print("Column '%s' not found. Available: %s" % (_col, ', '.join(all_cols)))
        sys.exit(1)
    nums = [v for v in (_try_float(r.get(_col,'')) for r in rows) if v is not None]
    _stats_for(nums, _col)
else:
    numeric_cols = [c for c in all_cols if sum(1 for r in rows if _try_float(r.get(c,'')) is not None) > len(rows)*0.5]
    if not numeric_cols:
        print("No numeric columns detected. Columns: %s" % ', '.join(all_cols))
        sys.exit(0)
    print("Numeric columns: %s" % ', '.join(numeric_cols))
    print("Pass --column NAME to focus on one, or showing all:\n")
    for c in numeric_cols:
        nums = [v for v in (_try_float(r.get(c,'')) for r in rows) if v is not None]
        _stats_for(nums, c)
        print()
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

// ── Matrix operations ─────────────────────────────────────────────────────────
// Supports: det, inv, solve, multiply, transpose, eigenvalues
// Matrix input format: "[[1,2],[3,4]]" (JSON array of rows)

pub async fn matrix_op(op: &str, matrix_a: &str, matrix_b: &str) -> Result<String, String> {
    let hex_op: String = op.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_a:  String = matrix_a.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_b:  String = matrix_b.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import json as _js, math, sys

_op = bytes.fromhex("{hex_op}").decode().strip().lower()
_sa = bytes.fromhex("{hex_a}").decode().strip()
_sb = bytes.fromhex("{hex_b}").decode().strip()

def _parse(s):
    if not s: return None
    try:
        d = _js.loads(s)
        if isinstance(d, list):
            if isinstance(d[0], list): return [list(map(float,r)) for r in d]
            return [[float(x)] for x in d]  # column vector
        return None
    except Exception as e:
        print("Parse error:", e, file=sys.stderr); sys.exit(1)

A = _parse(_sa)
B = _parse(_sb)

def _rows(M): return len(M)
def _cols(M): return len(M[0]) if M else 0
def _shape(M): return "(%d×%d)" % (_rows(M), _cols(M))

def _mul(A, B):
    r,k,c = _rows(A), _cols(A), _cols(B)
    if k != _rows(B): raise ValueError("Shape mismatch: A%s B%s" % (_shape(A), _shape(B)))
    return [[sum(A[i][p]*B[p][j] for p in range(k)) for j in range(c)] for i in range(r)]

def _T(M):
    return [[M[i][j] for i in range(_rows(M))] for j in range(_cols(M))]

def _LU(M):
    n = _rows(M)
    if n != _cols(M): raise ValueError("Square matrix required")
    U = [row[:] for row in M]
    L = [[1.0 if i==j else 0.0 for j in range(n)] for i in range(n)]
    P = list(range(n)); sign = 1
    for col in range(n):
        pivot = max(range(col, n), key=lambda r: abs(U[r][col]))
        if abs(U[pivot][col]) < 1e-14: raise ValueError("Matrix is singular")
        if pivot != col:
            U[col], U[pivot] = U[pivot], U[col]
            P[col], P[pivot] = P[pivot], P[col]
            if col > 0:
                for k in range(col): L[col][k], L[pivot][k] = L[pivot][k], L[col][k]
            sign *= -1
        for row in range(col+1, n):
            f = U[row][col] / U[col][col]
            L[row][col] = f
            for k in range(col, n): U[row][k] -= f * U[col][k]
    return L, U, P, sign

def _det(M):
    L, U, P, sign = _LU(M)
    d = sign
    for i in range(_rows(M)): d *= U[i][i]
    return d

def _inv(M):
    n = _rows(M)
    L, U, P, _ = _LU(M)
    inv = [[0.0]*n for _ in range(n)]
    for col in range(n):
        e = [1.0 if P[i]==col else 0.0 for i in range(n)]
        y = [0.0]*n
        for i in range(n):
            y[i] = e[i] - sum(L[i][k]*y[k] for k in range(i))
        x = [0.0]*n
        for i in range(n-1, -1, -1):
            x[i] = (y[i] - sum(U[i][k]*x[k] for k in range(i+1,n))) / U[i][i]
        for i in range(n): inv[i][col] = x[i]
    return inv

def _solve(M, b):
    n = _rows(M)
    bv = [row[0] for row in b]
    L, U, P, _ = _LU(M)
    pb = [bv[P[i]] for i in range(n)]
    y = [0.0]*n
    for i in range(n):
        y[i] = pb[i] - sum(L[i][k]*y[k] for k in range(i))
    x = [0.0]*n
    for i in range(n-1,-1,-1):
        x[i] = (y[i] - sum(U[i][k]*x[k] for k in range(i+1,n))) / U[i][i]
    return x

def _fmt_num(v):
    if abs(v) < 1e-10: return "0"
    return ("%.6g" % v)

def _print_matrix(M, label=""):
    if label: print(label)
    for row in M:
        print("  [ " + "  ".join("%10s" % _fmt_num(v) for v in row) + " ]")

def _qr_eigenvalues(M, iters=200):
    # QR iteration (Gram-Schmidt) — real eigenvalues only for symmetric matrices
    n = _rows(M)
    Ak = [row[:] for row in M]
    for _ in range(iters):
        # Gram-Schmidt QR
        Q = [[0.0]*n for _ in range(n)]
        R = [[0.0]*n for _ in range(n)]
        for j in range(n):
            v = [Ak[i][j] for i in range(n)]
            for k in range(j):
                R[k][j] = sum(Q[i][k]*v[i] for i in range(n))
                for i in range(n): v[i] -= R[k][j]*Q[i][k]
            norm = math.sqrt(sum(x*x for x in v))
            if norm < 1e-14: norm = 1.0
            R[j][j] = norm
            for i in range(n): Q[i][j] = v[i]/norm
        Ak = _mul(R, Q)
    return sorted([Ak[i][i] for i in range(n)], reverse=True)

if not A:
    print("Error: no matrix provided. Use JSON format: [[1,2],[3,4]]")
    sys.exit(1)

try:
    if _op in ('det', 'determinant'):
        d = _det(A)
        print("Determinant of %s matrix:" % _shape(A))
        print("  det(A) = %s" % _fmt_num(d))
    elif _op in ('inv', 'inverse', 'invert'):
        Ai = _inv(A)
        _print_matrix(A, "A %s:" % _shape(A))
        print()
        _print_matrix(Ai, "A⁻¹:")
    elif _op in ('T', 'transpose'):
        At = _T(A)
        _print_matrix(A, "A %s:" % _shape(A))
        print()
        _print_matrix(At, "Aᵀ %s:" % _shape(At))
    elif _op in ('mul', 'multiply', 'matmul'):
        if not B:
            print("Error: --matrix-b required for multiply"); sys.exit(1)
        C = _mul(A, B)
        _print_matrix(A, "A %s:" % _shape(A))
        print()
        _print_matrix(B, "B %s:" % _shape(B))
        print()
        _print_matrix(C, "A × B = %s:" % _shape(C))
    elif _op in ('solve',):
        if not B:
            print("Error: --matrix-b required for solve (the b vector/matrix)"); sys.exit(1)
        x = _solve(A, B)
        _print_matrix(A, "A %s (coefficient matrix):" % _shape(A))
        print()
        _print_matrix(B, "b %s (right-hand side):" % _shape(B))
        print()
        print("Solution x (Ax = b):")
        print("  [ " + "  ".join("%10s" % _fmt_num(v) for v in x) + " ]")
    elif _op in ('eig', 'eigen', 'eigenvalues'):
        eigs = _qr_eigenvalues(A)
        print("Eigenvalues of %s matrix (real, via QR iteration):" % _shape(A))
        for i, e in enumerate(eigs):
            print("  λ%d = %s" % (i+1, _fmt_num(e)))
        print()
        print("Note: QR iteration gives real eigenvalues for symmetric matrices.")
        print("Complex eigenvalues are shown only as real parts for non-symmetric matrices.")
    elif _op in ('rank',):
        # Gaussian elimination row rank
        M2 = [row[:] for row in A]
        r = 0
        for col in range(_cols(M2)):
            pivot = next((i for i in range(r, _rows(M2)) if abs(M2[i][col]) > 1e-10), None)
            if pivot is None: continue
            M2[r], M2[pivot] = M2[pivot], M2[r]
            for i in range(_rows(M2)):
                if i != r and abs(M2[i][col]) > 1e-10:
                    f = M2[i][col] / M2[r][col]
                    for k in range(_cols(M2)): M2[i][k] -= f * M2[r][k]
            r += 1
        print("Rank of %s matrix: %d" % (_shape(A), r))
    elif _op in ('trace',):
        if _rows(A) != _cols(A): print("Warning: trace of non-square matrix (using diagonal)")
        t = sum(A[i][i] for i in range(min(_rows(A),_cols(A))))
        print("Trace of %s matrix: %s" % (_shape(A), _fmt_num(t)))
    else:
        print("Unknown operation: '%s'" % _op)
        print("Available: det  inv  transpose  multiply  solve  eigenvalues  rank  trace")
        sys.exit(1)
except ValueError as e:
    print("Error:", e, file=sys.stderr); sys.exit(1)
"####,
        hex_op = hex_op,
        hex_a  = hex_a,
        hex_b  = hex_b,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Equation solver ───────────────────────────────────────────────────────────

pub async fn solve_equation(equation: &str, var: &str, x0: f64, x1: f64) -> Result<String, String> {
    let hex_eq:  String = equation.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_var: String = var.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import math, sys
from math import (sin,cos,tan,asin,acos,atan,atan2,sinh,cosh,tanh,
                  sqrt,log,log2,log10,exp,floor,ceil,pi,e,inf,nan)

_eq  = bytes.fromhex("{hex_eq}").decode().strip()
_var = bytes.fromhex("{hex_var}").decode().strip() or "x"
_x0  = {x0}
_x1  = {x1}

if '=' in _eq:
    parts = _eq.split('=', 1)
    _expr = "(%s) - (%s)" % (parts[0].strip(), parts[1].strip())
else:
    _expr = _eq
_expr = _expr.replace('^', '**')

_safe = dict(sin=sin,cos=cos,tan=tan,asin=asin,acos=acos,atan=atan,atan2=atan2,
             sinh=sinh,cosh=cosh,tanh=tanh,sqrt=sqrt,log=log,log2=log2,log10=log10,
             exp=exp,floor=floor,ceil=ceil,pi=pi,e=e,inf=inf,nan=nan,abs=abs)

def _f(xv):
    ns = dict(_safe); ns[_var] = xv
    return eval(_expr, {{"__builtins__": {{}}}}, ns)

def _bisect(lo, hi, tol=1e-12, iters=200):
    try: flo = _f(lo); fhi = _f(hi)
    except Exception as err: return None, str(err)
    if flo * fhi > 0: return None, "no sign change"
    for _ in range(iters):
        mid = (lo+hi)/2
        if (hi-lo) < tol: return mid, None
        try: fm = _f(mid)
        except: return None, "eval error"
        if flo*fm <= 0: hi=mid; fhi=fm
        else: lo=mid; flo=fm
    return (lo+hi)/2, None

def _newton(x, tol=1e-12, iters=100):
    for _ in range(iters):
        try: fx = _f(x)
        except: break
        if abs(fx) < tol: return x, None
        h = max(abs(x)*1e-7, 1e-9)
        try: fpx = (_f(x+h) - _f(x-h))/(2*h)
        except: break
        if abs(fpx) < 1e-30: break
        x2 = x - fx/fpx
        if abs(x2-x) < tol: return x2, None
        x = x2
    return None, "did not converge"

print("Equation: %s = 0" % _expr)
print("Variable: %s  |  Search: [%g, %g]" % (_var, _x0, _x1))
print()

candidates = []
n_scan = 50; step = (_x1-_x0)/n_scan
for i in range(n_scan):
    lo = _x0+i*step; hi = lo+step
    try:
        if _f(lo)*_f(hi) <= 0:
            root, _ = _bisect(lo, hi)
            if root is not None:
                nr, _ = _newton(root)
                if nr is not None: root = nr
                if not any(abs(root-c) < 1e-8 for c in candidates):
                    candidates.append(root)
    except: pass

if not candidates:
    nr, _ = _newton((_x0+_x1)/2)
    if nr is not None: candidates.append(nr)

if not candidates:
    print("No roots found in [%g, %g]." % (_x0, _x1))
    print("Try --solve-range to widen the search interval.")
    sys.exit(0)

print("Root(s) found:")
for r in sorted(candidates):
    try: chk = abs(_f(r))
    except: chk = float('nan')
    flag = "" if chk < 1e-8 else "  [residual: %.2e]" % chk
    print("  %s = %.10g%s" % (_var, r, flag))
"####,
        hex_eq  = hex_eq,
        hex_var = hex_var,
        x0      = x0,
        x1      = x1,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 20
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Curve fitting ─────────────────────────────────────────────────────────────

pub async fn curve_fit(
    file_path: &str,
    x_col: &str,
    y_col: &str,
    model: &str,
) -> Result<String, String> {
    let hex_path:  String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_xcol:  String = x_col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_ycol:  String = y_col.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_model: String = model.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math

_path  = bytes.fromhex("{hex_path}").decode().strip()
_xcol  = bytes.fromhex("{hex_xcol}").decode().strip()
_ycol  = bytes.fromhex("{hex_ycol}").decode().strip()
_model = bytes.fromhex("{hex_model}").decode().strip().lower() or "auto"

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
numeric  = [c for c in all_cols if sum(1 for r in rows if _tf(r.get(c,'')) is not None) > len(rows)*0.5]

if not _xcol: _xcol = numeric[0] if numeric else all_cols[0]
if not _ycol: _ycol = numeric[1] if len(numeric)>1 else all_cols[1]

pairs = [(_tf(r.get(_xcol,'')), _tf(r.get(_ycol,''))) for r in rows]
pairs = [(x,y) for x,y in pairs if x is not None and y is not None]
if len(pairs) < 3:
    print("Need at least 3 numeric rows. Found: %d" % len(pairs)); sys.exit(0)

xs = [p[0] for p in pairs]; ys = [p[1] for p in pairs]; n = len(xs)

def _dot(a,b): return sum(x*y for x,y in zip(a,b))
def _mean(v):  return sum(v)/len(v)
def _r2(pred):
    ybar = _mean(ys)
    ss_res = sum((y-p)**2 for y,p in zip(ys,pred))
    ss_tot = sum((y-ybar)**2 for y in ys)
    return 1.0 - ss_res/ss_tot if ss_tot else 0.0

def _vfit(deg):
    m = deg+1
    X = [[xi**j for j in range(m)] for xi in xs]
    Xt = [[X[i][j] for i in range(n)] for j in range(m)]
    XtX = [[sum(Xt[r][k]*X[k][c] for k in range(n)) for c in range(m)] for r in range(m)]
    Xty = [sum(Xt[r][k]*ys[k] for k in range(n)) for r in range(m)]
    A = [row[:]+[Xty[i]] for i,row in enumerate(XtX)]
    for col in range(m):
        pv = max(range(col,m), key=lambda r: abs(A[r][col]))
        A[col],A[pv] = A[pv],A[col]
        if abs(A[col][col]) < 1e-30: raise ValueError("Singular")
        f = A[col][col]; A[col] = [v/f for v in A[col]]
        for r in range(m):
            if r!=col:
                mu = A[r][col]; A[r] = [A[r][k]-mu*A[col][k] for k in range(m+1)]
    c = [A[i][m] for i in range(m)]
    return c, _r2([sum(c[j]*xi**j for j in range(m)) for xi in xs])

def _fp(c):
    t = []
    for j in range(len(c)-1,-1,-1):
        v = c[j]
        if abs(v)<1e-14: continue
        if j==0: t.append("%.6g"%v)
        elif j==1: t.append("%.6g*x"%v)
        else: t.append("%.6g*x^%d"%(v,j))
    return " + ".join(t) or "0"

res = {{}}
def _try(nm,fn):
    try: eq,r2=fn(); res[nm]=(r2,eq)
    except Exception as err: res[nm]=(None,str(err))

def _lin():
    c,r2=_vfit(1); return "y = %.6g + %.6g*x"%(c[0],c[1]),r2
def _q2():
    c,r2=_vfit(2); return "y = "+_fp(c),r2
def _q3():
    c,r2=_vfit(3); return "y = "+_fp(c),r2
def _ef():
    if any(y<=0 for y in ys): raise ValueError("y must be >0")
    lny=[math.log(y) for y in ys]; xm=_mean(xs); lym=_mean(lny)
    b=(_dot(xs,lny)-n*xm*lym)/(_dot(xs,xs)-n*xm**2); a=math.exp(lym-b*xm)
    return "y = %.6g*e^(%.6g*x)"%(a,b),_r2([a*math.exp(b*xi) for xi in xs])
def _pf():
    if any(x<=0 for x in xs) or any(y<=0 for y in ys): raise ValueError("x,y must be >0")
    lx=[math.log(x) for x in xs]; ly=[math.log(y) for y in ys]
    lxm=_mean(lx); lym=_mean(ly)
    b=(_dot(lx,ly)-n*lxm*lym)/(_dot(lx,lx)-n*lxm**2); a=math.exp(lym-b*lxm)
    return "y = %.6g*x^%.6g"%(a,b),_r2([a*(xi**b) for xi in xs])
def _lf():
    if any(x<=0 for x in xs): raise ValueError("x must be >0")
    lx=[math.log(x) for x in xs]; lxm=_mean(lx); ym=_mean(ys)
    b=(_dot(lx,ys)-n*lxm*ym)/(_dot(lx,lx)-n*lxm**2); a=ym-b*lxm
    return "y = %.6g + %.6g*ln(x)"%(a,b),_r2([a+b*lxi for lxi in lx])

mm = dict(linear=_lin,lin=_lin,poly2=_q2,quadratic=_q2,quad=_q2,poly3=_q3,cubic=_q3,
          exp=_ef,exponential=_ef,power=_pf,pow=_pf,log=_lf,logarithmic=_lf)

if _model in ('auto','all'):
    for nm,fn in [('linear',_lin),('poly2',_q2),('poly3',_q3),('exp',_ef),('power',_pf),('log',_lf)]:
        _try(nm,fn)
elif _model in mm:
    _try(_model, mm[_model])
else:
    print("Unknown model '%s'. Available: linear  poly2  poly3  exp  power  log  auto" % _model); sys.exit(1)

W=60
print("="*W)
print(" Curve Fit:  %s -> %s  (n=%d)" % (_xcol,_ycol,n))
print("-"*W)
valid=[(nm,(r2,eq)) for nm,(r2,eq) in res.items() if r2 is not None]
invalid=[(nm,(r2,eq)) for nm,(r2,eq) in res.items() if r2 is None]
valid.sort(key=lambda t:-t[1][0])
best=valid[0][0] if valid else None
for nm,(r2,eq) in valid:
    star=" *" if nm==best else ""
    print("  %-12s R2=%.4f%s"%(nm,r2,star))
    print("    %s"%eq)
if invalid:
    print(); print("  Skipped (domain):")
    for nm,(_,err) in invalid:
        print("    %-12s %s"%(nm+":",err))
print("="*W)
"####,
        hex_path  = hex_path,
        hex_xcol  = hex_xcol,
        hex_ycol  = hex_ycol,
        hex_model = hex_model,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Numerical integration ─────────────────────────────────────────────────────

pub async fn integrate(expr: &str, var: &str, lo: f64, hi: f64, n: usize) -> Result<String, String> {
    let hex_expr: String = expr.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_var:  String = var.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import math, sys
from math import (sin,cos,tan,asin,acos,atan,atan2,sinh,cosh,tanh,
                  sqrt,log,log2,log10,exp,floor,ceil,pi,e,inf,nan)

_expr = bytes.fromhex("{hex_expr}").decode().strip().replace('^','**')
_var  = bytes.fromhex("{hex_var}").decode().strip() or "x"
_lo   = {lo}
_hi   = {hi}
_n    = {n}
if _n < 2: _n = 1000

_safe = dict(sin=sin,cos=cos,tan=tan,asin=asin,acos=acos,atan=atan,atan2=atan2,
             sinh=sinh,cosh=cosh,tanh=tanh,sqrt=sqrt,log=log,log2=log2,log10=log10,
             exp=exp,floor=floor,ceil=ceil,pi=pi,e=e,inf=inf,nan=nan,abs=abs)

def _f(v):
    ns = dict(_safe); ns[_var] = v
    return eval(_expr, {{"__builtins__":{{}}}}, ns)

# Adaptive Simpson's rule (recursive)
def _simp(a, b, fa, fm, fb, tol, depth):
    m1 = (a+b)/4; m2 = 3*(a+b)/4
    fm1 = _f(m1); fm2 = _f(m2)
    s1 = (b-a)/12*(fa + 4*fm1 + 2*fm + 4*fm2 + fb)
    s0 = (b-a)/6*(fa + 4*fm + fb)
    err = abs(s1 - s0)/15
    if depth >= 12 or err < tol:
        return s1 + (s1-s0)/15
    else:
        mid = (a+b)/2
        return (_simp(a,mid,fa,fm1,fm,tol/2,depth+1) +
                _simp(mid,b,fm,fm2,fb,tol/2,depth+1))

# Also compute via Simpson's 1/3 rule with _n intervals for comparison
h = (_hi - _lo) / _n
vals = [_f(_lo + i*h) for i in range(_n+1)]
simp_basic = h/3 * sum(
    (vals[i] + 4*vals[i+1] + vals[i+2]) if (i+2 <= _n) else 0
    for i in range(0, _n, 2)
)

try:
    fa = _f(_lo); fm = _f((_lo+_hi)/2); fb = _f(_hi)
    result = _simp(_lo, _hi, fa, fm, fb, 1e-10, 0)
    method = "Adaptive Simpson"
except RecursionError:
    result = simp_basic
    method = "Simpson 1/3 (%d intervals)" % _n

print("Integral of  %s  d%s" % (_expr, _var))
print("From %g  to  %g" % (_lo, _hi))
print()
print("Result:  %.10g  (via %s)" % (result, method))
print()
# Relative error estimate vs basic Simpson
if method.startswith("Adaptive"):
    err_est = abs(result - simp_basic)
    print("Est. error: %.2e  (vs basic Simpson/%d)" % (err_est, _n))
"####,
        hex_expr = hex_expr,
        hex_var  = hex_var,
        lo       = lo,
        hi       = hi,
        n        = n,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 20
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Numerical derivative ──────────────────────────────────────────────────────

pub async fn differentiate(expr: &str, var: &str, at: f64, order: u8) -> Result<String, String> {
    let hex_expr: String = expr.bytes().map(|b| format!("{:02x}", b)).collect();
    let hex_var:  String = var.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import math, sys
from math import (sin,cos,tan,asin,acos,atan,atan2,sinh,cosh,tanh,
                  sqrt,log,log2,log10,exp,floor,ceil,pi,e,inf,nan)

_expr  = bytes.fromhex("{hex_expr}").decode().strip().replace('^','**')
_var   = bytes.fromhex("{hex_var}").decode().strip() or "x"
_at    = {at}
_order = {order}

_safe = dict(sin=sin,cos=cos,tan=tan,asin=asin,acos=acos,atan=atan,atan2=atan2,
             sinh=sinh,cosh=cosh,tanh=tanh,sqrt=sqrt,log=log,log2=log2,log10=log10,
             exp=exp,floor=floor,ceil=ceil,pi=pi,e=e,inf=inf,nan=nan,abs=abs)

def _f(v):
    ns = dict(_safe); ns[_var] = v
    return eval(_expr, {{"__builtins__":{{}}}}, ns)

def _deriv(f, x, h=None, order=1):
    if h is None: h = max(abs(x)*1e-5, 1e-7)
    if order == 1:
        # 5-point stencil: (-f(x+2h)+8f(x+h)-8f(x-h)+f(x-2h)) / 12h
        return (-f(x+2*h) + 8*f(x+h) - 8*f(x-h) + f(x-2*h)) / (12*h)
    elif order == 2:
        return (f(x+h) - 2*f(x) + f(x-h)) / (h*h)
    elif order == 3:
        return (-f(x+2*h) + 2*f(x+h) - 2*f(x-h) + f(x-2*h)) / (2*h**3)
    elif order == 4:
        return (f(x+2*h) - 4*f(x+h) + 6*f(x) - 4*f(x-h) + f(x-2*h)) / (h**4)
    else:
        # Higher orders: repeated difference quotient
        d = [f(x+i*h) for i in range(-order, order+1)]
        return sum((-1)**(order-i)*math.comb(order,i)*d[i] for i in range(order+1)) / (h**order)

ordinal = ["","1st","2nd","3rd","4th","5th","6th","7th","8th"]
label = ordinal[_order] if _order < len(ordinal) else "%dth"%_order

print("f(%s) = %s" % (_var, _expr))
print("%s derivative  at  %s = %g" % (label, _var, _at))
print()

try:
    fval = _f(_at)
    dval = _deriv(_f, _at, order=_order)
    print("f(%g)        = %.10g" % (_at, fval))
    print("f'(%g) [%s] = %.10g" % (_at, label, dval))
    if _order == 1:
        slope = dval
        print()
        print("Tangent line at %s=%g:  y = %.6g + %.6g*(%s - %g)" % (_var,_at,fval,slope,_var,_at))
except Exception as ex:
    print("Error:", ex, file=sys.stderr); sys.exit(1)
"####,
        hex_expr = hex_expr,
        hex_var  = hex_var,
        at       = at,
        order    = order,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 15
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}

// ── Data profile / summarize ──────────────────────────────────────────────────
// Model-free natural language summary of a data file: column types, ranges,
// missing values, outliers, duplicate rows.

pub async fn data_profile(file_path: &str) -> Result<String, String> {
    let hex_path: String = file_path.bytes().map(|b| format!("{:02x}", b)).collect();

    let script = format!(r####"import csv as _csv, json as _js, sqlite3 as _sq, os, sys, math

_path = bytes.fromhex("{hex_path}").decode().strip()

def _load(path):
    ext = os.path.splitext(path)[1].lower().lstrip('.')
    if ext in ('csv','tsv'):
        with open(path, encoding='utf-8-sig', errors='replace', newline='') as fh:
            r = _csv.DictReader(fh, delimiter='\t' if ext=='tsv' else ',')
            return list(r), None
    elif ext == 'json':
        with open(path, encoding='utf-8') as fh: d = _js.load(fh)
        rows = d if isinstance(d, list) else next(iter(d.values()), [])
        return rows, None
    elif ext in ('db','sqlite','sqlite3'):
        con = _sq.connect(path)
        cur = con.cursor()
        cur.execute("SELECT name FROM sqlite_master WHERE type='table' LIMIT 1")
        t = cur.fetchone()
        if not t: return [], None
        cur.execute("SELECT * FROM [%s]" % t[0])
        cols2 = [d[0] for d in cur.description]
        rows2 = [dict(zip(cols2, r)) for r in cur.fetchall()]
        con.close()
        return rows2, None
    return None, "Unsupported format: "+ext

rows, err = _load(_path)
if err: print("Error:", err, file=sys.stderr); sys.exit(1)
if not rows: print("No rows found."); sys.exit(0)

n = len(rows)
cols = list(rows[0].keys())
W = 64

print("=" * W)
print(" Data Profile:  %s" % os.path.basename(_path))
print(" Rows: %d  |  Columns: %d" % (n, len(cols)))
print("=" * W)

def _tf(v):
    try: return float(str(v).replace(',','').strip())
    except: return None

dup_check = set()
dups = 0
for r in rows:
    k = tuple(str(r.get(c,'')) for c in cols)
    if k in dup_check: dups += 1
    else: dup_check.add(k)

if dups:
    print(" Duplicate rows: %d (%.1f%%)" % (dups, 100*dups/n))
else:
    print(" No duplicate rows.")
print()

for col in cols:
    vals = [r.get(col,'') for r in rows]
    missing = sum(1 for v in vals if str(v).strip() in ('','None','null','NULL','NA','N/A','NaN'))
    nums = [_tf(v) for v in vals if _tf(v) is not None]
    pct_miss = 100*missing/n if n else 0

    print("-" * W)
    print("  %s" % col)
    if pct_miss > 0:
        flag = "  [!]" if pct_miss > 20 else ""
        print("    Missing:  %d / %d  (%.1f%%)%s" % (missing, n, pct_miss, flag))

    if len(nums) > n * 0.5:
        # Numeric column
        nums_s = sorted(nums)
        mn = _mean = sum(nums_s)/len(nums_s)
        med_idx = len(nums_s)//2
        med = nums_s[med_idx] if len(nums_s)%2 else (nums_s[med_idx-1]+nums_s[med_idx])/2
        std = math.sqrt(sum((x-mn)**2 for x in nums_s)/len(nums_s)) if len(nums_s)>1 else 0
        q1_i = len(nums_s)//4; q3_i = 3*len(nums_s)//4
        q1 = nums_s[q1_i]; q3 = nums_s[q3_i]; iqr = q3 - q1
        lo_fence = q1 - 1.5*iqr; hi_fence = q3 + 1.5*iqr
        outliers = sum(1 for x in nums_s if x < lo_fence or x > hi_fence)
        print("    Type:     numeric  (%d values)" % len(nums_s))
        print("    Range:    %g  to  %g" % (nums_s[0], nums_s[-1]))
        print("    Mean:     %g  |  Median: %g  |  Std: %g" % (mn, med, std))
        if outliers:
            print("    Outliers: %d (IQR fence: [%g, %g])" % (outliers, lo_fence, hi_fence))
    else:
        # Categorical column
        from collections import Counter
        vc = Counter(str(v).strip() for v in vals if str(v).strip() not in ('','None','null','NULL'))
        top = vc.most_common(5)
        unique = len(vc)
        print("    Type:     categorical  (%d unique values)" % unique)
        if unique <= 20:
            print("    Values:   " + "  ".join("%s(%d)"%(k,c) for k,c in top))
        else:
            print("    Top 5:    " + "  ".join("%s(%d)"%(k,c) for k,c in top))

print("=" * W)
"####,
        hex_path = hex_path,
    );

    let sandbox_args = serde_json::json!({
        "language": "python",
        "code": script,
        "timeout_seconds": 30
    });
    crate::tools::code_sandbox::execute(&sandbox_args).await
}
