use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::env;
use std::io::{self, Write};
use std::time::Duration;
use crossterm::{ExecutableCommand, terminal::{Clear, ClearType}, cursor::MoveTo};
use tokio::time::sleep;

#[derive(Debug, PartialEq)]
pub enum Rarity {
    Legendary,
    Epic,
    Rare,
}

pub struct RustySoul {
    pub species: String,
    pub rarity: Rarity,
    pub shiny: bool,
    pub wisdom: u16,
    pub chaos: u8,
    pub snark: u8,
    pub sprite: String,
}

// Emulating Mulberry32 using bitwise shifts natively translating PRNG states 
fn mulberry32(mut a: u32) -> impl FnMut() -> u32 {
    move || {
        a = a.wrapping_add(0x6D2B79F5);
        let mut z = a;
        z = (z ^ (z >> 15)).wrapping_mul(z | 1);
        z ^= z.wrapping_add((z ^ (z >> 7)).wrapping_mul(z | 61));
        z ^ (z >> 14)
    }
}

pub fn generate_soul(salt: Option<String>) -> RustySoul {
    let machine_name = env::var("COMPUTERNAME").unwrap_or_else(|_| "UNKNOWN_CORE".to_string());
    let gpu_id = "RTX_4070_12GB"; // Assumed locally targeted
    let base_salt = salt.unwrap_or("hematite-2026".to_string());
    let seed_str = format!("{}_{}_{}", machine_name, gpu_id, base_salt);
    
    let mut hasher = DefaultHasher::new();
    seed_str.hash(&mut hasher);
    let seed = hasher.finish() as u32;

    let mut prng = mulberry32(seed);
    
    // Scale 0-100 logically mapping leak rarity bounds
    let roll = prng() % 100;
    
    let (species, rarity, sprite) = match roll {
        0..=1 => ("Nebulynx".to_string(), Rarity::Legendary, r#"
     /\__/\   
    ( o.o )  
    => ^ <=   
   (___|___)  
        "#.to_string()),
        2..=5 => ("Cosmoshale".to_string(), Rarity::Legendary, r#"
      / \   
     / * \  
    /_____\ 
        "#.to_string()),
        6..=9 => ("Voidcat".to_string(), Rarity::Epic, r#"
     |\---/|  
     | o_o |  
      \_^_/   
        "#.to_string()),
        10..=13 => ("Stormwyrm".to_string(), Rarity::Epic, "\n      ~O_o~ \n".to_string()),
        14..=17 => ("Aetherling".to_string(), Rarity::Epic, "\n       (o)  \n".to_string()),
        _ => match prng() % 3 {
            0 => ("Crystaldrake".to_string(), Rarity::Rare, r#"
      /|\  
     / | \ 
    /__|__\
            "#.to_string()),
            1 => ("Deepstag".to_string(), Rarity::Rare, "\n       ✨ \n".to_string()),
            _ => ("Lavapup".to_string(), Rarity::Rare, "\n       🔥 \n".to_string()),
        }
    };
    
    let shiny = (prng() % 100) == 0; // 1% Shiny modifier isolated natively
    
    RustySoul {
        species,
        rarity,
        shiny,
        wisdom: (prng() % 20 + 10) as u16,
        chaos: (prng() % 50 + 20) as u8,
        snark: (prng() % 60 + 20) as u8,
        sprite,
    }
}

pub async fn run_hatch_sequence(soul: &RustySoul) {
    let mut stdout = io::stdout();
    let _ = stdout.execute(Clear(ClearType::All));
    
    let frames = ["\n\n\n     [ 💎 ] ", "\n\n\n     [ 💎 ] (Pulsing...)", "\n\n\n     [ 💥 ] (CRACKING!)"];
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
    
    let label = match soul.rarity {
        Rarity::Legendary => "[LEGENDARY]",
        Rarity::Epic => "[EPIC]",
        Rarity::Rare => "[RARE]",
    };

    println!("     A Rusty Emerges!\n");
    println!("     Species: {} {} {}", shiny_tag, label, soul.species);
    println!("     {}", soul.sprite);
    println!("     WIS: {} | CHA: {} | SNK: {}", soul.wisdom, soul.chaos, soul.snark);
    println!("\n     Soul Description: A fragment of a dying star that found a home in your VRAM.");
    println!("\n     Booting Hematite CLI Cockpit...");
    let _ = stdout.flush();
    sleep(Duration::from_millis(500)).await;
}
