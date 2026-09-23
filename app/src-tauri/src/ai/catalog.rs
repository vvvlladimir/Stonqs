//! Which providers this build can talk to, and how to build one. The single place that maps a
//! stored provider id to an adapter — `session.rs` takes a `&dyn AiProvider` and never learns
//! which one it got, the same way it never learns which model it is answering with.
//!
//! A provider id is a stored value (`ai_chats.provider`, the keychain account name), so it is
//! never translated and never renamed: a chat written by an older build must still resolve.

use super::anthropic::AnthropicProvider;
use super::compat::CompatProvider;
use super::gemini::GeminiProvider;
use super::openai::OpenAiProvider;
use super::{AiError, AiProvider, AiResult};
use serde::{Deserialize, Serialize};

/// One entry per provider the app can reach without being configured. `default_model` is the
/// **fallback** a chat starts on when the provider's own catalogue cannot be read (no key yet,
/// no network): what a chat normally lands on is the smallest tier that catalogue offers today
/// (`models::smallest`), so a build older than a model release does not pin every new chat to a
/// retired id. The fallback is the small tier too, for the same reason.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Provider {
    pub id: &'static str,
    pub default_model: &'static str,
}

/// In the order the picker offers them.
pub const PROVIDERS: &[Provider] = &[
    Provider {
        id: "openai",
        default_model: "gpt-5.6-luna",
    },
    Provider {
        id: "anthropic",
        default_model: "claude-haiku-4-5",
    },
    Provider {
        id: "gemini",
        default_model: "gemini-3.1-flash-lite",
    },
];

/// The id of the one provider the user configures themselves. A stored value like any other id,
/// and the keychain account its key lives under.
pub const CUSTOM: &str = "custom";

/// Which shape of API a server speaks. Three exist in practice and nothing else is worth
/// supporting: an endpoint that is neither is not "custom", it is a different integration.
///
/// `OpenAiChat` is the default because it is what "OpenAI-compatible" means everywhere outside
/// OpenAI itself — OpenRouter, Groq, Together, DeepSeek, Mistral, vLLM, llama.cpp, LM Studio,
/// Ollama and every LiteLLM proxy answer `POST {base}/chat/completions`. `OpenAiResponses` is
/// OpenAI's own newer shape, which almost nobody else implements. `Anthropic` is `/v1/messages`
/// and `Gemini` is `:streamGenerateContent`, both offered by the gateways that front those two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Wire {
    // Spelled out rather than derived: `SCREAMING_SNAKE_CASE` would write `OPEN_AI_CHAT`, and
    // this value is stored in `settings.json` and read by the frontend.
    #[default]
    #[serde(rename = "OPENAI_CHAT")]
    OpenAiChat,
    #[serde(rename = "OPENAI_RESPONSES")]
    OpenAiResponses,
    #[serde(rename = "ANTHROPIC")]
    Anthropic,
    #[serde(rename = "GEMINI")]
    Gemini,
}

/// A server the user points the app at: a gateway, a proxy, another vendor, or a model running
/// on this machine. Stored in `settings.json`; the key is not — that lives in the keychain under
/// `CUSTOM`, exactly like the built-in providers'.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CustomProvider {
    /// What to call it in the picker. The user's own words, so it crosses IPC as a value.
    #[serde(default)]
    pub label: String,
    /// Everything up to and including the version segment — `https://openrouter.ai/api/v1`,
    /// `http://localhost:11434/v1`. The path of the call is appended, so a base with a trailing
    /// slash and one without behave the same.
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub wire: Wire,
    /// The model id to send. Typed rather than picked: a server behind a base URL need not offer
    /// a catalogue at all, and the one it does offer may not be the one the user is paying for.
    #[serde(default)]
    pub model: String,
}

impl CustomProvider {
    /// Configured enough to be offered at all. A base URL and a model are both required: without
    /// either there is nothing to send a request to, or nothing to ask for.
    pub fn is_set(&self) -> bool {
        !self.base_url.trim().is_empty() && !self.model.trim().is_empty()
    }

    /// The base URL with no trailing slash, so `{base}/chat/completions` is one join away.
    pub fn base(&self) -> &str {
        self.base_url.trim().trim_end_matches('/')
    }
}

/// The model a chat gets when it moves to this provider. An id this build does not know keeps
/// whatever model the caller already had: refusing here would strand a chat written by a build
/// that knew one more provider than this one.
pub fn default_model(id: &str) -> Option<&'static str> {
    PROVIDERS.iter().find(|p| p.id == id).map(|p| p.default_model)
}

/// Whether this build can answer on this provider at all. The custom one exists only once it is
/// configured, which is why this asks rather than reading a constant.
pub fn is_known(id: &str, custom: &CustomProvider) -> bool {
    default_model(id).is_some() || (id == CUSTOM && custom.is_set())
}

pub fn build(id: &str, key: String, custom: &CustomProvider) -> AiResult<Box<dyn AiProvider>> {
    match id {
        "openai" => Ok(Box::new(OpenAiProvider::new(key))),
        "anthropic" => Ok(Box::new(AnthropicProvider::new(key))),
        "gemini" => Ok(Box::new(GeminiProvider::new(key))),
        CUSTOM if custom.is_set() => Ok(match custom.wire {
            Wire::OpenAiChat => Box::new(CompatProvider::new(key, custom.base())) as Box<dyn AiProvider>,
            Wire::OpenAiResponses => Box::new(OpenAiProvider::at(key, custom.base())),
            Wire::Anthropic => Box::new(AnthropicProvider::at(key, custom.base())),
            Wire::Gemini => Box::new(GeminiProvider::at(key, custom.base())),
        }),
        CUSTOM => Err(AiError::Provider(
            "the custom provider has no address or model yet".into(),
        )),
        other => Err(AiError::Provider(format!("unknown provider {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configured() -> CustomProvider {
        CustomProvider {
            label: "Home".into(),
            base_url: "http://localhost:11434/v1/".into(),
            wire: Wire::OpenAiChat,
            model: "qwen3".into(),
        }
    }

    #[test]
    fn every_listed_provider_can_actually_be_built() {
        for provider in PROVIDERS {
            assert!(
                build(provider.id, "sk-test".into(), &CustomProvider::default()).is_ok(),
                "{} is offered but has no adapter",
                provider.id
            );
            assert_eq!(default_model(provider.id), Some(provider.default_model));
        }
    }

    #[test]
    fn a_provider_this_build_does_not_know_is_an_error_not_a_guess() {
        let none = CustomProvider::default();
        assert!(build("mistral", "key".into(), &none).is_err());
        assert_eq!(default_model("mistral"), None);
        assert!(!is_known("mistral", &none));
    }

    #[test]
    fn the_custom_provider_exists_only_once_it_is_configured() {
        let none = CustomProvider::default();
        assert!(!is_known(CUSTOM, &none));
        assert!(build(CUSTOM, "key".into(), &none).is_err());

        let set = configured();
        assert!(is_known(CUSTOM, &set));
        assert!(build(CUSTOM, "key".into(), &set).is_ok());
        // A base URL is joined to a path, so the slash the user pasted is not kept.
        assert_eq!(set.base(), "http://localhost:11434/v1");
    }

    #[test]
    fn each_wire_builds_its_own_adapter() {
        for wire in [
            Wire::OpenAiChat,
            Wire::OpenAiResponses,
            Wire::Anthropic,
            Wire::Gemini,
        ] {
            let custom = CustomProvider { wire, ..configured() };
            assert!(
                build(CUSTOM, "key".into(), &custom).is_ok(),
                "{wire:?} has no adapter"
            );
        }
    }
}
