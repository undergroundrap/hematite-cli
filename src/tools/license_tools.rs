use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("a").is_some() && args.get("b").is_some() {
        "compare".to_string()
    } else if args.get("license").is_some()
        || args.get("id").is_some()
        || args.get("name").is_some()
    {
        "info".to_string()
    } else if args.get("text").is_some() || args.get("content").is_some() {
        "detect".to_string()
    } else {
        "list".to_string()
    };
    match action.as_str() {
        "info" => info_action(args),
        "detect" => detect_action(args),
        "compare" => compare_action(args),
        "list" => list_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: info, detect, compare, list",
            action
        )),
    }
}

#[derive(Debug, Clone)]
struct License {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    copyleft: bool,
    patent_grant: bool,
    trademark: bool,
    sublicensing: bool,
    commercial: bool,
    summary: &'static str,
    conditions: &'static [&'static str],
    permissions: &'static [&'static str],
    limitations: &'static [&'static str],
    keywords: &'static [&'static str],
}

static LICENSES: &[License] = &[
    License {
        id: "MIT",
        name: "MIT License",
        category: "Permissive",
        copyleft: false,
        patent_grant: false,
        trademark: false,
        sublicensing: true,
        commercial: true,
        summary: "Short and simple permissive license. Requires copyright notice preserved in copies.",
        conditions: &["Retain copyright notice", "Retain license text"],
        permissions: &["Commercial use", "Modification", "Distribution", "Private use"],
        limitations: &["No liability", "No warranty"],
        keywords: &["mit", "\"mit license\"", "permission is hereby granted", "the mit license"],
    },
    License {
        id: "Apache-2.0",
        name: "Apache License 2.0",
        category: "Permissive",
        copyleft: false,
        patent_grant: true,
        trademark: true,
        sublicensing: true,
        commercial: true,
        summary: "Permissive license with patent grant and trademark restrictions. Requires NOTICE file preservation.",
        conditions: &["Retain copyright notice", "State changes", "Include NOTICE file", "Retain license text"],
        permissions: &["Commercial use", "Modification", "Distribution", "Patent use", "Private use"],
        limitations: &["No liability", "No warranty", "No trademark use"],
        keywords: &["apache", "apache license", "apache-2.0", "version 2.0, january 2004"],
    },
    License {
        id: "GPL-2.0",
        name: "GNU General Public License v2.0",
        category: "Strong Copyleft",
        copyleft: true,
        patent_grant: false,
        trademark: false,
        sublicensing: false,
        commercial: true,
        summary: "Strong copyleft license. Derivative works must be distributed under GPL-2.0.",
        conditions: &["Source code disclosure", "Same license for derivatives", "State changes", "Retain copyright notice"],
        permissions: &["Commercial use", "Modification", "Distribution", "Private use"],
        limitations: &["No liability", "No warranty", "No sublicensing"],
        keywords: &["gpl-2.0", "gpl v2", "gnu general public license", "version 2", "gplv2"],
    },
    License {
        id: "GPL-3.0",
        name: "GNU General Public License v3.0",
        category: "Strong Copyleft",
        copyleft: true,
        patent_grant: true,
        trademark: false,
        sublicensing: false,
        commercial: true,
        summary: "Strong copyleft with patent protection. Designed to prevent tivoization.",
        conditions: &["Source code disclosure", "Same license for derivatives", "State changes", "Retain copyright notice", "Disclose installation instructions"],
        permissions: &["Commercial use", "Modification", "Distribution", "Patent use", "Private use"],
        limitations: &["No liability", "No warranty", "No sublicensing"],
        keywords: &["gpl-3.0", "gpl v3", "gplv3", "version 3", "gnu general public license"],
    },
    License {
        id: "LGPL-2.1",
        name: "GNU Lesser General Public License v2.1",
        category: "Weak Copyleft",
        copyleft: true,
        patent_grant: false,
        trademark: false,
        sublicensing: true,
        commercial: true,
        summary: "Weak copyleft designed for libraries. Allows linking from non-copyleft software.",
        conditions: &["Source code disclosure for modifications", "Same license for modifications", "State changes"],
        permissions: &["Commercial use", "Modification", "Distribution", "Private use", "Linking from non-copyleft"],
        limitations: &["No liability", "No warranty"],
        keywords: &["lgpl", "lesser gpl", "lgpl-2.1", "lgplv2", "lesser general public license"],
    },
    License {
        id: "LGPL-3.0",
        name: "GNU Lesser General Public License v3.0",
        category: "Weak Copyleft",
        copyleft: true,
        patent_grant: true,
        trademark: false,
        sublicensing: true,
        commercial: true,
        summary: "Weak copyleft with patent grant. Inherits GPL-3.0 permissions.",
        conditions: &["Source code disclosure for modifications", "Same license for modifications", "State changes"],
        permissions: &["Commercial use", "Modification", "Distribution", "Patent use", "Private use", "Linking from non-copyleft"],
        limitations: &["No liability", "No warranty"],
        keywords: &["lgpl-3.0", "lgplv3", "lesser general public license", "version 3"],
    },
    License {
        id: "MPL-2.0",
        name: "Mozilla Public License 2.0",
        category: "Weak Copyleft",
        copyleft: true,
        patent_grant: true,
        trademark: false,
        sublicensing: true,
        commercial: true,
        summary: "Copyleft applies at file level only. Modified files must be MPL-2.0; new files can be any license.",
        conditions: &["Source code disclosure for modified files", "State changes", "Retain copyright notice"],
        permissions: &["Commercial use", "Modification", "Distribution", "Patent use", "Private use"],
        limitations: &["No liability", "No warranty", "No trademark use"],
        keywords: &["mpl", "mozilla public license", "mpl-2.0", "mpl 2.0"],
    },
    License {
        id: "AGPL-3.0",
        name: "GNU Affero General Public License v3.0",
        category: "Strong Copyleft",
        copyleft: true,
        patent_grant: true,
        trademark: false,
        sublicensing: false,
        commercial: true,
        summary: "GPL-3.0 extended to cover network use. SaaS providers must release source code.",
        conditions: &["Source code disclosure (including network use)", "Same license for derivatives", "State changes", "Retain copyright notice"],
        permissions: &["Commercial use", "Modification", "Distribution", "Patent use", "Private use"],
        limitations: &["No liability", "No warranty"],
        keywords: &["agpl", "affero", "agpl-3.0", "agplv3", "gnu affero"],
    },
    License {
        id: "BSD-2-Clause",
        name: "BSD 2-Clause \"Simplified\" License",
        category: "Permissive",
        copyleft: false,
        patent_grant: false,
        trademark: false,
        sublicensing: true,
        commercial: true,
        summary: "Permissive license similar to MIT. Requires copyright notice and non-endorsement clause.",
        conditions: &["Retain copyright notice in source", "Retain copyright notice in binary"],
        permissions: &["Commercial use", "Modification", "Distribution", "Private use"],
        limitations: &["No liability", "No warranty"],
        keywords: &["bsd-2", "bsd 2-clause", "simplified bsd", "\"as-is\""],
    },
    License {
        id: "BSD-3-Clause",
        name: "BSD 3-Clause \"New\" or \"Revised\" License",
        category: "Permissive",
        copyleft: false,
        patent_grant: false,
        trademark: false,
        sublicensing: true,
        commercial: true,
        summary: "BSD-2-Clause with an additional non-endorsement restriction for advertising.",
        conditions: &["Retain copyright notice in source", "Retain copyright notice in binary", "No endorsement"],
        permissions: &["Commercial use", "Modification", "Distribution", "Private use"],
        limitations: &["No liability", "No warranty"],
        keywords: &["bsd-3", "bsd 3-clause", "new bsd", "revised bsd"],
    },
    License {
        id: "ISC",
        name: "ISC License",
        category: "Permissive",
        copyleft: false,
        patent_grant: false,
        trademark: false,
        sublicensing: true,
        commercial: true,
        summary: "Functionally equivalent to BSD-2-Clause and MIT. Very short and simple.",
        conditions: &["Retain copyright notice"],
        permissions: &["Commercial use", "Modification", "Distribution", "Private use"],
        limitations: &["No liability", "No warranty"],
        keywords: &["isc", "isc license", "internet systems consortium"],
    },
    License {
        id: "Unlicense",
        name: "The Unlicense",
        category: "Public Domain",
        copyleft: false,
        patent_grant: false,
        trademark: false,
        sublicensing: true,
        commercial: true,
        summary: "Releases the work into the public domain. No conditions on use or distribution.",
        conditions: &[],
        permissions: &["Commercial use", "Modification", "Distribution", "Private use"],
        limitations: &["No liability", "No warranty"],
        keywords: &["unlicense", "public domain", "this is free and unencumbered software"],
    },
    License {
        id: "CC0-1.0",
        name: "Creative Commons Zero v1.0 Universal",
        category: "Public Domain",
        copyleft: false,
        patent_grant: false,
        trademark: false,
        sublicensing: true,
        commercial: true,
        summary: "Public domain dedication. Waives all rights to the extent permitted by law.",
        conditions: &[],
        permissions: &["Commercial use", "Modification", "Distribution", "Private use"],
        limitations: &["No liability", "No warranty", "No patent use"],
        keywords: &["cc0", "creative commons zero", "cc-0", "public domain dedication"],
    },
    License {
        id: "EUPL-1.2",
        name: "European Union Public License 1.2",
        category: "Weak Copyleft",
        copyleft: true,
        patent_grant: true,
        trademark: false,
        sublicensing: false,
        commercial: true,
        summary: "Compatible with GPL-2.0, GPL-3.0, AGPL-3.0, and MPL-2.0. Required for EU public sector.",
        conditions: &["Source code disclosure", "Same license for derivatives", "Retain copyright"],
        permissions: &["Commercial use", "Modification", "Distribution", "Patent use", "Private use"],
        limitations: &["No liability", "No warranty"],
        keywords: &["eupl", "european union public license", "eupl-1.2"],
    },
];

