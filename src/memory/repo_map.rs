use anyhow::Result;
use ignore::WalkBuilder;
use petgraph::graph::DiGraph;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::{Language, Parser, Query, QueryCursor};

// ── Cached query bundles ──────────────────────────────────────────────────────
// Query::new compiles tree-sitter patterns against the grammar — non-trivial.
// Cache once per process; reuse across every generate() call.

fn rust_def_bundle() -> Option<&'static (Language, Query)> {
    static Q: OnceLock<Option<(Language, Query)>> = OnceLock::new();
    Q.get_or_init(|| {
        let lang: Language = tree_sitter_rust::LANGUAGE.into();
        let src = r#"
            (function_item name: (identifier) @name)
            (struct_item name: (type_identifier) @name)
            (impl_item type: (type_identifier) @name)
            (trait_item name: (type_identifier) @name)
            (enum_item name: (type_identifier) @name)
        "#;
        Query::new(&lang, src).ok().map(|q| (lang, q))
    })
    .as_ref()
}

fn rust_ref_bundle() -> Option<&'static (Language, Query)> {
    static Q: OnceLock<Option<(Language, Query)>> = OnceLock::new();
    Q.get_or_init(|| {
        let lang: Language = tree_sitter_rust::LANGUAGE.into();
        let src = r#"(identifier) @ref (type_identifier) @ref (field_identifier) @ref"#;
        Query::new(&lang, src).ok().map(|q| (lang, q))
    })
    .as_ref()
}

fn python_def_bundle() -> Option<&'static (Language, Query)> {
    static Q: OnceLock<Option<(Language, Query)>> = OnceLock::new();
    Q.get_or_init(|| {
        let lang: Language = tree_sitter_python::LANGUAGE.into();
        let src = r#"
            (class_definition name: (identifier) @name)
            (function_definition name: (identifier) @name)
        "#;
        Query::new(&lang, src).ok().map(|q| (lang, q))
    })
    .as_ref()
}

fn python_ref_bundle() -> Option<&'static (Language, Query)> {
    static Q: OnceLock<Option<(Language, Query)>> = OnceLock::new();
    Q.get_or_init(|| {
        let lang: Language = tree_sitter_python::LANGUAGE.into();
        Query::new(&lang, "(identifier) @ref")
            .ok()
            .map(|q| (lang, q))
    })
    .as_ref()
}

fn ts_def_bundle() -> Option<&'static (Language, Query)> {
    static Q: OnceLock<Option<(Language, Query)>> = OnceLock::new();
    Q.get_or_init(|| {
        let lang: Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        let src = r#"
            (interface_declaration name: (type_identifier) @name)
            (class_declaration name: (type_identifier) @name)
            (function_declaration name: (identifier) @name)
        "#;
        Query::new(&lang, src).ok().map(|q| (lang, q))
    })
    .as_ref()
}

fn ts_ref_bundle() -> Option<&'static (Language, Query)> {
    static Q: OnceLock<Option<(Language, Query)>> = OnceLock::new();
    Q.get_or_init(|| {
        let lang: Language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        let src = r#"(identifier) @ref (type_identifier) @ref"#;
        Query::new(&lang, src).ok().map(|q| (lang, q))
    })
    .as_ref()
}

fn js_def_bundle() -> Option<&'static (Language, Query)> {
    static Q: OnceLock<Option<(Language, Query)>> = OnceLock::new();
    Q.get_or_init(|| {
        let lang: Language = tree_sitter_javascript::LANGUAGE.into();
        let src = r#"
            (class_declaration name: (identifier) @name)
            (function_declaration name: (identifier) @name)
        "#;
        Query::new(&lang, src).ok().map(|q| (lang, q))
    })
    .as_ref()
}

fn js_ref_bundle() -> Option<&'static (Language, Query)> {
    static Q: OnceLock<Option<(Language, Query)>> = OnceLock::new();
    Q.get_or_init(|| {
        let lang: Language = tree_sitter_javascript::LANGUAGE.into();
        Query::new(&lang, "(identifier) @ref")
            .ok()
            .map(|q| (lang, q))
    })
    .as_ref()
}

