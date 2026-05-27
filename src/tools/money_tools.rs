use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("compound_interest");
    match action {
        "compound_interest" => compound_interest_action(args),
        "loan" => loan_action(args),
        "apr_to_apy" => apr_to_apy_action(args),
        "discount" => discount_action(args),
        "percent_of" => percent_of_action(args),
        "format_currency" => format_currency_action(args),
        "tip" => tip_action(args),
        "split_bill" => split_bill_action(args),
        other => Err(format!(
            "money_tools: unknown action '{other}'. Valid: compound_interest, loan, \
             apr_to_apy, discount, percent_of, format_currency, tip, split_bill"
        )),
    }
}

fn get_f64(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

fn fmt_money(amount: f64, symbol: &str, decimals: usize) -> String {
    let factor = 10f64.powi(decimals as i32);
    let rounded = (amount * factor).round() / factor;
    let whole = rounded.abs().trunc() as u64;
    let frac = ((rounded.abs() - rounded.abs().trunc()) * factor).round() as u64;
    let sign = if amount < 0.0 { "-" } else { "" };
    // Thousands separators
    let whole_str = whole.to_string();
    let with_commas = add_thousands(whole_str.as_str());
    if decimals == 0 {
        format!("{sign}{symbol}{with_commas}")
    } else {
        format!(
            "{sign}{symbol}{with_commas}.{:0>width$}",
            frac,
            width = decimals
        )
    }
}

fn add_thousands(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

fn compound_interest_action(args: &Value) -> Result<String, String> {
    let principal =
        get_f64(args, "principal").ok_or("money_tools compound_interest: 'principal' required")?;
    let rate =
        get_f64(args, "rate").ok_or("money_tools compound_interest: 'rate' (annual %) required")?;
    let periods = get_f64(args, "periods")
        .ok_or("money_tools compound_interest: 'periods' (years) required")?;
    let n = get_f64(args, "n").unwrap_or(1.0);

    if n <= 0.0 {
        return Err("money_tools compound_interest: 'n' must be > 0".to_string());
    }

    let r = rate / 100.0;
    let final_amount = principal * (1.0 + r / n).powf(n * periods);
    let interest_earned = final_amount - principal;

    let effective_rate = ((1.0 + r / n).powf(n) - 1.0) * 100.0;

    let symbol = "$";
    let mut out = format!("Compound Interest\n\n");
    out.push_str(&format!(
        "  Principal:          {}\n",
        fmt_money(principal, symbol, 2)
    ));
    out.push_str(&format!("  Annual Rate:        {rate:.4}%\n"));
    out.push_str(&format!("  Compounds/Year:     {n}\n"));
    out.push_str(&format!("  Term:               {periods} year(s)\n"));
    out.push_str(&format!(
        "  Effective Rate:     {effective_rate:.4}% per year\n\n"
    ));
    out.push_str(&format!(
        "  Final Amount:       {}\n",
        fmt_money(final_amount, symbol, 2)
    ));
    out.push_str(&format!(
        "  Interest Earned:    {}\n",
        fmt_money(interest_earned, symbol, 2)
    ));
    out.push_str(&format!(
        "  Growth:             {:.2}x principal\n",
        final_amount / principal
    ));
    Ok(out)
}

fn loan_action(args: &Value) -> Result<String, String> {
    let principal = get_f64(args, "principal").ok_or("money_tools loan: 'principal' required")?;
    let annual_rate = get_f64(args, "annual_rate")
        .ok_or("money_tools loan: 'annual_rate' (annual %) required")?;
    let term_months = get_f64(args, "term_months")
        .ok_or("money_tools loan: 'term_months' required")?
        .round() as u32;

    if term_months == 0 {
        return Err("money_tools loan: 'term_months' must be > 0".to_string());
    }

    let symbol = "$";

    if annual_rate == 0.0 {
        let monthly = principal / term_months as f64;
        let mut out = format!("Loan (0% interest)\n\n");
        out.push_str(&format!(
            "  Principal:        {}\n",
            fmt_money(principal, symbol, 2)
        ));
        out.push_str(&format!(
            "  Monthly Payment:  {}\n",
            fmt_money(monthly, symbol, 2)
        ));
        out.push_str(&format!("  Term:             {} months\n", term_months));
        out.push_str(&format!(
            "  Total Paid:       {}\n",
            fmt_money(principal, symbol, 2)
        ));
        out.push_str("  Total Interest:   $0.00\n");
        return Ok(out);
    }

    let monthly_rate = annual_rate / 100.0 / 12.0;
    let factor = (1.0 + monthly_rate).powi(term_months as i32);
    let monthly_payment = principal * monthly_rate * factor / (factor - 1.0);
    let total_paid = monthly_payment * term_months as f64;
    let total_interest = total_paid - principal;

    // Amortization milestones
    let mut balance = principal;
    let mut _total_interest_paid = 0.0;
    let mut halfway_month = 0u32;
    let mut halfway_balance = 0.0;

    for m in 1..=term_months {
        let interest_payment = balance * monthly_rate;
        let principal_payment = monthly_payment - interest_payment;
        _total_interest_paid += interest_payment;
        balance -= principal_payment;
        if m == term_months / 2 && halfway_month == 0 {
            halfway_month = m;
            halfway_balance = balance.max(0.0);
        }
    }

    let mut out = format!("Loan Amortization\n\n");
    out.push_str(&format!(
        "  Principal:        {}\n",
        fmt_money(principal, symbol, 2)
    ));
    out.push_str(&format!("  Annual Rate:      {annual_rate:.4}%\n"));
    out.push_str(&format!(
        "  Term:             {term_months} months ({:.1} years)\n",
        term_months as f64 / 12.0
    ));
    out.push_str(&format!(
        "\n  Monthly Payment:  {}\n",
        fmt_money(monthly_payment, symbol, 2)
    ));
    out.push_str(&format!(
        "  Total Paid:       {}\n",
        fmt_money(total_paid, symbol, 2)
    ));
    out.push_str(&format!(
        "  Total Interest:   {}\n",
        fmt_money(total_interest, symbol, 2)
    ));
    out.push_str(&format!(
        "  Interest Ratio:   {:.1}% of total cost\n",
        total_interest / total_paid * 100.0
    ));
    if halfway_month > 0 {
        out.push_str(&format!(
            "\n  At month {halfway_month} (halfway): remaining balance {}\n",
            fmt_money(halfway_balance, symbol, 2)
        ));
    }
    Ok(out)
}

fn apr_to_apy_action(args: &Value) -> Result<String, String> {
    let apr = get_f64(args, "apr").ok_or("money_tools apr_to_apy: 'apr' (%) required")?;
    let n = get_f64(args, "n").unwrap_or(12.0);
    if n <= 0.0 {
        return Err("money_tools apr_to_apy: 'n' must be > 0".to_string());
    }

    let apy = ((1.0 + apr / 100.0 / n).powf(n) - 1.0) * 100.0;

    let mut out = format!("APR to APY\n\n");
    out.push_str(&format!("  APR:                {apr:.4}%\n"));
    out.push_str(&format!("  Compounds/Year:     {n}\n"));
    out.push_str(&format!("  APY (effective):    {apy:.4}%\n"));
    out.push_str(&format!("  Difference:         {:.4}%\n", apy - apr));
    Ok(out)
}

fn discount_action(args: &Value) -> Result<String, String> {
    let price =
        get_f64(args, "price").ok_or("money_tools discount: 'price' (original) required")?;
    let percent =
        get_f64(args, "percent").ok_or("money_tools discount: 'percent' (% off) required")?;

    if !(0.0..=100.0).contains(&percent) {
        return Err(format!(
            "money_tools discount: 'percent' must be 0–100, got {percent}"
        ));
    }

    let symbol = "$";
    let savings = price * percent / 100.0;
    let sale_price = price - savings;

    let mut out = format!("Discount\n\n");
    out.push_str(&format!(
        "  Original Price:   {}\n",
        fmt_money(price, symbol, 2)
    ));
    out.push_str(&format!("  Discount:         {percent:.1}% off\n"));
    out.push_str(&format!(
        "  Savings:          {}\n",
        fmt_money(savings, symbol, 2)
    ));
    out.push_str(&format!(
        "  Sale Price:       {}\n",
        fmt_money(sale_price, symbol, 2)
    ));
    Ok(out)
}

fn percent_of_action(args: &Value) -> Result<String, String> {
    // Mode 1: what percent is 'a' of 'b'?
    if let (Some(a), Some(b)) = (
        get_f64(args, "a").or_else(|| get_f64(args, "value")),
        get_f64(args, "b").or_else(|| get_f64(args, "total")),
    ) {
        if b == 0.0 {
            return Err("money_tools percent_of: 'b'/'total' must not be zero".to_string());
        }
        let pct = a / b * 100.0;
        let mut out = format!("Percent Of\n\n");
        out.push_str(&format!("  {a} is {pct:.4}% of {b}\n"));
        out.push_str(&format!(
            "  {b} is {:.4}% more than {a}\n",
            (b - a) / a * 100.0
        ));
        return Ok(out);
    }

    // Mode 2: what is 'percent'% of 'of'?
    if let (Some(percent), Some(of)) = (get_f64(args, "percent"), get_f64(args, "of")) {
        let result = percent / 100.0 * of;
        let mut out = format!("Percent Of\n\n");
        out.push_str(&format!("  {percent}% of {of} = {result:.4}\n"));
        return Ok(out);
    }

    Err(
        "money_tools percent_of: provide ('a'/'value' + 'b'/'total') or ('percent' + 'of')"
            .to_string(),
    )
}

fn format_currency_action(args: &Value) -> Result<String, String> {
    let amount = get_f64(args, "amount").ok_or("money_tools format_currency: 'amount' required")?;
    let symbol = args.get("symbol").and_then(|v| v.as_str()).unwrap_or("$");
    let decimals = args.get("decimals").and_then(|v| v.as_u64()).unwrap_or(2) as usize;

    let formatted = fmt_money(amount, symbol, decimals);
    Ok(format!("{formatted}\n"))
}

fn tip_action(args: &Value) -> Result<String, String> {
    let bill = get_f64(args, "bill").ok_or("money_tools tip: 'bill' required")?;
    let tip_percent = get_f64(args, "tip_percent").unwrap_or(18.0);
    let people = get_f64(args, "people").unwrap_or(1.0).max(1.0);
    let symbol = "$";

    let tip_amount = bill * tip_percent / 100.0;
    let total = bill + tip_amount;
    let per_person_total = total / people;
    let per_person_tip = tip_amount / people;

    let mut out = format!("Tip Calculator\n\n");
    out.push_str(&format!(
        "  Bill:               {}\n",
        fmt_money(bill, symbol, 2)
    ));
    out.push_str(&format!(
        "  Tip:                {tip_percent:.1}% = {}\n",
        fmt_money(tip_amount, symbol, 2)
    ));
    out.push_str(&format!(
        "  Total:              {}\n",
        fmt_money(total, symbol, 2)
    ));
    if people > 1.0 {
        out.push_str(&format!("  People:             {}\n", people as u32));
        out.push_str(&format!(
            "  Per Person (tip):   {}\n",
            fmt_money(per_person_tip, symbol, 2)
        ));
        out.push_str(&format!(
            "  Per Person (total): {}\n",
            fmt_money(per_person_total, symbol, 2)
        ));
    }
    Ok(out)
}

fn split_bill_action(args: &Value) -> Result<String, String> {
    let total =
        get_f64(args, "total").ok_or("money_tools split_bill: 'total' (bill total) required")?;
    let people = get_f64(args, "people")
        .ok_or("money_tools split_bill: 'people' required")?
        .max(1.0);
    let tip_percent = get_f64(args, "tip_percent").unwrap_or(0.0);
    let symbol = "$";

    let tip_amount = total * tip_percent / 100.0;
    let grand_total = total + tip_amount;
    let per_person = grand_total / people;

    let mut out = format!("Split Bill\n\n");
    out.push_str(&format!(
        "  Bill Total:         {}\n",
        fmt_money(total, symbol, 2)
    ));
    if tip_percent > 0.0 {
        out.push_str(&format!(
            "  Tip ({tip_percent:.1}%):          {}\n",
            fmt_money(tip_amount, symbol, 2)
        ));
        out.push_str(&format!(
            "  Grand Total:        {}\n",
            fmt_money(grand_total, symbol, 2)
        ));
    }
    out.push_str(&format!("  People:             {}\n", people as u32));
    out.push_str(&format!(
        "  Per Person:         {}\n",
        fmt_money(per_person, symbol, 2)
    ));
    Ok(out)
}
