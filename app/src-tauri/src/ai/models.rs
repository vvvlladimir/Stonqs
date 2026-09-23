//! What models a provider offers. The list is the provider's to know, not this codebase's: which
//! models exist changes faster than the app ships, so the ids come from the provider's own
//! catalogue and never from a table compiled in here.
//!
//! What *is* decided here is how many of them are offered. A provider's catalogue runs to dozens
//! of entries — dated snapshots, transcription, embeddings, code variants — and a picker holding
//! all of them is a wall to scroll rather than a choice. Three are kept per provider: the
//! flagship, the middle one and the small one, newest of each, which is the shape every
//! catalogue already has (`gpt-…-sol / -terra / -luna`, `opus / sonnet / haiku`, `pro / flash /
//! flash-lite`).
//!
//! The shortlist is built from what the provider answered, not from a whitelist: a model released
//! after this build is picked up as soon as it is the newest of its tier. A naming this file has
//! never seen produces no tier at all, and the first three of the ranked catalogue are offered
//! instead — never an empty picker.

use super::catalog::{CUSTOM, CustomProvider, Wire};
use super::{AiError, AiResult};
use std::collections::BTreeMap;

/// How many models a picker is given. Three tiers, one model each.
const LIMIT: usize = 3;

pub fn list(provider: &str, key: &str, custom: &CustomProvider) -> AiResult<Vec<String>> {
    match provider {
        "openai" => openai(key),
        "anthropic" => anthropic(key),
        "gemini" => gemini(key),
        CUSTOM => Ok(configured(key, custom)),
        other => Err(AiError::Provider(format!("no model list for provider {other}"))),
    }
}

/// The custom provider's list always begins with the model the user typed: a server behind a
/// base URL need not offer a catalogue at all, and the one it offers may not be the one that
/// address is actually serving. `GET {base}/models` is part of the same compatibility contract
/// as the call itself, so what it answers is offered underneath — and a server that answers
/// nothing simply leaves the picker holding the configured model, which is the truth.
fn configured(key: &str, custom: &CustomProvider) -> Vec<String> {
    let typed = custom.model.trim().to_string();
    let mut listed = vec![typed.clone()];
    let fetched = catalogue(key, custom).unwrap_or_default();
    let tier: fn(&str) -> Option<u8> = match custom.wire {
        Wire::Anthropic => tier_anthropic,
        Wire::Gemini => tier_gemini,
        _ => tier_openai,
    };
    for id in shortlist(fetched, tier) {
        if listed.len() > LIMIT {
            break;
        }
        if id != typed {
            listed.push(id);
        }
    }
    listed
}

fn catalogue(key: &str, custom: &CustomProvider) -> AiResult<Vec<String>> {
    let url = format!("{}/models", custom.base());
    // Both header spellings, because the request is one or the other's and a server ignores the
    // name it does not know: this is a list of models, not a secret being handed somewhere new.
    let body = fetch(
        ureq::get(&url)
            .header("Authorization", &format!("Bearer {key}"))
            .header("x-api-key", key)
            .header("x-goog-api-key", key)
            .header("anthropic-version", "2023-06-01"),
    )?;
    Ok(ranked(&body, rank))
}

fn anthropic(key: &str) -> AiResult<Vec<String>> {
    let body = fetch(
        ureq::get("https://api.anthropic.com/v1/models?limit=1000")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01"),
    )?;
    Ok(shortlist(ranked(&body, rank_anthropic), tier_anthropic))
}

/// Lower sorts first. Everything Anthropic serves is a chat model; a family this table has never
/// heard of still ranks above nothing, because there is nothing to rank below.
fn rank_anthropic(id: &str) -> u8 {
    if id.starts_with("claude-") { 0 } else { 1 }
}

/// Google answers `{ "models": [{ "name": "models/gemini-…" }] }` — a different envelope and an
/// id carrying a collection prefix, both handled by the one reader below.
fn gemini(key: &str) -> AiResult<Vec<String>> {
    let body = fetch(
        ureq::get("https://generativelanguage.googleapis.com/v1beta/models?pageSize=1000")
            .header("x-goog-api-key", key),
    )?;
    Ok(shortlist(ranked(&body, rank_gemini), tier_gemini))
}

/// Lower sorts first. Google's catalogue holds embeddings, image and video models beside the
/// chat ones; they are still listed, just last.
fn rank_gemini(id: &str) -> u8 {
    const NOT_CHAT: &[&str] = &["embedding", "imagen", "veo", "aqa", "tts", "image", "learnlm"];
    if NOT_CHAT.iter().any(|marker| id.contains(marker)) {
        return 2;
    }
    if id.starts_with("gemini-") { 0 } else { 1 }
}

