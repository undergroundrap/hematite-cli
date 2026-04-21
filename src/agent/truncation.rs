/// Grounded Output Truncation Module.
/// Ports the "Middle-Truncation" patterns from Codex-RS to ensure
/// Hematite preserves exit codes and headers while providing line metadata.

pub fn formatted_truncate(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }

    let total_lines = content.lines().count();
    let truncated = truncate_middle(content, max_bytes);
    
    format!(
        "[TRUNCATED: total lines: {}]\n{}\n[... middle truncated to fit budget ...]\n{}",
        total_lines,
        truncated.head,
        truncated.tail
    )
}

pub struct TruncatedOutput {
    pub head: String,
    pub tail: String,
}

/// Truncate a string by keeping the beginning and end, removing the middle.
/// Ensures UTF-8 safety by finding valid character boundaries.
pub fn truncate_middle(content: &str, max_bytes: usize) -> TruncatedOutput {
    if content.len() <= max_bytes {
        return TruncatedOutput {
            head: content.to_string(),
            tail: String::new(),
        };
    }

    // Keep 40% at the start, 40% at the end (roughly).
    let head_size = (max_bytes as f32 * 0.4) as usize;
    let tail_size = (max_bytes as f32 * 0.4) as usize;

    // Find valid UTF-8 boundaries
    let head_boundary = find_valid_boundary_forward(content, head_size);
    let tail_boundary = find_valid_boundary_backward(content, content.len() - tail_size);

    TruncatedOutput {
        head: content[..head_boundary].to_string(),
        tail: content[tail_boundary..].to_string(),
    }
}

fn find_valid_boundary_forward(content: &str, target: usize) -> usize {
    let mut pos = target;
    while pos > 0 && !content.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

fn find_valid_boundary_backward(content: &str, target: usize) -> usize {
    let mut pos = target;
    while pos < content.len() && !content.is_char_boundary(pos) {
        pos += 1;
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middle_truncation() {
        let input = "1234567890";
        let result = truncate_middle(input, 4);
        // 4 bytes budget -> 40% is 1.6 bytes -> 1 byte head, 1 byte tail
        assert_eq!(result.head, "1");
        assert_eq!(result.tail, "0");
    }

    #[test]
    fn test_utf8_boundary_safety() {
        let input = "🦀🦀🦀🦀🦀"; // 每个螃蟹 4 字节, 总共 20 字节
        let result = truncate_middle(input, 10);
        // 10 bytes budget -> 4 byte head, 4 byte tail
        assert_eq!(result.head, "🦀");
        assert_eq!(result.tail, "🦀");
    }
}
