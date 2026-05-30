use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("convert");
    match action {
        "convert" => action_convert(args),
        "list" => action_list(args),
        "categories" => action_categories(args),
        other => Err(format!(
            "Unknown action '{other}'. Use: convert, list, categories"
        )),
    }
}

// Each entry: (name, symbol, conversion_factor_to_base, aliases...)
// Base units per category are listed first (factor = 1.0).
// Temperature uses a special path since it's affine, not linear.

#[derive(Clone)]
struct Unit {
    name: &'static str,
    symbol: &'static str,
    factor: f64, // multiply by this to get base unit
    offset: f64, // add before dividing (only used for temperature)
    aliases: &'static [&'static str],
}

impl Unit {
    const fn new(
        name: &'static str,
        symbol: &'static str,
        factor: f64,
        aliases: &'static [&'static str],
    ) -> Self {
        Unit {
            name,
            symbol,
            factor,
            offset: 0.0,
            aliases,
        }
    }
    const fn affine(
        name: &'static str,
        symbol: &'static str,
        factor: f64,
        offset: f64,
        aliases: &'static [&'static str],
    ) -> Self {
        Unit {
            name,
            symbol,
            factor,
            offset,
            aliases,
        }
    }
    fn to_base(&self, v: f64) -> f64 {
        (v + self.offset) * self.factor
    }
    fn from_base(&self, v: f64) -> f64 {
        v / self.factor - self.offset
    }
    fn matches(&self, s: &str) -> bool {
        let sl = s.to_lowercase();
        sl == self.name.to_lowercase()
            || sl == self.symbol.to_lowercase()
            || self.aliases.iter().any(|a| a.to_lowercase() == sl)
    }
}

struct Category {
    name: &'static str,
    description: &'static str,
    units: &'static [Unit],
}

