# 16: `AppState::scoped_portfolio` takes `&Store` as an argument


- Status: Accepted

## Context

`std::sync::Mutex` is not reentrant: a thread that locks it twice deadlocks
itself on the second call.

## Decision

`AppState::scoped_portfolio(&self, store: &Store)` and
`AppState::scope_selection` take the store as a parameter instead of calling
`self.store()` internally.

### The bug this avoided

An earlier version locked the store itself inside `scoped_portfolio`. Every
command that calls it already holds the store lock for its own core call, so
the internal lock deadlocked the calling thread — on every scope other than
"whole portfolio", and starting from app launch, because the chosen scope
survives a restart. The signature makes the rule visible at the call site;
while the lock was taken inside the method body, it wasn't visible without
reading the implementation.
