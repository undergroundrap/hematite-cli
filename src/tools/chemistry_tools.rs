use serde_json::{json, Value};
use std::collections::HashMap;

pub fn chemistry_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["balance", "stoichiometry", "solution", "ph", "gas"],
                "description": "balance: balance a chemical equation | stoichiometry: mole/mass calculations | solution: molarity/dilution | ph: pH/pOH/Henderson-Hasselbalch | gas: ideal gas law"
            },
            "equation": {"type": "string", "description": "Chemical equation (e.g. 'H2 + O2 -> H2O' or 'Fe + O2 -> Fe2O3')"},
            "reactant": {"type": "string", "description": "Reactant formula for stoichiometry (e.g. 'H2')"},
            "product": {"type": "string", "description": "Product formula for stoichiometry (e.g. 'H2O')"},
            "moles": {"type": "number", "description": "Moles of the reactant for stoichiometry"},
            "grams": {"type": "number", "description": "Grams of the reactant for stoichiometry"},
            "solute": {"type": "string", "description": "Solute formula for solution calculations (e.g. 'NaCl')"},
            "C1": {"type": "number", "description": "Initial concentration (mol/L) for dilution"},
            "V1": {"type": "number", "description": "Initial volume (L) for dilution"},
            "C2": {"type": "number", "description": "Final concentration (mol/L) for dilution"},
            "V2": {"type": "number", "description": "Final volume (L) for dilution"},
            "moles_solute": {"type": "number", "description": "Moles of solute"},
            "volume_L": {"type": "number", "description": "Volume in liters"},
            "concentration": {"type": "number", "description": "Molarity in mol/L"},
            "Ka": {"type": "number", "description": "Acid dissociation constant for Henderson-Hasselbalch"},
            "Kb": {"type": "number", "description": "Base dissociation constant"},
            "acid_conc": {"type": "number", "description": "Concentration of acid (mol/L)"},
            "base_conc": {"type": "number", "description": "Concentration of conjugate base (mol/L)"},
            "pH_value": {"type": "number", "description": "pH value for conversion"},
            "P": {"type": "number", "description": "Pressure (Pa) for gas law"},
            "V": {"type": "number", "description": "Volume (m³ or L with unit flag) for gas law"},
            "n": {"type": "number", "description": "Moles of gas"},
            "T": {"type": "number", "description": "Temperature (K) for gas law"},
            "unit": {"type": "string", "description": "Volume unit: 'L' (liters) or 'm3' (cubic meters, default)"}
        },
        "required": []
    })
}

