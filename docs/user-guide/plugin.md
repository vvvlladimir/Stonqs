# A plugin's screen

A screen listed under **Plugins** in the navigation is not part of the app: an installed plugin
brought it, and its header says which one ("From the … plugin"). The app draws the header — the
screen's name, the plugin's name and, when the screen reads over a period, the period control —
and everything below it is the plugin's own page. The app cannot say what that page shows or how it
computes anything: for that, the plugin's own documentation is the source. What the app can say is
what the page was given and what it can do.

**It is given only what it declared.** A plugin lists, when it is installed, what its screen reads:
value and results, positions, return and flows over the period, or the operations of the period.
Settings → Plugins shows that list. The screen receives exactly those, and nothing else is
reachable from it — not other data, not the internet, not the files on this machine.

**It follows the app's lenses.** Like every built-in screen, it reads the accounts the account
picker has in view and the date the app is set to. Operations are read over the screen's period.
Moving the picker or the period redraws the page with the new data.

**Its figures are the plugin's.** A plugin may compute numbers of its own from what it was given —
totals by category, shares, scores. The app does not check them, and they can differ from a figure
the app shows under a similar name, for example because the plugin counts only some kinds of
operation. Nothing the plugin computes changes any figure elsewhere in the app, and nothing it
shows is used by reports, exports or the assistant's other answers.

**It may keep one document of its own.** A plugin screen that says so can store its own settings
in the open profile — the rules a spending screen sorts withdrawals by, for instance. That document
is the plugin's, is encrypted with the profile when the profile has a password, and cannot change
an operation, an account or a figure. Removing the plugin leaves the document in place, so
installing it again finds its settings where they were.

**When it fails.** A page that stops with an error, or never starts, says so in place of its
content, in the plugin's own words; the rest of the app is unaffected. A removed plugin's screen
disappears from the navigation.
