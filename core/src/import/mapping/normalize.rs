//! Folding a header or a value down to something comparable: case, accents, separators and
//! punctuation all differ between one broker's export and the next, and none of them carry
//! meaning.

pub fn normalize_alias(value: &str) -> String {
    value
        .trim()
        .to_uppercase()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .collect()
}

/// A header without punctuation: "Fees & Comm" and "Buy/Sell" have to compare equal to
/// "feescomm" and "buysell".
pub(super) fn normalize_header(header: &str) -> String {
    header
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

pub(super) fn header_words(header: &str) -> Vec<String> {
    header
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// How many consecutive words spell the alias: "Trade Date (UTC)" names "tradedate" with
/// two of its three words, "No. of shares" names "noofshares" with all three.
pub(super) fn joined_words(words: &[String], alias: &str) -> Option<usize> {
    (0..words.len()).find_map(|start| {
        let mut joined = String::new();
        words[start..]
            .iter()
            .position(|word| {
                joined.push_str(word);
                joined.len() <= alias.len() && joined == alias
            })
            .map(|offset| offset + 1)
    })
}
