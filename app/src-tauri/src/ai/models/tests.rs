use super::*;

fn pick(ids: &[&str], tier: fn(&str) -> Option<u8>) -> Vec<String> {
    let mut ids: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    ids.sort_by_key(|id| (rank(id), id.clone()));
    shortlist(ids, tier)
}

#[test]
fn a_chat_starts_on_the_smallest_tier() {
    let listed: Vec<String> = ["gpt-6-astra", "gpt-6-terra", "gpt-6-luna"]
        .map(String::from)
        .into();
    assert_eq!(smallest("openai", &listed).as_deref(), Some("gpt-6-luna"));
    let listed: Vec<String> = ["claude-opus-5", "claude-sonnet-5", "claude-haiku-4-5"]
        .map(String::from)
        .into();
    assert_eq!(
        smallest("anthropic", &listed).as_deref(),
        Some("claude-haiku-4-5")
    );
    // The custom list begins with the model the user typed, and that is where it starts.
    let listed: Vec<String> = ["my-local-model", "gpt-6-luna"].map(String::from).into();
    assert_eq!(smallest(CUSTOM, &listed).as_deref(), Some("my-local-model"));
}

#[test]
fn a_remembered_model_survives_a_release_in_its_own_tier() {
    let listed: Vec<String> = ["gpt-6.1-astra", "gpt-6.1-terra", "gpt-6.1-luna"]
        .map(String::from)
        .into();
    assert_eq!(
        remembered("openai", "gpt-6.1-terra", &listed).as_deref(),
        Some("gpt-6.1-terra")
    );
    // Retired: the newest model of the same tier, not the flagship.
    assert_eq!(
        remembered("openai", "gpt-6-terra", &listed).as_deref(),
        Some("gpt-6.1-terra")
    );
    assert_eq!(remembered("openai", "some-unknown-id", &listed), None);
}

#[test]
fn chat_models_rank_above_anything_else() {
    let mut ids = vec![
        "text-embedding-3-large".to_string(),
        "gpt-5.1".to_string(),
        "some-new-family-1".to_string(),
    ];
    ids.sort_by_key(|id| (rank(id), id.clone()));
    assert_eq!(ids, ["gpt-5.1", "some-new-family-1", "text-embedding-3-large"]);
}

#[test]
fn openai_offers_the_newest_of_three_tiers_and_nothing_else() {
    let picked = pick(
        &[
            "gpt-4o",
            "gpt-5",
            "gpt-5-2025-08-07",
            "gpt-5-mini",
            "gpt-5-nano",
            "gpt-5.1",
            "gpt-5.1-codex",
            "gpt-5.4",
            "gpt-5.4-chat-latest",
            "gpt-5.2-mini",
            "gpt-4o-transcribe",
            "text-embedding-3-large",
            "o3",
        ],
        tier_openai,
    );
    assert_eq!(picked, ["gpt-5.4", "gpt-5.2-mini", "gpt-5-nano"]);
}

#[test]
fn openai_named_tiers_replace_the_older_plain_ones() {
    let picked = pick(
        &[
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.4-nano",
            "gpt-5.5",
            "gpt-5.6-sol",
            "gpt-5.6-sol-2026-07-09",
            "gpt-5.6-terra",
            "gpt-5.6-luna",
            "gpt-5.6-sol-pro",
            "gpt-6-astra",
        ],
        tier_openai,
    );
    assert_eq!(picked, ["gpt-6-astra", "gpt-5.6-terra", "gpt-5.6-luna"]);
}

#[test]
fn anthropic_offers_one_model_per_family_whichever_way_the_id_is_spelled() {
    let picked = pick(
        &[
            "claude-3-5-sonnet-latest",
            "claude-haiku-4-5-20251001",
            "claude-opus-4-1-20250805",
            "claude-opus-5",
            "claude-sonnet-4-5-20250929",
        ],
        tier_anthropic,
    );
    assert_eq!(
        picked,
        [
            "claude-opus-5",
            "claude-sonnet-4-5-20250929",
            "claude-haiku-4-5-20251001"
        ]
    );
}

#[test]
fn gemini_offers_one_model_per_tier_and_lite_is_not_flash() {
    let picked = pick(
        &[
            "gemini-2.5-flash",
            "gemini-3.1-flash-lite",
            "gemini-3.1-pro",
            "gemini-3.1-pro-preview",
            "gemini-3.8-flash",
            "gemini-embedding-001",
            "gemini-flash-latest",
        ],
        tier_gemini,
    );
    assert_eq!(
        picked,
        ["gemini-3.1-pro", "gemini-3.8-flash", "gemini-3.1-flash-lite"]
    );
}

#[test]
fn gemini_offers_a_preview_when_the_new_generation_has_no_stable_id_yet() {
    // What Google listed when `gemini-2.5-pro` was already refused to new keys.
    let picked = pick(
        &[
            "gemini-2.5-flash-preview-09-2025",
            "gemini-2.5-pro",
            "gemini-3.1-pro-preview",
            "gemini-3.5-flash-lite",
            "gemini-3.8-flash",
            "gemini-3.8-flash-preview",
        ],
        tier_gemini,
    );
    assert_eq!(
        picked,
        [
            "gemini-3.1-pro-preview",
            "gemini-3.8-flash",
            "gemini-3.5-flash-lite"
        ]
    );
}

#[test]
fn a_google_catalogue_is_read_through_the_same_reader() {
    let body = serde_json::json!({
        "models": [
            { "name": "models/gemini-3.1-pro" },
            { "name": "models/gemini-embedding-001" },
        ]
    });
    assert_eq!(
        ranked(&body, rank_gemini),
        ["gemini-3.1-pro", "gemini-embedding-001"]
    );
}

#[test]
fn a_pinned_snapshot_loses_to_the_same_version_without_a_date() {
    assert!(newer("gpt-5", "gpt-5-2025-08-07"));
    assert!(newer("claude-opus-5", "claude-opus-4-1-20250805"));
    // A date is not a version: 20250805 must not outrank the 5 in `claude-opus-4-5`.
    assert!(newer("claude-opus-4-5-20251101", "claude-opus-4-1-20250805"));
}

#[test]
fn a_catalogue_this_file_cannot_read_still_fills_the_picker() {
    let picked = pick(
        &["kepler-9", "kepler-9-mini", "kepler-8", "kepler-7"],
        tier_openai,
    );
    assert_eq!(picked, ["kepler-9", "kepler-9-mini", "kepler-8"]);
}

#[test]
fn a_family_this_file_only_half_recognises_is_still_topped_up_to_three() {
    // One tier matches, so the other two places go to the newest of what is left rather
    // than leaving the picker holding a single model.
    let picked = pick(
        &["gpt-5.4", "gpt-5.4-codex", "gpt-5.2-pro", "gpt-4o"],
        tier_openai,
    );
    assert_eq!(picked.len(), LIMIT);
    assert_eq!(picked[0], "gpt-5.4");
}
