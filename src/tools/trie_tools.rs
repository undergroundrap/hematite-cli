use serde_json::{json, Value};
use std::collections::HashMap;

pub fn trie_tools_schema() -> Value {
    json!({
        "name": "trie_tools",
        "description": "Trie (prefix tree) data structure operations without external utilities. Actions: build (insert words into a trie and show the resulting structure), search (exact match lookup), prefix (find all words with a given prefix), autocomplete (ranked completions for a prefix), count (node count, word count, depth), suggest (typo-tolerant suggestions within edit distance 1).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["build", "search", "prefix", "autocomplete", "count", "suggest"],
                    "description": "Action to perform (default: build)"
                },
                "words": {
                    "type": ["array", "string"],
                    "description": "List of words to build the trie from (JSON array or space/comma-separated string)"
                },
                "query": {
                    "type": "string",
                    "description": "Word or prefix to search/complete/suggest"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results for autocomplete/prefix/suggest (default 10)"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Normalize input to lowercase (default false)"
                }
            }
        }
    })
}

// ── Trie node ────────────────────────────────────────────────────────────────

#[derive(Default)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,
    count: usize, // how many words pass through here
}

struct Trie {
    root: TrieNode,
    word_count: usize,
}

fn collect_words(node: &TrieNode, prefix: String, words: &mut Vec<String>) {
    if node.is_end {
        words.push(prefix.clone());
    }
    let mut sorted_keys: Vec<char> = node.children.keys().cloned().collect();
    sorted_keys.sort();
    for ch in sorted_keys {
        let child = &node.children[&ch];
        let mut new_prefix = prefix.clone();
        new_prefix.push(ch);
        collect_words(child, new_prefix, words);
    }
}

impl Trie {
    fn new() -> Self {
        Trie {
            root: TrieNode::default(),
            word_count: 0,
        }
    }

    fn insert(&mut self, word: &str) {
        let mut node = &mut self.root;
        node.count += 1;
        for ch in word.chars() {
            node = node.children.entry(ch).or_default();
            node.count += 1;
        }
        if !node.is_end {
            node.is_end = true;
            self.word_count += 1;
        }
    }

    fn search(&self, word: &str) -> bool {
        let mut node = &self.root;
        for ch in word.chars() {
            match node.children.get(&ch) {
                Some(n) => node = n,
                None => return false,
            }
        }
        node.is_end
    }

    fn prefix_node(&self, prefix: &str) -> Option<&TrieNode> {
        let mut node = &self.root;
        for ch in prefix.chars() {
            match node.children.get(&ch) {
                Some(n) => node = n,
                None => return None,
            }
        }
        Some(node)
    }

    fn starts_with(&self, prefix: &str) -> bool {
        self.prefix_node(prefix).is_some()
    }

    fn autocomplete(&self, prefix: &str, limit: usize) -> Vec<String> {
        let mut words = Vec::new();
        if let Some(node) = self.prefix_node(prefix) {
            collect_words(node, prefix.to_string(), &mut words);
        }
        words.truncate(limit);
        words
    }

    fn node_count(&self) -> usize {
        fn count_nodes(node: &TrieNode) -> usize {
            1 + node.children.values().map(count_nodes).sum::<usize>()
        }
        count_nodes(&self.root)
    }

    fn max_depth(&self) -> usize {
        fn depth(node: &TrieNode) -> usize {
            if node.children.is_empty() {
                0
            } else {
                1 + node.children.values().map(depth).max().unwrap_or(0)
            }
        }
        depth(&self.root)
    }

