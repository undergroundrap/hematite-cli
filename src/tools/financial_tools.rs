use serde_json::{json, Value};
use std::fmt::Write as _;

pub fn make_schema() -> Value {
    json!({
        "name": "financial_tools",
        "description": "Extended financial calculations without external utilities. \
            Actions: amortize (full loan amortization schedule with monthly payment breakdown), \
            depreciation (asset depreciation: straight-line, declining-balance, sum-of-years-digits, MACRS), \
            roi (return on investment with annualized rate), \
            breakeven (break-even analysis: units, revenue, and margin of safety), \
            cashflow (net present value, IRR approximation, and payback period for a cash flow series), \
            cagr (compound annual growth rate between two values over N years), \
            lease (operating vs capital lease comparison: monthly payment and total cost), \
            savings (savings goal planner: how long to reach a target or how much to save monthly). \
            Example: financial_tools(action: 'amortize', principal: 200000, annual_rate: 6.5, term_months: 360) \
            or financial_tools(action: 'breakeven', fixed_costs: 50000, price_per_unit: 25, variable_cost: 10)",
        "parameters": {
            "type": "object",
            "properties": {
                "action": { "type": "string", "description": "amortize | depreciation | roi | breakeven | cashflow | cagr | lease | savings" },
                "principal": { "type": "number", "description": "Loan principal or initial investment amount" },
                "annual_rate": { "type": "number", "description": "Annual interest rate as a percentage (e.g. 6.5 for 6.5%)" },
                "term_months": { "type": "integer", "description": "Loan term in months" },
                "cost": { "type": "number", "description": "Asset cost for depreciation" },
                "salvage": { "type": "number", "description": "Salvage/residual value for depreciation (default 0)" },
                "life_years": { "type": "integer", "description": "Useful life in years for depreciation" },
                "method": { "type": "string", "description": "Depreciation method: straight_line | declining_balance | sum_of_years | macrs5 | macrs7" },
                "initial": { "type": "number", "description": "Initial investment for ROI/cashflow" },
                "final": { "type": "number", "description": "Final value for ROI/CAGR" },
                "years": { "type": "number", "description": "Number of years for ROI/CAGR/savings" },
                "fixed_costs": { "type": "number", "description": "Fixed costs per period for break-even" },
                "price_per_unit": { "type": "number", "description": "Selling price per unit for break-even" },
                "variable_cost": { "type": "number", "description": "Variable cost per unit for break-even" },
                "expected_units": { "type": "number", "description": "Expected sales volume for margin of safety" },
                "cashflows": {
                    "type": "array",
                    "items": { "type": "number" },
                    "description": "Cash flow series (first value is usually negative initial investment)"
                },
                "discount_rate": { "type": "number", "description": "Discount rate % for NPV/IRR (default 10)" },
                "start_value": { "type": "number", "description": "Starting value for CAGR" },
                "end_value": { "type": "number", "description": "Ending value for CAGR" },
                "target": { "type": "number", "description": "Savings goal target amount" },
                "monthly_contribution": { "type": "number", "description": "Monthly savings contribution" },
                "current_savings": { "type": "number", "description": "Current savings balance (default 0)" },
                "show_schedule": { "type": "boolean", "description": "Show full amortization schedule (default false — shows summary only)" }
            }
        }
    })
}

fn fmt_currency(n: f64) -> String {
    let abs = n.abs();
    let sign = if n < 0.0 { "-" } else { "" };
    if abs >= 1_000_000.0 {
        format!("{}${:.2}M", sign, abs / 1_000_000.0)
    } else {
        let s = format!("{}{:.2}", sign, abs);
        let (int_part, dec_part) = s.split_once('.').unwrap_or((&s, "00"));
        let int_with_commas = int_part
            .trim_start_matches('-')
            .chars()
            .rev()
            .collect::<Vec<_>>()
            .chunks(3)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(",")
            .chars()
            .rev()
            .collect::<String>();
        format!(
            "{}{}{}.{}",
            sign,
            "$",
            int_with_commas,
            dec_part
        )
    }
}