fn find_license(query: &str) -> Option<&'static License> {
    let q = query.to_lowercase();
    LICENSES.iter().find(|l| {
        l.id.to_lowercase() == q
            || l.name.to_lowercase().contains(&q)
            || l.keywords
                .iter()
                .any(|k| q.contains(k) || k.contains(q.as_str()))
    })
}

fn info_action(args: &Value) -> Result<String, String> {
    let query = args
        .get("license")
        .or_else(|| args.get("id"))
        .or_else(|| args.get("name"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'license' — e.g. 'MIT', 'Apache-2.0', 'GPL-3.0'")?;

    let lic = find_license(query).ok_or_else(|| {
        format!(
            "Unknown license '{}'. Use action='list' to see all options.",
            query
        )
    })?;

    let yn = |b: bool| if b { "Yes" } else { "No" };
    let mut out = format!("{}\n{}\n\n", lic.name, "=".repeat(44));
    out += &format!("SPDX ID:      {}\n", lic.id);
    out += &format!("Category:     {}\n", lic.category);
    out += &format!("Copyleft:     {}\n", yn(lic.copyleft));
    out += &format!("Patent grant: {}\n", yn(lic.patent_grant));
    out += &format!("Commercial:   {}\n", yn(lic.commercial));
    out += &format!("Sublicensing: {}\n\n", yn(lic.sublicensing));
    out += &format!("Summary: {}\n\n", lic.summary);

    if !lic.permissions.is_empty() {
        out += "Permissions:\n";
        for p in lic.permissions {
            out += &format!("  ✓ {}\n", p);
        }
        out += "\n";
    }
    if !lic.conditions.is_empty() {
        out += "Conditions:\n";
        for c in lic.conditions {
            out += &format!("  ⚑ {}\n", c);
        }
        out += "\n";
    }
    if !lic.limitations.is_empty() {
        out += "Limitations:\n";
        for l in lic.limitations {
            out += &format!("  ✗ {}\n", l);
        }
    }
    Ok(out)
}

fn detect_action(args: &Value) -> Result<String, String> {
    let text = args
        .get("text")
        .or_else(|| args.get("content"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'text' — pass the license file content")?
        .to_lowercase();

    let mut matches: Vec<(&License, usize)> = LICENSES
        .iter()
        .filter_map(|l| {
            let count = l.keywords.iter().filter(|k| text.contains(*k)).count();
            if count > 0 {
                Some((l, count))
            } else {
                None
            }
        })
        .collect();
    matches.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut out = format!("License Detection\n{}\n\n", "=".repeat(44));
    if matches.is_empty() {
        out += "No recognized license detected.\n";
        out += "The text does not match any known SPDX license signatures.\n";
    } else {
        let (best, score) = matches[0];
        out += &format!("Detected: {} ({})\n", best.name, best.id);
        out += &format!(
            "Category: {}  Copyleft: {}\n\n",
            best.category,
            if best.copyleft { "Yes" } else { "No" }
        );
        out += &format!("Summary: {}\n", best.summary);
        if matches.len() > 1 {
            out += "\nOther candidates:\n";
            for (lic, s) in matches.iter().skip(1).take(3) {
                out += &format!(
                    "  {} ({}) — {} keyword(s) matched\n",
                    lic.id, lic.category, s
                );
            }
        }
        let _ = score;
    }
    Ok(out)
}

fn compare_action(args: &Value) -> Result<String, String> {
    let a_query = args
        .get("a")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'a' — first license (e.g. 'MIT')")?;
    let b_query = args
        .get("b")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'b' — second license (e.g. 'GPL-3.0')")?;

    let a = find_license(a_query).ok_or_else(|| format!("Unknown license '{}'", a_query))?;
    let b = find_license(b_query).ok_or_else(|| format!("Unknown license '{}'", b_query))?;

    let yn = |bv: bool| if bv { "Yes" } else { "No" };
    let mut out = format!(
        "License Comparison: {} vs {}\n{}\n\n",
        a.id,
        b.id,
        "=".repeat(44)
    );
    out += &format!("{:<22} {:<14} {:<14}\n", "Property", a.id, b.id);
    out += &format!("{}\n", "-".repeat(52));
    out += &format!("{:<22} {:<14} {:<14}\n", "Category", a.category, b.category);
    out += &format!(
        "{:<22} {:<14} {:<14}\n",
        "Copyleft",
        yn(a.copyleft),
        yn(b.copyleft)
    );
    out += &format!(
        "{:<22} {:<14} {:<14}\n",
        "Patent grant",
        yn(a.patent_grant),
        yn(b.patent_grant)
    );
    out += &format!(
        "{:<22} {:<14} {:<14}\n",
        "Commercial use",
        yn(a.commercial),
        yn(b.commercial)
    );
    out += &format!(
        "{:<22} {:<14} {:<14}\n",
        "Sublicensing",
        yn(a.sublicensing),
        yn(b.sublicensing)
    );
    out += &format!(
        "{:<22} {:<14} {:<14}\n",
        "Trademark",
        yn(a.trademark),
        yn(b.trademark)
    );

    out += "\n";
    let compatible = !a.copyleft && !b.copyleft
        || a.id == b.id
        || (a.id == "LGPL-2.1" && !b.copyleft)
        || (b.id == "LGPL-2.1" && !a.copyleft);
    if compatible {
        out += "Compatibility: Generally compatible — both permissive or same license.\n";
    } else if a.copyleft && b.copyleft {
        out += "Compatibility: May be incompatible — both are copyleft; check specific version compatibility.\n";
    } else {
        out += "Compatibility: Check carefully — copyleft license may restrict permissive license combination.\n";
    }
    Ok(out)
}

fn list_action(args: &Value) -> Result<String, String> {
    let filter = args
        .get("category")
        .or_else(|| args.get("filter"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let mut out = format!("Software Licenses\n{}\n\n", "=".repeat(44));
    let filtered: Vec<&License> = LICENSES
        .iter()
        .filter(|l| {
            filter
                .as_ref()
                .map(|f| l.category.to_lowercase().contains(f.as_str()))
                .unwrap_or(true)
        })
        .collect();

    let mut by_cat: std::collections::HashMap<&str, Vec<&License>> =
        std::collections::HashMap::new();
    for l in &filtered {
        by_cat.entry(l.category).or_default().push(l);
    }
    let mut cats: Vec<&str> = by_cat.keys().copied().collect();
    cats.sort();

    for cat in cats {
        out += &format!("{}:\n", cat);
        for l in &by_cat[cat] {
            let flags = [
                if l.copyleft { "copyleft" } else { "" },
                if l.patent_grant { "patent-grant" } else { "" },
                if !l.commercial { "non-commercial" } else { "" },
            ]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
            let flag_str = if flags.is_empty() {
                String::new()
            } else {
                format!("  [{}]", flags)
            };
            out += &format!("  {:<16} {}{}\n", l.id, l.name, flag_str);
        }
        out += "\n";
    }
    out += &format!("{} license(s) listed.\n", filtered.len());
    Ok(out)
}
