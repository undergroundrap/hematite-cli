use crate::tts::vocab::VOCAB;

/// Tokenizes the given phonemes string into a vector of token indices.
///
/// This function takes a text string as input and converts it into a vector of token indices
/// by looking up each character in the global `VOCAB` map and mapping it to the corresponding
/// token index. The resulting vector contains the token indices for the input text.
///
/// # Arguments
/// * `text` - The input text string to be tokenized.
///
/// # Returns
/// A vector of `i64` token indices representing the input text.
pub fn tokenize(phonemes: &str) -> Vec<i64> {
    // Add start/end padding '$' as required by the Kokoro model
    let padded = format!("${}$", phonemes);
    
    padded
        .chars()
        .filter_map(|c| VOCAB.get(&c))
        .map(|&idx| idx as i64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let text = "heɪ ðɪs ɪz ˈlʌvliː!";
        let tokens = tokenize(text);

        // Expected tokens should now include padding '$' (index 0)
        assert_eq!(tokens[0], 0); // Start '$'
        assert_eq!(*tokens.last().unwrap(), 0); // End '$'
        assert!(tokens.len() > 2);

        // Test empty string
        let empty = "";
        let empty_tokens = tokenize(empty);
        assert_eq!(empty_tokens.len(), 2); // Should be ["$", "$"]
    }
}

use crate::tts::vocab::REVERSE_VOCAB;

pub fn tokens_to_phonemes(tokens: &[i64]) -> String {
    tokens
        .iter()
        .filter_map(|&t| REVERSE_VOCAB.get(&(t as usize)))
        .collect()
}

#[cfg(test)]
mod tests2 {
    use super::*;

    #[test]
    fn test_tokens_to_phonemes() {
        // Updated test data to include padding
        let tokens = vec![0, 24, 47, 54, 54, 57, 5, 0];
        let text = tokens_to_phonemes(&tokens);
        assert_eq!(text, "$Hello!$");

        let tokens = vec![
            0, 50, 83, 54, 156, 57, 135, 3, 16, 65, 156, 87, 158, 54, 46, 5, 0,
        ];

        let text = tokens_to_phonemes(&tokens);
        assert_eq!(text, "$həlˈoʊ, wˈɜːld!$");

        // Test empty vector
        let empty_tokens: Vec<i64> = vec![];
        assert_eq!(tokens_to_phonemes(&empty_tokens), "");
    }
}
