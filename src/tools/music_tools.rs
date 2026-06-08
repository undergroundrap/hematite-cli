use serde_json::{json, Value};

pub fn music_tools_schema() -> Value {
    json!({
        "name": "music_tools",
        "description": "Music theory calculations without external utilities. Actions: note (note name ↔ frequency in Hz; A4=440 Hz reference), chord (list the notes in a named chord or detect chord from a note list), scale (list all notes in a named scale starting from a root), interval (name the interval between two notes), midi (note name ↔ MIDI number; A4=69), tempo (BPM ↔ note duration in milliseconds).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["note", "chord", "scale", "interval", "midi", "tempo"],
                    "description": "Action to perform (default: note)"
                },
                "note": {
                    "type": "string",
                    "description": "Note name like 'A4', 'C#3', 'Bb5' for note/midi/interval actions"
                },
                "note2": {
                    "type": "string",
                    "description": "Second note for 'interval' action"
                },
                "frequency": {
                    "type": "number",
                    "description": "Frequency in Hz — reverse-lookup nearest note name"
                },
                "midi": {
                    "type": "integer",
                    "description": "MIDI note number (0–127) for 'midi' reverse-lookup"
                },
                "root": {
                    "type": "string",
                    "description": "Root note (e.g. 'C', 'F#', 'Bb') for chord/scale actions"
                },
                "quality": {
                    "type": "string",
                    "description": "Chord or scale quality (e.g. 'major', 'minor', 'dominant7', 'pentatonic', 'blues')"
                },
                "notes": {
                    "type": "array",
                    "description": "Array of note names to detect chord from (e.g. ['C','E','G'])",
                    "items": {"type": "string"}
                },
                "bpm": {
                    "type": "number",
                    "description": "BPM for 'tempo' action"
                },
                "duration": {
                    "type": "string",
                    "description": "Note duration for 'tempo' action: whole/half/quarter/eighth/sixteenth"
                }
            },
            "required": []
        }
    })
}

// ── Note names and frequencies ────────────────────────────────────────────────

