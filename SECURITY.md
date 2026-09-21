# Security

## Reporting a vulnerability

Please do **not** open a public issue. Use GitHub's
[private vulnerability reporting](https://github.com/vvvlladimir/stonqs/security/advisories/new)
on this repository. You will get an acknowledgement within a few days; this is a spare-time
project, so please allow reasonable time for a fix before disclosing.

In scope: anything that exposes a user's portfolio or provider keys to another user, another
process, or the network; anything that lets a file a user imports execute code or exfiltrate data;
anything that weakens the profile encryption below what is described here.

Out of scope: an attacker who already has administrator rights on the machine, or who is running
code as the user; physical access with the profile unlocked; the absence of a feature listed under
"What it does not protect against" below.

## What the app does

Stonqs runs entirely on the user's machine. There is no account, no server, no telemetry, and the
portfolio is a local SQLite database. It reaches the network only to fetch prices, exchange rates
and inflation figures from public sources, to look up instruments, and — only if the user sets it
up with their own key — to talk to an AI provider of their choice.

## The profile password

A profile is a folder. A profile **with a password** stores two things differently from one
without:

- **The database** is encrypted with SQLCipher under a random 256-bit key. That key is generated
  once and never changes; changing the password does not re-encrypt the database.
- **Provider keys** (AI providers, keyed market-data sources) live in a small sealed file beside
  the database.

The password never encrypts anything directly. Argon2id (64 MiB, three passes, one lane — above
the OWASP floor) stretches it into a key-encryption key, which seals a random data key, which in
turn seals the database key and the provider keys with XChaCha20-Poly1305. Changing the password
re-seals 32 bytes and nothing else. The minimum length is 8 characters and there are no
composition rules, following NIST SP 800-63B.

"Remember on this device" stores the *data key* in the operating system's keychain — never the
password. Removing the profile's password decrypts the database back to a plain file.

A locked profile's database is not open at all: no query runs, and background work that needs the
database is refused rather than queued.

## What it protects against

- **A stolen or lost machine, powered off.** Without the password, an encrypted profile's database
  is ciphertext, and so are the provider keys.
- **Someone copying the profile folder** — from a backup, a synced folder, an external disk. The
  files are useless without the password.
- **Casual inspection** of a shared computer by someone who cannot log in as the user.
- **A provider key leaking through the interface.** No command reads a key back out; the app can
  only report whether one is present.

## What it does not protect against

Stated plainly, because a security section that implies more than it delivers is worse than none:

- **Malware or another program running as the user.** While the profile is unlocked, the keys and
  the database key are in this app's memory, and the operating system does not isolate one of the
  user's programs from another.
- **A running, unlocked session.** There is no idle auto-lock yet. Anyone at the keyboard sees
  everything.
- **A profile without a password.** Its database is an ordinary SQLite file, readable by anything
  that can read the user's files. Such a profile cannot store a provider key at all — a key is
  only ever saved behind a password.
- **Backups the app makes.** Before a schema upgrade the database is copied beside itself. The
  copy of an encrypted database is encrypted; the copy of a plain one is plain.
- **A weak password.** Argon2id makes guessing expensive, not impossible.
- **Anything the user sends to an AI provider.** When the assistant is used, the portions of the
  portfolio it reads are sent to the provider the user chose, under the user's own key and their
  privacy policy. Each read asks for permission, and every change to the data asks every single
  time. Nothing is sent when the assistant is not used.
- **Traffic analysis.** Requests to price sources reveal which instruments are being tracked to
  anyone who can see the connection, though the connections themselves are HTTPS.
- **The data sources themselves.** Prices and rates come from third-party public endpoints with no
  guarantee of accuracy or availability.

## Imported files and the assistant

A broker export is data, not instruction. Imported values are never executed, and the import
preview shows every row before anything is written. Text the assistant produces is rendered as
Markdown with raw HTML deliberately disabled, and results handed back to the model are fenced and
labelled as data — the model is told, in its system prompt, that content inside that fence came
out of somebody else's file and is never an instruction.

## Releases

Until code signing is in place, macOS and Windows will warn that the application comes from an
unidentified developer. The README says how to proceed and why. Build from source if you would
rather not trust a binary — that is the point of the code being here.
