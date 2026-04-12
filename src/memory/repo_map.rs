use anyhow::Result;
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tree_sitter::{Language, Parser, Query, QueryCursor};

fn get_rust_query() -> Result<(Language, Query)> {
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

fn get_python_query() -> Result<(Language, Query)> {
    let language = tree_sitter_python::LANGUAGE.into();
    let query_src = r#"
        (class_definition name: (identifier) @name)
        (function_definition name: (identifier) @name)
    "#;
    let query = Query::new(&language, query_src)?;
    Ok((language, query))
}

fn get_ts_query() -> Result<(Language, Query)> {
    let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
    let query_src = r#"
        (interface_declaration name: (type_identifier) @name)
        (class_declaration name: (type_identifier) @name)
        (function_declaration name: (identifier) @name)
    "#;
    let query = Query::new(&language, query_src)?;
    Ok((language, query))
}

fn get_js_query() -> Result<(Language, Query)> {
    let language = tree_sitter_javascript::LANGUAGE.into();
    let query_src = r#"
        (class_declaration name: (identifier) @name)
        (function_declaration name: (identifier) @name)
    "#;
    let query = Query::new(&language, query_src)?;
    Ok((language, query))
}

pub struct RepoMapGenerator {
    root: std::path::PathBuf,
}

impl RepoMapGenerator {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn generate(&self) -> Result<String> {
        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        
        // Use WalkBuilder for high-speed gitignore-aware traversal
        let walker = WalkBuilder::new(&self.root)
            .hidden(true)
            .ignore(true)
            .git_ignore(true)
            .add_custom_ignore_filename(".hematiteignore")
            .filter_entry(|entry| {
                if let Some(name) = entry.file_name().to_str() {
                    // Quick-prune massive dirs
                    if name == ".git" || name == "target" || name == "node_modules" || name.ends_with(".min.js") {
                        return false;
                    }
                }
                true
            })
            .build();
            
        let mut rust_bundle = get_rust_query().ok();
        let mut python_bundle = get_python_query().ok();
        let mut ts_bundle = get_ts_query().ok();
        let mut js_bundle = get_js_query().ok();

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
            let bundle = match ext {
                "rs" => rust_bundle.as_mut(),
                "py" => python_bundle.as_mut(),
                "ts" | "tsx" => ts_bundle.as_mut(),
                "js" | "jsx" => js_bundle.as_mut(),
                _ => None,
            };
            
            if let Some((language, query)) = bundle {
                let Ok(source_code) = fs::read_to_string(path) else {
                    continue;
                };
                
                let rel_path = path.strip_prefix(&self.root).unwrap_or(path).to_string_lossy().replace('\\', "/");
                
                let mut parser = Parser::new();
                if parser.set_language(language).is_ok() {
                    if let Some(tree) = parser.parse(&source_code, None) {
                        let mut cursor = QueryCursor::new();
                        let matches = cursor.matches(query, tree.root_node(), source_code.as_bytes());
                        let mut tags = Vec::new();
                        
                        for m in matches {
                            for capture in m.captures {
                                if let Ok(text) = capture.node.utf8_text(source_code.as_bytes()) {
                                    tags.push(format!("  - {}", text));
                                }
                            }
                        }
                        
                        if !tags.is_empty() {
                            tags.dedup(); // Basic deduplication in case of multi-matches
                            map.insert(rel_path.to_string(), tags);
                        }
                    }
                }
            }
        }
        
        let mut output = String::new();
        output.push_str("=== Repository Map (Structural Overview) ===\n");
        let mut total_tags = 0;
        
        for (rel_path, tags) in map {
            // Prevent map from blowing up the context window on mega-repos
            if total_tags > 1500 {
                output.push_str("... (Repository Map Truncated due to size constraints)\n");
                break;
            }
            output.push_str(&format!("{}:\n", rel_path));
            for tag in tags {
                output.push_str(&format!("{}\n", tag));
                total_tags += 1;
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
}
