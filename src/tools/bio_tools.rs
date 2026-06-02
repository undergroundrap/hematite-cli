use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;

pub fn bio_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["info", "complement", "transcribe", "translate", "gc", "orfs", "codons", "parse_fasta"],
                "description": "info: sequence overview | complement: DNA reverse complement | transcribe: DNA to mRNA | translate: mRNA/DNA to protein | gc: GC content analysis | orfs: find open reading frames | codons: codon usage table | parse_fasta: parse FASTA format"
            },
            "sequence": {"type": "string", "description": "Nucleotide or protein sequence (raw or with whitespace/numbers)"},
            "file": {"type": "string", "description": "Path to a FASTA file"},
            "text": {"type": "string", "description": "Raw FASTA text"},
            "frame": {"type": "integer", "description": "Reading frame for translation (1, 2, 3, -1, -2, -3); default 1"},
            "min_length": {"type": "integer", "description": "Minimum ORF length in amino acids for orfs action (default 10)"},
            "all_frames": {"type": "boolean", "description": "Translate all 6 reading frames"},
            "type": {"type": "string", "description": "Sequence type override: dna, rna, protein"}
        },
        "required": []
    })
}

static CODON_TABLE: &[(&str, &str)] = &[
    ("UUU", "Phe"),
    ("UUC", "Phe"),
    ("UUA", "Leu"),
    ("UUG", "Leu"),
    ("CUU", "Leu"),
    ("CUC", "Leu"),
    ("CUA", "Leu"),
    ("CUG", "Leu"),
    ("AUU", "Ile"),
    ("AUC", "Ile"),
    ("AUA", "Ile"),
    ("AUG", "Met"),
    ("GUU", "Val"),
    ("GUC", "Val"),
    ("GUA", "Val"),
    ("GUG", "Val"),
    ("UCU", "Ser"),
    ("UCC", "Ser"),
    ("UCA", "Ser"),
    ("UCG", "Ser"),
    ("CCU", "Pro"),
    ("CCC", "Pro"),
    ("CCA", "Pro"),
    ("CCG", "Pro"),
    ("ACU", "Thr"),
    ("ACC", "Thr"),
    ("ACA", "Thr"),
    ("ACG", "Thr"),
    ("GCU", "Ala"),
    ("GCC", "Ala"),
    ("GCA", "Ala"),
    ("GCG", "Ala"),
    ("UAU", "Tyr"),
    ("UAC", "Tyr"),
    ("UAA", "*"),
    ("UAG", "*"),
    ("CAU", "His"),
    ("CAC", "His"),
    ("CAA", "Gln"),
    ("CAG", "Gln"),
    ("AAU", "Asn"),
    ("AAC", "Asn"),
    ("AAA", "Lys"),
    ("AAG", "Lys"),
    ("GAU", "Asp"),
    ("GAC", "Asp"),
    ("GAA", "Glu"),
    ("GAG", "Glu"),
    ("UGU", "Cys"),
    ("UGC", "Cys"),
    ("UGA", "*"),
    ("UGG", "Trp"),
    ("CGU", "Arg"),
    ("CGC", "Arg"),
    ("CGA", "Arg"),
    ("CGG", "Arg"),
    ("AGU", "Ser"),
    ("AGC", "Ser"),
    ("AGA", "Arg"),
    ("AGG", "Arg"),
    ("GGU", "Gly"),
    ("GGC", "Gly"),
    ("GGA", "Gly"),
    ("GGG", "Gly"),
];

fn codon_map() -> HashMap<&'static str, &'static str> {
    CODON_TABLE.iter().copied().collect()
}

fn clean_sequence(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

fn detect_type(seq: &str) -> &'static str {
    let upper = seq.to_ascii_uppercase();
    let has_u = upper.contains('U');
    let has_t = upper.contains('T');
    let non_nuc = upper.chars().any(|c| !"ACGTUN".contains(c));
    if non_nuc {
        "protein"
    } else if has_u && !has_t {
        "rna"
    } else {
        "dna"
    }
}

fn dna_complement(seq: &str) -> String {
    seq.chars()
        .rev()
        .map(|c| match c {
            'A' => 'T',
            'T' => 'A',
            'G' => 'C',
            'C' => 'G',
            'N' => 'N',
            other => other,
        })
        .collect()
}

