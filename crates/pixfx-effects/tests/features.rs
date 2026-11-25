//! Every effect has a cargo feature of its own, so a game can build a wasm with only the effects it plays.
//! Runs with the default features (`all`): every effect in [`EFFECTS`] must be reachable from `all` through
//! its category, under the feature named after its id.

use pixfx_effects::EFFECTS;
use std::collections::{BTreeMap, BTreeSet};

const MANIFEST: &str = include_str!("../Cargo.toml");

/// The `[features]` table: `name = ["member", ...]`, one feature per line.
fn features() -> BTreeMap<&'static str, Vec<&'static str>> {
    MANIFEST
        .lines()
        .skip_while(|l| l.trim() != "[features]")
        .skip(1)
        .take_while(|l| !l.starts_with('['))
        .filter_map(|l| l.split_once(" = "))
        .map(|(name, members)| {
            let members = members
                .trim_matches(|c| c == '[' || c == ']')
                .split(',')
                .map(|m| m.trim().trim_matches('"'))
                .filter(|m| !m.is_empty())
                .collect();
            (name.trim(), members)
        })
        .collect()
}

/// The feature that compiles in the effect with this id (`feedback/outline` is `feedback-outline`).
fn feature_of(id: &str) -> String {
    id.replace('/', "-")
}

#[test]
fn each_effect_has_its_own_feature_and_each_effect_feature_an_effect() {
    let features = features();
    let ids: BTreeSet<String> = EFFECTS.iter().map(|e| feature_of(e.meta().id)).collect();
    let categories = features.get("all").cloned().unwrap_or_default();
    // an effect feature turns on nothing else; group features list their members, and a category with no effect yet
    // (`appear`) lists none
    let effect_features: BTreeSet<String> = features
        .iter()
        .filter(|(name, members)| members.is_empty() && !categories.contains(name))
        .map(|(name, _)| (*name).to_string())
        .collect();
    assert_eq!(ids, effect_features);
}

#[test]
fn each_category_lists_exactly_its_effects_and_all_lists_every_category() {
    let features = features();
    let mut categories: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for e in EFFECTS {
        let id = e.meta().id;
        let category = id.split('/').next().unwrap_or(id);
        categories
            .entry(category)
            .or_default()
            .insert(feature_of(id));
    }
    for (category, effects) in &categories {
        let listed: BTreeSet<String> = features
            .get(category)
            .map(|m| m.iter().map(|s| (*s).to_string()).collect())
            .unwrap_or_default();
        assert_eq!(&listed, effects, "feature `{category}`");
    }
    let all: BTreeSet<&str> = features
        .get("all")
        .map(|m| m.iter().copied().collect())
        .unwrap_or_default();
    // every category with effects is in `all`; the rest of `all` are categories with none yet
    assert!(
        categories.keys().all(|c| all.contains(c)),
        "`all` misses a category"
    );
    for c in all.iter().filter(|c| !categories.contains_key(*c)) {
        assert_eq!(
            features.get(c),
            Some(&vec![]),
            "feature `{c}` is in `all` but no category"
        );
    }
    assert_eq!(features.get("default"), Some(&vec!["all"]));
}
