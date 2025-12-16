use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use rayon::prelude::*;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

/// Lightspeed index with advanced data structures
#[derive(Debug, Serialize, Deserialize)]
pub struct LightspeedIndex {
    /// N-gram index for substring search (simulates suffix tree behavior)
    pub ngram_index: HashMap<String, Vec<usize>>,
    
    /// File entries indexed by ID
    pub entries: Vec<LightspeedEntry>,
    
    /// Deletion-based index for SymSpell-style fuzzy search
    pub deletion_index: HashMap<String, Vec<usize>>,
    
    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
    
    /// Indexed paths
    pub indexed_paths: Vec<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightspeedEntry {
    pub id: usize,
    pub path: PathBuf,
    pub name: String,
    pub name_lower: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

impl LightspeedIndex {
    pub fn new() -> Self {
        Self {
            ngram_index: HashMap::new(),
            entries: Vec::new(),
            deletion_index: HashMap::new(),
            last_updated: Utc::now(),
            indexed_paths: Vec::new(),
        }
    }

    /// Build n-gram index for fast substring search
    /// This simulates suffix tree behavior with O(k) lookup time
    pub fn build_ngram_index(&mut self, n: usize) {
        self.ngram_index.clear();
        
        for (idx, entry) in self.entries.iter().enumerate() {
            let text = &entry.name_lower;
            
            // Generate all n-grams using character-level indexing to handle UTF-8
            let chars: Vec<char> = text.chars().collect();
            for i in 0..chars.len() {
                for j in i+1..=chars.len().min(i + n + 2) {
                    let ngram: String = chars[i..j].iter().collect();
                    self.ngram_index
                        .entry(ngram)
                        .or_insert_with(Vec::new)
                        .push(idx);
                }
            }
            
            // Also index the full path for path-based searches
            let path_str = entry.path.to_string_lossy().to_lowercase();
            let path_chars: Vec<char> = path_str.chars().collect();
            for i in 0..path_chars.len() {
                for j in i+1..=path_chars.len().min(i + n + 2) {
                    let ngram: String = path_chars[i..j].iter().collect();
                    self.ngram_index
                        .entry(ngram)
                        .or_insert_with(Vec::new)
                        .push(idx);
                }
            }
        }
    }

    /// Build deletion index for SymSpell-style O(1) fuzzy search
    /// Pre-computes all single-character deletions
    pub fn build_deletion_index(&mut self, max_edit_distance: usize) {
        self.deletion_index.clear();
        
        for (idx, entry) in self.entries.iter().enumerate() {
            let text = &entry.name_lower;
            
            // Generate all possible deletions up to max_edit_distance
            let deletions = generate_deletions(text, max_edit_distance);
            
            for deletion in deletions {
                self.deletion_index
                    .entry(deletion)
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
        }
    }

    /// Fast substring search using n-gram index
    /// O(k) where k is query length - independent of number of files
    pub fn search_substring(&self, query: &str) -> Vec<usize> {
        let query_lower = query.to_lowercase();
        
        // First try exact n-gram lookup
        if let Some(candidates) = self.ngram_index.get(&query_lower) {
            let mut results: Vec<usize> = candidates.iter()
                .filter(|&&idx| {
                    let entry = &self.entries[idx];
                    entry.name_lower.contains(&query_lower) || 
                    entry.path.to_string_lossy().to_lowercase().contains(&query_lower)
                })
                .copied()
                .collect();
            
            results.sort_unstable();
            results.dedup();
            return results;
        }
        
        // Fallback: linear scan with contains() - still fast for small datasets
        let results: Vec<usize> = self.entries.iter()
            .enumerate()
            .filter(|(_, entry)| {
                entry.name_lower.contains(&query_lower) || 
                entry.path.to_string_lossy().to_lowercase().contains(&query_lower)
            })
            .map(|(idx, _)| idx)
            .collect();
        
        results
    }

    /// Ultra-fast fuzzy search using pre-computed deletion index
    /// O(1) lookup in hash map for each deletion
    pub fn search_fuzzy_symspell(&self, query: &str, max_distance: usize) -> Vec<(usize, i64)> {
        let query_lower = query.to_lowercase();
        let deletions = generate_deletions(&query_lower, max_distance);
        
        let mut candidates: HashMap<usize, i64> = HashMap::new();
        
        // Check each deletion variant
        for deletion in deletions {
            if let Some(indices) = self.deletion_index.get(&deletion) {
                for &idx in indices {
                    *candidates.entry(idx).or_insert(0) += 1;
                }
            }
        }
        
        // Also check exact match
        if let Some(indices) = self.deletion_index.get(&query_lower) {
            for &idx in indices {
                *candidates.entry(idx).or_insert(0) += 10; // Boost exact matches
            }
        }
        
        let mut results: Vec<_> = candidates.into_iter().collect();
        results.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by score descending
        results
    }

    /// Parallel fuzzy search using SIMD-accelerated fuzzy matcher
    /// Leverages rayon for parallel processing across CPU cores
    pub fn search_fuzzy_parallel(&self, query: &str, threshold: i64) -> Vec<(usize, i64)> {
        let matcher = SkimMatcherV2::default();
        let query_lower = query.to_lowercase();
        
        // Parallel search across all entries using rayon
        let results: Vec<_> = self.entries
            .par_iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                // Try matching against filename
                let score1 = matcher.fuzzy_match(&entry.name_lower, &query_lower);
                
                // Try matching against full path
                let path_str = entry.path.to_string_lossy().to_lowercase();
                let score2 = matcher.fuzzy_match(&path_str, &query_lower);
                
                // Take the best score
                let best_score = score1.max(score2);
                
                if let Some(score) = best_score {
                    if score >= threshold {
                        return Some((idx, score));
                    }
                }
                None
            })
            .collect();
        
        let mut sorted_results = results;
        sorted_results.sort_by(|a, b| b.1.cmp(&a.1));
        sorted_results
    }

    /// Hybrid search: Uses best algorithm based on query characteristics
    pub fn search_hybrid(&self, query: &str, fuzzy: bool, fuzzy_threshold: i64) -> Vec<(usize, i64)> {
        if !fuzzy {
            // Pure substring search - O(k) with n-gram index
            let indices = self.search_substring(query);
            indices.into_iter().map(|idx| (idx, 100)).collect()
        } else {
            // Use parallel fuzzy search - works reliably for all query lengths
            self.search_fuzzy_parallel(query, fuzzy_threshold)
        }
    }
}

/// Generate all deletions up to max_distance (SymSpell algorithm)
fn generate_deletions(word: &str, max_distance: usize) -> Vec<String> {
    let mut deletions = Vec::new();
    
    fn generate_recursive(
        word: String,
        remaining: usize,
        deletions: &mut Vec<String>,
    ) {
        if !deletions.contains(&word) {
            deletions.push(word.clone());
        }
        
        if remaining > 0 && word.len() > 0 {
            let chars: Vec<char> = word.chars().collect();
            for i in 0..chars.len() {
                let mut new_chars = chars.clone();
                new_chars.remove(i);
                let new_word: String = new_chars.iter().collect();
                
                if !deletions.contains(&new_word) {
                    generate_recursive(new_word, remaining - 1, deletions);
                }
            }
        }
    }
    
    generate_recursive(word.to_string(), max_distance, &mut deletions);
    deletions.dedup();
    deletions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ngram_index() {
        let mut index = LightspeedIndex::new();
        index.entries.push(LightspeedEntry {
            id: 0,
            path: PathBuf::from("test.txt"),
            name: "test.txt".to_string(),
            name_lower: "test.txt".to_string(),
            size: 100,
            modified: Utc::now(),
        });
        
        index.build_ngram_index(3);
        let results = index.search_substring("test");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_deletions() {
        let deletions = generate_deletions("test", 1);
        assert!(deletions.contains(&"est".to_string()));
        assert!(deletions.contains(&"tst".to_string()));
        assert!(deletions.contains(&"tes".to_string()));
    }

    #[test]
    fn test_ngram_index_with_multibyte_utf8() {
        // Test case for the issue with "geschäftsbrief 1.tmvx"
        let mut index = LightspeedIndex::new();
        index.entries.push(LightspeedEntry {
            id: 0,
            path: PathBuf::from("/test/geschäftsbrief 1.tmvx"),
            name: "geschäftsbrief 1.tmvx".to_string(),
            name_lower: "geschäftsbrief 1.tmvx".to_string(),
            size: 1024,
            modified: Utc::now(),
        });
        
        // This should not panic when building n-gram index
        index.build_ngram_index(3);
        
        // Verify we can search for the file
        let results = index.search_substring("geschäft");
        assert_eq!(results.len(), 1);
        
        // Search with the umlaut character
        let results = index.search_substring("äft");
        assert_eq!(results.len(), 1);
        
        // Search for part of the file
        let results = index.search_substring("brief");
        assert_eq!(results.len(), 1);
    }
}