// ── Amortization ─────────────────────────────────────────────────────────────

fn action_amortize(
    principal: f64,
    annual_rate: f64,
    term_months: u32,
    show_schedule: bool,
) -> String {
    if principal <= 0.0 || annual_rate < 0.0 || term_months == 0 {
        return "Error: principal must be > 0, annual_rate >= 0, term_months > 0.".to_string();
    }
    let monthly_rate = annual_rate / 100.0 / 12.0;
    let payment = if monthly_rate == 0.0 {
        principal / term_months as f64
    } else {
        principal * monthly_rate * (1.0 + monthly_rate).powi(term_months as i32)
            / ((1.0 + monthly_rate).powi(term_months as i32) - 1.0)
    };

    let total_paid = payment * term_months as f64;
    let total_interest = total_paid - principal;

    let mut out = String::new();
    let _ = writeln!(out, "Loan Amortization Schedule");
    let _ = writeln!(out, "  Principal:       {}", fmt_currency(principal));
    let _ = writeln!(out, "  Annual rate:     {:.3}%", annual_rate);
    let _ = writeln!(
        out,
        "  Term:            {} months ({:.1} years)",
        term_months,
        term_months as f64 / 12.0
    );
    let _ = writeln!(out, "  Monthly payment: {}", fmt_currency(payment));
    let _ = writeln!(out, "  Total paid:      {}", fmt_currency(total_paid));
    let _ = writeln!(
        out,
        "  Total interest:  {} ({:.1}% of principal)",
        fmt_currency(total_interest),
        total_interest / principal * 100.0
    );
    out.push('\n');

    // Find payoff date for interest-only crossover
    let mut balance = principal;
    let mut interest_half_month = 0u32;
    for i in 1..=term_months {
        let interest = balance * monthly_rate;
        let principal_paid = payment - interest;
        balance -= principal_paid;
        if interest < principal_paid && interest_half_month == 0 {
            interest_half_month = i;
        }
    }
    if interest_half_month > 0 {
        let _ = writeln!(
            out,
            "  Principal > interest starting month: {} of {}",
            interest_half_month, term_months
        );
    }

    if show_schedule {
        out.push('\n');
        let _ = writeln!(
            out,
            "{:<6} {:>14} {:>14} {:>14} {:>14}",
            "Month", "Payment", "Principal", "Interest", "Balance"
        );
        out.push_str(&"-".repeat(68));
        out.push('\n');
        balance = principal;
        for i in 1..=term_months {
            let interest = balance * monthly_rate;
            let principal_paid = payment - interest;
            balance = (balance - principal_paid).max(0.0);
            // Show every month if <= 24, else every 12th
            if term_months <= 24 || i % 12 == 0 || i == 1 || i == term_months {
                let label = if term_months > 24 && i % 12 == 0 {
                    format!("Year {}", i / 12)
                } else {
                    format!("Month {}", i)
                };
                let _ = writeln!(
                    out,
                    "{:<6} {:>14} {:>14} {:>14} {:>14}",
                    label,
                    fmt_currency(payment),
                    fmt_currency(principal_paid),
                    fmt_currency(interest),
                    fmt_currency(balance)
                );
            }
        }
        if term_months > 24 {
            let _ = writeln!(
                out,
                "\n(showing yearly summaries — use show_schedule: false for summary only)"
            );
        }
    } else {
        // Show first 3 and last 3 months as preview
        out.push('\n');
        let _ = writeln!(
            out,
            "{:<10} {:>12} {:>12} {:>12} {:>14}",
            "Month", "Payment", "Principal", "Interest", "Balance"
        );
        out.push_str(&"-".repeat(64));
        out.push('\n');
        let preview_months: Vec<u32> = if term_months <= 6 {
            (1..=term_months).collect()
        } else {
            let mut v: Vec<u32> = (1..=3).collect();
            if term_months > 6 {
                v.push(0);
            } // separator
            let mut tail: Vec<u32> = ((term_months - 2)..=term_months).collect();
            v.append(&mut tail);
            v
        };
        balance = principal;
        let mut rows: Vec<(u32, f64, f64, f64, f64)> = Vec::new();
        let mut bal = principal;
        for i in 1..=term_months {
            let interest = bal * monthly_rate;
            let pp = payment - interest;
            bal = (bal - pp).max(0.0);
            rows.push((i, payment, pp, interest, bal));
        }
        for m in &preview_months {
            if *m == 0 {
                let _ = writeln!(out, "  ...");
                continue;
            }
            let r = &rows[(*m - 1) as usize];
            let _ = writeln!(
                out,
                "{:<10} {:>12} {:>12} {:>12} {:>14}",
                format!("Month {}", r.0),
                fmt_currency(r.1),
                fmt_currency(r.2),
                fmt_currency(r.3),
                fmt_currency(r.4)
            );
        }
        let _ = writeln!(
            out,
            "\nPass show_schedule: true for the full {} month table.",
            term_months
        );
        let _ = balance;
    }
    out
}