fn dna_to_rna(seq: &str) -> String {
    seq.replace('T', "U")
}

fn rna_to_dna(seq: &str) -> String {
    seq.replace('U', "T")
}

fn translate_rna(rna: &str, codons: &HashMap<&str, &str>) -> String {
    let mut protein = String::new();
    let bytes = rna.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        let codon = &rna[i..i + 3];
        match codons.get(codon) {
            Some(&"*") => break,
            Some(&aa) => protein.push_str(aa),
            None => protein.push('?'),
        }
        i += 3;
    }
    protein
}

fn gc_content(seq: &str) -> f64 {
    let total = seq.len();
    if total == 0 {
        return 0.0;
    }
    let gc = seq.chars().filter(|&c| c == 'G' || c == 'C').count();
    (gc as f64 / total as f64) * 100.0
}

fn action_info(seq: &str, seq_type: &str) -> String {
    let len = seq.len();
    let mut out = format!(
        "Sequence Analysis\n{}\n\nType:   {}\nLength: {} {}\n",
        "=".repeat(40),
        seq_type.to_uppercase(),
        len,
        if seq_type == "protein" { "aa" } else { "bp" }
    );

    match seq_type {
        "dna" | "rna" => {
            let a = seq.chars().filter(|&c| c == 'A').count();
            let c = seq.chars().filter(|&c| c == 'C').count();
            let g = seq.chars().filter(|&c| c == 'G').count();
            let t_or_u = seq.chars().filter(|&c| c == 'T' || c == 'U').count();
            let n = seq.chars().filter(|&c| c == 'N').count();
            let base = if seq_type == "dna" { "T" } else { "U" };
            out += &format!(
                "A:      {} ({:.1}%)\nC:      {} ({:.1}%)\nG:      {} ({:.1}%)\n{}:      {} ({:.1}%)\n",
                a, (a as f64 / len as f64) * 100.0,
                c, (c as f64 / len as f64) * 100.0,
                g, (g as f64 / len as f64) * 100.0,
                base, t_or_u, (t_or_u as f64 / len as f64) * 100.0,
            );
            if n > 0 {
                out += &format!("N:      {} ({:.1}%)\n", n, (n as f64 / len as f64) * 100.0);
            }
            let gc = gc_content(seq);
            out += &format!("GC%:    {:.2}%\n", gc);
            let mw_approx = len as f64 * 330.0; // rough ssDNA
            out += &format!("MW est: {:.0} Da (ssDNA)\n", mw_approx);
            if seq_type == "dna" {
                out += &format!("Codons: {} (frame 1)\n", len / 3);
            }
        }
        "protein" => {
            let unique: std::collections::HashSet<char> = seq.chars().collect();
            out += &format!("Unique residues: {}\n", unique.len());
            // rough MW: average aa is ~110 Da
            let mw = len as f64 * 110.0;
            out += &format!("MW est: {:.0} Da (~{:.1} kDa)\n", mw, mw / 1000.0);
        }
        _ => {}
    }

    let preview_len = seq.len().min(60);
    out += &format!("\nPreview: {}…\n", &seq[..preview_len]);
    out
}

fn action_complement(seq: &str) -> String {
    let rc = dna_complement(seq);
    format!(
        "Original:   5'-{}-3'\nRevComp:    3'-{}-5'\nRevComp:    5'-{}-3'\n",
        seq,
        rc.chars().rev().collect::<String>(),
        rc
    )
}

fn action_transcribe(seq: &str, seq_type: &str) -> String {
    match seq_type {
        "dna" => {
            let mrna = dna_to_rna(seq);
            format!("DNA template: 5'-{}-3'\nmRNA (5'→3'): {}\n", seq, mrna)
        }
        "rna" => {
            let dna = rna_to_dna(seq);
            format!("mRNA (5'→3'): {}\nDNA template: {}\n", seq, dna)
        }
        _ => "Transcription requires a DNA or RNA sequence.".to_string(),
    }
}

