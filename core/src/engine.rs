// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Categorization and explanation engine.
//!
//! Decision order (first match wins):
//! 1. Critical rules from the bundled database — never downgradable.
//! 2. Curated known items — publisher/app → category + explanation.
//! 3. Heuristic fallback — temp-folder location or invalid signature → Critical.
//! 4. Otherwise → Recommended ("not in the curated list, review it").

use crate::model::{Category, SignatureStatus, StartupItem};
use serde::Deserialize;

const BUNDLED_DB: &str = include_str!("../data/known-items.json");

#[derive(Debug, Deserialize)]
struct Rule {
    #[serde(rename = "match")]
    pattern: String,
    #[serde(default)]
    publisher: Option<String>,
    #[serde(default = "default_category")]
    category: Category,
    explanation: String,
}

fn default_category() -> Category {
    Category::Recommended
}

#[derive(Debug, Deserialize)]
pub struct KnownDb {
    pub version: u32,
    #[serde(default)]
    critical_rules: Vec<Rule>,
    #[serde(default)]
    known_items: Vec<Rule>,
}

/// Path fragments (lowercase, forward-slash normalized) that indicate a
/// temporary or user-writable scratch location.
const TEMP_MARKERS: &[&str] = &["/temp/", "/tmp/", "/var/tmp/", "\\temp\\"];

pub struct Engine {
    db: KnownDb,
}

impl Engine {
    /// Engine backed by the database compiled into the binary.
    pub fn bundled() -> Self {
        // The bundled file is part of this crate and checked by tests.
        Self::from_json(BUNDLED_DB).expect("bundled known-items.json must parse")
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        Ok(Self {
            db: serde_json::from_str(json)?,
        })
    }

    pub fn database_version(&self) -> u32 {
        self.db.version
    }

    /// Set `category` and `explanation` (and `publisher` when the database
    /// knows it). Any category already on the item is overwritten.
    pub fn classify(&self, item: &mut StartupItem) {
        let (category, explanation, publisher) = self.decide(item);
        item.category = category;
        item.explanation = explanation;
        if item.publisher.is_none() {
            item.publisher = publisher;
        }
    }

    fn decide(&self, item: &StartupItem) -> (Category, String, Option<String>) {
        let haystack = format!(
            "{} {}",
            item.display_name.to_lowercase(),
            item.command.as_deref().unwrap_or("").to_lowercase()
        );

        if let Some(rule) = find(&self.db.critical_rules, &haystack) {
            return (Category::Critical, rule.explanation.clone(), None);
        }

        if let Some(rule) = find(&self.db.known_items, &haystack) {
            return (
                rule.category,
                rule.explanation.clone(),
                rule.publisher.clone(),
            );
        }

        if in_temp_location(item) || item.signature == SignatureStatus::Invalid {
            return (
                Category::Critical,
                "Runs from a temporary folder or has an invalid publisher signature. \
                 Review where it came from before changing it."
                    .to_string(),
                None,
            );
        }

        (
            Category::Recommended,
            "Not in the curated list. Check the source location and publisher before changing it."
                .to_string(),
            None,
        )
    }
}

fn find<'a>(rules: &'a [Rule], haystack: &str) -> Option<&'a Rule> {
    rules
        .iter()
        .find(|r| haystack.contains(&r.pattern.to_lowercase()))
}

fn in_temp_location(item: &StartupItem) -> bool {
    let target = item
        .command
        .as_deref()
        .unwrap_or(item.location.as_str())
        .to_lowercase()
        .replace('\\', "/");
    // Pad so folder markers at the very start/end still match.
    let padded = format!("/{}/", target.trim_matches('/'));
    TEMP_MARKERS
        .iter()
        .any(|m| padded.contains(&m.replace('\\', "/")))
}
