use anyhow::Result;
use ignore::WalkBuilder;
use petgraph::graph::DiGraph;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use tree_sitter::{Language, Parser, Query, QueryCursor};

// ── Tag types ─────────────────────────────────────────────────────────────────

struct Tag {
    rel_path: String,
}

// ── Tree-sitter query factories ───────────────────────────────────────────────

fn get_rust_def_query() -> Result<(Language, Query)> {
    let language = tree_sitter_rust::LANGUAGE.into();
    let query_src = r#"
        (function_item name: (identifier) @name)
        (struct_item name: (type_identifier) @name)
        (impl_item type: (type_identifier) @name)
        (trait_item name: (type_identifier) @name)
        (enum_item name: (type_identifier) @name)
    "#;
    let query = Query::new(&language, query_src)?;
    Ok((language, query))
}

fn get_rust_ref_query() -> Result<(Language, Query)> {
    let language = tree_sitter_rust::LANGUAGE.into();
    let query_src = r#"
        (identifier) @ref
        (type_identifier) @ref
        (field_identifier) @ref
    "#;
    let query = Query::new(&language, query_src)?;
    Ok((language, query))
}

fn get_python_def_query() -> Result<(Language, Query)> {
    let language = tree_sitter_python::LANGUAGE.into();
    let query_src = r#"
        (class_definition name: (identifier) @name)
        (function_definition name: (identifier) @name)
    "#;
    let query = Query::new(&language, query_src)?;
    Ok((language, query))
}

fn get_python_ref_query() -> Result<(Language, Query)> {
    let language = tree_sitter_python::LANGUAGE.into();
    let query_src = "(identifier) @ref";
    let query = Query::new(&language, query_src)?;
    Ok((language, query))
}

fn get_ts_def_query() -> Result<(Language, Query)> {
    let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
    let query_src = r#"
        (interface_declaration name: (type_identifier) @name)
        (class_declaration name: (type_identifier) @name)
        (function_declaration name: (identifier) @name)
    "#;
    let query = Query::new(&language, query_src)?;
    Ok((language, query))
}

fn get_ts_ref_query() -> Result<(Language, Query)> {
    let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
    let query_src = r#"
        (identifier) @ref
        (type_identifier) @ref
    "#;
    let query = Query::new(&language, query_src)?;
    Ok((language, query))
}

fn get_js_def_query() -> Result<(Language, Query)> {
    let language = tree_sitter_javascript::LANGUAGE.into();
    let query_src = r#"
        (class_declaration name: (identifier) @name)
        (function_declaration name: (identifier) @name)
    "#;
    let query = Query::new(&language, query_src)?;
    Ok((language, query))
}

fn get_js_ref_query() -> Result<(Language, Query)> {
    let language = tree_sitter_javascript::LANGUAGE.into();
    let query_src = "(identifier) @ref";
    let query = Query::new(&language, query_src)?;
    Ok((language, query))
}

// ── RepoMapGenerator ──────────────────────────────────────────────────────────

pub struct RepoMapGenerator {
    root: std::path::PathBuf,
    hot_files: Vec<String>,
    max_symbols: usize,
}

