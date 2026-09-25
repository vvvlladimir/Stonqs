# Settings

A strip of categories, one panel each: Portfolio, Accounts, Attributes, Market data, AI assistant,
Appearance, Keyboard, Plugins, Profiles, Data and storage, Updates, About.

**Portfolio** — its name, the base currency every report converts into, and the **cost-basis
method** (how a sale decides which lots it consumed). The method is what determines every realised
result on the Reports and Trades screens, so changing it changes history's arithmetic, not just a
display.

**Inflation** sits under Portfolio and is what real returns are measured against. **Price-index
region** is where you spend, and it is deliberately not derived from the base currency: reporting
in dollars says nothing about which shops you walk into, and several countries share a currency.
Left empty, no real figures are shown anywhere — which is different from showing zero inflation.
Naming a region fetches that region's price history in the background; the panel says which month
the data reaches, and real figures stop there rather than running to today.

**Accounts** — which accounts belong to the portfolio. An account left outside is kept, with its
transactions, but appears in no report.

**Attributes** — facts no provider supplies, defined by the user: a name, a kind (text, number or
date) and an optional unit shown beside the value. Every instrument's form then offers them all,
and a text attribute can seed a whole classification tree from its values. A kind is fixed once the
attribute exists, because the values already stored were written as that kind.

**Market data** — whether quotes are fetched when the app starts, and how often at most. Below that
is the coverage table: per instrument, which venue, which provider, and the first and last quote
on file. This is the screen that answers "why does my chart stop moving in March" — because that is
where the data ends, not the market. A valuation on a day without a quote uses the last one before
it, never a later one.

**Data sources**, on the same panel, lists every source the app can ask and whether it does — on,
off, or **Needs a key** — with the address each one's requests go to. **No source is on until you
choose one.** Until then nothing is fetched at all: prices and exchange rates are only what was
typed in by hand. The rows on that panel are the picker itself: `Turn on` beside a source is the
whole of the answer, and from that press the source is asked. The panel shows an error while the
sources on cannot price a portfolio — one that publishes prices and one that publishes exchange
rates are both needed, and without the second anything held in another currency cannot be valued
at all. The error goes as soon as both are on. The same question is asked by a dialog on a new
profile and by the data chip beside the search box while nothing usable is chosen. In that dialog,
`Select all and continue` turns on every source that answers without a key of its own and applies
the choice in one press, `Add key` opens the field for the ones that need one, `Add source`
describes a price address of your own, and `Use these sources` applies whatever is switched on.

Two answers are needed before either applying button is allowed: something that publishes prices,
and something that publishes exchange rates. Without both, holdings in a currency other than the
portfolio's cannot be valued at all, so `Use these sources` stays disabled and says which of the
two is missing. `Decide later` still leaves without choosing — the portfolio is then priced by
hand, and the data chip beside the search box shows an error saying so until sources are chosen.

Turning a source on means its own terms apply to the requests the app makes to it. An instrument
added while no source was on has no price source and nothing fetches its prices; the dialog offers
those instruments the first source you turn on, as a single checkbox, and an instrument you gave a
source of its own keeps it. **Your own sources** holds price addresses the user described. How
sources are chosen and combined is the `data-sources` topic.

**AI assistant** — whether the panel exists at all, which provider new chats start on, whether the
assistant may use the provider's web search, and whether it shows a summary of its own reasoning.
There is no model to choose here. A new chat starts on the provider, model and thinking effort last
picked in a chat; before anything was picked, on the provider's smallest (cheapest) model. Choosing
a provider inside a chat also changes the provider shown here. Whether tools may run without asking
is never carried over: every new chat asks first.

Below that, **Provider keys** lists every provider with whether a key is saved for it. A key belongs
to the open profile and is kept only behind that profile's password: in a profile without one,
`Connect` first asks to `Set a password`. The key is stored encrypted with the password and is never
read back: the row can say a key is saved, and nothing more. `Connect` and `Replace` open a field for a new one; the old key is
overwritten, never shown. Connecting several providers is normal — an individual chat picks which
one answers it, and the choice above only decides where a fresh chat begins.

A chat's model list shows three models per provider: the newest of each size. The chip button on
a provider's row (`Add models to the picker`) adds more by hand, one model id per line, spelled
exactly as the provider spells it. They appear in the chat's model list after the provider's own,
immediately, including a model the provider's catalogue does not list. A misspelled id is not
checked here; the first message sent with it returns the provider's error.

**Your own provider** points the app at a server of your choosing: a gateway, another vendor, or a
model running on this machine. It needs an address ending at the version (for example
`https://openrouter.ai/api/v1`), the API format that server speaks, and the exact model id to ask
for. Start with the OpenAI-compatible chat-completions format — outside OpenAI itself that is what
"OpenAI-compatible" means. A key for it goes in the keys list under the name you gave it, and a
server running on this machine usually needs none. Once it is configured it appears in the provider
picker like any other. Web search and the reasoning summary are the built-in providers' features
and are simply not asked for here.

