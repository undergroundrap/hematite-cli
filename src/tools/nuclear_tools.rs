use serde_json::{Map, Value};

const LN2: f64 = std::f64::consts::LN_2;
const C: f64 = 2.99792458e8;
const EV_TO_J: f64 = 1.602176634e-19;
const M_U_KG: f64 = 1.66053906660e-27;
const MEV_PER_U: f64 = 931.494103;
const M_P_U: f64 = 1.007276466621;
const M_N_U: f64 = 1.008664916;

fn get_f64(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

fn get_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

pub fn schema() -> Value {
    let mut root = Map::new();
    root.insert("name".into(), Value::String("nuclear_tools".into()));
    root.insert(
        "description".into(),
        Value::String(
            "Nuclear physics calculations without external utilities. 8 actions: \
             decay (default — radioactive decay N(t)=N₀e^(−λt)), halflife (convert \
             between T½/λ/τ), binding_energy (semi-empirical mass formula), q_value \
             (Q-value from mass defect), activity (Bq/Ci/mCi/dps conversions), dose \
             (radiation dose Gy/Sv/rad/rem + weighting factors), carbon_dating \
             (C-14 age from ratio or age), reactions (preset nuclear reactions with \
             energy release). Uses CODATA 2018 constants."
                .into(),
        ),
    );

    let mut props = Map::new();

    let mut action = Map::new();
    action.insert("type".into(), Value::String("string".into()));
    action.insert(
        "description".into(),
        Value::String(
            "Action: decay (radioactive decay), halflife (T½/λ/τ conversions), \
         binding_energy (SEMF for given Z and A), q_value (Q from reactant/product masses), \
         activity (unit conversions Bq/Ci/dps), dose (Gy/Sv/rad/rem + radiation type), \
         carbon_dating (C-14 age from ratio), reactions (preset fusion/fission/decay reactions)"
                .into(),
        ),
    );
    props.insert("action".into(), Value::Object(action));

    let mut n0 = Map::new();
    n0.insert("type".into(), Value::String("number".into()));
    n0.insert(
        "description".into(),
        Value::String("Initial number of atoms (for decay)".into()),
    );
    props.insert("n0".into(), Value::Object(n0));

    let mut a0 = Map::new();
    a0.insert("type".into(), Value::String("number".into()));
    a0.insert(
        "description".into(),
        Value::String("Initial activity in Bq (for decay, alternative to n0)".into()),
    );
    props.insert("a0".into(), Value::Object(a0));

    let mut t = Map::new();
    t.insert("type".into(), Value::String("number".into()));
    t.insert(
        "description".into(),
        Value::String("Elapsed time in seconds (for decay)".into()),
    );
    props.insert("t".into(), Value::Object(t));

    let mut lambda = Map::new();
    lambda.insert("type".into(), Value::String("number".into()));
    lambda.insert(
        "description".into(),
        Value::String("Decay constant λ in s⁻¹".into()),
    );
    props.insert("lambda".into(), Value::Object(lambda));

    let mut t_half = Map::new();
    t_half.insert("type".into(), Value::String("number".into()));
    t_half.insert(
        "description".into(),
        Value::String("Half-life T½ in seconds".into()),
    );
    props.insert("t_half".into(), Value::Object(t_half));

    let mut tau = Map::new();
    tau.insert("type".into(), Value::String("number".into()));
    tau.insert(
        "description".into(),
        Value::String("Mean lifetime τ in seconds".into()),
    );
    props.insert("tau".into(), Value::Object(tau));

    let mut z = Map::new();
    z.insert("type".into(), Value::String("number".into()));
    z.insert(
        "description".into(),
        Value::String("Atomic number Z (proton count) for binding_energy".into()),
    );
    props.insert("z".into(), Value::Object(z));

    let mut a_mass = Map::new();
    a_mass.insert("type".into(), Value::String("number".into()));
    a_mass.insert(
        "description".into(),
        Value::String("Mass number A (protons + neutrons) for binding_energy".into()),
    );
    props.insert("a".into(), Value::Object(a_mass));

    let mut reactants = Map::new();
    reactants.insert("type".into(), Value::String("array".into()));
    reactants.insert(
        "description".into(),
        Value::String("Array of reactant masses in atomic mass units u (for q_value)".into()),
    );
    props.insert("reactants".into(), Value::Object(reactants));

    let mut products = Map::new();
    products.insert("type".into(), Value::String("array".into()));
    products.insert(
        "description".into(),
        Value::String("Array of product masses in atomic mass units u (for q_value)".into()),
    );
    props.insert("products".into(), Value::Object(products));

    let mut bq = Map::new();
    bq.insert("type".into(), Value::String("number".into()));
    bq.insert(
        "description".into(),
        Value::String("Activity in becquerel Bq (for activity)".into()),
    );
    props.insert("bq".into(), Value::Object(bq));

    let mut ci = Map::new();
    ci.insert("type".into(), Value::String("number".into()));
    ci.insert(
        "description".into(),
        Value::String("Activity in curie Ci (for activity)".into()),
    );
    props.insert("ci".into(), Value::Object(ci));

    let mut mci = Map::new();
    mci.insert("type".into(), Value::String("number".into()));
    mci.insert(
        "description".into(),
        Value::String("Activity in millicurie mCi (for activity)".into()),
    );
    props.insert("mci".into(), Value::Object(mci));

    let mut gy = Map::new();
    gy.insert("type".into(), Value::String("number".into()));
    gy.insert(
        "description".into(),
        Value::String("Absorbed dose in gray Gy (for dose)".into()),
    );
    props.insert("gy".into(), Value::Object(gy));

    let mut sv = Map::new();
    sv.insert("type".into(), Value::String("number".into()));
    sv.insert(
        "description".into(),
        Value::String("Equivalent dose in sievert Sv (for dose)".into()),
    );
    props.insert("sv".into(), Value::Object(sv));

    let mut radiation = Map::new();
    radiation.insert("type".into(), Value::String("string".into()));
    radiation.insert("description".into(), Value::String(
        "Radiation type for dose: gamma, alpha, beta, neutron, proton, heavy_ion (default: gamma)".into()
    ));
    props.insert("radiation".into(), Value::Object(radiation));

    let mut ratio = Map::new();
    ratio.insert("type".into(), Value::String("number".into()));
    ratio.insert(
        "description".into(),
        Value::String("Fraction of C-14 remaining 0–1 (for carbon_dating)".into()),
    );
    props.insert("ratio".into(), Value::Object(ratio));

    let mut t_years = Map::new();
    t_years.insert("type".into(), Value::String("number".into()));
    t_years.insert(
        "description".into(),
        Value::String("Sample age in years (for carbon_dating)".into()),
    );
    props.insert("t_years".into(), Value::Object(t_years));

    let mut reaction = Map::new();
    reaction.insert("type".into(), Value::String("string".into()));
    reaction.insert(
        "description".into(),
        Value::String(
            "Named reaction preset for 'reactions' action: fusion_dt, fusion_dd, fusion_pp, \
         fission_u235, fission_pu239, alpha_decay, proton_proton_chain. Use 'list' to show all."
                .into(),
        ),
    );
    props.insert("reaction".into(), Value::Object(reaction));

    let mut params = Map::new();
    params.insert("type".into(), Value::String("object".into()));
    params.insert("properties".into(), Value::Object(props));
    root.insert("parameters".into(), Value::Object(params));
    Value::Object(root)
}

fn get_lambda(args: &Value) -> Result<f64, String> {
    if let Some(lam) = get_f64(args, "lambda") {
        return Ok(lam);
    }
    if let Some(th) = get_f64(args, "t_half") {
        return Ok(LN2 / th);
    }
    if let Some(tau) = get_f64(args, "tau") {
        return Ok(1.0 / tau);
    }
    Err("Provide one of: 'lambda' (s⁻¹), 't_half' (s), or 'tau' (s)".into())
}

fn action_decay(args: &Value) -> Result<String, String> {
    let n0 = get_f64(args, "n0")
        .or_else(|| get_f64(args, "a0"))
        .ok_or("Provide initial atoms 'n0' or initial activity 'a0' (Bq)")?;
    let t = get_f64(args, "t").ok_or("Provide elapsed time 't' in seconds")?;
    let lambda = get_lambda(args)?;

    let t_half = LN2 / lambda;
    let tau = 1.0 / lambda;
    let n_t = n0 * (-lambda * t).exp();
    let frac_rem = (-lambda * t).exp();
    let hl_elapsed = t / t_half;

    let is_activity = args.get("a0").is_some();
    let label = if is_activity { "Activity" } else { "Amount " };
    let unit = if is_activity { "Bq" } else { "atoms" };

    let mut out = String::new();
    out.push_str("NUCLEAR PHYSICS — RADIOACTIVE DECAY\n");
    out.push_str("  N(t) = N₀ · e^(−λt)\n\n");
    out.push_str(&format!("  Initial {label} N₀   = {n0:.4e} {unit}\n"));
    out.push_str(&format!("  Elapsed time      t   = {t:.4e} s\n"));
    out.push_str(&format!("  Decay constant    λ   = {lambda:.4e} s⁻¹\n"));
    out.push_str(&format!("  Half-life         T½  = {t_half:.4e} s\n"));
    out.push_str(&format!("  Mean lifetime     τ   = {tau:.4e} s\n\n"));
    out.push_str(&format!("  {label} at t: N(t)  = {n_t:.4e} {unit}\n"));
    out.push_str(&format!(
        "  Remaining:           {:.4}%\n",
        frac_rem * 100.0
    ));
    out.push_str(&format!(
        "  Decayed:             {:.4}%\n",
        (1.0 - frac_rem) * 100.0
    ));
    out.push_str(&format!("  Half-lives elapsed:  {hl_elapsed:.4}\n"));
    Ok(out)
}

fn action_halflife(args: &Value) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("NUCLEAR PHYSICS — HALF-LIFE CONVERSIONS\n\n");

    let (t_half, lambda, tau) = if let Some(th) = get_f64(args, "t_half") {
        (th, LN2 / th, th / LN2)
    } else if let Some(lam) = get_f64(args, "lambda") {
        (LN2 / lam, lam, 1.0 / lam)
    } else if let Some(tau_val) = get_f64(args, "tau") {
        (tau_val * LN2, 1.0 / tau_val, tau_val)
    } else {
        return Err("Provide one of: 't_half' (s), 'lambda' (s⁻¹), or 'tau' (s)".into());
    };

    out.push_str(&format!("  T½ (half-life)     = {t_half:.6e} s\n"));
    out.push_str(&format!("  λ  (decay const)   = {lambda:.6e} s⁻¹\n"));
    out.push_str(&format!("  τ  (mean lifetime) = {tau:.6e} s\n\n"));
    out.push_str(&format!("  After  1 T½:  50.000% remains\n"));
    out.push_str(&format!("  After  2 T½:  25.000% remains\n"));
    out.push_str(&format!(
        "  After 10 T½: {:6.4}% remains\n",
        100.0 * 0.5_f64.powi(10)
    ));

    if t_half > 3.154e7 * 1e6 {
        out.push_str(&format!("  T½ = {:.4e} Myr\n", t_half / (3.154e7 * 1e6)));
    } else if t_half > 3.154e7 {
        out.push_str(&format!("  T½ = {:.4} years\n", t_half / 3.154e7));
    } else if t_half > 86400.0 {
        out.push_str(&format!("  T½ = {:.4} days\n", t_half / 86400.0));
    } else if t_half > 3600.0 {
        out.push_str(&format!("  T½ = {:.4} hours\n", t_half / 3600.0));
    } else if t_half > 60.0 {
        out.push_str(&format!("  T½ = {:.4} minutes\n", t_half / 60.0));
    }
    Ok(out)
}