/// Pro, Flash, Flash-Lite — the three tiers Google sells. `-lite` is checked by matching the
/// whole suffix, because it also contains `flash`; anything else after the version (`-preview`,
/// `-thinking`, `-latest`) is a variant for a different job and is not offered here.
fn tier_gemini(id: &str) -> Option<u8> {
    let rest = dated(id).unwrap_or(id).strip_prefix("gemini-")?;
    if !rest.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    match rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.') {
        "-pro" => Some(0),
        "-flash" => Some(1),
        "-flash-lite" => Some(2),
        _ => None,
    }
}

fn openai(key: &str) -> AiResult<Vec<String>> {
    let body = fetch(
        ureq::get("https://api.openai.com/v1/models").header("Authorization", &format!("Bearer {key}")),
    )?;
    Ok(shortlist(ranked(&body, rank), tier_openai))
}

/// The bodies differ by vendor, and `ranked` knows all three shapes.
fn fetch(request: ureq::RequestBuilder<ureq::typestate::WithoutBody>) -> AiResult<serde_json::Value> {
    let mut response = request.call().map_err(|e| match e {
        ureq::Error::StatusCode(401) | ureq::Error::StatusCode(403) => {
            AiError::Auth("the provider rejected this key".into())
        }
        other => AiError::Network(other.to_string()),
    })?;

    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| AiError::Network(e.to_string()))?;
    serde_json::from_str(&text).map_err(|e| AiError::Provider(e.to_string()))
}

/// OpenAI and Anthropic answer `{ "data": [{ "id": … }] }`; Google answers
/// `{ "models": [{ "name": "models/…" }] }`. One reader takes either, so a custom server is read
/// by whichever of the two it happens to speak rather than by what it was configured as.
fn ranked(body: &serde_json::Value, rank: fn(&str) -> u8) -> Vec<String> {
    let listed = body["data"].as_array().or_else(|| body["models"].as_array());
    let mut ids: Vec<String> = listed
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|model| model["id"].as_str().or_else(|| model["name"].as_str()))
        // `models/gemini-3.1-pro` is the resource; the id to send is what follows the collection.
        .map(|id| id.rsplit('/').next().unwrap_or(id).to_string())
        .collect();
    ids.sort_by_key(|id| (rank(id), id.clone()));
    ids
}

/// How this provider's ids map to tiers; `None` for the custom provider, whose list begins with
/// the model the user typed rather than with a flagship.
fn tiers(provider: &str) -> Option<fn(&str) -> Option<u8>> {
    match provider {
        "openai" => Some(tier_openai),
        "anthropic" => Some(tier_anthropic),
        "gemini" => Some(tier_gemini),
        _ => None,
    }
}

/// The smallest tier on offer, which is where a chat starts until the user picks otherwise. A
/// list that names no tier (or the custom provider's) starts on its first entry.
pub fn smallest(provider: &str, listed: &[String]) -> Option<String> {
    tiers(provider)
        .and_then(|tier| {
            listed
                .iter()
                .filter_map(|id| tier(id).map(|slot| (slot, id)))
                .max_by_key(|(slot, _)| *slot)
                .map(|(_, id)| id.clone())
        })
        .or_else(|| listed.first().cloned())
}

/// A remembered choice as the list stands today: the id itself while it is still offered,
/// otherwise the newest model of the same tier — a release must not quietly move the user from
/// the small model they chose to the flagship, or back.
pub fn remembered(provider: &str, chosen: &str, listed: &[String]) -> Option<String> {
    if listed.iter().any(|id| id == chosen) {
        return Some(chosen.to_string());
    }
    let tier = tiers(provider)?;
    let slot = tier(chosen)?;
    listed.iter().find(|id| tier(id) == Some(slot)).cloned()
}

/// The newest id of each tier, in tier order. `tier` returns `None` for everything that is not a
/// plain chat model of a known family — a dated snapshot still counts, it simply loses to the
/// undated id of the same version.
fn shortlist(ids: Vec<String>, tier: fn(&str) -> Option<u8>) -> Vec<String> {
    let mut best: BTreeMap<u8, String> = BTreeMap::new();
    for id in &ids {
        let Some(slot) = tier(id) else { continue };
        match best.get(&slot) {
            Some(current) if !newer(id, current) => {}
            _ => {
                best.insert(slot, id.clone());
            }
        }
    }
    let mut picked: Vec<String> = best.into_values().collect();
    // A catalogue that names its families some other way would leave the picker with one entry
    // or none. The ranked list tops it up, newest first, so three are always offered as long as
    // the provider answered with anything at all.
    if picked.len() < LIMIT {
        let mut rest: Vec<&String> = ids.iter().filter(|id| !picked.contains(id)).collect();
        // Newest first, then the plain id over a variant of it: a shorter name is the family
        // itself, a longer one is that family doing some other job.
        rest.sort_by(|a, b| {
            (version(b), dateless(b))
                .cmp(&(version(a), dateless(a)))
                .then_with(|| (a.len(), a.as_str()).cmp(&(b.len(), b.as_str())))
        });
        for id in rest {
            if picked.len() == LIMIT {
                break;
            }
            picked.push(id.clone());
        }
    }
    picked
}