static CATEGORIES: &[Category] = &[
    Category {
        name: "length",
        description: "Distance and length",
        units: &[
            Unit::new("metre", "m", 1.0, &["meter", "meters", "metres", "m"]),
            Unit::new(
                "kilometre",
                "km",
                1000.0,
                &["kilometer", "kilometers", "kilometres"],
            ),
            Unit::new(
                "centimetre",
                "cm",
                0.01,
                &["centimeter", "centimeters", "centimetres"],
            ),
            Unit::new(
                "millimetre",
                "mm",
                0.001,
                &["millimeter", "millimeters", "millimetres"],
            ),
            Unit::new(
                "micrometre",
                "µm",
                1e-6,
                &["micrometer", "micron", "microns"],
            ),
            Unit::new(
                "nanometre",
                "nm",
                1e-9,
                &["nanometer", "nanometers", "nanometres"],
            ),
            Unit::new("mile", "mi", 1609.344, &["miles"]),
            Unit::new("yard", "yd", 0.9144, &["yards"]),
            Unit::new("foot", "ft", 0.3048, &["feet", "foot", "ft"]),
            Unit::new("inch", "in", 0.0254, &["inches", "inch", "in", "\""]),
            Unit::new("nautical mile", "nmi", 1852.0, &["nmi", "nautical miles"]),
            Unit::new("light year", "ly", 9.461e15, &["light-year", "light years"]),
            Unit::new("astronomical unit", "au", 1.496e11, &["au"]),
            Unit::new("parsec", "pc", 3.086e16, &["parsecs"]),
            Unit::new("furlong", "fur", 201.168, &["furlongs"]),
            Unit::new("league", "lea", 4828.032, &["leagues"]),
            Unit::new("chain", "ch", 20.1168, &["chains"]),
            Unit::new("fathom", "ftm", 1.8288, &["fathoms"]),
        ],
    },
    Category {
        name: "mass",
        description: "Weight and mass",
        units: &[
            Unit::new("kilogram", "kg", 1.0, &["kilograms", "kg"]),
            Unit::new("gram", "g", 0.001, &["grams", "g"]),
            Unit::new("milligram", "mg", 1e-6, &["milligrams"]),
            Unit::new("microgram", "µg", 1e-9, &["micrograms"]),
            Unit::new(
                "tonne",
                "t",
                1000.0,
                &["metric ton", "metric tons", "tonnes"],
            ),
            Unit::new("pound", "lb", 0.453592, &["pounds", "lbs"]),
            Unit::new("ounce", "oz", 0.028349, &["ounces"]),
            Unit::new("stone", "st", 6.35029, &["stones"]),
            Unit::new("short ton", "tn", 907.185, &["us ton", "us tons"]),
            Unit::new("long ton", "ton", 1016.05, &["imperial ton"]),
            Unit::new("carat", "ct", 0.0002, &["carats"]),
            Unit::new("grain", "gr", 0.0000648, &["grains"]),
            Unit::new("troy ounce", "troy oz", 0.031103, &["troy ounces"]),
        ],
    },
    Category {
        name: "temperature",
        description: "Temperature (affine conversion)",
        units: &[
            Unit::affine("celsius", "°C", 1.0, 0.0, &["c", "centigrade"]),
            Unit::affine("fahrenheit", "°F", 5.0 / 9.0, -32.0, &["f"]),
            Unit::affine("kelvin", "K", 1.0, -273.15, &["k"]),
            Unit::affine("rankine", "°R", 5.0 / 9.0, -491.67, &["r"]),
        ],
    },
    Category {
        name: "area",
        description: "Surface area",
        units: &[
            Unit::new(
                "square metre",
                "m²",
                1.0,
                &["m2", "sq m", "square meter", "square meters"],
            ),
            Unit::new(
                "square kilometre",
                "km²",
                1e6,
                &["km2", "sq km", "square kilometer"],
            ),
            Unit::new("square centimetre", "cm²", 1e-4, &["cm2", "sq cm"]),
            Unit::new("square millimetre", "mm²", 1e-6, &["mm2", "sq mm"]),
            Unit::new("hectare", "ha", 10000.0, &["hectares"]),
            Unit::new("acre", "ac", 4046.86, &["acres"]),
            Unit::new("square mile", "mi²", 2.59e6, &["sq mi", "square miles"]),
            Unit::new("square yard", "yd²", 0.836127, &["sq yd", "square yards"]),
            Unit::new(
                "square foot",
                "ft²",
                0.092903,
                &["ft2", "sq ft", "square feet"],
            ),
            Unit::new(
                "square inch",
                "in²",
                6.4516e-4,
                &["in2", "sq in", "square inches"],
            ),
        ],
    },
    Category {
        name: "volume",
        description: "Volume and capacity",
        units: &[
            Unit::new("litre", "L", 0.001, &["liter", "liters", "litres", "l"]),
            Unit::new("cubic metre", "m³", 1.0, &["m3", "cubic meter"]),
            Unit::new("millilitre", "mL", 1e-6, &["milliliter", "ml"]),
            Unit::new("centilitre", "cL", 1e-5, &["centiliter", "cl"]),
            Unit::new(
                "cubic centimetre",
                "cm³",
                1e-6,
                &["cm3", "cc", "cubic centimeter"],
            ),
            Unit::new("cubic foot", "ft³", 0.028317, &["ft3", "cubic feet"]),
            Unit::new("cubic inch", "in³", 1.6387e-5, &["in3", "cubic inches"]),
            Unit::new(
                "US gallon",
                "gal",
                0.003785,
                &["us gallon", "us gallons", "gallon", "gallons"],
            ),
            Unit::new("US quart", "qt", 9.464e-4, &["quart", "quarts"]),
            Unit::new("US pint", "pt", 4.732e-4, &["pint", "pints"]),
            Unit::new("US cup", "cup", 2.366e-4, &["cups"]),
            Unit::new(
                "US fluid ounce",
                "fl oz",
                2.957e-5,
                &["fl oz", "fluid ounce", "fluid ounces"],
            ),
            Unit::new("tablespoon", "tbsp", 1.479e-5, &["tablespoons", "tbsp"]),
            Unit::new("teaspoon", "tsp", 4.929e-6, &["teaspoons", "tsp"]),
            Unit::new(
                "imperial gallon",
                "imp gal",
                0.004546,
                &["imp gallon", "imperial gallons"],
            ),
            Unit::new("imperial pint", "imp pt", 5.683e-4, &["imperial pint"]),
            Unit::new("barrel", "bbl", 0.158987, &["barrels", "oil barrel"]),
        ],
    },
    Category {
        name: "speed",
        description: "Velocity and speed",
        units: &[
            Unit::new("metres per second", "m/s", 1.0, &["mps", "m/s"]),
            Unit::new(
                "kilometres per hour",
                "km/h",
                1.0 / 3.6,
                &["kph", "kmh", "km/h"],
            ),
            Unit::new("miles per hour", "mph", 0.44704, &["mph"]),
            Unit::new("feet per second", "ft/s", 0.3048, &["fps", "ft/s"]),
            Unit::new("knot", "kn", 0.514444, &["knots", "kt"]),
            Unit::new("mach", "Ma", 343.0, &["mach number"]),
        ],
    },
    Category {
        name: "energy",
        description: "Energy, heat, and work",
        units: &[
            Unit::new("joule", "J", 1.0, &["joules"]),
            Unit::new("kilojoule", "kJ", 1000.0, &["kilojoules"]),
            Unit::new("megajoule", "MJ", 1e6, &["megajoules"]),
            Unit::new("calorie", "cal", 4.184, &["calories", "small calorie"]),
            Unit::new(
                "kilocalorie",
                "kcal",
                4184.0,
                &["kilocalories", "dietary calorie", "food calorie"],
            ),
            Unit::new("watt-hour", "Wh", 3600.0, &["watt hour", "watt hours"]),
            Unit::new("kilowatt-hour", "kWh", 3.6e6, &["kwh", "kilowatt hour"]),
            Unit::new("BTU", "BTU", 1055.06, &["british thermal unit"]),
            Unit::new("electronvolt", "eV", 1.602e-19, &["ev", "electron volt"]),
            Unit::new("foot-pound", "ft·lb", 1.35582, &["ft-lb", "foot pound"]),
            Unit::new("therm", "thm", 1.055e8, &["therms"]),
        ],
    },
    Category {
        name: "power",
        description: "Power and radiant flux",
        units: &[
            Unit::new("watt", "W", 1.0, &["watts"]),
            Unit::new("kilowatt", "kW", 1000.0, &["kilowatts"]),
            Unit::new("megawatt", "MW", 1e6, &["megawatts"]),
            Unit::new("gigawatt", "GW", 1e9, &["gigawatts"]),
            Unit::new("milliwatt", "mW", 0.001, &["milliwatts"]),
            Unit::new("horsepower", "hp", 745.7, &["hp", "mechanical horsepower"]),
            Unit::new("metric horsepower", "PS", 735.499, &["ps", "cv"]),
            Unit::new("BTU/hour", "BTU/h", 0.293071, &["btu/hr", "btu per hour"]),
        ],
    },
    Category {
        name: "pressure",
        description: "Pressure and stress",
        units: &[
            Unit::new("pascal", "Pa", 1.0, &["pascals"]),
            Unit::new("kilopascal", "kPa", 1000.0, &["kilopascals"]),
            Unit::new("megapascal", "MPa", 1e6, &["megapascals"]),
            Unit::new("bar", "bar", 100000.0, &["bars"]),
            Unit::new("millibar", "mbar", 100.0, &["millibars"]),
            Unit::new("atmosphere", "atm", 101325.0, &["atm", "atmospheres"]),
            Unit::new("psi", "psi", 6894.76, &["pounds per square inch"]),
            Unit::new("mmHg", "mmHg", 133.322, &["millimetre of mercury", "torr"]),
            Unit::new("inHg", "inHg", 3386.39, &["inch of mercury"]),
        ],
    },
    Category {
        name: "time",
        description: "Duration and time intervals",
        units: &[
            Unit::new("second", "s", 1.0, &["seconds", "sec", "secs"]),
            Unit::new("millisecond", "ms", 0.001, &["milliseconds"]),
            Unit::new("microsecond", "µs", 1e-6, &["microseconds"]),
            Unit::new("nanosecond", "ns", 1e-9, &["nanoseconds"]),
            Unit::new("minute", "min", 60.0, &["minutes", "mins"]),
            Unit::new("hour", "h", 3600.0, &["hours", "hr", "hrs"]),
            Unit::new("day", "d", 86400.0, &["days"]),
            Unit::new("week", "wk", 604800.0, &["weeks"]),
            Unit::new("month", "mo", 2628000.0, &["months"]),
            Unit::new("year", "yr", 31536000.0, &["years"]),
            Unit::new("decade", "dec", 315360000.0, &["decades"]),
            Unit::new("century", "cent", 3153600000.0, &["centuries"]),
        ],
    },
    Category {
        name: "angle",
        description: "Angles and rotations",
        units: &[
            Unit::new("radian", "rad", 1.0, &["radians"]),
            Unit::new(
                "degree",
                "°",
                std::f64::consts::PI / 180.0,
                &["degrees", "deg"],
            ),
            Unit::new(
                "gradian",
                "grad",
                std::f64::consts::PI / 200.0,
                &["gradians", "gon"],
            ),
            Unit::new(
                "turn",
                "tr",
                std::f64::consts::TAU,
                &["turns", "revolution", "revolutions", "rev"],
            ),
            Unit::new(
                "arcminute",
                "′",
                std::f64::consts::PI / 10800.0,
                &["arcmin", "arc minute"],
            ),
            Unit::new(
                "arcsecond",
                "″",
                std::f64::consts::PI / 648000.0,
                &["arcsec", "arc second"],
            ),
        ],
    },
    Category {
        name: "fuel",
        description: "Fuel economy / consumption",
        units: &[
            Unit::new("litres per 100km", "L/100km", 1.0, &["l/100km", "l/100 km"]),
            Unit::new("mpg (US)", "mpg", 235.215, &["us mpg", "miles per gallon"]),
            Unit::new(
                "mpg (Imperial)",
                "mpg imp",
                282.481,
                &["imperial mpg", "uk mpg"],
            ),
            Unit::new(
                "km per litre",
                "km/L",
                100.0,
                &["km/l", "kilometres per litre"],
            ),
        ],
    },
    Category {
        name: "frequency",
        description: "Frequency and angular velocity",
        units: &[
            Unit::new("hertz", "Hz", 1.0, &["hz"]),
            Unit::new("kilohertz", "kHz", 1000.0, &["khz"]),
            Unit::new("megahertz", "MHz", 1e6, &["mhz"]),
            Unit::new("gigahertz", "GHz", 1e9, &["ghz"]),
            Unit::new("rpm", "rpm", 1.0 / 60.0, &["revolutions per minute"]),
        ],
    },
];

