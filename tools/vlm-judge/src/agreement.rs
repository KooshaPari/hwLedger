//! Inlined intent ↔ blind-description agreement scoring.
//!
//! Mirrors `phenotype_journey_core::agreement::score` (Jaccard overlap on
//! stemmed, stop-word-filtered tokens) so vlm-judge can compute the same
//! Green / Yellow / Red buckets without pulling phenotype-journey-core into
//! the hwLedger workspace graph. When the upstream crate lands as a vendored
//! dependency, swap this file for a re-export.

use rust_stemmers::{Algorithm, Stemmer};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agreement {
    Green,
    Yellow,
    Red,
}

impl Agreement {
    pub fn as_str(&self) -> &'static str {
        match self {
            Agreement::Green => "green",
            Agreement::Yellow => "yellow",
            Agreement::Red => "red",
        }
    }
    /// Green/Yellow -> passed, Red -> not passed.
    pub fn is_passed(&self) -> bool {
        matches!(self, Agreement::Green | Agreement::Yellow)
    }
}

#[derive(Debug, Clone)]
pub struct AgreementReport {
    pub status: Agreement,
    pub overlap: f64,
    pub missing_in_blind: Vec<String>,
    pub extras_in_blind: Vec<String>,
}

const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "if", "then", "else", "of", "for", "to", "in", "on",
    "at", "by", "with", "from", "as", "is", "are", "was", "were", "be", "been", "being", "it",
    "its", "this", "that", "these", "those", "i", "you", "he", "she", "we", "they", "them", "his",
    "her", "their", "our", "your", "my", "me", "him", "us", "do", "does", "did", "have", "has",
    "had", "will", "would", "should", "could", "can", "may", "might", "must", "so", "than", "when",
    "while", "where", "who", "what", "which", "some", "any", "all", "no", "not", "out", "up",
    "down", "into", "over", "under", "again", "about", "after", "before", "just", "also", "only",
    "very", "too", "there", "here", "s", "t",
];

fn is_stopword(w: &str) -> bool {
    w.len() <= 1 || STOPWORDS.iter().any(|s| *s == w)
}

pub fn tokenise(text: &str) -> Vec<String> {
    let stemmer = Stemmer::create(Algorithm::English);
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for c in ch.to_lowercase() {
                current.push(c);
            }
        } else if !current.is_empty() {
            push_token(&stemmer, &current, &mut out);
            current.clear();
        }
    }
    if !current.is_empty() {
        push_token(&stemmer, &current, &mut out);
    }
    out.into_iter().collect()
}

fn push_token(stemmer: &Stemmer, word: &str, out: &mut BTreeSet<String>) {
    if is_stopword(word) {
        return;
    }
    let stem = stemmer.stem(word).to_string();
    if stem.is_empty() || is_stopword(&stem) {
        return;
    }
    out.insert(stem);
}

pub fn score(intent: &str, blind: &str) -> AgreementReport {
    let intent_tokens = tokenise(intent);
    let blind_tokens = tokenise(blind);
    let intent_set: BTreeSet<&String> = intent_tokens.iter().collect();
    let blind_set: BTreeSet<&String> = blind_tokens.iter().collect();

    let overlap = if intent_set.is_empty() && blind_set.is_empty() {
        1.0
    } else if intent_set.is_empty() || blind_set.is_empty() {
        0.0
    } else {
        let inter = intent_set.intersection(&blind_set).count() as f64;
        let union = intent_set.union(&blind_set).count() as f64;
        inter / union
    };
    let status = if overlap >= 0.6 {
        Agreement::Green
    } else if overlap >= 0.3 {
        Agreement::Yellow
    } else {
        Agreement::Red
    };
    let missing_in_blind: Vec<String> = intent_set.difference(&blind_set).map(|s| (*s).clone()).collect();
    let extras_in_blind: Vec<String> = blind_set.difference(&intent_set).map(|s| (*s).clone()).collect();
    AgreementReport { status, overlap, missing_in_blind, extras_in_blind }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_intent_and_blind_scores_green() {
        let r = score(
            "Show the plan command help options available",
            "Terminal shows plan command help options available",
        );
        assert!(r.overlap >= 0.6);
        assert_eq!(r.status, Agreement::Green);
    }

    #[test]
    fn divergent_intent_and_blind_scores_red() {
        let r = score(
            "Show the plan command help text with all available options",
            "A photograph of a cat sitting on a windowsill bathed in sunlight.",
        );
        assert!(r.overlap < 0.3);
        assert_eq!(r.status, Agreement::Red);
    }
}
