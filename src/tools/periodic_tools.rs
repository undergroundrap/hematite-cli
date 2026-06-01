use serde_json::{json, Value};

pub fn periodic_tools_schema() -> Value {
    json!({
        "name": "periodic_tools",
        "description": "Periodic table lookup without external utilities. Actions: element (look up an element by symbol, name, or atomic number — returns all properties), search (fuzzy search by name substring or category), list (list all elements or filter by category/period/group), compare (side-by-side property comparison of two elements), mass (compute molar mass of a chemical formula like 'H2O' or 'C6H12O6').",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["element", "search", "list", "compare", "mass"],
                    "description": "Action to perform (default: element)"
                },
                "symbol": {
                    "type": "string",
                    "description": "Element symbol (e.g. 'Au', 'Fe', 'H') for 'element' action"
                },
                "name": {
                    "type": "string",
                    "description": "Element name (e.g. 'Gold', 'Iron') or formula for 'mass' action"
                },
                "number": {
                    "type": "integer",
                    "description": "Atomic number (1–118) for 'element' action"
                },
                "query": {
                    "type": "string",
                    "description": "Search query for 'search' action — name substring or category"
                },
                "category": {
                    "type": "string",
                    "description": "Filter by category for 'list': alkali, alkaline, transition, post-transition, metalloid, nonmetal, halogen, noble, lanthanide, actinide"
                },
                "period": {
                    "type": "integer",
                    "description": "Filter by period (1–7) for 'list'"
                },
                "group": {
                    "type": "integer",
                    "description": "Filter by group (1–18) for 'list'"
                },
                "formula": {
                    "type": "string",
                    "description": "Chemical formula for 'mass' action (e.g. 'H2O', 'NaCl', 'C6H12O6')"
                },
                "element2": {
                    "type": "string",
                    "description": "Second element (symbol or name) for 'compare' action"
                }
            },
            "required": []
        }
    })
}

// ── Element data ──────────────────────────────────────────────────────────────
// (symbol, name, atomic_number, atomic_mass, category, period, group,
//  density_g_cm3, melting_k, boiling_k, electronegativity, electron_config)
// density/melting/boiling: 0.0 = unknown/N/A

#[derive(Clone)]
struct Element {
    symbol: &'static str,
    name: &'static str,
    z: u32,
    mass: f64,
    category: &'static str,
    period: u8,
    group: u8,
    density: f64,
    melting: f64,
    boiling: f64,
    electronegativity: f64,
    config: &'static str,
}

