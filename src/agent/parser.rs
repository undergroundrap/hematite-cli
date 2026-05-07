// ── src/agent/parser.rs: XML-ish Parser for Swarm Workloads ─────────────────────
/// Resilient parser that maps XML-like tags to structured data without regex overhead.
/// Designed to handle LLM output quirks (trailing commas, broken escapes) gracefully.

#[derive(Debug, Clone)]
pub struct WorkerTask {
    /// Unique identifier for the worker task (e.g., "w-001", "worker-alpha")
    pub id: String,

    /// Target file path or directory where the work should be applied
    pub target: String,

    /// The instruction/payload describing what work to perform on the target
    pub instruction: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Hunk {
    /// Starting line number (1-indexed) of the patch region
    pub start_line: usize,

    /// Ending line number (1-indexed, inclusive) of the patch region
    pub end_line: usize,

    /// The actual content/patch to apply within the specified line range
    pub content: String,

    /// Identifier of the worker that generated this hunk (for attribution)
    pub worker_id: String,
}

impl Hunk {
    #[allow(dead_code)]
    /// Returns a sort key for ordering hunks by position.
    /// Uses reverse ordering so higher line numbers come first (useful for bottom-up processing).
    ///
    /// # Returns
    /// A tuple of `(Reverse(start_line), Reverse(end_line))` for stable multi-key sorting.
    pub fn sort_key(&self) -> (std::cmp::Reverse<usize>, std::cmp::Reverse<usize>) {
        (
            std::cmp::Reverse(self.start_line),
            std::cmp::Reverse(self.end_line),
        )
    }
}

/// Parses master specification XML content into a vector of [`WorkerTask`] items.
///
/// This function splits the input by `<worker_task` tags and extracts:
/// - `id`: Unique task identifier from `id="..."` attribute
/// - `target`: Target file/path from `target="..."` attribute  
/// - `instruction`: The payload content between opening tag and `</worker_task>`
///
/// # Arguments
/// * `xml_content` — Raw XML-ish string containing worker task definitions
///
/// # Returns
/// A `Vec<WorkerTask>` with all parsed tasks (skips malformed blocks)
///
/// # Example
/// ```ignore
/// let xml = r#"<worker_task id="w-001" target="src/main.rs">
///     // Do something
/// </worker_task>"#;
/// let tasks = parse_master_spec(xml);
/// assert_eq!(tasks.len(), 1);
/// ```
pub fn parse_master_spec(xml_content: &str) -> Vec<WorkerTask> {
    let mut tasks = Vec::new();
    let iter = xml_content.split("<worker_task");

    // Skip the first block because the payload physically starts after `<worker_task`
    for block in iter.skip(1) {
        let Some(tag_end) = block.find('>') else {
            continue;
        };
        let tag_attrs = &block[..tag_end];

        // Parse ID — skip block if attribute is absent or unclosed
        let Some(id_attr_pos) = tag_attrs.find("id=\"") else {
            continue;
        };
        let id_start = id_attr_pos + 4;
        let Some(id_end_rel) = tag_attrs[id_start..].find('"') else {
            continue;
        };
        let id = &tag_attrs[id_start..id_start + id_end_rel];

        // Parse Target — skip block if attribute is absent or unclosed
        let Some(target_attr_pos) = tag_attrs.find("target=\"") else {
            continue;
        };
        let target_start = target_attr_pos + 8;
        let Some(target_end_rel) = tag_attrs[target_start..].find('"') else {
            continue;
        };
        let target = &tag_attrs[target_start..target_start + target_end_rel];

        // Retrieve instruction payload bounds
        let content_block = &block[tag_end + 1..];
        let Some(content_end) = content_block.find("</worker_task>") else {
            continue;
        };
        let instruction = content_block[..content_end].trim();

        tasks.push(WorkerTask {
            id: id.to_string(),
            target: target.to_string(),
            instruction: instruction.to_string(),
        });
    }

    tasks
}

/// Parses scratchpad diff content from `.hematite_scratch` files into [`Hunk`] objects.
///
/// This function scans raw XML-ish content for `<patch>` tags and extracts:
/// - `start`: Starting line number (from `start="..."` attribute)
/// - `end`: Ending line number (from `end="..."` attribute)
/// - `content`: The patch content between `<patch>` and `</patch>`
///
/// # Arguments
/// * `raw_content` — Raw string from scratchpad file containing patch tags
/// * `worker_id` — Identifier of the worker processing this content
///
/// # Returns
/// A `Vec<Hunk>` with all parsed patches (stops at malformed or unclosed tags)
///
/// # Example
/// ```ignore
/// let xml = r#"<patch start="10" end="20">
///     // Some diff content
/// </patch>"#;
/// let hunks = parse_scratchpad_diffs(xml, "worker-1".to_string());
/// assert_eq!(hunks[0].start_line, 10);
/// ```
#[allow(dead_code)]
pub fn parse_scratchpad_diffs(raw_content: &str, worker_id: String) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    let mut current_pos = 0;

    fn parse_attr(attr_str: &str, key: &str) -> Option<usize> {
        let klen = key.len();
        let bytes = attr_str.as_bytes();
        let mut pos = 0;
        while let Some(rel) = attr_str[pos..].find(key) {
            let abs = pos + rel;
            let after = abs + klen;
            if bytes.get(after).copied() == Some(b'=')
                && bytes.get(after + 1).copied() == Some(b'"')
            {
                let val_start = after + 2;
                let end = attr_str[val_start..].find('"')? + val_start;
                return attr_str[val_start..end].parse().ok();
            }
            pos = abs + 1;
        }
        None
    }

    while let Some(start_idx) = raw_content[current_pos..].find("<patch") {
        let absolute_start = current_pos + start_idx;
        let Some(tag_end) = raw_content[absolute_start..].find('>') else {
            break;
        };
        let attr_str = &raw_content[absolute_start..absolute_start + tag_end];

        let start_line = parse_attr(attr_str, "start").unwrap_or(0);
        let end_line = parse_attr(attr_str, "end").unwrap_or(0);

        let body_start = absolute_start + tag_end + 1;
        if let Some(end_idx) = raw_content[body_start..].find("</patch>") {
            let content = raw_content[body_start..body_start + end_idx]
                .trim()
                .to_string();
            hunks.push(Hunk {
                start_line,
                end_line,
                content,
                worker_id: worker_id.clone(),
            });
            current_pos = body_start + end_idx + 8;
        } else {
            break;
        }
    }
    hunks
}
