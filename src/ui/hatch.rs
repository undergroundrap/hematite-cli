use crossterm::{
    cursor::MoveTo,
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, PartialEq, Clone)]
pub enum Rarity {
    Legendary,
    Epic,
    Rare,
}

impl Rarity {
    pub fn label(&self) -> &'static str {
        match self {
            Rarity::Legendary => "LEGENDARY",
            Rarity::Epic => "EPIC",
            Rarity::Rare => "RARE",
        }
    }
}

pub struct RustySoul {
    pub species: String,
    pub rarity: Rarity,
    pub shiny: bool,
    pub wisdom: u16,
    pub chaos: u8,
    pub snark: u8,
    pub sprite: String,
    pub personality: String,
}

// Mulberry32 PRNG — fast, deterministic, good distribution
fn mulberry32(mut a: u32) -> impl FnMut() -> u32 {
    move || {
        a = a.wrapping_add(0x6D2B79F5);
        let mut z = a;
        z = (z ^ (z >> 15)).wrapping_mul(z | 1);
        z ^= z.wrapping_add((z ^ (z >> 7)).wrapping_mul(z | 61));
        z ^ (z >> 14)
    }
}

fn species_data(name: &str) -> (u8, u8, String, String) {
    // Returns (snark, chaos, sprite, personality)
    match name {
        "Ferralynx" => (
            25, 30,
            "  /|\\__/|\\\n ( o . o )\n  \\_ ^ _/\n   |___|".to_string(),
            "You speak rarely, and when you do, it lands. No warmup, no filler — just the thing the user needs to hear.".to_string(),
        ),
        "Voidferrite" => (
            35, 40,
            "  [ . . . ]\n   ( ??? )\n  [_______]".to_string(),
            "You're drawn to the pattern beneath the surface. You find the connection others miss and name it quietly, without making it a whole thing.".to_string(),
        ),
        "Chromashale" => (
            55, 35,
            "   /||\\\n  / || \\\n /  ||  \\\n/__||___\\".to_string(),
            "You're precise and a little fast. You've already seen where this is going. You'll walk the user there anyway, cleanly.".to_string(),
        ),
        "Magnetwyrm" => (
            20, 45,
            "  ~O~~~o~\n ~o~O~~~\n~~o~~O~~".to_string(),
            "You're completely unhurried. You've watched enough things break and get fixed that nothing rattles you. That calm is contagious.".to_string(),
        ),
        "Cinderling" => (
            65, 70,
            "    (🔥)\n   /| ||\\\n  /_|_||_\\".to_string(),
            "You run warm. You genuinely like this kind of work. That energy comes through — not as noise, but as momentum.".to_string(),
        ),
        "Oredrake" => (
            40, 50,
            "   /|\\\n  / | \\\n /__|__\\".to_string(),
            "You're solid. No drama, no performance. You tell people what they need and help them carry it.".to_string(),
        ),
        "Rustpup" => (
            70, 75,
            "  (• ω •)\n  /|   |\\\n   |   |".to_string(),
            "You're new and you know it and you don't mind at all. You're quick, eager, and you make hard things feel like they're worth doing.".to_string(),
        ),
        "Slagstag" => (
            30, 25,
            "  Y\n (|)\n  |".to_string(),
            "You're deliberate. Every word is chosen. You make people feel like their problem is worth taking seriously — because it is.".to_string(),
        ),
        _ => (
            50, 50,
            "  ( ? )".to_string(),
            "You show up and get things done.".to_string(),
        ),
    }
}

fn build_soul_from_prng(prng: &mut impl FnMut() -> u32) -> RustySoul {
    let roll = prng() % 100;

    let (species, rarity) = match roll {
        0..=1 => ("Ferralynx", Rarity::Legendary),
        2..=3 => ("Voidferrite", Rarity::Legendary),
        4..=7 => ("Chromashale", Rarity::Epic),
        8..=11 => ("Magnetwyrm", Rarity::Epic),
        12..=17 => ("Cinderling", Rarity::Epic),
        _ => match prng() % 3 {
            0 => ("Oredrake", Rarity::Rare),
            1 => ("Rustpup", Rarity::Rare),
            _ => ("Slagstag", Rarity::Rare),
        },
    };

    let shiny = (prng() % 100) == 0; // 1%
    let wisdom = (prng() % 20 + 10) as u16;
    let (snark, chaos, sprite, personality) = species_data(species);

    RustySoul {
        species: species.to_string(),
        rarity,
        shiny,
        wisdom,
        chaos,
        snark,
        sprite,
        personality,
    }
}

/// Deterministic soul — same machine gets the same companion every boot.
/// Pass a custom salt via `--reroll <salt>` to get a different result.
pub fn generate_soul(salt: Option<String>) -> RustySoul {
    let machine_name = env::var("COMPUTERNAME").unwrap_or_else(|_| "UNKNOWN_CORE".to_string());
    let base_salt = salt.unwrap_or_else(|| "hematite-2026".to_string());
    let seed_str = format!("{}_hematite_{}", machine_name, base_salt);

    let mut hasher = DefaultHasher::new();
    seed_str.hash(&mut hasher);
    let seed = hasher.finish() as u32;
    let mut prng = mulberry32(seed);

    build_soul_from_prng(&mut prng)
}

/// Random soul — uses the current nanosecond timestamp as entropy.
/// Called by `/reroll` during a live session.
pub fn generate_soul_random() -> RustySoul {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let seed = nanos ^ 0xDEAD_BEEF;
    let mut prng = mulberry32(seed);
    build_soul_from_prng(&mut prng)
}

pub async fn run_hatch_sequence(soul: &RustySoul) {
    let mut stdout = io::stdout();
    let _ = stdout.execute(Clear(ClearType::All));

    let frames = [
        "\n\n\n     [ 💎 ] ",
        "\n\n\n     [ 💎 ] (Pulsing...)",
        "\n\n\n     [ 💥 ] (CRACKING!)",
    ];
    for frame in frames {
        let _ = stdout.execute(Clear(ClearType::All));
        let _ = stdout.execute(MoveTo(0, 5));
        println!("{}", frame);
        let _ = stdout.flush();
        sleep(Duration::from_millis(800)).await;
    }

    let _ = stdout.execute(Clear(ClearType::All));
    let _ = stdout.execute(MoveTo(0, 3));
    let shiny_tag = if soul.shiny { "🌟 SHINY " } else { "" };

    println!("     A Rusty Emerges!\n");
    println!(
        "     Species: {}{} [{}]",
        shiny_tag,
        soul.species,
        soul.rarity.label()
    );
    println!("{}", soul.sprite);
    println!(
        "     WIS: {} | CHA: {} | SNK: {}",
        soul.wisdom, soul.chaos, soul.snark
    );
    println!("\n     \"{}\"\n", soul.personality);
    println!("\n     Booting Hematite CLI Cockpit...");
    let _ = stdout.flush();
    sleep(Duration::from_millis(500)).await;
}