const ELEMENTS: &[Element] = &[
    Element {
        symbol: "H",
        name: "Hydrogen",
        z: 1,
        mass: 1.008,
        category: "nonmetal",
        period: 1,
        group: 1,
        density: 0.00009,
        melting: 14.01,
        boiling: 20.28,
        electronegativity: 2.20,
        config: "1s1",
    },
    Element {
        symbol: "He",
        name: "Helium",
        z: 2,
        mass: 4.0026,
        category: "noble",
        period: 1,
        group: 18,
        density: 0.00018,
        melting: 0.95,
        boiling: 4.22,
        electronegativity: 0.00,
        config: "1s2",
    },
    Element {
        symbol: "Li",
        name: "Lithium",
        z: 3,
        mass: 6.94,
        category: "alkali",
        period: 2,
        group: 1,
        density: 0.534,
        melting: 453.69,
        boiling: 1560.0,
        electronegativity: 0.98,
        config: "[He] 2s1",
    },
    Element {
        symbol: "Be",
        name: "Beryllium",
        z: 4,
        mass: 9.0122,
        category: "alkaline",
        period: 2,
        group: 2,
        density: 1.85,
        melting: 1560.0,
        boiling: 2742.0,
        electronegativity: 1.57,
        config: "[He] 2s2",
    },
    Element {
        symbol: "B",
        name: "Boron",
        z: 5,
        mass: 10.81,
        category: "metalloid",
        period: 2,
        group: 13,
        density: 2.34,
        melting: 2349.0,
        boiling: 4200.0,
        electronegativity: 2.04,
        config: "[He] 2s2 2p1",
    },
    Element {
        symbol: "C",
        name: "Carbon",
        z: 6,
        mass: 12.011,
        category: "nonmetal",
        period: 2,
        group: 14,
        density: 2.27,
        melting: 3823.0,
        boiling: 5100.0,
        electronegativity: 2.55,
        config: "[He] 2s2 2p2",
    },
    Element {
        symbol: "N",
        name: "Nitrogen",
        z: 7,
        mass: 14.007,
        category: "nonmetal",
        period: 2,
        group: 15,
        density: 0.00125,
        melting: 63.15,
        boiling: 77.36,
        electronegativity: 3.04,
        config: "[He] 2s2 2p3",
    },
    Element {
        symbol: "O",
        name: "Oxygen",
        z: 8,
        mass: 15.999,
        category: "nonmetal",
        period: 2,
        group: 16,
        density: 0.00143,
        melting: 54.36,
        boiling: 90.20,
        electronegativity: 3.44,
        config: "[He] 2s2 2p4",
    },
    Element {
        symbol: "F",
        name: "Fluorine",
        z: 9,
        mass: 18.998,
        category: "halogen",
        period: 2,
        group: 17,
        density: 0.00170,
        melting: 53.53,
        boiling: 85.03,
        electronegativity: 3.98,
        config: "[He] 2s2 2p5",
    },
    Element {
        symbol: "Ne",
        name: "Neon",
        z: 10,
        mass: 20.180,
        category: "noble",
        period: 2,
        group: 18,
        density: 0.00090,
        melting: 24.56,
        boiling: 27.07,
        electronegativity: 0.00,
        config: "[He] 2s2 2p6",
    },
    Element {
        symbol: "Na",
        name: "Sodium",
        z: 11,
        mass: 22.990,
        category: "alkali",
        period: 3,
        group: 1,
        density: 0.971,
        melting: 370.87,
        boiling: 1156.0,
        electronegativity: 0.93,
        config: "[Ne] 3s1",
    },
    Element {
        symbol: "Mg",
        name: "Magnesium",
        z: 12,
        mass: 24.305,
        category: "alkaline",
        period: 3,
        group: 2,
        density: 1.738,
        melting: 923.0,
        boiling: 1363.0,
        electronegativity: 1.31,
        config: "[Ne] 3s2",
    },
    Element {
        symbol: "Al",
        name: "Aluminum",
        z: 13,
        mass: 26.982,
        category: "post-transition",
        period: 3,
        group: 13,
        density: 2.70,
        melting: 933.47,
        boiling: 2792.0,
        electronegativity: 1.61,
        config: "[Ne] 3s2 3p1",
    },
    Element {
        symbol: "Si",
        name: "Silicon",
        z: 14,
        mass: 28.085,
        category: "metalloid",
        period: 3,
        group: 14,
        density: 2.329,
        melting: 1687.0,
        boiling: 3538.0,
        electronegativity: 1.90,
        config: "[Ne] 3s2 3p2",
    },
    Element {
        symbol: "P",
        name: "Phosphorus",
        z: 15,
        mass: 30.974,
        category: "nonmetal",
        period: 3,
        group: 15,
        density: 1.82,
        melting: 317.30,
        boiling: 553.65,
        electronegativity: 2.19,
        config: "[Ne] 3s2 3p3",
    },
    Element {
        symbol: "S",
        name: "Sulfur",
        z: 16,
        mass: 32.06,
        category: "nonmetal",
        period: 3,
        group: 16,
        density: 2.067,
        melting: 388.36,
        boiling: 717.87,
        electronegativity: 2.58,
        config: "[Ne] 3s2 3p4",
    },
    Element {
        symbol: "Cl",
        name: "Chlorine",
        z: 17,
        mass: 35.45,
        category: "halogen",
        period: 3,
        group: 17,
        density: 0.00321,
        melting: 171.65,
        boiling: 239.11,
        electronegativity: 3.16,
        config: "[Ne] 3s2 3p5",
    },
    Element {
        symbol: "Ar",
        name: "Argon",
        z: 18,
        mass: 39.948,
        category: "noble",
        period: 3,
        group: 18,
        density: 0.00178,
        melting: 83.80,
        boiling: 87.30,
        electronegativity: 0.00,
        config: "[Ne] 3s2 3p6",
    },
    Element {
        symbol: "K",
        name: "Potassium",
        z: 19,
        mass: 39.098,
        category: "alkali",
        period: 4,
        group: 1,
        density: 0.862,
        melting: 336.53,
        boiling: 1032.0,
        electronegativity: 0.82,
        config: "[Ar] 4s1",
    },
    Element {
        symbol: "Ca",
        name: "Calcium",
        z: 20,
        mass: 40.078,
        category: "alkaline",
        period: 4,
        group: 2,
        density: 1.55,
        melting: 1115.0,
        boiling: 1757.0,
        electronegativity: 1.00,
        config: "[Ar] 4s2",
    },
    Element {
        symbol: "Sc",
        name: "Scandium",
        z: 21,
        mass: 44.956,
        category: "transition",
        period: 4,
        group: 3,
        density: 2.985,
        melting: 1814.0,
        boiling: 3109.0,
        electronegativity: 1.36,
        config: "[Ar] 3d1 4s2",
    },
    Element {
        symbol: "Ti",
        name: "Titanium",
        z: 22,
        mass: 47.867,
        category: "transition",
        period: 4,
        group: 4,
        density: 4.506,
        melting: 1941.0,
        boiling: 3560.0,
        electronegativity: 1.54,
        config: "[Ar] 3d2 4s2",
    },
    Element {
        symbol: "V",
        name: "Vanadium",
        z: 23,
        mass: 50.942,
        category: "transition",
        period: 4,
        group: 5,
        density: 6.11,
        melting: 2183.0,
        boiling: 3680.0,
        electronegativity: 1.63,
        config: "[Ar] 3d3 4s2",
    },
    Element {
        symbol: "Cr",
        name: "Chromium",
        z: 24,
        mass: 51.996,
        category: "transition",
        period: 4,
        group: 6,
        density: 7.19,
        melting: 2180.0,
        boiling: 2944.0,
        electronegativity: 1.66,
        config: "[Ar] 3d5 4s1",
    },
    Element {
        symbol: "Mn",
        name: "Manganese",
        z: 25,
        mass: 54.938,
        category: "transition",
        period: 4,
        group: 7,
        density: 7.21,
        melting: 1519.0,
        boiling: 2334.0,
        electronegativity: 1.55,
        config: "[Ar] 3d5 4s2",
    },
    Element {
        symbol: "Fe",
        name: "Iron",
        z: 26,
        mass: 55.845,
        category: "transition",
        period: 4,
        group: 8,
        density: 7.874,
        melting: 1811.0,
        boiling: 3134.0,
        electronegativity: 1.83,
        config: "[Ar] 3d6 4s2",
    },
    Element {
        symbol: "Co",
        name: "Cobalt",
        z: 27,
        mass: 58.933,
        category: "transition",
        period: 4,
        group: 9,
        density: 8.90,
        melting: 1768.0,
        boiling: 3200.0,
        electronegativity: 1.88,
        config: "[Ar] 3d7 4s2",
    },
    Element {
        symbol: "Ni",
        name: "Nickel",
        z: 28,
        mass: 58.693,
        category: "transition",
        period: 4,
        group: 10,
        density: 8.908,
        melting: 1728.0,
        boiling: 3186.0,
        electronegativity: 1.91,
        config: "[Ar] 3d8 4s2",
    },
    Element {
        symbol: "Cu",
        name: "Copper",
        z: 29,
        mass: 63.546,
        category: "transition",
        period: 4,
        group: 11,
        density: 8.96,
        melting: 1357.8,
        boiling: 2835.0,
        electronegativity: 1.90,
        config: "[Ar] 3d10 4s1",
    },
    Element {
        symbol: "Zn",
        name: "Zinc",
        z: 30,
        mass: 65.38,
        category: "transition",
        period: 4,
        group: 12,
        density: 7.133,
        melting: 692.68,
        boiling: 1180.0,
        electronegativity: 1.65,
        config: "[Ar] 3d10 4s2",
    },
    Element {
        symbol: "Ga",
        name: "Gallium",
        z: 31,
        mass: 69.723,
        category: "post-transition",
        period: 4,
        group: 13,
        density: 5.91,
        melting: 302.91,
        boiling: 2477.0,
        electronegativity: 1.81,
        config: "[Ar] 3d10 4s2 4p1",
    },
    Element {
        symbol: "Ge",
        name: "Germanium",
        z: 32,
        mass: 72.630,
        category: "metalloid",
        period: 4,
        group: 14,
        density: 5.323,
        melting: 1211.4,
        boiling: 3106.0,
        electronegativity: 2.01,
        config: "[Ar] 3d10 4s2 4p2",
    },
    Element {
        symbol: "As",
        name: "Arsenic",
        z: 33,
        mass: 74.922,
        category: "metalloid",
        period: 4,
        group: 15,
        density: 5.727,
        melting: 1090.0,
        boiling: 887.0,
        electronegativity: 2.18,
        config: "[Ar] 3d10 4s2 4p3",
    },
    Element {
        symbol: "Se",
        name: "Selenium",
        z: 34,
        mass: 78.971,
        category: "nonmetal",
        period: 4,
        group: 16,
        density: 4.81,
        melting: 494.0,
        boiling: 958.0,
        electronegativity: 2.55,
        config: "[Ar] 3d10 4s2 4p4",
    },
    Element {
        symbol: "Br",
        name: "Bromine",
        z: 35,
        mass: 79.904,
        category: "halogen",
        period: 4,
        group: 17,
        density: 3.11,
        melting: 265.8,
        boiling: 332.0,
        electronegativity: 2.96,
        config: "[Ar] 3d10 4s2 4p5",
    },
    Element {
        symbol: "Kr",
        name: "Krypton",
        z: 36,
        mass: 83.798,
        category: "noble",
        period: 4,
        group: 18,
        density: 0.00374,
        melting: 115.79,
        boiling: 119.93,
        electronegativity: 3.00,
        config: "[Ar] 3d10 4s2 4p6",
    },
    Element {
        symbol: "Rb",
        name: "Rubidium",
        z: 37,
        mass: 85.468,
        category: "alkali",
        period: 5,
        group: 1,
        density: 1.532,
        melting: 312.46,
        boiling: 961.0,
        electronegativity: 0.82,
        config: "[Kr] 5s1",
    },
    Element {
        symbol: "Sr",
        name: "Strontium",
        z: 38,
        mass: 87.62,
        category: "alkaline",
        period: 5,
        group: 2,
        density: 2.64,
        melting: 1050.0,
        boiling: 1655.0,
        electronegativity: 0.95,
        config: "[Kr] 5s2",
    },
    Element {
        symbol: "Y",
        name: "Yttrium",
        z: 39,
        mass: 88.906,
        category: "transition",
        period: 5,
        group: 3,
        density: 4.472,
        melting: 1799.0,
        boiling: 3609.0,
        electronegativity: 1.22,
        config: "[Kr] 4d1 5s2",
    },
    Element {
        symbol: "Zr",
        name: "Zirconium",
        z: 40,
        mass: 91.224,
        category: "transition",
        period: 5,
        group: 4,
        density: 6.52,
        melting: 2128.0,
        boiling: 4682.0,
        electronegativity: 1.33,
        config: "[Kr] 4d2 5s2",
    },
    Element {
        symbol: "Nb",
        name: "Niobium",
        z: 41,
        mass: 92.906,
        category: "transition",
        period: 5,
        group: 5,
        density: 8.57,
        melting: 2750.0,
        boiling: 5017.0,
        electronegativity: 1.60,
        config: "[Kr] 4d4 5s1",
    },
    Element {
        symbol: "Mo",
        name: "Molybdenum",
        z: 42,
        mass: 95.95,
        category: "transition",
        period: 5,
        group: 6,
        density: 10.28,
        melting: 2896.0,
        boiling: 4912.0,
        electronegativity: 2.16,
        config: "[Kr] 4d5 5s1",
    },
    Element {
        symbol: "Tc",
        name: "Technetium",
        z: 43,
        mass: 97.0,
        category: "transition",
        period: 5,
        group: 7,
        density: 11.0,
        melting: 2430.0,
        boiling: 4538.0,
        electronegativity: 1.90,
        config: "[Kr] 4d5 5s2",
    },
    Element {
        symbol: "Ru",
        name: "Ruthenium",
        z: 44,
        mass: 101.07,
        category: "transition",
        period: 5,
        group: 8,
        density: 12.37,
        melting: 2607.0,
        boiling: 4423.0,
        electronegativity: 2.20,
        config: "[Kr] 4d7 5s1",
    },
    Element {
        symbol: "Rh",
        name: "Rhodium",
        z: 45,
        mass: 102.91,
        category: "transition",
        period: 5,
        group: 9,
        density: 12.41,
        melting: 2237.0,
        boiling: 3968.0,
        electronegativity: 2.28,
        config: "[Kr] 4d8 5s1",
    },
    Element {
        symbol: "Pd",
        name: "Palladium",
        z: 46,
        mass: 106.42,
        category: "transition",
        period: 5,
        group: 10,
        density: 12.02,
        melting: 1828.0,
        boiling: 3236.0,
        electronegativity: 2.20,
        config: "[Kr] 4d10",
    },
    Element {
        symbol: "Ag",
        name: "Silver",
        z: 47,
        mass: 107.87,
        category: "transition",
        period: 5,
        group: 11,
        density: 10.49,
        melting: 1234.9,
        boiling: 2435.0,
        electronegativity: 1.93,
        config: "[Kr] 4d10 5s1",
    },
    Element {
        symbol: "Cd",
        name: "Cadmium",
        z: 48,
        mass: 112.41,
        category: "transition",
        period: 5,
        group: 12,
        density: 8.65,
        melting: 594.22,
        boiling: 1040.0,
        electronegativity: 1.69,
        config: "[Kr] 4d10 5s2",
    },
    Element {
        symbol: "In",
        name: "Indium",
        z: 49,
        mass: 114.82,
        category: "post-transition",
        period: 5,
        group: 13,
        density: 7.31,
        melting: 429.75,
        boiling: 2345.0,
        electronegativity: 1.78,
        config: "[Kr] 4d10 5s2 5p1",
    },
    Element {
        symbol: "Sn",
        name: "Tin",
        z: 50,
        mass: 118.71,
        category: "post-transition",
        period: 5,
        group: 14,
        density: 7.287,
        melting: 505.08,
        boiling: 2875.0,
        electronegativity: 1.96,
        config: "[Kr] 4d10 5s2 5p2",
    },
    Element {
        symbol: "Sb",
        name: "Antimony",
        z: 51,
        mass: 121.76,
        category: "metalloid",
        period: 5,
        group: 15,
        density: 6.685,
        melting: 903.78,
        boiling: 1908.0,
        electronegativity: 2.05,
        config: "[Kr] 4d10 5s2 5p3",
    },
    Element {
        symbol: "Te",
        name: "Tellurium",
        z: 52,
        mass: 127.60,
        category: "metalloid",
        period: 5,
        group: 16,
        density: 6.232,
        melting: 722.66,
        boiling: 1261.0,
        electronegativity: 2.10,
        config: "[Kr] 4d10 5s2 5p4",
    },
    Element {
        symbol: "I",
        name: "Iodine",
        z: 53,
        mass: 126.90,
        category: "halogen",
        period: 5,
        group: 17,
        density: 4.93,
        melting: 386.85,
        boiling: 457.55,
        electronegativity: 2.66,
        config: "[Kr] 4d10 5s2 5p5",
    },
    Element {
        symbol: "Xe",
        name: "Xenon",
        z: 54,
        mass: 131.29,
        category: "noble",
        period: 5,
        group: 18,
        density: 0.00589,
        melting: 161.36,
        boiling: 165.03,
        electronegativity: 2.60,
        config: "[Kr] 4d10 5s2 5p6",
    },
    Element {
        symbol: "Cs",
        name: "Cesium",
        z: 55,
        mass: 132.91,
        category: "alkali",
        period: 6,
        group: 1,
        density: 1.873,
        melting: 301.59,
        boiling: 944.0,
        electronegativity: 0.79,
        config: "[Xe] 6s1",
    },
    Element {
        symbol: "Ba",
        name: "Barium",
        z: 56,
        mass: 137.33,
        category: "alkaline",
        period: 6,
        group: 2,
        density: 3.62,
        melting: 1000.0,
        boiling: 2118.0,
        electronegativity: 0.89,
        config: "[Xe] 6s2",
    },
    Element {
        symbol: "La",
        name: "Lanthanum",
        z: 57,
        mass: 138.91,
        category: "lanthanide",
        period: 6,
        group: 3,
        density: 6.162,
        melting: 1193.0,
        boiling: 3737.0,
        electronegativity: 1.10,
        config: "[Xe] 5d1 6s2",
    },
    Element {
        symbol: "Ce",
        name: "Cerium",
        z: 58,
        mass: 140.12,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 6.770,
        melting: 1068.0,
        boiling: 3716.0,
        electronegativity: 1.12,
        config: "[Xe] 4f1 5d1 6s2",
    },
    Element {
        symbol: "Pr",
        name: "Praseodymium",
        z: 59,
        mass: 140.91,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 6.77,
        melting: 1208.0,
        boiling: 3793.0,
        electronegativity: 1.13,
        config: "[Xe] 4f3 6s2",
    },
    Element {
        symbol: "Nd",
        name: "Neodymium",
        z: 60,
        mass: 144.24,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 7.01,
        melting: 1297.0,
        boiling: 3347.0,
        electronegativity: 1.14,
        config: "[Xe] 4f4 6s2",
    },
    Element {
        symbol: "Pm",
        name: "Promethium",
        z: 61,
        mass: 145.0,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 7.26,
        melting: 1315.0,
        boiling: 3273.0,
        electronegativity: 1.13,
        config: "[Xe] 4f5 6s2",
    },
    Element {
        symbol: "Sm",
        name: "Samarium",
        z: 62,
        mass: 150.36,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 7.52,
        melting: 1345.0,
        boiling: 2067.0,
        electronegativity: 1.17,
        config: "[Xe] 4f6 6s2",
    },
    Element {
        symbol: "Eu",
        name: "Europium",
        z: 63,
        mass: 151.96,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 5.244,
        melting: 1099.0,
        boiling: 1802.0,
        electronegativity: 1.20,
        config: "[Xe] 4f7 6s2",
    },
    Element {
        symbol: "Gd",
        name: "Gadolinium",
        z: 64,
        mass: 157.25,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 7.90,
        melting: 1585.0,
        boiling: 3546.0,
        electronegativity: 1.20,
        config: "[Xe] 4f7 5d1 6s2",
    },
    Element {
        symbol: "Tb",
        name: "Terbium",
        z: 65,
        mass: 158.93,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 8.23,
        melting: 1629.0,
        boiling: 3503.0,
        electronegativity: 1.10,
        config: "[Xe] 4f9 6s2",
    },
    Element {
        symbol: "Dy",
        name: "Dysprosium",
        z: 66,
        mass: 162.50,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 8.551,
        melting: 1685.0,
        boiling: 2840.0,
        electronegativity: 1.22,
        config: "[Xe] 4f10 6s2",
    },
    Element {
        symbol: "Ho",
        name: "Holmium",
        z: 67,
        mass: 164.93,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 8.795,
        melting: 1734.0,
        boiling: 2993.0,
        electronegativity: 1.23,
        config: "[Xe] 4f11 6s2",
    },
    Element {
        symbol: "Er",
        name: "Erbium",
        z: 68,
        mass: 167.26,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 9.066,
        melting: 1802.0,
        boiling: 3141.0,
        electronegativity: 1.24,
        config: "[Xe] 4f12 6s2",
    },
    Element {
        symbol: "Tm",
        name: "Thulium",
        z: 69,
        mass: 168.93,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 9.32,
        melting: 1818.0,
        boiling: 2223.0,
        electronegativity: 1.25,
        config: "[Xe] 4f13 6s2",
    },
    Element {
        symbol: "Yb",
        name: "Ytterbium",
        z: 70,
        mass: 173.05,
        category: "lanthanide",
        period: 6,
        group: 0,
        density: 6.90,
        melting: 1097.0,
        boiling: 1469.0,
        electronegativity: 1.10,
        config: "[Xe] 4f14 6s2",
    },
    Element {
        symbol: "Lu",
        name: "Lutetium",
        z: 71,
        mass: 174.97,
        category: "lanthanide",
        period: 6,
        group: 3,
        density: 9.841,
        melting: 1925.0,
        boiling: 3675.0,
        electronegativity: 1.27,
        config: "[Xe] 4f14 5d1 6s2",
    },
    Element {
        symbol: "Hf",
        name: "Hafnium",
        z: 72,
        mass: 178.49,
        category: "transition",
        period: 6,
        group: 4,
        density: 13.31,
        melting: 2506.0,
        boiling: 4876.0,
        electronegativity: 1.30,
        config: "[Xe] 4f14 5d2 6s2",
    },
    Element {
        symbol: "Ta",
        name: "Tantalum",
        z: 73,
        mass: 180.95,
        category: "transition",
        period: 6,
        group: 5,
        density: 16.69,
        melting: 3290.0,
        boiling: 5731.0,
        electronegativity: 1.50,
        config: "[Xe] 4f14 5d3 6s2",
    },
    Element {
        symbol: "W",
        name: "Tungsten",
        z: 74,
        mass: 183.84,
        category: "transition",
        period: 6,
        group: 6,
        density: 19.25,
        melting: 3695.0,
        boiling: 5828.0,
        electronegativity: 2.36,
        config: "[Xe] 4f14 5d4 6s2",
    },
    Element {
        symbol: "Re",
        name: "Rhenium",
        z: 75,
        mass: 186.21,
        category: "transition",
        period: 6,
        group: 7,
        density: 21.02,
        melting: 3459.0,
        boiling: 5869.0,
        electronegativity: 1.90,
        config: "[Xe] 4f14 5d5 6s2",
    },
    Element {
        symbol: "Os",
        name: "Osmium",
        z: 76,
        mass: 190.23,
        category: "transition",
        period: 6,
        group: 8,
        density: 22.59,
        melting: 3306.0,
        boiling: 5285.0,
        electronegativity: 2.20,
        config: "[Xe] 4f14 5d6 6s2",
    },
    Element {
        symbol: "Ir",
        name: "Iridium",
        z: 77,
        mass: 192.22,
        category: "transition",
        period: 6,
        group: 9,
        density: 22.56,
        melting: 2719.0,
        boiling: 4403.0,
        electronegativity: 2.20,
        config: "[Xe] 4f14 5d7 6s2",
    },
    Element {
        symbol: "Pt",
        name: "Platinum",
        z: 78,
        mass: 195.08,
        category: "transition",
        period: 6,
        group: 10,
        density: 21.45,
        melting: 2041.4,
        boiling: 4098.0,
        electronegativity: 2.28,
        config: "[Xe] 4f14 5d9 6s1",
    },
    Element {
        symbol: "Au",
        name: "Gold",
        z: 79,
        mass: 196.97,
        category: "transition",
        period: 6,
        group: 11,
        density: 19.30,
        melting: 1337.3,
        boiling: 3129.0,
        electronegativity: 2.54,
        config: "[Xe] 4f14 5d10 6s1",
    },
    Element {
        symbol: "Hg",
        name: "Mercury",
        z: 80,
        mass: 200.59,
        category: "transition",
        period: 6,
        group: 12,
        density: 13.534,
        melting: 234.32,
        boiling: 629.88,
        electronegativity: 2.00,
        config: "[Xe] 4f14 5d10 6s2",
    },
    Element {
        symbol: "Tl",
        name: "Thallium",
        z: 81,
        mass: 204.38,
        category: "post-transition",
        period: 6,
        group: 13,
        density: 11.85,
        melting: 577.0,
        boiling: 1746.0,
        electronegativity: 1.62,
        config: "[Xe] 4f14 5d10 6s2 6p1",
    },
    Element {
        symbol: "Pb",
        name: "Lead",
        z: 82,
        mass: 207.2,
        category: "post-transition",
        period: 6,
        group: 14,
        density: 11.34,
        melting: 600.61,
        boiling: 2022.0,
        electronegativity: 2.33,
        config: "[Xe] 4f14 5d10 6s2 6p2",
    },
    Element {
        symbol: "Bi",
        name: "Bismuth",
        z: 83,
        mass: 208.98,
        category: "post-transition",
        period: 6,
        group: 15,
        density: 9.78,
        melting: 544.55,
        boiling: 1837.0,
        electronegativity: 2.02,
        config: "[Xe] 4f14 5d10 6s2 6p3",
    },
    Element {
        symbol: "Po",
        name: "Polonium",
        z: 84,
        mass: 209.0,
        category: "post-transition",
        period: 6,
        group: 16,
        density: 9.196,
        melting: 527.0,
        boiling: 1235.0,
        electronegativity: 2.00,
        config: "[Xe] 4f14 5d10 6s2 6p4",
    },
    Element {
        symbol: "At",
        name: "Astatine",
        z: 85,
        mass: 210.0,
        category: "halogen",
        period: 6,
        group: 17,
        density: 7.0,
        melting: 575.0,
        boiling: 610.0,
        electronegativity: 2.20,
        config: "[Xe] 4f14 5d10 6s2 6p5",
    },
    Element {
        symbol: "Rn",
        name: "Radon",
        z: 86,
        mass: 222.0,
        category: "noble",
        period: 6,
        group: 18,
        density: 0.00973,
        melting: 202.0,
        boiling: 211.5,
        electronegativity: 2.20,
        config: "[Xe] 4f14 5d10 6s2 6p6",
    },
    Element {
        symbol: "Fr",
        name: "Francium",
        z: 87,
        mass: 223.0,
        category: "alkali",
        period: 7,
        group: 1,
        density: 1.87,
        melting: 300.0,
        boiling: 950.0,
        electronegativity: 0.70,
        config: "[Rn] 7s1",
    },
    Element {
        symbol: "Ra",
        name: "Radium",
        z: 88,
        mass: 226.0,
        category: "alkaline",
        period: 7,
        group: 2,
        density: 5.0,
        melting: 973.0,
        boiling: 2010.0,
        electronegativity: 0.90,
        config: "[Rn] 7s2",
    },
    Element {
        symbol: "Ac",
        name: "Actinium",
        z: 89,
        mass: 227.0,
        category: "actinide",
        period: 7,
        group: 3,
        density: 10.07,
        melting: 1323.0,
        boiling: 3471.0,
        electronegativity: 1.10,
        config: "[Rn] 6d1 7s2",
    },
    Element {
        symbol: "Th",
        name: "Thorium",
        z: 90,
        mass: 232.04,
        category: "actinide",
        period: 7,
        group: 0,
        density: 11.72,
        melting: 2115.0,
        boiling: 5061.0,
        electronegativity: 1.30,
        config: "[Rn] 6d2 7s2",
    },
    Element {
        symbol: "Pa",
        name: "Protactinium",
        z: 91,
        mass: 231.04,
        category: "actinide",
        period: 7,
        group: 0,
        density: 15.37,
        melting: 1841.0,
        boiling: 4300.0,
        electronegativity: 1.50,
        config: "[Rn] 5f2 6d1 7s2",
    },
    Element {
        symbol: "U",
        name: "Uranium",
        z: 92,
        mass: 238.03,
        category: "actinide",
        period: 7,
        group: 0,
        density: 19.05,
        melting: 1405.3,
        boiling: 4404.0,
        electronegativity: 1.38,
        config: "[Rn] 5f3 6d1 7s2",
    },
    Element {
        symbol: "Np",
        name: "Neptunium",
        z: 93,
        mass: 237.0,
        category: "actinide",
        period: 7,
        group: 0,
        density: 20.45,
        melting: 912.0,
        boiling: 4175.0,
        electronegativity: 1.36,
        config: "[Rn] 5f4 6d1 7s2",
    },
    Element {
        symbol: "Pu",
        name: "Plutonium",
        z: 94,
        mass: 244.0,
        category: "actinide",
        period: 7,
        group: 0,
        density: 19.816,
        melting: 912.5,
        boiling: 3501.0,
        electronegativity: 1.28,
        config: "[Rn] 5f6 7s2",
    },
    Element {
        symbol: "Am",
        name: "Americium",
        z: 95,
        mass: 243.0,
        category: "actinide",
        period: 7,
        group: 0,
        density: 13.67,
        melting: 1449.0,
        boiling: 2880.0,
        electronegativity: 1.13,
        config: "[Rn] 5f7 7s2",
    },
    Element {
        symbol: "Cm",
        name: "Curium",
        z: 96,
        mass: 247.0,
        category: "actinide",
        period: 7,
        group: 0,
        density: 13.51,
        melting: 1613.0,
        boiling: 3383.0,
        electronegativity: 1.28,
        config: "[Rn] 5f7 6d1 7s2",
    },
    Element {
        symbol: "Bk",
        name: "Berkelium",
        z: 97,
        mass: 247.0,
        category: "actinide",
        period: 7,
        group: 0,
        density: 14.78,
        melting: 1259.0,
        boiling: 2900.0,
        electronegativity: 1.30,
        config: "[Rn] 5f9 7s2",
    },
    Element {
        symbol: "Cf",
        name: "Californium",
        z: 98,
        mass: 251.0,
        category: "actinide",
        period: 7,
        group: 0,
        density: 15.1,
        melting: 1173.0,
        boiling: 1743.0,
        electronegativity: 1.30,
        config: "[Rn] 5f10 7s2",
    },
    Element {
        symbol: "Es",
        name: "Einsteinium",
        z: 99,
        mass: 252.0,
        category: "actinide",
        period: 7,
        group: 0,
        density: 8.84,
        melting: 1133.0,
        boiling: 1269.0,
        electronegativity: 1.30,
        config: "[Rn] 5f11 7s2",
    },
    Element {
        symbol: "Fm",
        name: "Fermium",
        z: 100,
        mass: 257.0,
        category: "actinide",
        period: 7,
        group: 0,
        density: 0.0,
        melting: 1800.0,
        boiling: 0.0,
        electronegativity: 1.30,
        config: "[Rn] 5f12 7s2",
    },
    Element {
        symbol: "Md",
        name: "Mendelevium",
        z: 101,
        mass: 258.0,
        category: "actinide",
        period: 7,
        group: 0,
        density: 0.0,
        melting: 1100.0,
        boiling: 0.0,
        electronegativity: 1.30,
        config: "[Rn] 5f13 7s2",
    },
    Element {
        symbol: "No",
        name: "Nobelium",
        z: 102,
        mass: 259.0,
        category: "actinide",
        period: 7,
        group: 0,
        density: 0.0,
        melting: 1100.0,
        boiling: 0.0,
        electronegativity: 1.30,
        config: "[Rn] 5f14 7s2",
    },
    Element {
        symbol: "Lr",
        name: "Lawrencium",
        z: 103,
        mass: 262.0,
        category: "actinide",
        period: 7,
        group: 3,
        density: 0.0,
        melting: 1900.0,
        boiling: 0.0,
        electronegativity: 1.30,
        config: "[Rn] 5f14 7p1",
    },
    Element {
        symbol: "Rf",
        name: "Rutherfordium",
        z: 104,
        mass: 267.0,
        category: "transition",
        period: 7,
        group: 4,
        density: 23.2,
        melting: 2400.0,
        boiling: 5800.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d2 7s2",
    },
    Element {
        symbol: "Db",
        name: "Dubnium",
        z: 105,
        mass: 268.0,
        category: "transition",
        period: 7,
        group: 5,
        density: 29.3,
        melting: 0.0,
        boiling: 0.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d3 7s2",
    },
    Element {
        symbol: "Sg",
        name: "Seaborgium",
        z: 106,
        mass: 271.0,
        category: "transition",
        period: 7,
        group: 6,
        density: 35.0,
        melting: 0.0,
        boiling: 0.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d4 7s2",
    },
    Element {
        symbol: "Bh",
        name: "Bohrium",
        z: 107,
        mass: 272.0,
        category: "transition",
        period: 7,
        group: 7,
        density: 37.1,
        melting: 0.0,
        boiling: 0.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d5 7s2",
    },
    Element {
        symbol: "Hs",
        name: "Hassium",
        z: 108,
        mass: 270.0,
        category: "transition",
        period: 7,
        group: 8,
        density: 40.7,
        melting: 0.0,
        boiling: 0.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d6 7s2",
    },
    Element {
        symbol: "Mt",
        name: "Meitnerium",
        z: 109,
        mass: 276.0,
        category: "transition",
        period: 7,
        group: 9,
        density: 37.4,
        melting: 0.0,
        boiling: 0.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d7 7s2",
    },
    Element {
        symbol: "Ds",
        name: "Darmstadtium",
        z: 110,
        mass: 281.0,
        category: "transition",
        period: 7,
        group: 10,
        density: 34.8,
        melting: 0.0,
        boiling: 0.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d8 7s2",
    },
    Element {
        symbol: "Rg",
        name: "Roentgenium",
        z: 111,
        mass: 280.0,
        category: "transition",
        period: 7,
        group: 11,
        density: 28.7,
        melting: 0.0,
        boiling: 0.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d9 7s2",
    },
    Element {
        symbol: "Cn",
        name: "Copernicium",
        z: 112,
        mass: 285.0,
        category: "transition",
        period: 7,
        group: 12,
        density: 23.7,
        melting: 0.0,
        boiling: 357.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d10 7s2",
    },
    Element {
        symbol: "Nh",
        name: "Nihonium",
        z: 113,
        mass: 284.0,
        category: "post-transition",
        period: 7,
        group: 13,
        density: 16.0,
        melting: 700.0,
        boiling: 1430.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d10 7s2 7p1",
    },
    Element {
        symbol: "Fl",
        name: "Flerovium",
        z: 114,
        mass: 289.0,
        category: "post-transition",
        period: 7,
        group: 14,
        density: 14.0,
        melting: 340.0,
        boiling: 420.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d10 7s2 7p2",
    },
    Element {
        symbol: "Mc",
        name: "Moscovium",
        z: 115,
        mass: 288.0,
        category: "post-transition",
        period: 7,
        group: 15,
        density: 13.5,
        melting: 670.0,
        boiling: 1400.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d10 7s2 7p3",
    },
    Element {
        symbol: "Lv",
        name: "Livermorium",
        z: 116,
        mass: 293.0,
        category: "post-transition",
        period: 7,
        group: 16,
        density: 12.9,
        melting: 709.0,
        boiling: 1085.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d10 7s2 7p4",
    },
    Element {
        symbol: "Ts",
        name: "Tennessine",
        z: 117,
        mass: 294.0,
        category: "halogen",
        period: 7,
        group: 17,
        density: 7.2,
        melting: 723.0,
        boiling: 883.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d10 7s2 7p5",
    },
    Element {
        symbol: "Og",
        name: "Oganesson",
        z: 118,
        mass: 294.0,
        category: "noble",
        period: 7,
        group: 18,
        density: 5.0,
        melting: 325.0,
        boiling: 450.0,
        electronegativity: 0.00,
        config: "[Rn] 5f14 6d10 7s2 7p6",
    },
];