// Atomic masses for all 118 elements (g/mol)
fn atomic_mass(sym: &str) -> Option<f64> {
    match sym {
        "H" => Some(1.008),
        "He" => Some(4.003),
        "Li" => Some(6.941),
        "Be" => Some(9.012),
        "B" => Some(10.811),
        "C" => Some(12.011),
        "N" => Some(14.007),
        "O" => Some(15.999),
        "F" => Some(18.998),
        "Ne" => Some(20.180),
        "Na" => Some(22.990),
        "Mg" => Some(24.305),
        "Al" => Some(26.982),
        "Si" => Some(28.086),
        "P" => Some(30.974),
        "S" => Some(32.065),
        "Cl" => Some(35.453),
        "Ar" => Some(39.948),
        "K" => Some(39.098),
        "Ca" => Some(40.078),
        "Sc" => Some(44.956),
        "Ti" => Some(47.867),
        "V" => Some(50.942),
        "Cr" => Some(51.996),
        "Mn" => Some(54.938),
        "Fe" => Some(55.845),
        "Co" => Some(58.933),
        "Ni" => Some(58.693),
        "Cu" => Some(63.546),
        "Zn" => Some(65.38),
        "Ga" => Some(69.723),
        "Ge" => Some(72.640),
        "As" => Some(74.922),
        "Se" => Some(78.960),
        "Br" => Some(79.904),
        "Kr" => Some(83.798),
        "Rb" => Some(85.468),
        "Sr" => Some(87.620),
        "Y" => Some(88.906),
        "Zr" => Some(91.224),
        "Nb" => Some(92.906),
        "Mo" => Some(95.960),
        "Tc" => Some(98.0),
        "Ru" => Some(101.070),
        "Rh" => Some(102.906),
        "Pd" => Some(106.420),
        "Ag" => Some(107.868),
        "Cd" => Some(112.411),
        "In" => Some(114.818),
        "Sn" => Some(118.710),
        "Sb" => Some(121.760),
        "Te" => Some(127.600),
        "I" => Some(126.904),
        "Xe" => Some(131.293),
        "Cs" => Some(132.905),
        "Ba" => Some(137.327),
        "La" => Some(138.905),
        "Ce" => Some(140.116),
        "Pr" => Some(140.908),
        "Nd" => Some(144.242),
        "Pm" => Some(145.0),
        "Sm" => Some(150.360),
        "Eu" => Some(151.964),
        "Gd" => Some(157.250),
        "Tb" => Some(158.925),
        "Dy" => Some(162.500),
        "Ho" => Some(164.930),
        "Er" => Some(167.259),
        "Tm" => Some(168.934),
        "Yb" => Some(173.054),
        "Lu" => Some(174.967),
        "Hf" => Some(178.490),
        "Ta" => Some(180.948),
        "W" => Some(183.840),
        "Re" => Some(186.207),
        "Os" => Some(190.230),
        "Ir" => Some(192.217),
        "Pt" => Some(195.084),
        "Au" => Some(196.967),
        "Hg" => Some(200.590),
        "Tl" => Some(204.383),
        "Pb" => Some(207.200),
        "Bi" => Some(208.980),
        "Po" => Some(209.0),
        "At" => Some(210.0),
        "Rn" => Some(222.0),
        "Fr" => Some(223.0),
        "Ra" => Some(226.0),
        "Ac" => Some(227.0),
        "Th" => Some(232.038),
        "Pa" => Some(231.036),
        "U" => Some(238.029),
        "Np" => Some(237.0),
        "Pu" => Some(244.0),
        "Am" => Some(243.0),
        "Cm" => Some(247.0),
        "Bk" => Some(247.0),
        "Cf" => Some(251.0),
        "Es" => Some(252.0),
        "Fm" => Some(257.0),
        "Md" => Some(258.0),
        "No" => Some(259.0),
        "Lr" => Some(262.0),
        "Rf" => Some(267.0),
        "Db" => Some(268.0),
        "Sg" => Some(271.0),
        "Bh" => Some(272.0),
        "Hs" => Some(270.0),
        "Mt" => Some(276.0),
        "Ds" => Some(281.0),
        "Rg" => Some(280.0),
        "Cn" => Some(285.0),
        "Nh" => Some(284.0),
        "Fl" => Some(289.0),
        "Mc" => Some(288.0),
        "Lv" => Some(293.0),
        "Ts" => Some(294.0),
        "Og" => Some(294.0),
        _ => None,
    }
}

fn molar_mass(formula: &str) -> Result<f64, String> {
    parse_formula(formula).map(|map| {
        map.iter()
            .map(|(sym, &count)| atomic_mass(sym).unwrap_or(0.0) * count as f64)
            .sum()
    })
}

fn parse_formula(formula: &str) -> Result<HashMap<String, usize>, String> {
    let chars: Vec<char> = formula.chars().collect();
    let mut stack: Vec<HashMap<String, usize>> = vec![HashMap::new()];
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '(' | '[' => {
                stack.push(HashMap::new());
                i += 1;
            }
            ')' | ']' => {
                i += 1;
                let mut num_s = String::new();
                while i < chars.len() && chars[i].is_ascii_digit() {
                    num_s.push(chars[i]);
                    i += 1;
                }
                let mult = if num_s.is_empty() {
                    1
                } else {
                    num_s.parse::<usize>().unwrap_or(1)
                };
                let top = stack.pop().ok_or("Unbalanced parentheses")?;
                let cur = stack.last_mut().ok_or("Unbalanced parentheses")?;
                for (sym, &cnt) in &top {
                    *cur.entry(sym.clone()).or_insert(0) += cnt * mult;
                }
            }
            c if c.is_ascii_uppercase() => {
                let mut sym = String::new();
                sym.push(chars[i]);
                i += 1;
                while i < chars.len() && chars[i].is_ascii_lowercase() {
                    sym.push(chars[i]);
                    i += 1;
                }
                let mut num_s = String::new();
                while i < chars.len() && chars[i].is_ascii_digit() {
                    num_s.push(chars[i]);
                    i += 1;
                }
                let count = if num_s.is_empty() {
                    1
                } else {
                    num_s.parse::<usize>().unwrap_or(1)
                };
                if atomic_mass(&sym).is_none() {
                    return Err(format!("Unknown element symbol: {}", sym));
                }
                *stack
                    .last_mut()
                    .ok_or("Stack empty")?
                    .entry(sym)
                    .or_insert(0) += count;
            }
            _ => {
                i += 1;
            }
        }
    }
    if stack.len() != 1 {
        return Err("Unbalanced parentheses in formula".to_string());
    }
    Ok(stack.pop().unwrap())
}