/// Version first, then the undated id: `gpt-5.4` beats `gpt-5`, and `gpt-5` beats
/// `gpt-5-2025-08-07`, which is the same model pinned to a day.
fn newer(id: &str, than: &str) -> bool {
    (version(id), dateless(id)) > (version(than), dateless(than))
}

/// The numbers in an id, with a trailing release date removed first: `claude-opus-4-1-20250805`
/// is version 4.1, and comparing 20250805 against a minor number would rank it above 4.5.
fn version(id: &str) -> Vec<u64> {
    let stem = dated(id).unwrap_or(id);
    let mut numbers = Vec::new();
    let mut digits = String::new();
    for ch in stem.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else if !digits.is_empty() {
            numbers.push(digits.parse().unwrap_or(0));
            digits.clear();
        }
    }
    if !digits.is_empty() {
        numbers.push(digits.parse().unwrap_or(0));
    }
    numbers
}

fn dateless(id: &str) -> bool {
    dated(id).is_none()
}

/// The id without its trailing release date, in either spelling providers use
/// (`-2025-08-07`, `-20250805`), or `None` when it carries no date.
fn dated(id: &str) -> Option<&str> {
    let (stem, tail) = id.rsplit_once('-')?;
    if tail.len() == 8 && tail.chars().all(|c| c.is_ascii_digit()) {
        return Some(stem);
    }
    // `-YYYY-MM-DD`: the day and the month are split off one at a time.
    let (stem, month) = stem.rsplit_once('-')?;
    let (stem, year) = stem.rsplit_once('-')?;
    let dashed = year.len() == 4
        && month.len() == 2
        && tail.len() == 2
        && [year, month, tail]
            .iter()
            .all(|p| p.chars().all(|c| c.is_ascii_digit()));
    dashed.then_some(stem)
}

/// Flagship, middle, small — the three OpenAI sells as one family, under two namings: the plain
/// `gpt-5.4 / -mini / -nano`, and since GPT-5.6 a named tier per generation (`gpt-6-astra`,
/// `gpt-5.6-sol / -terra / -luna`). Both land in the same three slots, so the newest generation
/// wins a slot whichever way it spells it. Anything carrying another suffix (`-codex`, `-pro`,
/// `-chat-latest`, `-search-api`) is a variant for a different job and is not offered here; the
/// o-series is not a tier of this family either.
fn tier_openai(id: &str) -> Option<u8> {
    let rest = dated(id).unwrap_or(id).strip_prefix("gpt-")?;
    if !rest.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    match rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.') {
        "" | "-astra" | "-sol" => Some(0),
        "-mini" | "-terra" => Some(1),
        "-nano" | "-luna" => Some(2),
        _ => None,
    }
}

/// Opus, Sonnet, Haiku — named in the id whichever way round the family is spelled
/// (`claude-opus-4-1`, `claude-3-5-sonnet-latest`).
fn tier_anthropic(id: &str) -> Option<u8> {
    let rest = id.strip_prefix("claude-")?;
    if rest.contains("opus") {
        Some(0)
    } else if rest.contains("sonnet") {
        Some(1)
    } else if rest.contains("haiku") {
        Some(2)
    } else {
        None
    }
}

/// Lower sorts first. The chat-shaped families, then everything else, then what is definitely
/// not a chat model — this only orders the fallback, since the shortlist above picks by tier.
fn rank(id: &str) -> u8 {
    const NOT_CHAT: &[&str] = &[
        "embedding",
        "moderation",
        "tts",
        "whisper",
        "dall-e",
        "audio",
        "realtime",
        "image",
    ];
    if NOT_CHAT.iter().any(|marker| id.contains(marker)) {
        return 2;
    }
    if id.starts_with("gpt-") || id.starts_with("o1") || id.starts_with("o3") || id.starts_with("o4") {
        return 0;
    }
    1
}

#[cfg(test)]
mod tests {
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
}