// ── Lookup helpers ────────────────────────────────────────────────────────────

fn find_element(query: &str) -> Option<&'static Element> {
    let q = query.trim().to_lowercase();
    // Try atomic number
    if let Ok(n) = q.parse::<u32>() {
        return ELEMENTS.iter().find(|e| e.z == n);
    }
    // Try symbol (exact, case-insensitive)
    if let Some(e) = ELEMENTS.iter().find(|e| e.symbol.to_lowercase() == q) {
        return Some(e);
    }
    // Try name
    ELEMENTS.iter().find(|e| e.name.to_lowercase() == q)
}

fn fmt_temp(k: f64) -> String {
    if k <= 0.0 {
        return "N/A".to_string();
    }
    let c = k - 273.15;
    format!("{:.2} K ({:.2}°C / {:.2}°F)", k, c, c * 9.0 / 5.0 + 32.0)
}

fn fmt_density(d: f64) -> String {
    if d <= 0.0 {
        return "N/A".to_string();
    }
    format!("{} g/cm³", d)
}

fn element_detail(e: &Element) -> String {
    let mut out = format!("─── {} ({}) — Z={} ───\n", e.name, e.symbol, e.z);
    out += &format!("Atomic mass:        {:.4} u\n", e.mass);
    out += &format!("Category:           {}\n", e.category);
    out += &format!(
        "Period / Group:     {} / {}\n",
        e.period,
        if e.group == 0 {
            "—".to_string()
        } else {
            e.group.to_string()
        }
    );
    out += &format!("Density:            {}\n", fmt_density(e.density));
    out += &format!("Melting point:      {}\n", fmt_temp(e.melting));
    out += &format!("Boiling point:      {}\n", fmt_temp(e.boiling));
    out += &format!(
        "Electronegativity:  {}\n",
        if e.electronegativity == 0.0 {
            "N/A".to_string()
        } else {
            format!("{:.2} (Pauling)", e.electronegativity)
        }
    );
    out += &format!("Electron config:    {}\n", e.config);
    out
}