// Rational arithmetic for balancing
type Rat = (i64, i64); // (numerator, denominator)
fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}
fn rat_reduce(r: Rat) -> Rat {
    if r.1 == 0 {
        return r;
    }
    let g = gcd(r.0.abs(), r.1.abs());
    let sign = if r.1 < 0 { -1 } else { 1 };
    (sign * r.0 / g, sign * r.1 / g)
}
fn rat_add(a: Rat, b: Rat) -> Rat {
    rat_reduce((a.0 * b.1 + b.0 * a.1, a.1 * b.1))
}
fn rat_sub(a: Rat, b: Rat) -> Rat {
    rat_add(a, (-b.0, b.1))
}
fn rat_mul(a: Rat, b: Rat) -> Rat {
    rat_reduce((a.0 * b.0, a.1 * b.1))
}
fn rat_div(a: Rat, b: Rat) -> Rat {
    rat_mul(a, (b.1, b.0))
}

fn balance_equation(equation: &str) -> Result<String, String> {
    let (lhs, rhs) = if let Some(p) = equation.find("->") {
        (&equation[..p], &equation[p + 2..])
    } else if let Some(p) = equation.find('=') {
        (&equation[..p], &equation[p + 1..])
    } else {
        return Err(
            "Use '->' or '=' to separate reactants from products (e.g. 'H2 + O2 -> H2O')"
                .to_string(),
        );
    };

    let parse_side = |side: &str| -> Vec<(HashMap<String, usize>, String)> {
        side.split('+')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|formula| {
                let f = formula.trim().to_string();
                (parse_formula(&f).unwrap_or_default(), f)
            })
            .collect()
    };

    let reactants = parse_side(lhs.trim());
    let products = parse_side(rhs.trim());
    if reactants.is_empty() || products.is_empty() {
        return Err("Could not parse equation. Check formula syntax.".to_string());
    }
    let n_compounds = reactants.len() + products.len();
    let mut all_elements: Vec<String> = {
        let mut elems: std::collections::BTreeSet<String> = Default::default();
        for (m, _) in &reactants {
            for k in m.keys() {
                elems.insert(k.clone());
            }
        }
        for (m, _) in &products {
            for k in m.keys() {
                elems.insert(k.clone());
            }
        }
        elems.into_iter().collect()
    };
    all_elements.sort();
    let n_elements = all_elements.len();
    // Build matrix: rows = elements, cols = compounds (reactants positive, products negative)
    let mut matrix: Vec<Vec<Rat>> = vec![vec![(0, 1); n_compounds + 1]; n_elements];
    for (col, (comp_map, _)) in reactants.iter().enumerate() {
        for (row, elem) in all_elements.iter().enumerate() {
            let count = *comp_map.get(elem).unwrap_or(&0) as i64;
            matrix[row][col] = (count, 1);
        }
    }
    for (col2, (comp_map, _)) in products.iter().enumerate() {
        let col = reactants.len() + col2;
        for (row, elem) in all_elements.iter().enumerate() {
            let count = *comp_map.get(elem).unwrap_or(&0) as i64;
            matrix[row][col] = (-count, 1);
        }
    }
    // Gaussian elimination
    let rows = n_elements;
    let cols = n_compounds;
    let mut pivot_col = vec![usize::MAX; rows];
    let mut cur_row = 0;
    for col in 0..cols {
        let pivot = (cur_row..rows).find(|&r| matrix[r][col].0 != 0);
        if let Some(pr) = pivot {
            matrix.swap(cur_row, pr);
            pivot_col[cur_row] = col;
            let pv = matrix[cur_row][col];
            for r in 0..rows {
                if r != cur_row && matrix[r][col].0 != 0 {
                    let factor = rat_div(matrix[r][col], pv);
                    for c in 0..=cols {
                        let sub = rat_mul(factor, matrix[cur_row][c]);
                        let old = matrix[r][c];
                        matrix[r][c] = rat_sub(old, sub);
                    }
                }
            }
            cur_row += 1;
        }
    }
    // Free variable = last column, set to 1
    let mut coeff = vec![(0i64, 1i64); n_compounds];
    let free_idx = n_compounds - 1;
    coeff[free_idx] = (1, 1);
    for r in (0..cur_row).rev() {
        if pivot_col[r] == usize::MAX {
            continue;
        }
        let pc = pivot_col[r];
        let pv = matrix[r][pc];
        let mut rhs_val = rat_mul(matrix[r][cols], (1, 1));
        for (c, &mc) in matrix[r][(pc + 1)..cols]
            .iter()
            .enumerate()
            .map(|(i, v)| (pc + 1 + i, v))
        {
            let contrib = rat_mul(mc, coeff[c]);
            rhs_val = rat_sub(rhs_val, contrib);
        }
        coeff[pc] = rat_reduce(rat_div(rhs_val, pv));
    }
    // Scale to integers using LCM of denominators
    let mut lcm_den = 1i64;
    for &(_, d) in &coeff {
        lcm_den = lcm_den / gcd(lcm_den, d.abs()) * d.abs();
    }
    let int_coeff: Vec<i64> = coeff.iter().map(|&(n, d)| n * lcm_den / d).collect();
    if int_coeff.iter().all(|&c| c == 0) {
        return Err(
            "Could not balance equation — check that elements match on both sides.".to_string(),
        );
    }
    // Find minimum positive scalar
    let min_val = int_coeff
        .iter()
        .filter(|&&c| c.abs() > 0)
        .map(|c| c.abs())
        .min()
        .unwrap_or(1);
    let scaled: Vec<i64> = int_coeff.iter().map(|&c| c / min_val).collect();
    if scaled.iter().any(|&c| c <= 0) {
        return Err(
            "Balance failed: non-positive coefficient. Equation may be unbalanceable.".to_string(),
        );
    }
    // Build output
    let fmt_compound = |coeff: i64, formula: &str| -> String {
        if coeff == 1 {
            formula.to_string()
        } else {
            format!("{}{}", coeff, formula)
        }
    };
    let lhs_str: Vec<String> = reactants
        .iter()
        .zip(scaled.iter())
        .map(|((_, f), &c)| fmt_compound(c, f))
        .collect();
    let rhs_str: Vec<String> = products
        .iter()
        .zip(scaled[reactants.len()..].iter())
        .map(|((_, f), &c)| fmt_compound(c, f))
        .collect();
    let balanced = format!("{} \u{2192} {}", lhs_str.join(" + "), rhs_str.join(" + "));
    let mut out = String::from("CHEMICAL EQUATION BALANCE\n=========================\n\n");
    out.push_str(&format!("Input    : {}\n", equation));
    out.push_str(&format!("Balanced : {}\n\n", balanced));
    out.push_str("Coefficients:\n");
    for (i, (_, formula)) in reactants.iter().enumerate() {
        out.push_str(&format!(
            "  {:15} \u{2192} coefficient {}\n",
            formula, scaled[i]
        ));
    }
    for (i, (_, formula)) in products.iter().enumerate() {
        out.push_str(&format!(
            "  {:15} \u{2192} coefficient {}\n",
            formula,
            scaled[reactants.len() + i]
        ));
    }
    Ok(out.trim_end().to_string())
}

