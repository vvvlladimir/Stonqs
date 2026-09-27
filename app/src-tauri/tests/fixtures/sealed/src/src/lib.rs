//! A reader that asks for a password: the smallest component that exercises the host's unlock
//! path (`Plugins::read_file`, `UiError::FileProtected`). Not an example of any real format.
//!
//! `OPEN:` followed by a `stonqs.transactions` document is read as it stands.
//! `SEALED:<password>` on the first line, the document after it, is read only when the host hands
//! over that password — the "sealing" is a stand-in for a real encrypted file.

wit_bindgen::generate!({
    path: "wit",
    world: "reader",
});

struct Sealed;

export!(Sealed);

impl Guest for Sealed {
    fn read(bytes: Vec<u8>, hints: FileHints) -> Result<Reading, ReadError> {
        let text = String::from_utf8(bytes).map_err(|_| ReadError::NotMine)?;
        if let Some(document) = text.strip_prefix("OPEN:\n") {
            return Ok(reading(document));
        }
        let Some(rest) = text.strip_prefix("SEALED:") else {
            return Err(ReadError::NotMine);
        };
        let (password, document) = rest.split_once('\n').ok_or(ReadError::NotMine)?;
        match hints.password {
            Some(given) if given == password => Ok(reading(document)),
            _ => Err(ReadError::NeedsPassword),
        }
    }
}

fn reading(document: &str) -> Reading {
    Reading {
        canonical: document.to_string(),
        warnings: Vec::new(),
    }
}
