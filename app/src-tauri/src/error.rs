//! UI-boundary errors; core errors cannot implement `Serialize` here.

use serde::Serialize;
use sq_core::Error;

/// Structured error payload returned to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum UiError {
    Storage {
        message: String,
    },
    Network {
        message: String,
    },
    NotFound {
        message: String,
    },
    Invalid {
        message: String,
    },
    MissingMarketData {
        kind: String,
        key: String,
        date: String,
        message: String,
    },
    Math {
        message: String,
    },
    /// A provider rejected the saved key (HTTP 401) — the user's fix is to replace the key, not
    /// to retry.
    Auth {
        message: String,
    },
    /// A provider is throttling (HTTP 429); `retry_after` is the header value in seconds, when
    /// the provider sent one.
    RateLimit {
        message: String,
        retry_after: Option<u64>,
    },
    /// A provider answered with an error of its own — a model it does not serve, a request it
    /// rejected. `message` is the provider's own line.
    Provider {
        message: String,
    },
    /// The model declined to answer.
    Refused {
        message: String,
    },
    /// The answer reached the model's output limit and ends mid-way.
    Truncated {
        message: String,
    },
    /// The open profile has a password and has not been unlocked in this run: nothing is read
    /// until it is.
    Locked {
        message: String,
    },
    /// A key is kept only behind a profile password, and this profile has none yet.
    PasswordRequired {
        message: String,
    },
    /// The password given does not open the profile.
    WrongPassword {
        message: String,
    },
    /// Something else holds what this needs — a refresh or an AI answer using the database while
    /// it is being converted. Trying again once it finishes works.
    Busy {
        message: String,
    },
    /// Host failure, such as an unavailable data directory or poisoned mutex.
    Internal {
        message: String,
    },
}

impl UiError {
    pub fn internal(message: impl Into<String>) -> Self {
        UiError::Internal {
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        UiError::NotFound {
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        UiError::Invalid {
            message: message.into(),
        }
    }
}

impl From<Error> for UiError {
    fn from(e: Error) -> Self {
        let message = e.to_string();
        match e {
            Error::Storage(_) => UiError::Storage { message },
            Error::Network(_) => UiError::Network { message },
            Error::Unavailable(_) | Error::RateLimited(_) | Error::Unauthorized(_) => {
                UiError::Network { message }
            }
            Error::BadProviderData { .. } => UiError::Network { message },
            Error::NotFound(_) => UiError::NotFound { message },
            Error::Invalid(_) => UiError::Invalid { message },
            Error::MissingMarketData { kind, key, date } => UiError::MissingMarketData {
                kind: kind.to_string(),
                key,
                date: date.to_string(),
                message,
            },
            Error::Math(_) => UiError::Math { message },
            Error::Backup(_) => UiError::Storage { message },
        }
    }
}

impl From<tauri::Error> for UiError {
    fn from(e: tauri::Error) -> Self {
        UiError::internal(e.to_string())
    }
}

impl From<crate::ai::AiError> for UiError {
    fn from(e: crate::ai::AiError) -> Self {
        use crate::ai::AiError;
        match e {
            AiError::Auth(message) => UiError::Auth { message },
            AiError::RateLimit { message, retry_after } => UiError::RateLimit { message, retry_after },
            AiError::Network(message) => UiError::Network { message },
            // Not `Network`: the provider was reached and said no, and retrying the same request
            // gets the same answer. What it said is the useful part.
            AiError::Provider(message) => UiError::Provider { message },
            AiError::Refused => UiError::Refused {
                message: AiError::Refused.to_string(),
            },
            AiError::Truncated => UiError::Truncated {
                message: AiError::Truncated.to_string(),
            },
            AiError::Storage(message) => UiError::Internal { message },
            // Only ever reached when a tool fails outside the loop that hands it to the model.
            AiError::Tool(message) => UiError::Invalid { message },
        }
    }
}

/// A keychain that has no entry is the "connect your key" case, not a broken machine — the two
/// lead to different sentences, so they must not share a code.
impl From<keyring::Error> for UiError {
    fn from(e: keyring::Error) -> Self {
        match e {
            keyring::Error::NoEntry => UiError::Auth {
                message: "no key is saved for this provider".into(),
            },
            other => UiError::internal(other.to_string()),
        }
    }
}

impl std::fmt::Display for UiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            UiError::Storage { message }
            | UiError::Network { message }
            | UiError::NotFound { message }
            | UiError::Invalid { message }
            | UiError::MissingMarketData { message, .. }
            | UiError::Math { message }
            | UiError::Auth { message }
            | UiError::RateLimit { message, .. }
            | UiError::Provider { message }
            | UiError::Refused { message }
            | UiError::Truncated { message }
            | UiError::Locked { message }
            | UiError::PasswordRequired { message }
            | UiError::WrongPassword { message }
            | UiError::Busy { message }
            | UiError::Internal { message } => message,
        };
        f.write_str(message)
    }
}

impl std::error::Error for UiError {}

pub type UiResult<T> = std::result::Result<T, UiError>;
