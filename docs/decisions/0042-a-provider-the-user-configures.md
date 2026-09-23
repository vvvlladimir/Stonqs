# 42: A provider the user configures

- Status: Accepted; the model a new chat starts on superseded by ADR-0069

## Context

The assistant shipped with two adapters, each pinned to its vendor's host: OpenAI's Responses API
and Anthropic's Messages API (ADR-0037). Everything else a user might already pay for — a gateway
in front of several models (OpenRouter, LiteLLM, Vercel AI Gateway), another vendor (DeepSeek,
Mistral, Groq, xAI), or a model running on their own machine (Ollama, LM Studio, llama.cpp, vLLM)
— was unreachable, although almost all of them answer an API this app already knows how to speak.

Three shapes cover that field. `POST {base}/chat/completions` is what "OpenAI-compatible" means
everywhere outside OpenAI itself, and is implemented by every gateway, proxy and local runner.
`POST {base}/messages` is Anthropic's and `:streamGenerateContent` is Google's, both offered by the
gateways that front those models. OpenAI's own newer Responses shape is the fourth, and is almost
exclusively OpenAI's.

Also: `AppSettings::ai_model` held the id a new chat started on. It was written into the build as a
default, and a model id has a shorter life than a release — the shipped default outlived the model
it named and every new chat began on something the provider had retired.

## Decision

One extra provider, id `custom`, configured by the user: a label, a base URL, the wire it speaks,
and the model id to ask for. It exists as a provider only once it has an address and a model, and
its key is optional — a server on the user's own machine authenticates nothing, and demanding a key
would be demanding one be invented. The key, when there is one, lives in the keychain under
`custom` like every other provider's.

`ai/compat.rs` implements the chat-completions wire, which is the default: assembled from deltas
because that wire hands over no finished turn, with `stream_options.include_usage` asked for so a
turn's cost is counted rather than guessed. The other two wires reuse the existing adapters with
their base URL replaced — `OpenAiProvider::at`, `AnthropicProvider::at`.

`AppSettings::ai_model` is removed. A chat lands on the first model its provider lists today
(`commands::ai::newest_model`), and the compiled-in `Provider::default_model` answers only when
that list cannot be read. The custom provider's model is the one the user typed: it comes first in
its list, and whatever `GET {base}/models` answers is offered underneath.

## Alternatives

- **Ship a table of vendors** (DeepSeek, Groq, …). Every entry is a URL that changes without us,
  and the list is never complete; one configurable entry covers all of them and the next one too.
- **Only override the base URL of the existing adapters.** Cheaper, but the Responses API is the
  shape nobody else implements: the setting would exist and almost never work.
- **Fetch the model list and make the user pick, as for the built-in providers.** A server behind a
  base URL need not offer a catalogue, and the one it offers may not be what that address is
  serving. The typed id is the fact; the catalogue is a convenience beside it.

## Consequences

- A third adapter to keep working, and it is the loosest of the three: what a compatible server
  supports varies, so the adapter asks for little (no hosted web search, no reasoning summary
  beyond the `reasoning_content` delta the field has settled on) and tolerates absence.
- `settings.json` gains `ai_custom` and loses `ai_model`; an older file simply has its `ai_model`
  ignored, which is the behaviour we want — the id in it was stale by definition.
- A chat's provider may stop being configured. The chat keeps saying which provider it is on, the
  picker shows it is no longer available, and the failure arrives at the next send rather than
  silently answering from somewhere else.