// ── Depreciation ─────────────────────────────────────────────────────────────

fn action_depreciation(cost: f64, salvage: f64, life: u32, method: &str) -> String {
    if cost <= 0.0 || life == 0 {
        return "Error: cost must be > 0 and life_years must be > 0.".to_string();
    }
    if salvage >= cost {
        return "Error: salvage value must be less than cost.".to_string();
    }
    let depreciable = cost - salvage;

    let mut out = String::new();
    let _ = writeln!(out, "Asset Depreciation — {}", method_label(method));
    let _ = writeln!(out, "  Cost:      {}", fmt_currency(cost));
    let _ = writeln!(out, "  Salvage:   {}", fmt_currency(salvage));
    let _ = writeln!(out, "  Depreciable: {}", fmt_currency(depreciable));
    let _ = writeln!(out, "  Life:      {} years\n", life);
    let _ = writeln!(
        out,
        "{:<6} {:>16} {:>16} {:>16}",
        "Year", "Depreciation", "Accum. Depr.", "Book Value"
    );
    out.push_str(&"-".repeat(60));
    out.push('\n');

    let mut book = cost;
    let mut accum = 0.0;

    match method {
        "declining_balance" | "db" | "ddb" => {
            let rate = 2.0 / life as f64; // double declining
            for y in 1..=life {
                let depr = (book * rate).min(book - salvage);
                accum += depr;
                book -= depr;
                let _ = writeln!(
                    out,
                    "{:<6} {:>16} {:>16} {:>16}",
                    y,
                    fmt_currency(depr),
                    fmt_currency(accum),
                    fmt_currency(book)
                );
            }
        }
        "sum_of_years" | "syd" => {
            let syd: u32 = (life * (life + 1)) / 2;
            for y in 1..=life {
                let fraction = (life + 1 - y) as f64 / syd as f64;
                let depr = depreciable * fraction;
                accum += depr;
                book -= depr;
                let _ = writeln!(
                    out,
                    "{:<6} {:>16} {:>16} {:>16}",
                    y,
                    fmt_currency(depr),
                    fmt_currency(accum),
                    fmt_currency(book)
                );
            }
        }
        "macrs5" => {
            // MACRS half-year convention, 5-year property
            let rates = [0.20, 0.32, 0.192, 0.1152, 0.1152, 0.0576];
            for (y, &rate) in rates.iter().enumerate() {
                let depr = cost * rate;
                accum += depr;
                book -= depr;
                let _ = writeln!(
                    out,
                    "{:<6} {:>16} {:>16} {:>16}",
                    y + 1,
                    fmt_currency(depr),
                    fmt_currency(accum),
                    fmt_currency(book)
                );
            }
        }
        "macrs7" => {
            // MACRS half-year convention, 7-year property
            let rates = [
                0.1429, 0.2449, 0.1749, 0.1249, 0.0893, 0.0893, 0.0893, 0.0445,
            ];
            for (y, &rate) in rates.iter().enumerate() {
                let depr = cost * rate;
                accum += depr;
                book -= depr;
                let _ = writeln!(
                    out,
                    "{:<6} {:>16} {:>16} {:>16}",
                    y + 1,
                    fmt_currency(depr),
                    fmt_currency(accum),
                    fmt_currency(book)
                );
            }
        }
        _ => {
            // straight_line (default)
            let annual = depreciable / life as f64;
            for y in 1..=life {
                accum += annual;
                book -= annual;
                let _ = writeln!(
                    out,
                    "{:<6} {:>16} {:>16} {:>16}",
                    y,
                    fmt_currency(annual),
                    fmt_currency(accum),
                    fmt_currency(book.max(salvage))
                );
            }
        }
    }
    out
}