fn action_translate(seq: &str, seq_type: &str, frame: i32, all_frames: bool) -> String {
    let codons = codon_map();
    let rna = if seq_type == "dna" {
        dna_to_rna(seq)
    } else if seq_type == "rna" {
        seq.to_string()
    } else {
        return "Translation requires a DNA or RNA sequence.".to_string();
    };

    if all_frames {
        let mut out = String::from("Translation — all 6 reading frames:\n");
        let rc_dna = dna_complement(seq);
        let rc_rna = dna_to_rna(&rc_dna);
        for (label, src) in &[
            ("Frame +1", rna.clone()),
            ("Frame +2", rna[1.min(rna.len())..].to_string()),
            ("Frame +3", rna[2.min(rna.len())..].to_string()),
            ("Frame -1", rc_rna.clone()),
            ("Frame -2", rc_rna[1.min(rc_rna.len())..].to_string()),
            ("Frame -3", rc_rna[2.min(rc_rna.len())..].to_string()),
        ] {
            let protein = translate_rna(src, &codons);
            out += &format!("{}: {} ({}aa)\n", label, protein, protein.len());
        }
        return out;
    }

    let (src, label) = if frame >= 1 {
        let offset = (frame - 1) as usize;
        (
            rna[offset.min(rna.len())..].to_string(),
            format!("Frame +{}", frame),
        )
    } else {
        let rc_dna = dna_complement(seq);
        let rc_rna = dna_to_rna(&rc_dna);
        let offset = (-frame - 1) as usize;
        (
            rc_rna[offset.min(rc_rna.len())..].to_string(),
            format!("Frame {}", frame),
        )
    };

    let protein = translate_rna(&src, &codons);
    format!("{}: {}\nLength: {} aa\n", label, protein, protein.len())
}

fn action_gc(seq: &str) -> String {
    let len = seq.len();
    if len == 0 {
        return "Empty sequence.\n".to_string();
    }
    let gc = gc_content(seq);
    let g = seq.chars().filter(|&c| c == 'G').count();
    let c = seq.chars().filter(|&c| c == 'C').count();
    let a = seq.chars().filter(|&c| c == 'A').count();
    let t_u = seq.chars().filter(|&c| c == 'T' || c == 'U').count();

    // sliding window GC (window=10 if seq < 100, else 50)
    let window = if len < 100 { 10 } else { 50 }.min(len);
    let mut min_gc = 100.0f64;
    let mut max_gc = 0.0f64;
    for i in 0..=(len.saturating_sub(window)) {
        let w = &seq[i..i + window];
        let wgc = gc_content(w);
        if wgc < min_gc {
            min_gc = wgc;
        }
        if wgc > max_gc {
            max_gc = wgc;
        }
    }

    format!(
        "GC Content Analysis\n{}\n\nOverall GC: {:.2}%\nG:          {} ({:.1}%)\nC:          {} ({:.1}%)\nA:          {} ({:.1}%)\nT/U:        {} ({:.1}%)\n\nWindow ({} bp):\n  Min GC: {:.2}%\n  Max GC: {:.2}%\n  Range:  {:.2}%\n",
        "=".repeat(40),
        gc,
        g, (g as f64 / len as f64) * 100.0,
        c, (c as f64 / len as f64) * 100.0,
        a, (a as f64 / len as f64) * 100.0,
        t_u, (t_u as f64 / len as f64) * 100.0,
        window, min_gc, max_gc, max_gc - min_gc,
    )
}