impl RepoMapGenerator {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            hot_files: Vec::new(),
            max_symbols: 1500,
        }
    }

    /// Bias PageRank toward files the user is actively editing.
    pub fn with_hot_files(mut self, paths: &[String]) -> Self {
        self.hot_files = paths.to_vec();
        self
    }

    pub fn generate(&self) -> Result<String> {
        // ── Pass 1: Collect defs + refs from every source file ────────────
        let mut all_tags: Vec<Tag> = Vec::new();
        // Map: symbol_name → set of files that define it
        let mut defines: HashMap<String, HashSet<String>> = HashMap::new();
        // Map: symbol_name → list of files that reference it
        let mut references: HashMap<String, Vec<String>> = HashMap::new();
        // Map: (file, symbol_name) → list of definition tag names for display
        let mut definitions_display: HashMap<String, Vec<String>> = HashMap::new();

        let walker = WalkBuilder::new(&self.root)
            .hidden(true)
            .ignore(true)
            .git_ignore(true)
            .add_custom_ignore_filename(".hematiteignore")
            .filter_entry(|entry| {
                if let Some(name) = entry.file_name().to_str() {
                    if name == ".git"
                        || name == "target"
                        || name == "node_modules"
                        || name.ends_with(".min.js")
                    {
                        return false;
                    }
                }
                true
            })
            .build();

        let rust_def = get_rust_def_query().ok();
        let rust_ref = get_rust_ref_query().ok();
        let python_def = get_python_def_query().ok();
        let python_ref = get_python_ref_query().ok();
        let ts_def = get_ts_def_query().ok();
        let ts_ref = get_ts_ref_query().ok();
        let js_def = get_js_def_query().ok();
        let js_ref = get_js_ref_query().ok();

        for result in walker {
            let entry = match result {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let (def_bundle, ref_bundle) = match ext {
                "rs" => (rust_def.as_ref(), rust_ref.as_ref()),
                "py" => (python_def.as_ref(), python_ref.as_ref()),
                "ts" | "tsx" => (ts_def.as_ref(), ts_ref.as_ref()),
                "js" | "jsx" => (js_def.as_ref(), js_ref.as_ref()),
                _ => continue,
            };

            let Ok(source_code) = fs::read_to_string(path) else {
                continue;
            };

            let rel_path = path
                .strip_prefix(&self.root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            // Extract definitions
            if let Some((lang, query)) = def_bundle {
                let mut parser = Parser::new();
                if parser.set_language(lang).is_ok() {
                    if let Some(tree) = parser.parse(&source_code, None) {
                        let mut cursor = QueryCursor::new();
                        let matches =
                            cursor.matches(query, tree.root_node(), source_code.as_bytes());
                        for m in matches {
                            for capture in m.captures {
                                if let Ok(text) = capture.node.utf8_text(source_code.as_bytes()) {
                                    let name = text.to_string();
                                    all_tags.push(Tag {
                                        rel_path: rel_path.clone(),
                                    });
                                    defines
                                        .entry(name.clone())
                                        .or_default()
                                        .insert(rel_path.clone());
                                    definitions_display
                                        .entry(rel_path.clone())
                                        .or_default()
                                        .push(name);
                                }
                            }
                        }
                    }
                }
            }

            // Extract references
            if let Some((lang, query)) = ref_bundle {
                let mut parser = Parser::new();
                if parser.set_language(lang).is_ok() {
                    if let Some(tree) = parser.parse(&source_code, None) {
                        let mut cursor = QueryCursor::new();
                        let matches =
                            cursor.matches(query, tree.root_node(), source_code.as_bytes());
                        let mut seen_refs: HashSet<String> = HashSet::new();
                        for m in matches {
                            for capture in m.captures {
                                if let Ok(text) = capture.node.utf8_text(source_code.as_bytes()) {
                                    let name = text.to_string();
                                    // Only count each unique identifier once per file
                                    if seen_refs.insert(name.clone()) {
                                        all_tags.push(Tag {
                                            rel_path: rel_path.clone(),
                                        });
                                        references
                                            .entry(name)
                                            .or_default()
                                            .push(rel_path.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Deduplicate definition display lists
        for defs in definitions_display.values_mut() {
            defs.sort();
            defs.dedup();
        }

        // If there are no references at all (e.g. tiny repo), treat defs as refs
        if references.is_empty() {
            for (name, files) in &defines {
                references.insert(name.clone(), files.iter().cloned().collect());
            }
        }

        // ── Pass 2: Build the PageRank graph ──────────────────────────────
        let defined_names: HashSet<&String> = defines.keys().collect();
        let referenced_names: HashSet<&String> = references.keys().collect();
        let shared_idents: HashSet<&&String> = defined_names.intersection(&referenced_names).collect();

        // Collect all file paths that appear as nodes
        let mut all_files: HashSet<String> = HashSet::new();
        for tag in &all_tags {
            all_files.insert(tag.rel_path.clone());
        }

        // Node index map
        let mut graph = DiGraph::<String, f64>::new();
        let mut node_map: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();
        for file in &all_files {
            let idx = graph.add_node(file.clone());
            node_map.insert(file.clone(), idx);
        }

        // Build edges: referencer → definer
        for ident in &shared_idents {
            let ident: &String = ident;
            let definers = match defines.get(ident) {
                Some(d) => d,
                None => continue,
            };
            let referencers = match references.get(ident) {
                Some(r) => r,
                None => continue,
            };

            // Weight multiplier based on identifier quality
            let mut mul: f64 = 1.0;
            let is_snake = ident.contains('_') && ident.chars().any(|c| c.is_alphabetic());
            let is_camel = ident.chars().any(|c| c.is_uppercase())
                && ident.chars().any(|c| c.is_lowercase());
            if (is_snake || is_camel) && ident.len() >= 8 {
                mul *= 10.0;
            }
            if ident.starts_with('_') {
                mul *= 0.1;
            }
            // Overly generic names defined in 5+ files get downweighted
            if definers.len() > 5 {
                mul *= 0.1;
            }

            for referencer in referencers {
                let Some(&src) = node_map.get(referencer) else {
                    continue;
                };
                for definer in definers {
                    let Some(&dst) = node_map.get(definer) else {
                        continue;
                    };
                    // Accumulate weight on the edge
                    graph.add_edge(src, dst, mul);
                }
            }
        }

        // ── Pass 3: PageRank ──────────────────────────────────────────────
        let node_count = graph.node_count();
        if node_count == 0 {
            return Ok("=== Repository Map (Structural Overview) ===\n(no parseable source files found)\n".to_string());
        }

        let damping = 0.85;
        let iterations = 30;
        let base_score = 1.0 / node_count as f64;

        // Personalization: boost hot files
        let mut personalization: HashMap<petgraph::graph::NodeIndex, f64> = HashMap::new();
        let hot_set: HashSet<&str> = self.hot_files.iter().map(|s| s.as_str()).collect();
        let boost = 100.0 / node_count.max(1) as f64;
        for (file, &idx) in &node_map {
            if hot_set.contains(file.as_str()) {
                personalization.insert(idx, boost);
            }
        }

        // Initialize scores
        let mut scores: HashMap<petgraph::graph::NodeIndex, f64> = HashMap::new();
        for idx in graph.node_indices() {
            scores.insert(idx, base_score);
        }

        // Iterate PageRank
        for _ in 0..iterations {
            let mut new_scores: HashMap<petgraph::graph::NodeIndex, f64> = HashMap::new();
            for idx in graph.node_indices() {
                new_scores.insert(idx, (1.0 - damping) * base_score);
            }

            for edge in graph.edge_indices() {
                let (src, dst) = graph.edge_endpoints(edge).unwrap();
                let weight = graph[edge];
                // Total outgoing weight from src
                let out_weight: f64 = graph
                    .edges(src)
                    .map(|e| *e.weight())
                    .sum::<f64>()
                    .max(1.0);
                let contrib = damping * scores[&src] * (weight / out_weight);
                *new_scores.entry(dst).or_default() += contrib;
            }

            // Apply personalization
            for (&idx, &pers) in &personalization {
                *new_scores.entry(idx).or_default() += pers * base_score;
            }

            scores = new_scores;
        }

        // ── Pass 4: Render ranked output ──────────────────────────────────
        let mut ranked_files: Vec<(String, f64)> = scores
            .iter()
            .map(|(&idx, &score)| (graph[idx].clone(), score))
            .collect();
        ranked_files.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut output = String::new();
        output.push_str("=== Repository Map (Structural Overview) ===\n");
        let mut total_symbols = 0;

        for (rel_path, _score) in &ranked_files {
            if total_symbols >= self.max_symbols {
                output.push_str("... (Repository Map Truncated — showing most important files)\n");
                break;
            }

            if let Some(defs) = definitions_display.get(rel_path) {
                output.push_str(&format!("{}:\n", rel_path));
                for def in defs {
                    output.push_str(&format!("  - {}\n", def));
                    total_symbols += 1;
                    if total_symbols >= self.max_symbols {
                        break;
                    }
                }
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_repo_map_generation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("main.rs");

        let mock_code = r#"
        struct MyDatabase {
            id: String,
        }

        impl MyDatabase {
            fn save(&self) {}
        }

        fn launch_system() {}
        "#;

        fs::write(&file_path, mock_code).unwrap();

        let gen = RepoMapGenerator::new(dir.path());
        let map = gen.generate().unwrap();

        assert!(map.contains("main.rs:"));
        assert!(map.contains("MyDatabase"));
        assert!(map.contains("launch_system"));
    }

    #[test]
    fn test_pagerank_orders_central_files_first() {
        let dir = tempdir().unwrap();

        // "core.rs" defines a struct used everywhere
        fs::write(
            dir.path().join("core.rs"),
            "pub struct Engine {\n    pub id: u32,\n}\n\npub fn init_engine() -> Engine { Engine { id: 0 } }\n",
        )
        .unwrap();

        // "user.rs" references Engine
        fs::write(
            dir.path().join("user.rs"),
            "use crate::core::Engine;\n\nfn use_engine(e: Engine) {\n    let _ = e;\n}\n",
        )
        .unwrap();

        // "admin.rs" also references Engine
        fs::write(
            dir.path().join("admin.rs"),
            "use crate::core::Engine;\n\nfn admin_engine(e: Engine) {\n    let _ = e;\n}\n",
        )
        .unwrap();

        // "leaf.rs" defines something nobody uses
        fs::write(
            dir.path().join("leaf.rs"),
            "fn unused_leaf_function() {}\n\nstruct OrphanStruct {}\n",
        )
        .unwrap();

        let gen = RepoMapGenerator::new(dir.path());
        let map = gen.generate().unwrap();

        // core.rs should appear before leaf.rs because it's referenced by 2 files
        let core_pos = map.find("core.rs:").unwrap_or(usize::MAX);
        let leaf_pos = map.find("leaf.rs:").unwrap_or(usize::MAX);
        assert!(
            core_pos < leaf_pos,
            "core.rs (referenced by 2 files) should rank before leaf.rs (referenced by 0). Map:\n{}",
            map
        );
    }
}