    fn render_ascii(&self, limit: usize) -> String {
        fn render_char(
            node: &TrieNode,
            ch: char,
            prefix: &str,
            is_last: bool,
            lines: &mut Vec<String>,
            limit: usize,
        ) {
            if lines.len() >= limit {
                return;
            }
            let connector = if is_last { "└── " } else { "├── " };
            let end_mark = if node.is_end { " *" } else { "" };
            // strip trailing "    " from prefix to get parent prefix
            let parent_prefix = &prefix[..prefix.len().saturating_sub(4)];
            lines.push(format!(
                "{}{}'{}'{}",
                parent_prefix, connector, ch, end_mark
            ));
            let mut sorted_keys: Vec<char> = node.children.keys().cloned().collect();
            sorted_keys.sort();
            let last_idx = sorted_keys.len().saturating_sub(1);
            for (i, &c) in sorted_keys.iter().enumerate() {
                let child = &node.children[&c];
                let new_prefix = format!(
                    "{}{}    ",
                    parent_prefix,
                    if is_last { "    " } else { "│   " }
                );
                render_char(child, c, &new_prefix, i == last_idx, lines, limit);
            }
        }
        let mut lines = vec!["(root)".to_string()];
        let mut sorted_keys: Vec<char> = self.root.children.keys().cloned().collect();
        sorted_keys.sort();
        let last_idx = sorted_keys.len().saturating_sub(1);
        for (i, &ch) in sorted_keys.iter().enumerate() {
            let child = &self.root.children[&ch];
            render_char(child, ch, "", i == last_idx, &mut lines, limit);
        }
        if lines.len() >= limit {
            lines.push(format!("  ... (truncated at {} nodes)", limit));
        }
        lines.join("\n")
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_words(v: &Value) -> Result<Vec<String>, String> {
    if let Some(arr) = v.as_array() {
        arr.iter()
            .map(|x| {
                x.as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| "non-string word".to_string())
            })
            .collect()
    } else if let Some(s) = v.as_str() {
        Ok(s.split([',', ' ', '\n'])
            .map(|w| w.trim().to_string())
            .filter(|w| !w.is_empty())
            .collect())
    } else {
        Err("pass 'words' as JSON array or space/comma-separated string".to_string())
    }
}

fn build_trie(args: &Value) -> Result<Trie, String> {
    let words_val = args.get("words").ok_or("pass 'words' to build the trie")?;
    let mut words = parse_words(words_val)?;
    if words.is_empty() {
        return Err("'words' list is empty".to_string());
    }
    let ci = args
        .get("case_insensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if ci {
        words = words.into_iter().map(|w| w.to_lowercase()).collect();
    }
    let mut trie = Trie::new();
    for w in &words {
        trie.insert(w);
    }
    Ok(trie)
}

// ── Actions ──────────────────────────────────────────────────────────────────

fn action_build(args: &Value) -> Result<String, String> {
    let trie = build_trie(args)?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let mut lines = Vec::new();
    lines.push(format!(
        "Words: {}  |  Nodes: {}  |  Depth: {}",
        trie.word_count,
        trie.node_count(),
        trie.max_depth()
    ));
    lines.push(String::new());
    lines.push(trie.render_ascii(limit.min(200)));
    lines.push(String::new());
    lines.push("(* = end of word)".to_string());
    Ok(lines.join("\n"))
}

fn action_search(args: &Value) -> Result<String, String> {
    let trie = build_trie(args)?;
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("pass 'query' to search")?;
    let ci = args
        .get("case_insensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let q = if ci {
        query.to_lowercase()
    } else {
        query.to_string()
    };
    let found = trie.search(&q);
    let prefix_match = !found && trie.starts_with(&q);
    let status = if found {
        "FOUND"
    } else if prefix_match {
        "NOT FOUND (but prefix exists)"
    } else {
        "NOT FOUND"
    };
    Ok(format!("Search '{}': {}", query, status))
}

fn action_prefix(args: &Value) -> Result<String, String> {
    let trie = build_trie(args)?;
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("pass 'query' (prefix)")?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let ci = args
        .get("case_insensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let q = if ci {
        query.to_lowercase()
    } else {
        query.to_string()
    };
    let words = trie.autocomplete(&q, limit);
    if words.is_empty() {
        return Ok(format!("No words found with prefix '{}'", query));
    }
    let mut lines = vec![format!("{} word(s) with prefix '{}':", words.len(), query)];
    for w in &words {
        lines.push(format!("  {}", w));
    }
    Ok(lines.join("\n"))
}

fn action_autocomplete(args: &Value) -> Result<String, String> {
    let trie = build_trie(args)?;
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("pass 'query' (prefix to complete)")?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let ci = args
        .get("case_insensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let q = if ci {
        query.to_lowercase()
    } else {
        query.to_string()
    };
    let words = trie.autocomplete(&q, limit);
    if words.is_empty() {
        return Ok(format!("No completions for '{}'", query));
    }
    let mut lines = vec![format!(
        "Completions for '{}' (top {}):",
        query,
        words.len()
    )];
    for (i, w) in words.iter().enumerate() {
        lines.push(format!("  {}. {}", i + 1, w));
    }
    Ok(lines.join("\n"))
}

fn action_count(args: &Value) -> Result<String, String> {
    let trie = build_trie(args)?;
    Ok(format!(
        "Words: {}  |  Nodes: {}  |  Max depth: {}",
        trie.word_count,
        trie.node_count(),
        trie.max_depth()
    ))
}

fn action_suggest(args: &Value) -> Result<String, String> {
    let trie = build_trie(args)?;
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("pass 'query' to get suggestions")?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let ci = args
        .get("case_insensitive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let q = if ci {
        query.to_lowercase()
    } else {
        query.to_string()
    };

    // collect all words in trie
    let mut all_words = Vec::new();
    collect_words(&trie.root, String::new(), &mut all_words);

    // find words within edit distance 1 (deletion, substitution, insertion, transposition)
    let mut suggestions: Vec<(&str, usize)> = all_words
        .iter()
        .filter_map(|w| {
            let d = edit_distance(&q, w);
            if d <= 1 {
                Some((w.as_str(), d))
            } else {
                None
            }
        })
        .collect();
    suggestions.sort_by_key(|(_, d)| *d);
    suggestions.truncate(limit);

    if suggestions.is_empty() {
        // broaden to edit distance 2 if nothing found
        let mut wider: Vec<(&str, usize)> = all_words
            .iter()
            .filter_map(|w| {
                let d = edit_distance(&q, w);
                if d <= 2 {
                    Some((w.as_str(), d))
                } else {
                    None
                }
            })
            .collect();
        wider.sort_by_key(|(_, d)| *d);
        wider.truncate(limit);
        if wider.is_empty() {
            return Ok(format!(
                "No suggestions for '{}' (edit distance ≤ 2)",
                query
            ));
        }
        let mut lines = vec![format!("Suggestions for '{}' (edit distance ≤ 2):", query)];
        for (w, d) in &wider {
            lines.push(format!("  {} (distance {})", w, d));
        }
        return Ok(lines.join("\n"));
    }

    let mut lines = vec![format!("Suggestions for '{}' (edit distance ≤ 1):", query)];
    for (w, d) in &suggestions {
        lines.push(format!("  {} (distance {})", w, d));
    }
    Ok(lines.join("\n"))
}

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j - 1].min(dp[i - 1][j]).min(dp[i][j - 1])
            };
        }
    }
    dp[m][n]
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("build");
    match action {
        "build" => action_build(args),
        "search" => action_search(args),
        "prefix" => action_prefix(args),
        "autocomplete" => action_autocomplete(args),
        "count" => action_count(args),
        "suggest" => action_suggest(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: build, search, prefix, autocomplete, count, suggest",
            action
        )),
    }
}