fn action_binding_energy(args: &Value) -> Result<String, String> {
    let zf = get_f64(args, "z").ok_or("Provide atomic number 'z' (protons)")?;
    let af = get_f64(args, "a").ok_or("Provide mass number 'a' (protons + neutrons)")?;
    let z = zf as i64;
    let a = af as i64;

    if z < 1 || a < 1 || a < z {
        return Err(format!("Invalid Z={z}, A={a}. Need Z ≥ 1 and A ≥ Z."));
    }
    let n = a - z;

    // Bethe-Weizsäcker SEMF coefficients (MeV)
    let a_v = 15.835;
    let a_s = 18.33;
    let a_c = 0.714;
    let a_a = 23.2;
    let a_p = 11.2;

    let b_v = a_v * af;
    let b_s = -a_s * af.powf(2.0 / 3.0);
    let b_c = -a_c * zf * zf / af.powf(1.0 / 3.0);
    let b_a = -a_a * (af - 2.0 * zf).powi(2) / af;
    let delta = if z % 2 == 0 && n % 2 == 0 {
        a_p / af.sqrt()
    } else if z % 2 == 1 && n % 2 == 1 {
        -a_p / af.sqrt()
    } else {
        0.0
    };

    let b_total = b_v + b_s + b_c + b_a + delta;
    let b_per_a = b_total / af;
    let nucleus_mass_u = zf * M_P_U + (n as f64) * M_N_U - b_total / MEV_PER_U;

    let mut out = String::new();
    out.push_str("NUCLEAR PHYSICS — SEMI-EMPIRICAL BINDING ENERGY\n");
    out.push_str("  (Bethe-Weizsäcker liquid drop model)\n\n");
    out.push_str(&format!("  Nucleus: Z = {z}, N = {n}, A = {a}\n\n"));
    out.push_str(&format!(
        "  Volume term     aᵥ·A          = {:+.3} MeV\n",
        b_v
    ));
    out.push_str(&format!(
        "  Surface term   −aₛ·A^(2/3)    = {:+.3} MeV\n",
        b_s
    ));
    out.push_str(&format!(
        "  Coulomb term   −aC·Z²/A^(1/3) = {:+.3} MeV\n",
        b_c
    ));
    out.push_str(&format!(
        "  Asymmetry      −aA·(A−2Z)²/A  = {:+.3} MeV\n",
        b_a
    ));
    out.push_str(&format!(
        "  Pairing δ                     = {:+.3} MeV\n",
        delta
    ));
    out.push_str("  ──────────────────────────────────────────\n");
    out.push_str(&format!(
        "  Total B(Z,A)                  = {b_total:.3} MeV\n"
    ));
    out.push_str(&format!(
        "  B/A  (per nucleon)            = {b_per_a:.3} MeV/nucleon\n\n"
    ));
    out.push_str(&format!("  Nucleus mass ≈ {nucleus_mass_u:.6} u\n"));
    out.push_str("  (SEMF is empirical; use tabulated masses for precision)\n");
    Ok(out)
}