fn action_balance(args: &Value) -> String {
    let eq = match args.get("equation").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => return "Provide 'equation' (e.g. 'H2 + O2 -> H2O').".to_string(),
    };
    balance_equation(eq).unwrap_or_else(|e| e)
}

fn action_stoichiometry(args: &Value) -> String {
    let eq = match args.get("equation").and_then(|v| v.as_str()) {
        Some(e) => e,
        None => {
            return "Provide 'equation' (balanced or unbalanced) and 'reactant'/'product' formulas."
                .to_string()
        }
    };
    let reactant_sym = args.get("reactant").and_then(|v| v.as_str()).unwrap_or("");
    let product_sym = args.get("product").and_then(|v| v.as_str()).unwrap_or("");
    let given_moles = args.get("moles").and_then(|v| v.as_f64());
    let given_grams = args.get("grams").and_then(|v| v.as_f64());
    if reactant_sym.is_empty() {
        return "Provide 'reactant' formula to start calculation.".to_string();
    }
    // Try to balance and parse coefficients
    let balanced_eq = balance_equation(eq).unwrap_or_else(|_| eq.to_string());
    // Parse molar masses
    let reactant_mm = match molar_mass(reactant_sym) {
        Ok(mm) => mm,
        Err(e) => return format!("Error parsing reactant '{}': {}", reactant_sym, e),
    };
    let product_mm = if !product_sym.is_empty() {
        match molar_mass(product_sym) {
            Ok(mm) => Some(mm),
            Err(e) => return format!("Error parsing product '{}': {}", product_sym, e),
        }
    } else {
        None
    };
    let given_mol = given_moles.or_else(|| given_grams.map(|g| g / reactant_mm));
    let mol = match given_mol {
        Some(m) => m,
        None => return "Provide 'moles' or 'grams' of the reactant.".to_string(),
    };
    let given_g = given_grams.unwrap_or(mol * reactant_mm);
    let mut out = String::from("STOICHIOMETRY\n=============\n\n");
    out.push_str(&format!(
        "Equation  : {}\n",
        balanced_eq.lines().nth(1).unwrap_or(eq)
    ));
    out.push_str(&format!(
        "Reactant  : {} (M = {:.3} g/mol)\n",
        reactant_sym, reactant_mm
    ));
    out.push_str(&format!(
        "Given     : {:.4} mol = {:.3} g\n\n",
        mol, given_g
    ));
    if let Some(mm) = product_mm {
        out.push_str(&format!(
            "Product   : {} (M = {:.3} g/mol)\n",
            product_sym, mm
        ));
        out.push_str(&format!("Produced  : {:.4} mol = {:.3} g\n", mol, mol * mm));
    } else {
        out.push_str("(Provide 'product' formula to calculate product yield)\n");
    }
    out.trim_end().to_string()
}