// ── Molar mass calculator ─────────────────────────────────────────────────────

fn parse_formula_mass(formula: &str) -> Result<f64, String> {
    let chars: Vec<char> = formula.chars().collect();
    let mut total = 0.0;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_uppercase() {
            // Parse element symbol
            let mut sym = String::from(c);
            i += 1;
            while i < chars.len() && chars[i].is_lowercase() {
                sym.push(chars[i]);
                i += 1;
            }
            // Parse optional count
            let mut count_str = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                count_str.push(chars[i]);
                i += 1;
            }
            let count: f64 = if count_str.is_empty() {
                1.0
            } else {
                count_str.parse().unwrap_or(1.0)
            };
            let elem = ELEMENTS
                .iter()
                .find(|e| e.symbol == sym)
                .ok_or_else(|| format!("Unknown element symbol '{}'", sym))?;
            total += elem.mass * count;
        } else if c == '(' || c == '[' {
            // Nested groups not supported — skip
            i += 1;
        } else if c == ')' || c == ']' {
            i += 1;
        } else if c.is_ascii_digit() {
            // Trailing number after ) — not handled
            i += 1;
        } else {
            i += 1;
        }
    }
    Ok(total)
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_element(args: &Value) -> Result<String, String> {
    let query = args
        .get("symbol")
        .and_then(|v| v.as_str())
        .or_else(|| args.get("name").and_then(|v| v.as_str()))
        .or_else(|| args.get("query").and_then(|v| v.as_str()));

    if let Some(q) = query {
        let e = find_element(q).ok_or_else(|| format!("Element not found: '{}'", q))?;
        return Ok(element_detail(e));
    }

    if let Some(n) = args.get("number").and_then(|v| v.as_u64()) {
        let e = ELEMENTS
            .iter()
            .find(|e| e.z == n as u32)
            .ok_or_else(|| format!("No element with atomic number {}", n))?;
        return Ok(element_detail(e));
    }

    Err("Provide 'symbol', 'name', or 'number' to look up an element".to_string())
}

