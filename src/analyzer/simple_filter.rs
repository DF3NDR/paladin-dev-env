// src/analyzer/filter.rs
pub fn basic_filter(content: &str, keywords: &[&str]) -> bool {
    for keyword in keywords {
        if content.contains(keyword) {
            return true;
        }
    }
    false
}
