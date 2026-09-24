# Settings

A strip of categories, one panel each: Portfolio, Accounts, Attributes, Market data, AI assistant,
Appearance, Keyboard, Profiles, Data and storage, Updates.

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
off, or **Needs a key** — and **Your own sources** holds price addresses the user described. How
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

**Plugins** — what the app has been extended with, and what each one brings. A plugin is installed
from a folder on this machine; there is no catalogue to browse and nothing is downloaded. Three
kinds of content are honoured: colour themes, which then appear under Appearance; broker import
layouts, which appear in the import wizard's layout list; and readers for file formats the app
cannot open by itself, which are used automatically when such a file is imported.

A plugin never changes what a figure means — how a return, a cost basis or a position is computed
is fixed in the app. A package built for another version of the app is listed with that as the
reason and is not used, rather than half-loaded. Removing a plugin takes everything it brought with
it, which is why a layout that came from one cannot be deleted on its own.

A file reader in particular has no access to the internet, to the disk or to the clock, so it
cannot send the statement it is reading anywhere, and it must prove itself against the sample it
ships before it can be installed at all.

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