fn action_orfs(seq: &str, min_len: usize) -> String {
    let codons = codon_map();
    let rna = dna_to_rna(seq);
    let rc_dna = dna_complement(seq);
    let rc_rna = dna_to_rna(&rc_dna);

    struct Orf {
        frame: i32,
        start: usize,
        end: usize,
        protein: String,
    }

    let mut orfs: Vec<Orf> = Vec::new();

    for (frame_idx, (src, is_rc)) in [
        (rna.clone(), false),
        (rna[1.min(rna.len())..].to_string(), false),
        (rna[2.min(rna.len())..].to_string(), false),
        (rc_rna.clone(), true),
        (rc_rna[1.min(rc_rna.len())..].to_string(), true),
        (rc_rna[2.min(rc_rna.len())..].to_string(), true),
    ]
    .iter()
    .enumerate()
    {
        let frame_num = if !is_rc {
            (frame_idx as i32) + 1
        } else {
            -(((frame_idx as i32) - 3) + 1)
        };
        let bytes = src.as_bytes();
        let mut i = 0;
        while i + 2 < bytes.len() {
            let codon = &src[i..i + 3];
            if codon == "AUG" {
                // found start, extend
                let start_pos = i;
                let mut protein = String::new();
                let mut j = i;
                let mut found_stop = false;
                while j + 2 < bytes.len() {
                    let c = &src[j..j + 3];
                    match codons.get(c) {
                        Some(&"*") => {
                            found_stop = true;
                            j += 3;
                            break;
                        }
                        Some(&aa) => protein.push_str(aa),
                        None => {}
                    }
                    j += 3;
                }
                if protein.len() >= min_len && (found_stop || protein.len() >= min_len) {
                    let end_pos = j;
                    // Convert back to nucleotide positions
                    let nt_start = start_pos + (frame_idx % 3);
                    let nt_end = end_pos + (frame_idx % 3);
                    let _ = (nt_start, nt_end);
                    orfs.push(Orf {
                        frame: frame_num,
                        start: start_pos,
                        end: j,
                        protein,
                    });
                    i = j; // skip past found ORF
                    continue;
                }
            }
            i += 3;
        }
    }

    if orfs.is_empty() {
        return format!("No ORFs found with minimum length {} aa.\n", min_len);
    }

    let mut out = format!(
        "Open Reading Frames (min {} aa)\n{}\n\n",
        min_len,
        "=".repeat(40)
    );
    out += &format!(
        "{:<6} {:>8} {:>8} {:>6}  Sequence\n",
        "Frame", "Start", "End", "Length"
    );
    out += &format!("{}\n", "-".repeat(70));

    for orf in &orfs {
        let preview = if orf.protein.len() > 30 {
            format!("{}…", &orf.protein[..30])
        } else {
            orf.protein.clone()
        };
        out += &format!(
            "{:<6} {:>8} {:>8} {:>6}  {}\n",
            if orf.frame > 0 {
                format!("+{}", orf.frame)
            } else {
                orf.frame.to_string()
            },
            orf.start + 1,
            orf.end,
            orf.protein.len(),
            preview
        );
    }
    out += &format!("\nTotal: {} ORF(s) found\n", orfs.len());
    out
}

fn action_codons(seq: &str, seq_type: &str) -> String {
    let rna = match seq_type {
        "dna" => dna_to_rna(seq),
        "rna" => seq.to_string(),
        _ => return "Codon analysis requires a DNA or RNA sequence.".to_string(),
    };

    let codons_map = codon_map();
    let mut freq: HashMap<String, usize> = HashMap::new();
    let total_codons = rna.len() / 3;

    for i in (0..rna.len().saturating_sub(2)).step_by(3) {
        let codon = &rna[i..i + 3];
        *freq.entry(codon.to_string()).or_insert(0) += 1;
    }

    // group by amino acid
    let mut aa_codons: HashMap<String, Vec<(String, usize)>> = HashMap::new();
    for (codon, count) in &freq {
        let aa = codons_map
            .get(codon.as_str())
            .copied()
            .unwrap_or("?")
            .to_string();
        aa_codons
            .entry(aa)
            .or_default()
            .push((codon.clone(), *count));
    }

    let mut aas: Vec<String> = aa_codons.keys().cloned().collect();
    aas.sort();

    let mut out = format!(
        "Codon Usage Table (frame 1)\n{}\n\nTotal codons: {}\n\n",
        "=".repeat(40),
        total_codons
    );
    out += &format!(
        "{:<8} {:<6} {:>6}  {:>7}\n",
        "Codon", "AA", "Count", "Freq%"
    );
    out += &format!("{}\n", "-".repeat(35));

    for aa in &aas {
        let mut entries = aa_codons[aa].clone();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        for (codon, count) in entries {
            let pct = if total_codons > 0 {
                (count as f64 / total_codons as f64) * 100.0
            } else {
                0.0
            };
            out += &format!("{:<8} {:<6} {:>6}  {:>6.2}%\n", codon, aa, count, pct);
        }
    }
    out
}

struct FastaRecord {
    header: String,
    sequence: String,
}