fn action_search(args: &Value) -> Result<String, String> {
    let query = args
        .get("query")
        .or_else(|| args.get("q"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("");

    let lower = query.to_lowercase();
    let cat_lower = category.to_lowercase();

    let matches: Vec<&Element> = ELEMENTS
        .iter()
        .filter(|e| {
            let name_ok = lower.is_empty()
                || e.name.to_lowercase().contains(&lower)
                || e.symbol.to_lowercase().contains(&lower);
            let cat_ok = cat_lower.is_empty() || e.category.to_lowercase().contains(&cat_lower);
            name_ok && cat_ok
        })
        .collect();

    if matches.is_empty() {
        return Ok(format!("No elements found matching '{}'", query));
    }

    let mut out = format!("Found {} element(s):\n\n", matches.len());
    out += &format!(
        "{:<4} {:<14} {:<18} {:<12} {:<8} {:<8}\n",
        "Z", "Symbol/Name", "Category", "Mass (u)", "Period", "Group"
    );
    out += &format!("{}\n", "─".repeat(68));
    for e in &matches {
        out += &format!(
            "{:<4} {:<3} {:<12} {:<18} {:<12.4} {:<8} {:<8}\n",
            e.z,
            e.symbol,
            e.name,
            e.category,
            e.mass,
            e.period,
            if e.group == 0 {
                "—".to_string()
            } else {
                e.group.to_string()
            }
        );
    }
    Ok(out)
}

fn action_list(args: &Value) -> Result<String, String> {
    let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("");
    let period = args.get("period").and_then(|v| v.as_u64());
    let group = args.get("group").and_then(|v| v.as_u64());
    let cat_lower = category.to_lowercase();

    let matches: Vec<&Element> = ELEMENTS
        .iter()
        .filter(|e| {
            let cat_ok = cat_lower.is_empty() || e.category.to_lowercase().contains(&cat_lower);
            let period_ok = period.map(|p| e.period as u64 == p).unwrap_or(true);
            let group_ok = group.map(|g| e.group as u64 == g).unwrap_or(true);
            cat_ok && period_ok && group_ok
        })
        .collect();

    let mut out = format!("Listing {} element(s)", matches.len());
    if !category.is_empty() {
        out += &format!(" | Category: {}", category);
    }
    if let Some(p) = period {
        out += &format!(" | Period: {}", p);
    }
    if let Some(g) = group {
        out += &format!(" | Group: {}", g);
    }
    out += "\n\n";
    out += &format!(
        "{:<4} {:<3} {:<14} {:<18} {:<12}\n",
        "Z", "Sym", "Name", "Category", "Mass (u)"
    );
    out += &format!("{}\n", "─".repeat(55));
    for e in &matches {
        out += &format!(
            "{:<4} {:<3} {:<14} {:<18} {:.4}\n",
            e.z, e.symbol, e.name, e.category, e.mass
        );
    }
    Ok(out)
}

fn action_compare(args: &Value) -> Result<String, String> {
    let q1 = args
        .get("symbol")
        .or_else(|| args.get("name"))
        .or_else(|| args.get("query"))
        .and_then(|v| v.as_str())
        .ok_or("Provide first element via 'symbol' or 'name'")?;
    let q2 = args
        .get("element2")
        .and_then(|v| v.as_str())
        .ok_or("Provide second element via 'element2'")?;

    let e1 = find_element(q1).ok_or_else(|| format!("Element not found: '{}'", q1))?;
    let e2 = find_element(q2).ok_or_else(|| format!("Element not found: '{}'", q2))?;

    let mut out = format!("{:<22} {:<20} {:<20}\n", "Property", e1.name, e2.name);
    out += &format!("{}\n", "─".repeat(62));
    let rows: &[(&str, String, String)] = &[
        ("Symbol", e1.symbol.to_string(), e2.symbol.to_string()),
        ("Atomic number", e1.z.to_string(), e2.z.to_string()),
        (
            "Atomic mass (u)",
            format!("{:.4}", e1.mass),
            format!("{:.4}", e2.mass),
        ),
        ("Category", e1.category.to_string(), e2.category.to_string()),
        ("Period", e1.period.to_string(), e2.period.to_string()),
        (
            "Group",
            if e1.group == 0 {
                "—".to_string()
            } else {
                e1.group.to_string()
            },
            if e2.group == 0 {
                "—".to_string()
            } else {
                e2.group.to_string()
            },
        ),
        (
            "Density (g/cm³)",
            fmt_density(e1.density),
            fmt_density(e2.density),
        ),
        (
            "Melting (K)",
            if e1.melting > 0.0 {
                format!("{:.2}", e1.melting)
            } else {
                "N/A".to_string()
            },
            if e2.melting > 0.0 {
                format!("{:.2}", e2.melting)
            } else {
                "N/A".to_string()
            },
        ),
        (
            "Boiling (K)",
            if e1.boiling > 0.0 {
                format!("{:.2}", e1.boiling)
            } else {
                "N/A".to_string()
            },
            if e2.boiling > 0.0 {
                format!("{:.2}", e2.boiling)
            } else {
                "N/A".to_string()
            },
        ),
        (
            "Electronegativity",
            if e1.electronegativity > 0.0 {
                format!("{:.2}", e1.electronegativity)
            } else {
                "N/A".to_string()
            },
            if e2.electronegativity > 0.0 {
                format!("{:.2}", e2.electronegativity)
            } else {
                "N/A".to_string()
            },
        ),
        (
            "Electron config",
            e1.config.to_string(),
            e2.config.to_string(),
        ),
    ];
    for (prop, v1, v2) in rows {
        out += &format!("{:<22} {:<20} {:<20}\n", prop, v1, v2);
    }
    Ok(out)
}

fn action_mass(args: &Value) -> Result<String, String> {
    let formula = args
        .get("formula")
        .or_else(|| args.get("name"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'formula' (e.g. 'H2O', 'NaCl', 'C6H12O6')")?;

    let mass = parse_formula_mass(formula)?;

    let mut out = format!("Formula: {}\n", formula);
    out += &format!("Molar mass: {:.4} g/mol\n\n", mass);

    // Break down by element
    out += "Composition:\n";
    let chars: Vec<char> = formula.chars().collect();
    let mut i = 0;
    let mut total_mass = 0.0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_uppercase() {
            let mut sym = String::from(c);
            i += 1;
            while i < chars.len() && chars[i].is_lowercase() {
                sym.push(chars[i]);
                i += 1;
            }
            let mut count_str = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                count_str.push(chars[i]);
                i += 1;
            }
            let count: f64 = if count_str.is_empty() {
                1.0
            } else {
                count_str.parse().unwrap_or(1.0)
            };
            if let Some(elem) = ELEMENTS.iter().find(|e| e.symbol == sym) {
                let contribution = elem.mass * count;
                total_mass += contribution;
                let pct = 100.0 * contribution / mass;
                out += &format!(
                    "  {}{}: {:.4} × {:.4} = {:.4} g/mol ({:.2}%)\n",
                    sym,
                    if count as u32 == 1 {
                        String::new()
                    } else {
                        format!("{}", count as u32)
                    },
                    count,
                    elem.mass,
                    contribution,
                    pct
                );
            }
        } else {
            i += 1;
        }
    }
    let _ = total_mass;
    Ok(out)
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("element");
    match action {
        "element" => action_element(args),
        "search" => action_search(args),
        "list" => action_list(args),
        "compare" => action_compare(args),
        "mass" => action_mass(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: element, search, list, compare, mass",
            action
        )),
    }
}