const NOTE_NAMES: &[&str] = &[
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const ENHARMONICS: &[(&str, &str)] = &[
    ("Db", "C#"),
    ("Eb", "D#"),
    ("Fb", "E"),
    ("Gb", "F#"),
    ("Ab", "G#"),
    ("Bb", "A#"),
    ("Cb", "B"),
    ("E#", "F"),
    ("B#", "C"),
];

fn normalize_note_name(name: &str) -> String {
    let trimmed = name.trim();
    for &(enh, sharp) in ENHARMONICS {
        if trimmed.eq_ignore_ascii_case(enh) {
            return sharp.to_string();
        }
    }
    // Capitalize first letter
    let mut chars = trimmed.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn parse_note(s: &str) -> Option<(usize, i32)> {
    // Returns (semitone index 0-11, octave)
    let s = s.trim();
    let norm = normalize_note_name(s);
    // Try 2-char accidental first (e.g. "C#"), then 1-char
    let (pitch_str, octave_str) =
        if norm.len() >= 2 && (norm.as_bytes()[1] == b'#' || norm.as_bytes()[1] == b'b') {
            (&norm[..2], &norm[2..])
        } else {
            (&norm[..1], &norm[1..])
        };
    let semitone = NOTE_NAMES.iter().position(|&n| n == pitch_str)?;
    let octave: i32 = octave_str.parse().ok().unwrap_or(4);
    Some((semitone, octave))
}

fn note_to_midi(semitone: usize, octave: i32) -> i32 {
    // MIDI 69 = A4
    (octave + 1) * 12 + semitone as i32
}

fn midi_to_note(midi: i32) -> String {
    let octave = midi / 12 - 1;
    let semitone = (midi % 12) as usize;
    format!("{}{}", NOTE_NAMES[semitone], octave)
}

fn midi_to_freq(midi: i32) -> f64 {
    // A4 = 69 = 440 Hz
    440.0 * 2f64.powf((midi - 69) as f64 / 12.0)
}

fn freq_to_midi(freq: f64) -> i32 {
    (69.0 + 12.0 * (freq / 440.0).log2()).round() as i32
}

// ── Intervals ─────────────────────────────────────────────────────────────────

const INTERVAL_NAMES: &[&str] = &[
    "Unison",
    "Minor Second",
    "Major Second",
    "Minor Third",
    "Major Third",
    "Perfect Fourth",
    "Tritone",
    "Perfect Fifth",
    "Minor Sixth",
    "Major Sixth",
    "Minor Seventh",
    "Major Seventh",
    "Octave",
];

fn semitones_to_interval(semitones: u32) -> String {
    let wrapped = (semitones % 12) as usize;
    if semitones == 0 {
        return "Unison (0 semitones)".to_string();
    }
    if semitones % 12 == 0 {
        return format!("{} octave(s)", semitones / 12);
    }
    let octaves = semitones / 12;
    let name = INTERVAL_NAMES[wrapped];
    if octaves > 0 {
        format!("{} + {} octave(s) ({} semitones)", name, octaves, semitones)
    } else {
        format!(
            "{} ({} semitone{})",
            name,
            semitones,
            if semitones == 1 { "" } else { "s" }
        )
    }
}

// ── Chords ────────────────────────────────────────────────────────────────────

// Returns semitone intervals from root
#[allow(dead_code)]
fn chord_intervals(quality: &str) -> Option<Vec<u32>> {
    let q = quality.to_lowercase();
    let q = q.trim();
    match q {
        "major" | "maj" | "M" | "m" => None, // handled below
        _ => None,
    }?
}

fn chord_intervals_str(quality: &str) -> Option<Vec<u32>> {
    let q = quality.to_lowercase();
    let q = q.trim();
    Some(match q {
        "major" | "maj" | "" => vec![0, 4, 7],
        "minor" | "min" | "m" => vec![0, 3, 7],
        "diminished" | "dim" => vec![0, 3, 6],
        "augmented" | "aug" => vec![0, 4, 8],
        "sus2" => vec![0, 2, 7],
        "sus4" => vec![0, 5, 7],
        "dominant7" | "dom7" | "7" => vec![0, 4, 7, 10],
        "major7" | "maj7" | "M7" => vec![0, 4, 7, 11],
        "minor7" | "min7" | "m7" => vec![0, 3, 7, 10],
        "diminished7" | "dim7" => vec![0, 3, 6, 9],
        "half-diminished" | "half_diminished" | "m7b5" => vec![0, 3, 6, 10],
        "augmented7" | "aug7" => vec![0, 4, 8, 10],
        "major9" | "maj9" => vec![0, 4, 7, 11, 14],
        "minor9" | "min9" | "m9" => vec![0, 3, 7, 10, 14],
        "dominant9" | "9" => vec![0, 4, 7, 10, 14],
        "add9" => vec![0, 4, 7, 14],
        "power" | "5" => vec![0, 7],
        "6" | "major6" => vec![0, 4, 7, 9],
        "minor6" | "m6" => vec![0, 3, 7, 9],
        _ => return None,
    })
}

// ── Scales ────────────────────────────────────────────────────────────────────

fn scale_intervals(quality: &str) -> Option<Vec<u32>> {
    let q = quality.to_lowercase();
    let q = q.trim();
    Some(match q {
        "major" | "ionian" => vec![0, 2, 4, 5, 7, 9, 11],
        "natural minor" | "minor" | "aeolian" => vec![0, 2, 3, 5, 7, 8, 10],
        "harmonic minor" => vec![0, 2, 3, 5, 7, 8, 11],
        "melodic minor" => vec![0, 2, 3, 5, 7, 9, 11],
        "dorian" => vec![0, 2, 3, 5, 7, 9, 10],
        "phrygian" => vec![0, 1, 3, 5, 7, 8, 10],
        "lydian" => vec![0, 2, 4, 6, 7, 9, 11],
        "mixolydian" => vec![0, 2, 4, 5, 7, 9, 10],
        "locrian" => vec![0, 1, 3, 5, 6, 8, 10],
        "pentatonic" | "major pentatonic" => vec![0, 2, 4, 7, 9],
        "minor pentatonic" => vec![0, 3, 5, 7, 10],
        "blues" => vec![0, 3, 5, 6, 7, 10],
        "whole tone" => vec![0, 2, 4, 6, 8, 10],
        "diminished" | "octatonic" => vec![0, 2, 3, 5, 6, 8, 9, 11],
        "chromatic" => vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        "in" | "japanese in" => vec![0, 1, 5, 7, 8],
        "insen" => vec![0, 1, 5, 7, 10],
        "yo" => vec![0, 2, 5, 7, 9],
        "hirajoshi" => vec![0, 4, 6, 7, 11],
        "enigmatic" => vec![0, 1, 4, 6, 8, 10, 11],
        "prometheus" => vec![0, 2, 4, 6, 9, 10],
        _ => return None,
    })
}

#[allow(dead_code)]
fn semitone_to_name(semitone: u32) -> &'static str {
    NOTE_NAMES[semitone as usize % 12]
}

// ── Chord detection ───────────────────────────────────────────────────────────

fn detect_chord(notes: &[&str]) -> String {
    // Normalize to semitone classes
    let mut semitones: Vec<u32> = notes
        .iter()
        .filter_map(|n| {
            let norm = normalize_note_name(n);
            NOTE_NAMES
                .iter()
                .position(|&x| x == norm.trim_end_matches(|c: char| c.is_ascii_digit()))
                .map(|i| i as u32)
        })
        .collect();
    semitones.sort_unstable();
    semitones.dedup();
    if semitones.is_empty() {
        return "No recognized notes".to_string();
    }

    let known_chords = [
        ("major", vec![0u32, 4, 7]),
        ("minor", vec![0, 3, 7]),
        ("diminished", vec![0, 3, 6]),
        ("augmented", vec![0, 4, 8]),
        ("sus2", vec![0, 2, 7]),
        ("sus4", vec![0, 5, 7]),
        ("dominant7", vec![0, 4, 7, 10]),
        ("major7", vec![0, 4, 7, 11]),
        ("minor7", vec![0, 3, 7, 10]),
        ("diminished7", vec![0, 3, 6, 9]),
        ("half-diminished", vec![0, 3, 6, 10]),
        ("power", vec![0, 7]),
        ("major6", vec![0, 4, 7, 9]),
        ("minor6", vec![0, 3, 7, 9]),
    ];

    let mut matches = Vec::new();
    for root in 0u32..12 {
        // Normalize all semitones relative to this root
        let normalized: Vec<u32> = semitones.iter().map(|&s| (s + 12 - root) % 12).collect();
        let mut sorted = normalized.clone();
        sorted.sort_unstable();
        sorted.dedup();
        for (name, pattern) in &known_chords {
            if sorted == *pattern {
                matches.push(format!("{} {}", NOTE_NAMES[root as usize], name));
            }
        }
    }

    if matches.is_empty() {
        format!("No standard chord match for {}", notes.join(", "))
    } else {
        format!("Possible chords: {}", matches.join(", "))
    }
}

// ── Tempo ─────────────────────────────────────────────────────────────────────

fn note_duration_ms(bpm: f64, duration: &str) -> Option<f64> {
    let beat_ms = 60_000.0 / bpm;
    Some(match duration.to_lowercase().trim() {
        "whole" | "1" => beat_ms * 4.0,
        "half" | "2" => beat_ms * 2.0,
        "quarter" | "4" => beat_ms,
        "eighth" | "8" => beat_ms / 2.0,
        "sixteenth" | "16" => beat_ms / 4.0,
        "thirty-second" | "32" => beat_ms / 8.0,
        "dotted whole" => beat_ms * 6.0,
        "dotted half" => beat_ms * 3.0,
        "dotted quarter" => beat_ms * 1.5,
        "dotted eighth" => beat_ms * 0.75,
        "triplet quarter" => beat_ms * 2.0 / 3.0,
        "triplet eighth" => beat_ms / 3.0,
        _ => return None,
    })
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_note(args: &Value) -> Result<String, String> {
    if let Some(freq) = args.get("frequency").and_then(|v| v.as_f64()) {
        let midi = freq_to_midi(freq);
        let midi = midi.clamp(0, 127);
        let name = midi_to_note(midi);
        let exact_freq = midi_to_freq(midi);
        let cents = 1200.0 * (freq / exact_freq).log2();
        let cents_str = if cents.abs() < 0.5 {
            "exact".to_string()
        } else {
            format!("{:+.1} cents", cents)
        };
        return Ok(format!(
            "Nearest note: {}\nMIDI number:  {}\nNote freq:    {:.3} Hz\nInput freq:   {:.3} Hz\nTuning:       {}\n",
            name, midi, exact_freq, freq, cents_str
        ));
    }

    let note_str = args
        .get("note")
        .and_then(|v| v.as_str())
        .ok_or("'note' or 'frequency' is required")?;

    let (semitone, octave) =
        parse_note(note_str).ok_or_else(|| format!("Cannot parse note '{}'", note_str))?;
    let midi = note_to_midi(semitone, octave);
    let freq = midi_to_freq(midi);
    let norm = midi_to_note(midi);

    let mut out = format!("Note:       {}\n", norm);
    out += &format!("Frequency:  {:.3} Hz\n", freq);
    out += &format!("MIDI:       {}\n", midi);
    out += &format!("Semitone:   {} ({})\n", semitone, NOTE_NAMES[semitone]);
    out += &format!("Octave:     {}\n", octave);
    // Nearby notes
    out += "\nNeighboring notes:\n";
    for delta in [-2i32, -1, 1, 2] {
        let m = midi + delta;
        if (0..=127).contains(&m) {
            out += &format!(
                "  {:+}: {} = {:.3} Hz\n",
                delta,
                midi_to_note(m),
                midi_to_freq(m)
            );
        }
    }
    Ok(out)
}

fn action_chord(args: &Value) -> Result<String, String> {
    // Detect from list
    if let Some(arr) = args.get("notes").and_then(|v| v.as_array()) {
        let notes: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
        if !notes.is_empty() {
            return Ok(detect_chord(&notes) + "\n");
        }
    }

    let root = args
        .get("root")
        .and_then(|v| v.as_str())
        .ok_or("'root' note is required for chord action")?;
    let quality = args
        .get("quality")
        .and_then(|v| v.as_str())
        .unwrap_or("major");

    let intervals = chord_intervals_str(quality)
        .ok_or_else(|| format!("Unknown chord quality '{}'. Try: major, minor, dominant7, major7, minor7, diminished, augmented, sus2, sus4, power, diminished7, half-diminished, major9, minor9, add9", quality))?;

    let root_norm = normalize_note_name(root);
    let root_idx = NOTE_NAMES
        .iter()
        .position(|&n| n == root_norm)
        .ok_or_else(|| format!("Cannot parse root note '{}'", root))?;

    let notes: Vec<String> = intervals
        .iter()
        .map(|&i| NOTE_NAMES[(root_idx + i as usize) % 12].to_string())
        .collect();

    let mut out = format!("{} {} chord\n", root_norm, quality);
    out += &format!("Notes: {}\n", notes.join(" – "));
    out += &format!(
        "Intervals from root: {}\n",
        intervals
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    out += "\nNote details:\n";
    for (note, &interval) in notes.iter().zip(intervals.iter()) {
        out += &format!("  {} — {}\n", note, INTERVAL_NAMES[interval as usize % 13]);
    }
    Ok(out)
}

fn action_scale(args: &Value) -> Result<String, String> {
    let root = args
        .get("root")
        .and_then(|v| v.as_str())
        .ok_or("'root' note is required for scale action")?;
    let quality = args
        .get("quality")
        .and_then(|v| v.as_str())
        .unwrap_or("major");

    let intervals = scale_intervals(quality).ok_or_else(|| {
        format!(
            "Unknown scale '{}'. Try: major, minor, harmonic minor, melodic minor, pentatonic, minor pentatonic, blues, dorian, phrygian, lydian, mixolydian, locrian, whole tone, diminished, chromatic",
            quality
        )
    })?;

    let root_norm = normalize_note_name(root);
    let root_idx = NOTE_NAMES
        .iter()
        .position(|&n| n == root_norm)
        .ok_or_else(|| format!("Cannot parse root note '{}'", root))?;

    let notes: Vec<String> = intervals
        .iter()
        .map(|&i| NOTE_NAMES[(root_idx + i as usize) % 12].to_string())
        .collect();

    let mut out = format!("{} {} scale\n", root_norm, quality);
    out += &format!("Notes ({} total): {}\n", notes.len(), notes.join(" – "));
    out += &format!(
        "Intervals: {}\n",
        intervals
            .iter()
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    out += "\nDegrees:\n";
    let degree_names = [
        "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII",
    ];
    for (i, (note, &interval)) in notes.iter().zip(intervals.iter()).enumerate() {
        let degree = if i < degree_names.len() {
            degree_names[i]
        } else {
            "?"
        };
        out += &format!("  {} — {} (semitone {})\n", degree, note, interval);
    }
    Ok(out)
}

fn action_interval(args: &Value) -> Result<String, String> {
    let note1 = args
        .get("note")
        .and_then(|v| v.as_str())
        .ok_or("'note' is required")?;
    let note2 = args
        .get("note2")
        .and_then(|v| v.as_str())
        .ok_or("'note2' is required")?;

    let (s1, o1) = parse_note(note1).ok_or_else(|| format!("Cannot parse '{}'", note1))?;
    let (s2, o2) = parse_note(note2).ok_or_else(|| format!("Cannot parse '{}'", note2))?;
    let midi1 = note_to_midi(s1, o1);
    let midi2 = note_to_midi(s2, o2);
    let diff = (midi2 - midi1).unsigned_abs();
    let direction = if midi2 >= midi1 {
        "ascending"
    } else {
        "descending"
    };
    let name = semitones_to_interval(diff);

    let mut out = format!(
        "{} → {}: {} ({})\n",
        midi_to_note(midi1),
        midi_to_note(midi2),
        name,
        direction
    );
    out += &format!("Semitone distance: {}\n", diff);
    out += &format!(
        "Frequency ratio:   {:.4} : 1\n",
        midi_to_freq(midi2) / midi_to_freq(midi1)
    );
    Ok(out)
}

fn action_midi(args: &Value) -> Result<String, String> {
    if let Some(midi_num) = args.get("midi").and_then(|v| v.as_i64()) {
        let midi = midi_num.clamp(0, 127) as i32;
        let name = midi_to_note(midi);
        let freq = midi_to_freq(midi);
        return Ok(format!("MIDI {}: {} = {:.3} Hz\n", midi, name, freq));
    }

    let note_str = args
        .get("note")
        .and_then(|v| v.as_str())
        .ok_or("'note' or 'midi' number is required")?;
    let (semitone, octave) =
        parse_note(note_str).ok_or_else(|| format!("Cannot parse note '{}'", note_str))?;
    let midi = note_to_midi(semitone, octave);
    let freq = midi_to_freq(midi);

    Ok(format!(
        "{}: MIDI {} = {:.3} Hz\n",
        midi_to_note(midi),
        midi,
        freq
    ))
}

fn action_tempo(args: &Value) -> Result<String, String> {
    if let Some(bpm) = args.get("bpm").and_then(|v| v.as_f64()) {
        let duration = args.get("duration").and_then(|v| v.as_str()).unwrap_or("");

        let beat_ms = 60_000.0 / bpm;
        let mut out = format!("BPM: {}\nQuarter note: {:.1} ms\n", bpm, beat_ms);

        if !duration.is_empty() {
            match note_duration_ms(bpm, duration) {
                Some(ms) => {
                    out += &format!("{}: {:.1} ms ({:.3} sec)\n", duration, ms, ms / 1000.0);
                }
                None => {
                    out += &format!(
                        "Unknown duration '{}'. Use: whole, half, quarter, eighth, sixteenth\n",
                        duration
                    );
                }
            }
        } else {
            out += "\nNote durations at this tempo:\n";
            let durations = [
                ("Whole", 4.0f64),
                ("Half", 2.0),
                ("Quarter", 1.0),
                ("Eighth", 0.5),
                ("Sixteenth", 0.25),
                ("Dotted half", 3.0),
                ("Dotted quarter", 1.5),
                ("Dotted eighth", 0.75),
                ("Triplet quarter", 2.0 / 3.0),
                ("Triplet eighth", 1.0 / 3.0),
            ];
            for (name, beats) in &durations {
                let ms = beat_ms * beats;
                out += &format!("  {:<18} {:.1} ms\n", name, ms);
            }
        }
        return Ok(out);
    }

    // Reverse: given ms, what BPM?
    Err(
        "Provide 'bpm' to compute note durations. Example: {bpm: 120, duration: 'quarter'}"
            .to_string(),
    )
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("note");

    match action {
        "note" => action_note(args),
        "chord" => action_chord(args),
        "scale" => action_scale(args),
        "interval" => action_interval(args),
        "midi" => action_midi(args),
        "tempo" => action_tempo(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: note, chord, scale, interval, midi, tempo",
            action
        )),
    }
}