fn action_solution(args: &Value) -> String {
    let solute = args.get("solute").and_then(|v| v.as_str()).unwrap_or("");
    let c1 = args.get("C1").and_then(|v| v.as_f64());
    let v1 = args.get("V1").and_then(|v| v.as_f64());
    let c2 = args.get("C2").and_then(|v| v.as_f64());
    let v2 = args.get("V2").and_then(|v| v.as_f64());
    let moles_solute = args.get("moles_solute").and_then(|v| v.as_f64());
    let volume = args.get("volume_L").and_then(|v| v.as_f64());
    let conc = args.get("concentration").and_then(|v| v.as_f64());
    let mut out = String::from("SOLUTION CALCULATIONS\n=====================\n\n");
    if !solute.is_empty() {
        match molar_mass(solute) {
            Ok(mm) => out.push_str(&format!("Solute    : {} (M = {:.3} g/mol)\n\n", solute, mm)),
            Err(_) => out.push_str(&format!("Solute    : {}\n\n", solute)),
        }
    }
    // Molarity triangle: M = n/V
    let (n, v_l, m) = (moles_solute, volume, conc);
    match (n, v_l, m) {
        (Some(n), Some(v), None) => {
            out.push_str(&format!(
                "Molarity  : C = n/V = {:.4} mol / {:.4} L = {:.4} mol/L\n",
                n,
                v,
                n / v
            ));
        }
        (Some(n), None, Some(m)) => {
            out.push_str(&format!(
                "Volume    : V = n/C = {:.4} mol / {:.4} mol/L = {:.4} L\n",
                n,
                m,
                n / m
            ));
        }
        (None, Some(v), Some(m)) => {
            out.push_str(&format!(
                "Moles     : n = C\u{00B7}V = {:.4} mol/L \u{00D7} {:.4} L = {:.4} mol\n",
                m,
                v,
                m * v
            ));
        }
        _ => {}
    }
    // Dilution: C1V1 = C2V2
    match (c1, v1, c2, v2) {
        (Some(c1), Some(v1), Some(c2), None) => {
            out.push_str(&format!("Dilution  : C\u{2081}V\u{2081} = C\u{2082}V\u{2082} \u{2192} V\u{2082} = ({:.4}\u{00D7}{:.4})/{:.4} = {:.4} L\n", c1, v1, c2, c1*v1/c2));
        }
        (Some(c1), Some(v1), None, Some(v2)) => {
            out.push_str(&format!("Dilution  : C\u{2081}V\u{2081} = C\u{2082}V\u{2082} \u{2192} C\u{2082} = ({:.4}\u{00D7}{:.4})/{:.4} = {:.4} mol/L\n", c1, v1, v2, c1*v1/v2));
        }
        (Some(c1), None, Some(c2), Some(v2)) => {
            out.push_str(&format!("Dilution  : C\u{2081}V\u{2081} = C\u{2082}V\u{2082} \u{2192} V\u{2081} = ({:.4}\u{00D7}{:.4})/{:.4} = {:.4} L\n", c2, v2, c1, c2*v2/c1));
        }
        (None, Some(v1), Some(c2), Some(v2)) => {
            out.push_str(&format!("Dilution  : C\u{2081}V\u{2081} = C\u{2082}V\u{2082} \u{2192} C\u{2081} = ({:.4}\u{00D7}{:.4})/{:.4} = {:.4} mol/L\n", c2, v2, v1, c2*v2/v1));
        }
        (Some(c1), Some(v1), Some(c2), Some(v2)) => {
            let lhs = c1 * v1;
            let rhs = c2 * v2;
            out.push_str(&format!("Dilution check: C\u{2081}V\u{2081} = {:.4}, C\u{2082}V\u{2082} = {:.4} \u{2192} {}\n", lhs, rhs, if (lhs-rhs).abs() < 1e-9 {"BALANCED"} else {"NOT BALANCED"}));
        }
        _ => {
            if n.is_none() && v_l.is_none() && m.is_none() {
                out.push_str("Provide: moles_solute + volume_L (find molarity), or three of C1/V1/C2/V2 (dilution).\n");
            }
        }
    }
    out.trim_end().to_string()
}