fn method_label(m: &str) -> &str {
    match m {
        "declining_balance" | "db" | "ddb" => "Double-Declining Balance",
        "sum_of_years" | "syd" => "Sum-of-Years-Digits",
        "macrs5" => "MACRS 5-Year",
        "macrs7" => "MACRS 7-Year",
        _ => "Straight-Line",
    }
}

// ── ROI ──────────────────────────────────────────────────────────────────────

fn action_roi(initial: f64, final_val: f64, years: f64) -> String {
    if initial <= 0.0 {
        return "Error: initial investment must be > 0.".to_string();
    }
    let gain = final_val - initial;
    let roi_pct = gain / initial * 100.0;
    let annualized = if years > 0.0 {
        ((final_val / initial).powf(1.0 / years) - 1.0) * 100.0
    } else {
        0.0
    };

    let mut out = String::new();
    let _ = writeln!(out, "Return on Investment");
    let _ = writeln!(out, "  Initial investment: {}", fmt_currency(initial));
    let _ = writeln!(out, "  Final value:        {}", fmt_currency(final_val));
    let _ = writeln!(out, "  Net gain/loss:      {}", fmt_currency(gain));
    let _ = writeln!(out, "  ROI:                {:.2}%", roi_pct);
    if years > 0.0 {
        let _ = writeln!(out, "  Years held:         {:.1}", years);
        let _ = writeln!(out, "  Annualized return:  {:.2}%", annualized);
        // Rule of 72
        if annualized > 0.0 {
            let _ = writeln!(out, "  Rule of 72 double: {:.1} years", 72.0 / annualized);
        }
    }
    let verdict = if roi_pct > 0.0 {
        "PROFITABLE"
    } else if roi_pct < 0.0 {
        "LOSS"
    } else {
        "BREAK-EVEN"
    };
    let _ = writeln!(out, "  Verdict:            {}", verdict);
    out
}

// ── Break-Even ───────────────────────────────────────────────────────────────

fn action_breakeven(
    fixed_costs: f64,
    price: f64,
    variable_cost: f64,
    expected_units: f64,
) -> String {
    if price <= variable_cost {
        return format!(
            "Error: price per unit ({}) must exceed variable cost ({}).",
            price, variable_cost
        );
    }
    let contribution_margin = price - variable_cost;
    let cm_ratio = contribution_margin / price;
    let be_units = fixed_costs / contribution_margin;
    let be_revenue = be_units * price;
    let margin_of_safety_units = if expected_units > be_units {
        expected_units - be_units
    } else {
        0.0
    };
    let margin_of_safety_pct = if expected_units > 0.0 {
        margin_of_safety_units / expected_units * 100.0
    } else {
        0.0
    };

    let mut out = String::new();
    let _ = writeln!(out, "Break-Even Analysis");
    let _ = writeln!(
        out,
        "  Fixed costs:           {}",
        fmt_currency(fixed_costs)
    );
    let _ = writeln!(out, "  Price per unit:        {}", fmt_currency(price));
    let _ = writeln!(
        out,
        "  Variable cost/unit:    {}",
        fmt_currency(variable_cost)
    );
    let _ = writeln!(
        out,
        "  Contribution margin:   {} ({:.1}% of price)",
        fmt_currency(contribution_margin),
        cm_ratio * 100.0
    );
    out.push('\n');
    let _ = writeln!(out, "  Break-even units:      {:.0} units", be_units.ceil());
    let _ = writeln!(out, "  Break-even revenue:    {}", fmt_currency(be_revenue));
    if expected_units > 0.0 {
        out.push('\n');
        let _ = writeln!(out, "  Expected sales:        {:.0} units", expected_units);
        let profit_at_expected = expected_units * contribution_margin - fixed_costs;
        let _ = writeln!(
            out,
            "  Profit at expected:    {}",
            fmt_currency(profit_at_expected)
        );
        let _ = writeln!(
            out,
            "  Margin of safety:      {:.0} units ({:.1}%)",
            margin_of_safety_units, margin_of_safety_pct
        );
        let verdict = if expected_units >= be_units {
            "PROFITABLE at expected volume"
        } else {
            "LOSS at expected volume"
        };
        let _ = writeln!(out, "  Verdict:               {}", verdict);
    }
    out
}