This panel also shows what has been spent so far, in tokens the provider counted, per model. The
app never prices them — that is the provider's bill.

**Appearance** — interface language and colour scheme, each of which can simply follow the
operating system.

**Keyboard** — `Single-key shortcuts` switches off every shortcut that is one plain key or a
sequence of plain keys (a letter to create an item, a letter pair to jump to a screen, a question
mark for the list). It is for people who press them by accident or dictate with speech input,
where a spoken word can arrive as those keys. Shortcuts held with ⌘ or Ctrl keep working either
way. `All keyboard shortcuts` opens the full list; that list is the app's own answer to "which key
does what" and changes with the platform, so it is not repeated here.

**Profiles** — independent sets of data on this device: each has its own portfolio, accounts,
transactions, quotes, settings (language and theme included), import layouts, dashboards, AI chats
and AI spending, and its own provider keys. Nothing is shared between two profiles. `New profile` creates an empty one without leaving the current profile;
`Switch` opens another and reloads the window, and a fresh profile starts at the setup screen, which
also offers the way back. When the device has more than one profile, the app asks `Choose a
profile` at launch; the one used last is marked. Only the open profile can be deleted — deleting one takes being in it — and a profile with a
password asks for it again first. The app then opens another profile. The last profile cannot be
deleted, and deleting a profile removes all of its data permanently.

A profile may have a **password**; it is optional until the first API key is saved, which requires
one. Setting it encrypts the profile's data on disk as well as its keys, and removing it decrypts
the data again. A profile with a password opens at a lock screen (`Unlock`), and nothing of it is shown until
the password is typed — after that the app does not ask again until it is closed or `Lock now` is
pressed, and the AI assistant never asks at all. `Remember on this device` lets this computer open
the profile without asking, which also lets anyone using it do so. `Change password` asks for the
current one first. `Remove password` deletes the profile's API keys with it. A forgotten password
cannot be recovered, and it takes the profile's data and keys with it — there is no reset. The
interface language, theme, dashboard layouts and the dashboard summary texts are stored unencrypted
beside the data, because the lock screen needs them before any password is typed. Setting or
removing a password can be refused while a quote refresh or an AI answer is running; it works once
that finishes.

**Updates** — which version is running, and whether the app may look for a newer one. With `Check
for updates automatically` on, it asks once a day whether a newer version has been published;
`Check for updates` asks straight away. Finding one changes nothing by itself: the app says what
the new version is and what the release notes say, and waits. `Update now` downloads it, `Later`
leaves it for the next check, and `Skip this version` means that exact version is never offered
again — a later one still is, and `Offer it again` takes the refusal back. An installed update
takes effect on the next start, which `Restart now` does immediately. A download that fails changes
nothing: the running version keeps working.

Updates are the one thing besides market data and the AI assistant that reaches the internet, and
the app only ever asks a release feed whether a version exists — it sends nothing about the
portfolio. A version that fails its signature check is refused, so an update can only come from
whoever holds the project's signing key.

**Data and storage** — what is stored (accounts, instruments, quotes, the first transaction),
saved import layouts (including restoring the shipped ones that were removed), and the database
itself. The whole portfolio is one SQLite file, one per profile: copying that file is the backup, and there is no
cloud copy of it anywhere.

**About** is what the macOS application menu's `About Stonqs` opens, and it is in the command
palette under the same name. It opens with **Learning the app**. `Start` runs the guided tour: a dozen short cards over
the real screens, saying what each one is for. It writes nothing, changes no setting, and can be
left at any point — the same tour is offered once in a new profile and is reachable afterwards from
the command palette and, on macOS, the Help menu. `Open it` beside **Demo portfolio** switches to a
separate profile filled with three years of made-up history, so anything tried there is nowhere near
real data; the other profiles are unaffected and switching back is the profile picker.

Under it is what the app is and what it does not promise: a tool for recording and analysing your
own investments, not financial advice; the assistant's answers are generated by AI and can be wrong;
market data comes from free third-party sources with no guarantee of accuracy, completeness or
availability — so anything that matters, especially anything going on a tax return, is worth
checking against the broker's own statements. The version running and the licence the app itself is
published under are named here too.

**Third-party software** is the software the app is built from. `Show list` opens it — every
component with its version and licence, thirty to a page with `Previous` and `Next`, sortable by
any column and searchable by name or by licence. It is not loaded until asked for, and a component's
name is a link that opens its own page in your browser. `Save to a file…` writes the
complete licence texts, which is what each of those components asks to travel with a copy of the
app; the list on screen names them, the saved file carries the wording.