fn find_unit(token: &str) -> Option<(&'static Category, &'static Unit)> {
    for cat in CATEGORIES {
        for unit in cat.units {
            if unit.matches(token) {
                return Some((cat, unit));
            }
        }
    }
    None
}

fn action_convert(args: &Value) -> Result<String, String> {
    let value = args
        .get("value")
        .or_else(|| args.get("amount"))
        .and_then(|v| v.as_f64())
        .ok_or("Missing 'value' (number to convert)")?;
    let from_str = args
        .get("from")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'from' unit")?;
    let to_str = args.get("to").and_then(|v| v.as_str());

    let (from_cat, from_unit) = find_unit(from_str).ok_or_else(|| {
        format!(
            "Unknown unit '{}'. Use `unit_tools(action:'list')` to see all units.",
            from_str
        )
    })?;

    let base = from_unit.to_base(value);

    let mut out = String::from("unit_tools — convert\n\n");
    out.push_str(&format!(
        "Input: {} {} ({})\n\n",
        value, from_unit.symbol, from_unit.name
    ));

    if let Some(to_str) = to_str {
        let (to_cat, to_unit) = find_unit(to_str).ok_or_else(|| {
            format!(
                "Unknown unit '{}'. Use `unit_tools(action:'list')` to see all units.",
                to_str
            )
        })?;

        if to_cat.name != from_cat.name {
            return Err(format!(
                "Cannot convert '{}' ({}) to '{}' ({}): different categories",
                from_unit.name, from_cat.name, to_unit.name, to_cat.name
            ));
        }

        let result = to_unit.from_base(base);
        out.push_str(&format!(
            "{} {} = {} {}\n",
            value,
            from_unit.symbol,
            format_value(result),
            to_unit.symbol
        ));
        out.push_str(&format!("({} → {})\n", from_unit.name, to_unit.name));
    } else {
        // Show all units in the same category
        out.push_str(&format!("All {} conversions:\n\n", from_cat.name));
        for unit in from_cat.units {
            let result = unit.from_base(base);
            out.push_str(&format!(
                "  {:>12} {:<8}  ({})\n",
                format_value(result),
                unit.symbol,
                unit.name
            ));
        }
    }
    Ok(out)
}

fn format_value(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let abs = v.abs();
    if abs >= 1e-4 && abs < 1e10 {
        let s = format!("{:.6}", v);
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    } else {
        format!("{:.4e}", v)
    }
}

fn action_list(args: &Value) -> Result<String, String> {
    let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("");

    let mut out = String::from("unit_tools — list\n\n");
    for cat in CATEGORIES {
        if !category.is_empty() && !cat.name.contains(category) {
            continue;
        }
        out.push_str(&format!("{}:\n", cat.name.to_uppercase()));
        for unit in cat.units {
            out.push_str(&format!("  {:<20} {}\n", unit.symbol, unit.name));
        }
        out.push('\n');
    }
    Ok(out)
}

fn action_categories(args: &Value) -> Result<String, String> {
    let _ = args;
    let mut out = String::from("unit_tools — categories\n\n");
    for cat in CATEGORIES {
        out.push_str(&format!(
            "  {:<12} — {} ({} units)\n",
            cat.name,
            cat.description,
            cat.units.len()
        ));
    }
    Ok(out)
}