fn action_q_value(args: &Value) -> Result<String, String> {
    let reactants = args
        .get("reactants")
        .and_then(|v| v.as_array())
        .ok_or("Provide 'reactants' as array of masses in u")?;
    let products = args
        .get("products")
        .and_then(|v| v.as_array())
        .ok_or("Provide 'products' as array of masses in u")?;

    let sum_r: f64 = reactants.iter().filter_map(|v| v.as_f64()).sum();
    let sum_p: f64 = products.iter().filter_map(|v| v.as_f64()).sum();

    let valid_r = reactants.iter().all(|v| v.as_f64().is_some());
    let valid_p = products.iter().all(|v| v.as_f64().is_some());
    if !valid_r || !valid_p {
        return Err("All masses in reactants and products must be numbers (in u)".into());
    }

    let delta_m_u = sum_r - sum_p;
    let q_mev = delta_m_u * MEV_PER_U;
    let q_j = q_mev * 1e6 * EV_TO_J;

    let fmt_arr = |arr: &Vec<Value>| -> String {
        arr.iter()
            .map(|v| format!("{:.6}", v.as_f64().unwrap_or(0.0)))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let mut out = String::new();
    out.push_str("NUCLEAR PHYSICS — Q-VALUE\n");
    out.push_str("  Q = (Σm_reactants − Σm_products) × c²\n\n");
    out.push_str(&format!(
        "  Reactants [{}] = {sum_r:.8} u\n",
        fmt_arr(reactants)
    ));
    out.push_str(&format!(
        "  Products  [{}] = {sum_p:.8} u\n",
        fmt_arr(products)
    ));
    out.push_str(&format!("  Δm = {delta_m_u:+.8} u\n\n"));
    out.push_str(&format!("  Q = {q_mev:+.4} MeV\n"));
    out.push_str(&format!("  Q = {q_j:+.4e} J\n\n"));

    if q_mev > 1e-6 {
        out.push_str("  Q > 0: Exothermic — energy released\n");
    } else if q_mev < -1e-6 {
        out.push_str(&format!(
            "  Q < 0: Endothermic — threshold kinetic energy needed: {:.4} MeV\n",
            -q_mev
        ));
    } else {
        out.push_str("  Q ≈ 0\n");
    }
    Ok(out)
}

fn action_activity(args: &Value) -> Result<String, String> {
    let input_bq = if let Some(bq) = get_f64(args, "bq") {
        bq
    } else if let Some(ci) = get_f64(args, "ci") {
        ci * 3.7e10
    } else if let Some(mci) = get_f64(args, "mci") {
        mci * 3.7e7
    } else if let Some(dps) = get_f64(args, "dps") {
        dps
    } else {
        return Err("Provide one of: 'bq', 'ci', 'mci', or 'dps'".into());
    };

    let ci = input_bq / 3.7e10;
    let mci = input_bq / 3.7e7;
    let uci = input_bq / 3.7e4;
    let dpm = input_bq * 60.0;

    let mut out = String::new();
    out.push_str("NUCLEAR PHYSICS — ACTIVITY CONVERSIONS\n\n");
    out.push_str(&format!("  {input_bq:.4e} Bq\n"));
    out.push_str(&format!("  {ci:.4e} Ci   (curie)\n"));
    out.push_str(&format!("  {mci:.4e} mCi  (millicurie)\n"));
    out.push_str(&format!("  {uci:.4e} μCi  (microcurie)\n"));
    out.push_str(&format!("  {input_bq:.4e} dps  (= Bq)\n"));
    out.push_str(&format!(
        "  {dpm:.4e} dpm  (disintegrations per minute)\n\n"
    ));
    out.push_str("  1 Ci = 3.7×10¹⁰ Bq  (activity of 1 g Ra-226)\n");
    out.push_str("  1 Bq = 1 disintegration/second\n");
    Ok(out)
}

fn action_dose(args: &Value) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("NUCLEAR PHYSICS — RADIATION DOSE\n\n");

    if let Some(gy_val) = get_f64(args, "gy") {
        let rad = gy_val * 100.0;
        out.push_str(&format!(
            "  Absorbed dose: {gy_val:.4e} Gy = {rad:.4e} rad\n"
        ));

        let radiation = get_str(args, "radiation").unwrap_or("gamma");
        let wr: f64 = match radiation.to_lowercase().as_str() {
            "alpha" => 20.0,
            "beta" | "electron" | "positron" => 1.0,
            "gamma" | "x-ray" | "xray" | "photon" => 1.0,
            "neutron" | "thermal_neutron" => 5.0,
            "fast_neutron" => 20.0,
            "proton" => 5.0,
            "heavy_ion" => 20.0,
            _ => 1.0,
        };
        let sv = gy_val * wr;
        let rem = sv * 100.0;
        out.push_str(&format!("  Radiation type: {radiation} (Wᵣ = {wr})\n"));
        out.push_str(&format!(
            "  H = D × Wᵣ = {gy_val:.4e} Gy × {wr} = {sv:.4e} Sv = {rem:.4e} rem\n\n"
        ));
    } else if let Some(sv_val) = get_f64(args, "sv") {
        let rem = sv_val * 100.0;
        out.push_str(&format!(
            "  Equivalent dose: {sv_val:.4e} Sv = {rem:.4e} rem\n\n"
        ));
    } else if let Some(rad_val) = get_f64(args, "rad") {
        let gy_val = rad_val / 100.0;
        out.push_str(&format!(
            "  Absorbed dose: {rad_val:.4e} rad = {gy_val:.4e} Gy\n\n"
        ));
    } else if let Some(rem_val) = get_f64(args, "rem") {
        let sv_val = rem_val / 100.0;
        out.push_str(&format!(
            "  Equivalent dose: {rem_val:.4e} rem = {sv_val:.4e} Sv\n\n"
        ));
    }

    out.push_str("  RADIATION WEIGHTING FACTORS (Wᵣ)\n");
    out.push_str("  Gamma / X-ray / Beta:    Wᵣ = 1\n");
    out.push_str("  Proton / thermal neutron: Wᵣ = 5\n");
    out.push_str("  Alpha / fast neutron / heavy ion: Wᵣ = 20\n\n");
    out.push_str("  CONTEXT\n");
    out.push_str("  Background radiation:         ~3 mSv/year\n");
    out.push_str("  Chest X-ray:                  ~0.1 mSv\n");
    out.push_str("  CT scan (chest):              ~7 mSv\n");
    out.push_str("  Annual limit (rad. workers): 20 mSv/year\n");
    out.push_str("  Acute radiation syndrome:     ≥ 1 Sv (single dose)\n");
    Ok(out)
}

