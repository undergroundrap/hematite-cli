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