fn action_ph(args: &Value) -> String {
    let mut out = String::from("pH CALCULATIONS\n===============\n\n");
    // Direct pH/pOH/[H+]/[OH-] conversions
    if let Some(ph) = args.get("pH_value").and_then(|v| v.as_f64()) {
        let poh = 14.0 - ph;
        let h_conc = 10f64.powf(-ph);
        let oh_conc = 10f64.powf(-poh);
        out.push_str(&format!("pH         = {:.4}\n", ph));
        out.push_str(&format!("pOH        = {:.4}\n", poh));
        out.push_str(&format!("[H\u{207A}]       = {:.4e} mol/L\n", h_conc));
        out.push_str(&format!("[OH\u{207B}]      = {:.4e} mol/L\n", oh_conc));
        out.push_str(&format!(
            "Acidic/Basic: {}\n",
            if ph < 7.0 {
                "Acidic"
            } else if ph > 7.0 {
                "Basic"
            } else {
                "Neutral"
            }
        ));
        return out.trim_end().to_string();
    }
    // Henderson-Hasselbalch
    let ka = args.get("Ka").and_then(|v| v.as_f64());
    let kb = args.get("Kb").and_then(|v| v.as_f64());
    let acid = args.get("acid_conc").and_then(|v| v.as_f64());
    let base = args.get("base_conc").and_then(|v| v.as_f64());
    if let Some(ka) = ka {
        let pka = -ka.log10();
        out.push_str(&format!("Ka = {:.4e}  \u{2192}  pKa = {:.4}\n", ka, pka));
        if let (Some(a), Some(b)) = (acid, base) {
            let ph = pka + (b / a).log10();
            out.push_str("Henderson-Hasselbalch: pH = pKa + log([A\u{207B}]/[HA])\n");
            out.push_str(&format!(
                "pH = {:.4} + log({:.4}/{:.4}) = {:.4}\n",
                pka, b, a, ph
            ));
        } else {
            out.push_str("Provide acid_conc and base_conc for Henderson-Hasselbalch.\n");
        }
    } else if let Some(kb) = kb {
        let pkb = -kb.log10();
        let pka = 14.0 - pkb;
        let ka_val = 10f64.powf(-pka);
        out.push_str(&format!(
            "Kb = {:.4e}  \u{2192}  pKb = {:.4}  \u{2192}  pKa = {:.4}  \u{2192}  Ka = {:.4e}\n",
            kb, pkb, pka, ka_val
        ));
    } else {
        out.push_str("Provide one of:\n");
        out.push_str("  pH_value          \u{2192} pH/pOH/[H+]/[OH-] breakdown\n");
        out.push_str("  Ka                \u{2192} pKa conversion\n");
        out.push_str("  Ka + acid_conc + base_conc \u{2192} Henderson-Hasselbalch buffer pH\n");
        out.push_str("  Kb                \u{2192} pKb/pKa/Ka conversion\n");
    }
    out.trim_end().to_string()
}

