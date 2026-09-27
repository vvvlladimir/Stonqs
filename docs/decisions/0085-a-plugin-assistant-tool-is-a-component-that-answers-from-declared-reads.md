# 85: A plugin's assistant tool is a component that answers from its declared reads

- Status: Accepted; the two copies of the projection superseded by ADR-0088

## Context

The assistant answers from a compiled-in catalogue (`ai/tools/`, ADR-0037): one entry per
question, a strict schema, an `Access` that decides whether the user is asked, and a body that
reads the portfolio through the lens snapshotted at `ai_send`. ADR-0070 lists "assistant tool" as
a kind of compute plugin, and ADR-0082 settled that a plugin may state figures of its own under
its own name. What is missing is how a stranger's answer reaches a model that talks to the user
about their money.

Three facts constrain it. The turn runs on a host thread with its own `Store`, never in the
frontend, so the data a widget is handed (`lib/pluginBridge.ts`) has to be built in Rust too. The
catalogue is sent in OpenAI's strict mode, and a provider refuses the **whole request** over one
schema it does not accept — a careless plugin would break every chat, not only its own tool. And
the model reads everything a tool returns as the app's word unless it is told otherwise.

## Decision

- **The body is a WASM component** in the sandbox the reader and the writer share: world
  `stonqs:tool@1.0.0`, `call(args, data) -> result<string, string>`, JSON in and out. No network,
  no filesystem, a frozen clock; an answer over 64 KiB or not JSON is a failure.
- **Data is the bridge's projection, built by the host.** The manifest declares `reads` from the
  same closed list as a page (`valuation`, `positions`, `performance`, `transactions`), answered
  under the chat's snapshotted lens, with the bridge's field names. Two copies of one shape exist —
  TypeScript for frames, Rust for tools — and a test reads the TypeScript interfaces back and
  fails when the Rust keys differ, the way `guide::SCREEN_IDS` is pinned to `nav.tsx`.
- **The model never writes a date.** A tool reading over a period gets the shipped `period`
  argument added to its schema by the host and resolved through the same axis as every other tool
  (ADR-0018); `reason` is added as for every asked-about tool. A plugin's schema may name neither.
- **The schema is checked at install** against the strict subset: an object, `additionalProperties:
  false`, every property required, every property a plain value. A package outside it installs
  nothing — the failure would otherwise land on every conversation.
- **Always `Access::Ask`.** Never `Free`, because a stranger's code runs on the user's data;
  never `Write`, because a tool has no channel to change anything. A `session` grant and a chat's
  `AUTO` mode cover it as they cover any read.
- **The answer is labelled as the plugin's.** The model is offered the tool as
  `plugin_<plugin>_<tool>`, with a description that begins "From the … plugin, not the app", the
  result comes back as `{ plugin, tool, answer }`, and the system prompt says a `plugin_` tool's
  figures are never to be presented as the app's (ADR-0082). The consent card names the plugin and
  the tool from values, like every card.
- **It proves itself before it installs**, like a reader (ADR-0086): the package ships a sample
  call (`args` + `data`) and the answer it must give, compared as JSON.
- Tools are loaded for each message, so one installed mid-chat answers from the next message.

## Alternatives

- **A JavaScript tool in a frame, fed by the frontend.** One projection instead of two, and the
  turn would need a round trip through the window on every call — a turn that dies when the window
  reloads, for a body that draws nothing.
- **Let the tool ask for data by calling the app's own tools.** A second permission path inside a
  permission path, and a plugin able to reach reads it never declared.
- **Trust the schema and let the provider reject it.** The rejection is of the whole request, so
  one package would silently disable the assistant.
- **`Free` for a tool that declares no reads.** It still runs a stranger's code in the middle of a
  conversation about money; being asked once per chat is the cheaper mistake.

## Consequences

- `ai::session::Session` carries the loaded tools, and `run_one` reaches either kind through one
  `Entry` whose `access` is branched on in exactly one place — the invariant that `Write` is always
  asked still has a single home.
- The bridge's field names are now a contract in two languages; changing one is a new `api`.
- A plugin tool cannot be part of the dashboard brief: its readings are a fixed list (ADR-0039).