fn action_carbon_dating(args: &Value) -> Result<String, String> {
    let secs_per_year = 365.25 * 24.0 * 3600.0;
    let c14_t_half = 5730.0 * secs_per_year;
    let lambda = LN2 / c14_t_half;

    let mut out = String::new();
    out.push_str("NUCLEAR PHYSICS — RADIOCARBON DATING (C-14)\n");
    out.push_str(&format!(
        "  T½(C-14) = 5730 years  λ = {lambda:.4e} s⁻¹\n\n"
    ));

    if let Some(ratio) = get_f64(args, "ratio") {
        if ratio <= 0.0 || ratio > 1.0 {
            return Err("'ratio' must be in (0, 1] — fraction of original C-14 remaining".into());
        }
        let t_s = -ratio.ln() / lambda;
        let t_years = t_s / secs_per_year;
        out.push_str(&format!(
            "  C-14 remaining: {ratio:.4} ({:.2}%)\n",
            ratio * 100.0
        ));
        out.push_str(&format!("  t = −(1/λ)·ln(N/N₀)\n"));
        out.push_str(&format!("    = {t_years:.0} years  ({t_s:.3e} s)\n\n"));
        out.push_str("  Note: assumes constant atmospheric C-14.\n");
        out.push_str("  Calibration (dendrochronology) needed for accuracy.\n");
    } else if let Some(t_years) = get_f64(args, "t_years") {
        let t_s = t_years * secs_per_year;
        let ratio = (-lambda * t_s).exp();
        out.push_str(&format!("  Age: {t_years:.0} years\n"));
        out.push_str(&format!(
            "  N/N₀ = e^(−λt) = {ratio:.6}  ({:.2}% remains)\n",
            ratio * 100.0
        ));
        out.push_str(&format!("  Half-lives elapsed: {:.2}\n", t_years / 5730.0));
    } else {
        return Err("Provide 'ratio' (fraction C-14 remaining, 0–1) or 't_years' (age)".into());
    }

    out.push_str("\n  Useful range: ~300 to ~50,000 years\n");
    out.push_str("  After ~10 T½ (57,300 yr) signal is undetectable.\n");
    Ok(out)
}