fn action_gas(args: &Value) -> String {
    const R: f64 = 8.314_462_618;
    let p = args.get("P").and_then(|v| v.as_f64());
    let mut v = args.get("V").and_then(|v_val| v_val.as_f64());
    let n = args.get("n").and_then(|v| v.as_f64());
    let t = args.get("T").and_then(|v| v.as_f64());
    let unit = args.get("unit").and_then(|v| v.as_str()).unwrap_or("m3");
    if unit == "L" {
        v = v.map(|vl| vl / 1000.0);
    }
    let mut out = String::from("IDEAL GAS LAW: PV = nRT\n=======================\n");
    out.push_str("R = 8.314 J/(mol\u{00B7}K)\n\n");
    let v_unit = if unit == "L" { "L" } else { "m\u{00B3}" };
    match (p, v, n, t) {
        (None, Some(v), Some(n), Some(t)) => {
            let p_calc = n * R * t / v;
            let v_disp = if unit == "L" { v * 1000.0 } else { v };
            out.push_str(&format!(
                "Given: V={:.4}{}, n={:.4}mol, T={:.2}K\n",
                v_disp, v_unit, n, t
            ));
            out.push_str(&format!(
                "P = nRT/V = {:.4} Pa = {:.4} atm = {:.4} kPa\n",
                p_calc,
                p_calc / 101325.0,
                p_calc / 1000.0
            ));
        }
        (Some(p), None, Some(n), Some(t)) => {
            let v_calc = n * R * t / p;
            let v_disp = if unit == "L" { v_calc * 1000.0 } else { v_calc };
            out.push_str(&format!(
                "Given: P={:.4}Pa, n={:.4}mol, T={:.2}K\n",
                p, n, t
            ));
            out.push_str(&format!(
                "V = nRT/P = {:.6} {} = {:.4} mL\n",
                v_disp,
                v_unit,
                v_calc * 1e6
            ));
        }
        (Some(p), Some(v), None, Some(t)) => {
            let v_disp = if unit == "L" { v * 1000.0 } else { v };
            let n_calc = p * v / (R * t);
            out.push_str(&format!(
                "Given: P={:.4}Pa, V={:.4}{}, T={:.2}K\n",
                p, v_disp, v_unit, t
            ));
            out.push_str(&format!(
                "n = PV/(RT) = {:.6} mol = {:.4} mmol\n",
                n_calc,
                n_calc * 1000.0
            ));
        }
        (Some(p), Some(v), Some(n), None) => {
            let v_disp = if unit == "L" { v * 1000.0 } else { v };
            let t_calc = p * v / (n * R);
            out.push_str(&format!(
                "Given: P={:.4}Pa, V={:.4}{}, n={:.4}mol\n",
                p, v_disp, v_unit, n
            ));
            out.push_str(&format!(
                "T = PV/(nR) = {:.4} K = {:.2} \u{00B0}C\n",
                t_calc,
                t_calc - 273.15
            ));
        }
        _ => {
            out.push_str(&format!(
                "Provide three of: P (Pa), V ({} or L with unit='L'), n (mol), T (K)\n",
                v_unit
            ));
        }
    }
    out.trim_end().to_string()
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("balance");
    Ok(match action {
        "balance" => action_balance(args),
        "stoichiometry" => action_stoichiometry(args),
        "solution" => action_solution(args),
        "ph" => action_ph(args),
        "gas" => action_gas(args),
        other => format!(
            "Unknown action '{}'. Use: balance, stoichiometry, solution, ph, gas",
            other
        ),
    })
}
