# Corporate actions and instrument events

Two different things share this ground and should not be confused.

**A corporate action** — a split, a reverse split, a merger — changes the holding. It adjusts the
lots: ten shares at 100 become twenty at 50 after a 2-for-1, with the cost basis unchanged in total.
Prices need no adjustment, because the stored price series is already split-adjusted.

**An instrument event** is a record that something happened: a dividend the provider reported, a
split it reported, or a note the user wrote themselves. An event moves no money and no quantity. A
reported split is *offered* as a corporate action for the user to accept; it is never applied on its
own, because the app cannot know whether the broker already adjusted the position.

A dividend that the user actually received is a transaction on their deposit account and is part of
income. A dividend in the event history is what the instrument distributed, whether or not this
portfolio held it at the time. Both are useful; they answer different questions, and a payment
schedule read from events is about the instrument, not about the user's receipts.