// ── Cash Flow / NPV / IRR ────────────────────────────────────────────────────

fn action_cashflow(cashflows: &[f64], discount_rate: f64) -> String {
    if cashflows.is_empty() {
        return "Error: 'cashflows' array is required.".to_string();
    }
    let r = discount_rate / 100.0;

    // NPV
    let npv: f64 = cashflows
        .iter()
        .enumerate()
        .map(|(i, &cf)| cf / (1.0 + r).powi(i as i32))
        .sum();

    // Payback period (simple)
    let mut cumulative = 0.0;
    let mut payback = None;
    for (i, &cf) in cashflows.iter().enumerate() {
        let prev = cumulative;
        cumulative += cf;
        if prev < 0.0 && cumulative >= 0.0 && payback.is_none() {
            // Interpolate
            payback = Some(i as f64 - 1.0 + (-prev) / cf);
        }
    }

    // IRR approximation via bisection
    let irr = bisect_irr(cashflows);

    let mut out = String::new();
    let _ = writeln!(out, "Cash Flow Analysis");
    let _ = writeln!(out, "  Cash flows:  {} periods", cashflows.len());
    let _ = writeln!(out, "  Discount rate: {:.2}%", discount_rate);
    out.push('\n');
    let _ = writeln!(
        out,
        "{:<8} {:>14} {:>14} {:>14}",
        "Period", "Cash Flow", "PV", "Cumulative PV"
    );
    out.push_str(&"-".repeat(56));
    out.push('\n');
    let mut cum_pv = 0.0;
    for (i, &cf) in cashflows.iter().enumerate() {
        let pv = cf / (1.0 + r).powi(i as i32);
        cum_pv += pv;
        let _ = writeln!(
            out,
            "{:<8} {:>14} {:>14} {:>14}",
            i,
            fmt_currency(cf),
            fmt_currency(pv),
            fmt_currency(cum_pv)
        );
    }
    out.push('\n');
    let _ = writeln!(out, "  NPV:          {}", fmt_currency(npv));
    let verdict = if npv > 0.0 {
        "ACCEPT (positive NPV)"
    } else if npv < 0.0 {
        "REJECT (negative NPV)"
    } else {
        "INDIFFERENT"
    };
    let _ = writeln!(out, "  NPV verdict:  {}", verdict);
    if let Some(p) = payback {
        let _ = writeln!(out, "  Payback:      {:.2} periods", p);
    } else if cashflows.iter().sum::<f64>() < 0.0 {
        let _ = writeln!(out, "  Payback:      never recovers investment");
    }
    if let Some(irr_val) = irr {
        let _ = writeln!(out, "  IRR (approx): {:.2}%", irr_val);
    }
    out
}