fn action_reactions(args: &Value) -> Result<String, String> {
    let reaction = get_str(args, "reaction").unwrap_or("fusion_dt");

    struct R {
        name: &'static str,
        eq: &'static str,
        q_mev: f64,
        desc: &'static str,
        fuel_u: Option<f64>,
    }

    let table: &[R] = &[
        R {
            name: "fusion_dt",
            eq: "D + T → ⁴He + n",
            q_mev: 17.59,
            desc: "Deuterium-tritium fusion (primary ITER reaction)",
            fuel_u: Some(2.014 + 3.016),
        },
        R {
            name: "fusion_dd",
            eq: "D + D → ³He + n  (50%)  or  T + p  (50%)",
            q_mev: 3.27,
            desc: "Deuterium-deuterium fusion (lower Q, no tritium needed)",
            fuel_u: Some(2.014 + 2.014),
        },
        R {
            name: "fusion_pp",
            eq: "p + p → ²D + e⁺ + νₑ",
            q_mev: 0.42,
            desc: "Proton-proton chain — primary solar energy source",
            fuel_u: Some(1.00728 + 1.00728),
        },
        R {
            name: "fission_u235",
            eq: "²³⁵U + n → fission products + ~2.5n",
            q_mev: 202.5,
            desc: "Uranium-235 fission (typical reactor)",
            fuel_u: None,
        },
        R {
            name: "fission_pu239",
            eq: "²³⁹Pu + n → fission products + ~3n",
            q_mev: 210.0,
            desc: "Plutonium-239 fission",
            fuel_u: None,
        },
        R {
            name: "alpha_decay",
            eq: "²²⁶Ra → ²²²Rn + ⁴He  (α)",
            q_mev: 4.871,
            desc: "Radium-226 alpha decay (T½ = 1600 yr)",
            fuel_u: None,
        },
        R {
            name: "beta_decay",
            eq: "n → p + e⁻ + ν̄ₑ",
            q_mev: 0.782,
            desc: "Free neutron beta decay (T½ = 611.0 s)",
            fuel_u: None,
        },
    ];

    if reaction == "list" {
        let mut out = String::new();
        out.push_str("NUCLEAR REACTIONS — AVAILABLE PRESETS\n\n");
        for r in table {
            out.push_str(&format!(
                "  {:16} {:42} Q = {:.2} MeV\n",
                r.name, r.eq, r.q_mev
            ));
        }
        out.push_str("\n  Usage: action='reactions', reaction='fusion_dt'\n");
        return Ok(out);
    }

    let r = table.iter().find(|r| r.name == reaction).ok_or_else(|| {
        format!("Unknown reaction '{reaction}'. Use reaction='list' to see options.")
    })?;

    let q_j = r.q_mev * 1e6 * EV_TO_J;

    let mut out = String::new();
    out.push_str(&format!(
        "NUCLEAR PHYSICS — REACTION: {}\n\n",
        r.name.to_uppercase()
    ));
    out.push_str(&format!("  {}\n", r.eq));
    out.push_str(&format!("  {}\n\n", r.desc));
    out.push_str(&format!("  Q = {:.3} MeV = {:.4e} J\n\n", r.q_mev, q_j));

    if let Some(fuel_u) = r.fuel_u {
        let fuel_kg = fuel_u * M_U_KG;
        let energy_per_kg = q_j / fuel_kg;
        out.push_str(&format!(
            "  Energy density: {energy_per_kg:.3e} J/kg of fuel\n"
        ));
        out.push_str(&format!("  = {:.2} TJ/kg\n", energy_per_kg / 1e12));
        out.push_str(&format!(
            "  = {:.0}× energy density of coal (~30 MJ/kg)\n",
            energy_per_kg / 30e6
        ));
    }

    if reaction.starts_with("fission") {
        let a_num = if reaction.contains("pu239") {
            239.0
        } else {
            235.0
        };
        let kg_per_nucleus = a_num * M_U_KG;
        let e_per_kg = q_j / kg_per_nucleus;
        out.push_str(&format!(
            "  Energy per kg (fully fissioned): {e_per_kg:.3e} J ≈ {:.0} TJ/kg\n",
            e_per_kg / 1e12
        ));
        out.push_str("  Note: actual reactor burn-up is far less than 100%.\n");
    }

    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("decay");
    match action {
        "decay" => action_decay(args),
        "halflife" => action_halflife(args),
        "binding_energy" => action_binding_energy(args),
        "q_value" => action_q_value(args),
        "activity" => action_activity(args),
        "dose" => action_dose(args),
        "carbon_dating" => action_carbon_dating(args),
        "reactions" => action_reactions(args),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: decay, halflife, binding_energy, q_value, activity, dose, carbon_dating, reactions"
        )),
    }
}