fn parse_fasta_text(text: &str) -> Vec<FastaRecord> {
    let mut records = Vec::new();
    let mut current_header = String::new();
    let mut current_seq = String::new();

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('>') {
            if !current_header.is_empty() {
                records.push(FastaRecord {
                    header: current_header.clone(),
                    sequence: clean_sequence(&current_seq),
                });
                current_seq.clear();
            }
            current_header = line[1..].to_string();
        } else if !line.is_empty() && !line.starts_with(';') {
            current_seq.push_str(line);
        }
    }
    if !current_header.is_empty() {
        records.push(FastaRecord {
            header: current_header,
            sequence: clean_sequence(&current_seq),
        });
    }
    records
}

fn action_parse_fasta(text: &str) -> String {
    let records = parse_fasta_text(text);
    if records.is_empty() {
        return "No FASTA records found. Ensure sequences start with '>' headers.\n".to_string();
    }

    let mut out = format!(
        "FASTA Records\n{}\n\nTotal: {} record(s)\n\n",
        "=".repeat(40),
        records.len()
    );

    for (i, rec) in records.iter().enumerate() {
        let seq_type = detect_type(&rec.sequence);
        let gc = if seq_type != "protein" {
            format!("  GC: {:.1}%\n", gc_content(&rec.sequence))
        } else {
            String::new()
        };
        let unit = if seq_type == "protein" { "aa" } else { "bp" };
        out += &format!(
            "Record {}: {}\n  Type:   {}\n  Length: {} {}\n{}",
            i + 1,
            rec.header,
            seq_type.to_uppercase(),
            rec.sequence.len(),
            unit,
            gc
        );
        let preview_len = rec.sequence.len().min(50);
        out += &format!("  Seq:    {}…\n\n", &rec.sequence[..preview_len]);
    }
    out
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    // FASTA parse needs text, not sequence
    if action == "parse_fasta" {
        let text = if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
            fs::read_to_string(f).map_err(|e| format!("Cannot read '{}': {}", f, e))?
        } else if let Some(t) = args
            .get("text")
            .or_else(|| args.get("sequence"))
            .and_then(|v| v.as_str())
        {
            t.to_string()
        } else {
            return Err("Provide 'file' path or 'text'/'sequence' with FASTA content.".to_string());
        };
        return Ok(action_parse_fasta(&text));
    }

    // All other actions need a sequence
    let raw = args
        .get("sequence")
        .or_else(|| args.get("text"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Provide 'sequence' (or 'text') with the nucleotide or protein sequence, or use action='parse_fasta' with 'text'/'file'.".to_string())?;

    let seq = clean_sequence(raw);
    if seq.is_empty() {
        return Err("Sequence is empty after cleaning.".to_string());
    }

    let type_override = args.get("type").and_then(|v| v.as_str());
    let seq_type = type_override.unwrap_or_else(|| detect_type(&seq));

    let frame = args.get("frame").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
    let all_frames = args
        .get("all_frames")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let min_len = args
        .get("min_length")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    Ok(match action {
        "info" => action_info(&seq, seq_type),
        "complement" => {
            if seq_type == "protein" {
                "Reverse complement is only valid for DNA sequences.".to_string()
            } else {
                let dna = if seq_type == "rna" {
                    rna_to_dna(&seq)
                } else {
                    seq.clone()
                };
                action_complement(&dna)
            }
        }
        "transcribe" => action_transcribe(&seq, seq_type),
        "translate" => action_translate(&seq, seq_type, frame, all_frames),
        "gc" => {
            if seq_type == "protein" {
                "GC content is only valid for DNA/RNA sequences.".to_string()
            } else {
                action_gc(&seq)
            }
        }
        "orfs" => {
            if seq_type == "protein" {
                "ORF finding is only valid for DNA sequences.".to_string()
            } else {
                let dna = if seq_type == "rna" {
                    rna_to_dna(&seq)
                } else {
                    seq.clone()
                };
                action_orfs(&dna, min_len)
            }
        }
        "codons" => action_codons(&seq, seq_type),
        other => format!(
            "Unknown action '{}'. Use: info, complement, transcribe, translate, gc, orfs, codons, parse_fasta",
            other
        ),
    })
}
