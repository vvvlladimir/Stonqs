# Settings

A strip of categories, one panel each: Portfolio, Accounts, Attributes, Market data, AI assistant,
Appearance, Profiles, Data and storage.

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
There is no model to choose here: a chat starts on whatever its provider offers today, and the chat
itself can switch to one of the others that provider serves.

Below that, **Provider keys** lists every provider with whether a key is saved for it. A key belongs
to the open profile and is kept only behind that profile's password: in a profile without one,
`Connect` first asks to `Set a password`. The key is stored encrypted with the password and is never
read back: the row can say a key is saved, and nothing more. `Connect` and `Replace` open a field for a new one; the old key is
overwritten, never shown. Connecting several providers is normal — an individual chat picks which
one answers it, and the choice above only decides where a fresh chat begins.

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

**Data and storage** — what is stored (accounts, instruments, quotes, the first transaction),
saved import layouts (including restoring the shipped ones that were removed), and the database
itself. The whole portfolio is one SQLite file, one per profile: copying that file is the backup, and there is no
cloud copy of it anywhere.