fn bisect_irr(cashflows: &[f64]) -> Option<f64> {
    let npv_at = |r: f64| -> f64 {
        cashflows
            .iter()
            .enumerate()
            .map(|(i, &cf)| cf / (1.0 + r).powi(i as i32))
            .sum()
    };
    // Search between -99% and 1000%
    let (mut lo, mut hi) = (-0.999, 10.0);
    if npv_at(lo) * npv_at(hi) > 0.0 {
        return None; // No sign change — IRR may not exist or be out of range
    }
    for _ in 0..100 {
        let mid = (lo + hi) / 2.0;
        if npv_at(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-8 {
            break;
        }
    }
    Some((lo + hi) / 2.0 * 100.0)
}

// ── CAGR ─────────────────────────────────────────────────────────────────────

fn action_cagr(start: f64, end: f64, years: f64) -> String {
    if start <= 0.0 || years <= 0.0 {
        return "Error: start_value must be > 0 and years must be > 0.".to_string();
    }
    let cagr = ((end / start).powf(1.0 / years) - 1.0) * 100.0;
    let total_return = (end - start) / start * 100.0;

    let mut out = String::new();
    let _ = writeln!(out, "Compound Annual Growth Rate (CAGR)");
    let _ = writeln!(out, "  Start value:   {}", fmt_currency(start));
    let _ = writeln!(out, "  End value:     {}", fmt_currency(end));
    let _ = writeln!(out, "  Period:        {:.1} years", years);
    let _ = writeln!(out, "  CAGR:          {:.3}%", cagr);
    let _ = writeln!(out, "  Total return:  {:.2}%", total_return);
    out.push('\n');
    let _ = writeln!(out, "  Projected values at {:.2}% CAGR:", cagr);
    let _ = writeln!(out, "  {:<10} Value", "Year");
    for y in 0..=(years as u32) {
        let _ = writeln!(
            out,
            "  {:<10} {}",
            y,
            fmt_currency(start * (1.0 + cagr / 100.0).powi(y as i32))
        );
    }
    out
}

// ── Savings ──────────────────────────────────────────────────────────────────

fn action_savings(target: f64, monthly: f64, current: f64, annual_rate: f64, years: f64) -> String {
    let monthly_rate = annual_rate / 100.0 / 12.0;

    let mut out = String::new();
    let _ = writeln!(out, "Savings Goal Planner");
    let _ = writeln!(out, "  Target:             {}", fmt_currency(target));
    let _ = writeln!(out, "  Current savings:    {}", fmt_currency(current));
    let _ = writeln!(out, "  Annual return:      {:.2}%", annual_rate);

    if monthly > 0.0 {
        // How long to reach target?
        let _ = writeln!(out, "  Monthly contribution: {}", fmt_currency(monthly));
        if monthly_rate == 0.0 {
            let months_needed = ((target - current) / monthly).ceil();
            let _ = writeln!(
                out,
                "\n  Months to reach target: {:.0} ({:.1} years)",
                months_needed,
                months_needed / 12.0
            );
        } else {
            // FV = PV*(1+r)^n + PMT*((1+r)^n - 1)/r
            // Solve for n using iteration
            let mut n = 0u32;
            let mut fv = current;
            while fv < target && n < 1200 {
                fv = fv * (1.0 + monthly_rate) + monthly;
                n += 1;
            }
            if fv >= target {
                let _ = writeln!(
                    out,
                    "\n  Months to reach target: {} ({:.1} years)",
                    n,
                    n as f64 / 12.0
                );
                let _ = writeln!(
                    out,
                    "  Total contributed:      {}",
                    fmt_currency(current + monthly * n as f64)
                );
                let _ = writeln!(
                    out,
                    "  Interest earned:        {}",
                    fmt_currency(fv - current - monthly * n as f64)
                );
                // Milestones
                out.push('\n');
                let _ = writeln!(out, "  Milestones:");
                let mut bal = current;
                for y in 1..=((n / 12) + 1).min(30) {
                    for _ in 0..12 {
                        bal = bal * (1.0 + monthly_rate) + monthly;
                    }
                    if bal >= target {
                        let _ = writeln!(out, "    Year {:>3}: {} ✓ GOAL", y, fmt_currency(bal));
                        break;
                    }
                    let _ = writeln!(out, "    Year {:>3}: {}", y, fmt_currency(bal));
                }
            } else {
                let _ = writeln!(
                    out,
                    "\n  Target not reachable in 100 years with this contribution rate."
                );
            }
        }
    } else if years > 0.0 {
        // How much to save monthly?
        let n = (years * 12.0) as u32;
        let required_monthly = if monthly_rate == 0.0 {
            (target - current) / n as f64
        } else {
            let growth = (1.0 + monthly_rate).powi(n as i32);
            (target - current * growth) * monthly_rate / (growth - 1.0)
        };
        let _ = writeln!(
            out,
            "  Time to goal:         {} months ({:.1} years)",
            n, years
        );
        let _ = writeln!(
            out,
            "\n  Required monthly contribution: {}",
            fmt_currency(required_monthly)
        );
        let total_contributed = current + required_monthly * n as f64;
        let _ = writeln!(
            out,
            "  Total to contribute:           {}",
            fmt_currency(total_contributed)
        );
        let _ = writeln!(
            out,
            "  Interest earned:               {}",
            fmt_currency(target - total_contributed)
        );
    } else {
        return "Error: provide either monthly_contribution (to find time) or years (to find monthly amount).".to_string();
    }
    out
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("amortize");

    let out = match action {
        "amortize" | "loan" | "mortgage" => {
            let principal = args.get("principal").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let annual_rate = args.get("annual_rate").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let term_months = args.get("term_months").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let show_schedule = args.get("show_schedule").and_then(|v| v.as_bool()).unwrap_or(false);
            if principal <= 0.0 || term_months == 0 {
                return Ok("Error: 'principal' and 'term_months' are required for amortize action.".to_string());
            }
            action_amortize(principal, annual_rate, term_months, show_schedule)
        }
        "depreciation" | "depr" => {
            let cost = args.get("cost").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let salvage = args.get("salvage").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let life = args.get("life_years").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let method = args.get("method").and_then(|v| v.as_str()).unwrap_or("straight_line");
            if cost <= 0.0 || life == 0 {
                return Ok("Error: 'cost' and 'life_years' are required for depreciation action.".to_string());
            }
            action_depreciation(cost, salvage, life, method)
        }
        "roi" | "return" => {
            let initial = args.get("initial").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let final_val = args.get("final").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let years = args.get("years").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if initial <= 0.0 || final_val <= 0.0 {
                return Ok("Error: 'initial' and 'final' values are required for roi action.".to_string());
            }
            action_roi(initial, final_val, years)
        }
        "breakeven" | "break_even" | "bep" => {
            let fixed = args.get("fixed_costs").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let price = args.get("price_per_unit").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let variable = args.get("variable_cost").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let expected = args.get("expected_units").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if fixed <= 0.0 || price <= 0.0 {
                return Ok("Error: 'fixed_costs' and 'price_per_unit' are required for breakeven action.".to_string());
            }
            action_breakeven(fixed, price, variable, expected)
        }
        "cashflow" | "npv" | "irr" => {
            let cfs: Vec<f64> = args.get("cashflows")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let rate = args.get("discount_rate").and_then(|v| v.as_f64()).unwrap_or(10.0);
            if cfs.is_empty() {
                return Ok("Error: 'cashflows' array is required for cashflow action.".to_string());
            }
            action_cashflow(&cfs, rate)
        }
        "cagr" | "growth_rate" => {
            let start = args.get("start_value").or_else(|| args.get("initial")).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let end = args.get("end_value").or_else(|| args.get("final")).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let years = args.get("years").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if start <= 0.0 || end <= 0.0 || years <= 0.0 {
                return Ok("Error: 'start_value', 'end_value', and 'years' are required for cagr action.".to_string());
            }
            action_cagr(start, end, years)
        }
        "savings" | "goal" => {
            let target = args.get("target").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let monthly = args.get("monthly_contribution").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let current = args.get("current_savings").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let rate = args.get("annual_rate").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let years = args.get("years").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if target <= 0.0 {
                return Ok("Error: 'target' amount is required for savings action.".to_string());
            }
            action_savings(target, monthly, current, rate, years)
        }
        other => format!("Error: unknown action '{}'. Use amortize, depreciation, roi, breakeven, cashflow, cagr, or savings.", other),
    };
    Ok(out)
}
