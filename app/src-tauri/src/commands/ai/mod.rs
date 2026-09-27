//! AI panel commands: parse arguments, call the vault or `ai::*`, map the error. No command
//! returns a saved key (`ai::keys`).

mod chats;
mod providers;
mod stream;
mod summary;
mod turn;

pub use chats::*;
pub use providers::*;
pub use stream::*;
pub use summary::*;
pub use turn::*;