// ── RepoMapGenerator ──────────────────────────────────────────────────────────

pub struct RepoMapGenerator {
    root: std::path::PathBuf,
    /// Hot files with normalized heat weights in [0.0, 1.0].
    hot_files: Vec<(String, f64)>,
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
    /// Accepts paths with normalized heat weights [0.0, 1.0].
    /// Hottest file gets 100x boost; others scale proportionally.
    pub fn with_hot_files(mut self, files: &[(String, f64)]) -> Self {
        self.hot_files = files.to_vec();
        self
    }

    pub fn generate(&self) -> Result<String> {
        // Map: symbol_name → set of files that define it
        let mut defines: HashMap<String, HashSet<String>> = HashMap::new();
        // Map: symbol_name → list of files that reference it
        let mut references: HashMap<String, Vec<String>> = HashMap::new();
        // Map: file → list of definition names for display
        let mut definitions_display: HashMap<String, Vec<String>> = HashMap::new();
        // All file paths that produced at least one def or ref tag.
        let mut all_files: HashSet<String> = HashSet::new();

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

        // One parser reused across all files; set_language() switches between grammars.
        let mut parser = Parser::new();

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
                "rs" => (rust_def_bundle(), rust_ref_bundle()),
                "py" => (python_def_bundle(), python_ref_bundle()),
                "ts" | "tsx" => (ts_def_bundle(), ts_ref_bundle()),
                "js" | "jsx" => (js_def_bundle(), js_ref_bundle()),
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

            // Parse each file once and run both def and ref queries on the same tree.
            match (def_bundle, ref_bundle) {
                (Some((lang, def_q)), Some((_, ref_q))) => {
                    if parser.set_language(lang).is_ok() {
                        if let Some(tree) = parser.parse(&source_code, None) {
                            // Def pass
                            let mut cursor = QueryCursor::new();
                            for m in cursor.matches(def_q, tree.root_node(), source_code.as_bytes())
                            {
                                for capture in m.captures {
                                    if let Ok(text) = capture.node.utf8_text(source_code.as_bytes())
                                    {
                                        let name = text.to_string();
                                        all_files.insert(rel_path.clone());
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

                            // Ref pass — seen_refs borrows source_code bytes to avoid String clones
                            let mut cursor = QueryCursor::new();
                            let mut seen_refs: HashSet<&str> = HashSet::new();
                            for m in cursor.matches(ref_q, tree.root_node(), source_code.as_bytes())
                            {
                                for capture in m.captures {
                                    if let Ok(text) = capture.node.utf8_text(source_code.as_bytes())
                                    {
                                        if seen_refs.insert(text) {
                                            all_files.insert(rel_path.clone());
                                            references
                                                .entry(text.to_string())
                                                .or_default()
                                                .push(rel_path.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                (Some((lang, def_q)), None) => {
                    if parser.set_language(lang).is_ok() {
                        if let Some(tree) = parser.parse(&source_code, None) {
                            let mut cursor = QueryCursor::new();
                            for m in cursor.matches(def_q, tree.root_node(), source_code.as_bytes())
                            {
                                for capture in m.captures {
                                    if let Ok(text) = capture.node.utf8_text(source_code.as_bytes())
                                    {
                                        let name = text.to_string();
                                        all_files.insert(rel_path.clone());
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
                (None, Some((lang, ref_q))) => {
                    if parser.set_language(lang).is_ok() {
                        if let Some(tree) = parser.parse(&source_code, None) {
                            let mut cursor = QueryCursor::new();
                            let mut seen_refs: HashSet<&str> = HashSet::new();
                            for m in cursor.matches(ref_q, tree.root_node(), source_code.as_bytes())
                            {
                                for capture in m.captures {
                                    if let Ok(text) = capture.node.utf8_text(source_code.as_bytes())
                                    {
                                        if seen_refs.insert(text) {
                                            all_files.insert(rel_path.clone());
                                            references
                                                .entry(text.to_string())
                                                .or_default()
                                                .push(rel_path.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                (None, None) => {}
            }
        }

        // Deduplicate definition display lists
        for defs in definitions_display.values_mut() {
            defs.sort_unstable();
            defs.dedup();
        }

        // If there are no references at all (e.g. tiny repo), treat defs as refs
        if references.is_empty() {
            for (name, files) in &defines {
                references.insert(name.clone(), files.iter().cloned().collect());
            }
        }

        // ── Build PageRank graph ──────────────────────────────────────────────
        let defined_names: HashSet<&String> = defines.keys().collect();
        let referenced_names: HashSet<&String> = references.keys().collect();
        let shared_idents: HashSet<&&String> =
            defined_names.intersection(&referenced_names).collect();

        let mut graph = DiGraph::<String, f64>::new();
        let mut node_map: HashMap<String, petgraph::graph::NodeIndex> =
            HashMap::with_capacity(all_files.len());
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

            let mut mul: f64 = 1.0;
            let is_snake = ident.contains('_') && ident.chars().any(|c| c.is_alphabetic());
            let is_camel =
                ident.chars().any(|c| c.is_uppercase()) && ident.chars().any(|c| c.is_lowercase());
            if (is_snake || is_camel) && ident.len() >= 8 {
                mul *= 10.0;
            }
            if ident.starts_with('_') {
                mul *= 0.1;
            }
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
                    graph.add_edge(src, dst, mul);
                }
            }
        }

        // ── PageRank ──────────────────────────────────────────────────────────
        let node_count = graph.node_count();
        if node_count == 0 {
            return Ok(
                "=== Repository Map (Structural Overview) ===\n(no parseable source files found)\n"
                    .to_string(),
            );
        }

        let damping = 0.85_f64;
        let iterations = 30_usize;
        let base_score = 1.0 / node_count as f64;
        let base_decay = (1.0 - damping) * base_score;

        // Precompute total outgoing edge-weight per node once — constant across iterations.
        // Avoids O(E × avg_degree) recomputation inside the 30-iteration loop.
        let out_weights: Vec<f64> = graph
            .node_indices()
            .map(|idx| graph.edges(idx).map(|e| *e.weight()).sum::<f64>().max(1.0))
            .collect();

        // Personalization boosts for hot files, stored as a dense Vec.
        let base_boost = 100.0 / node_count.max(1) as f64;
        let mut pers_boosts: Vec<f64> = vec![0.0; node_count];
        for (file, weight) in &self.hot_files {
            if let Some(&idx) = node_map.get(file.as_str()) {
                pers_boosts[idx.index()] = base_boost * weight;
            }
        }

        // Vec-based PageRank — replaces 30 HashMap allocations with 2 pre-allocated Vecs.
        let mut scores: Vec<f64> = vec![base_score; node_count];
        let mut new_scores: Vec<f64> = vec![0.0; node_count];

        for _ in 0..iterations {
            new_scores.iter_mut().for_each(|s| *s = base_decay);

            for edge in graph.edge_indices() {
                let (src, dst) = graph.edge_endpoints(edge).unwrap();
                let weight = graph[edge];
                let contrib = damping * scores[src.index()] * (weight / out_weights[src.index()]);
                new_scores[dst.index()] += contrib;
            }

            for (i, &pers) in pers_boosts.iter().enumerate() {
                if pers > 0.0 {
                    new_scores[i] += pers * base_score;
                }
            }

            std::mem::swap(&mut scores, &mut new_scores);
        }

        // ── Render ranked output ──────────────────────────────────────────────
        let mut ranked_files: Vec<(String, f64)> = graph
            .node_indices()
            .map(|idx| (graph[idx].clone(), scores[idx.index()]))
            .collect();
        ranked_files
            .sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut output = String::with_capacity(self.max_symbols * 40 + 64);
        output.push_str("=== Repository Map (Structural Overview) ===\n");
        let mut total_symbols = 0;

        for (rel_path, _score) in &ranked_files {
            if total_symbols >= self.max_symbols {
                output.push_str("... (Repository Map Truncated — showing most important files)\n");
                break;
            }

            if let Some(defs) = definitions_display.get(rel_path) {
                let _ = writeln!(output, "{}:", rel_path);
                for def in defs {
                    let _ = writeln!(output, "  - {}", def);
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
