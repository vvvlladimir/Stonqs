//! A reader for MT940, the SWIFT bank statement most European banks still export.
//!
//! It exists as an example of the contract (ADR-0073), and it is a real reader: MT940 is a
//! tagged text format that no column mapping can express, which is exactly the case a broker
//! layout cannot cover and a plugin can.
//!
//! Nothing here reads a file, opens a socket or asks the time — there is no way to, which is the
//! point. The output is a `stonqs.transactions` document; the app reads it the same way it reads
//! one a user wrote by hand.

wit_bindgen::generate!({
    path: "wit",
    world: "reader",
});

struct Mt940;

export!(Mt940);

impl Guest for Mt940 {
    fn read(bytes: Vec<u8>, _hints: FileHints) -> Result<Reading, ReadError> {
        // MT940 is ASCII with the occasional Latin-1 name in the description; neither the tags
        // nor the numbers can be damaged by a lossy decode.
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let fields = fields(&text);

        // A statement has a reference and at least one line. Both missing means somebody else's
        // file, which is not a failure — the app moves on to the next reader.
        if !fields.iter().any(|(tag, _)| tag == "20") || !fields.iter().any(|(tag, _)| tag == "61") {
            return Err(ReadError::NotMine);
        }

        let currency = fields
            .iter()
            .find(|(tag, _)| tag.starts_with("60"))
            .and_then(|(_, value)| opening_currency(value))
            .ok_or_else(|| {
                ReadError::Malformed("the statement states no opening balance, so its currency is unknown".into())
            })?;

        let mut rows = Vec::new();
        let mut warnings = Vec::new();
        for (tag, value) in &fields {
            match tag.as_str() {
                "61" => match statement_line(value, &currency) {
                    Some(row) => rows.push(row),
                    None => warnings.push(Warning {
                        row: Some(rows.len() as u32),
                        code: "unreadable-line".into(),
                        message: format!("a statement line was skipped: {value}"),
                    }),
                },
                // The description belongs to the line above it. A statement that opens with one
                // has nothing to describe.
                "86" => {
                    if let Some(row) = rows.last_mut() {
                        row.note = Some(collapse(value));
                    }
                }
                _ => {}
            }
        }

        if rows.is_empty() {
            return Err(ReadError::Malformed("the statement holds no readable line".into()));
        }

        Ok(Reading {
            canonical: document(&rows),
            warnings,
        })
    }
}

/// One operation, before it is written out. Only the fields this format can state: an MT940 line
/// carries money and never an instrument.
struct Row {
    date: String,
    kind: &'static str,
    amount: String,
    currency: String,
    external_id: Option<String>,
    note: Option<String>,
}

/// Splits the statement into `(tag, value)`, joining the continuation lines a tag's value runs
/// onto. A line that does not begin a tag continues the previous one — which is how `:86:`
/// carries four lines of a payer's name.
fn fields(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        match tag_of(line) {
            Some((tag, rest)) => out.push((tag, rest.to_string())),
            None => {
                if let Some((_, value)) = out.last_mut() {
                    value.push('\n');
                    value.push_str(line);
                }
            }
        }
    }
    out
}

/// `:61:` at the start of a line, and what follows it. A tag is two digits with an optional
/// letter, which is why `60F` and `60M` are one tag to us.
fn tag_of(line: &str) -> Option<(String, &str)> {
    let rest = line.strip_prefix(':')?;
    let end = rest.find(':')?;
    let tag = &rest[..end];
    let shaped = (2..=3).contains(&tag.len())
        && tag[..2].chars().all(|c| c.is_ascii_digit())
        && tag[2..].chars().all(|c| c.is_ascii_alphabetic());
    shaped.then(|| (tag.to_string(), &rest[end + 1..]))
}

/// `C240101EUR1234,56` — the mark, the date, and then the currency the whole statement is in.
fn opening_currency(value: &str) -> Option<String> {
    let after_mark = value.strip_prefix(['C', 'D'])?;
    let after_date = after_mark.get(6..)?;
    let code = after_date.get(..3)?;
    code.chars()
        .all(|c| c.is_ascii_alphabetic())
        .then(|| code.to_ascii_uppercase())
}

/// `2401020102C1234,56NTRFNONREF//BANKREF`
fn statement_line(value: &str, currency: &str) -> Option<Row> {
    let mut rest = value.lines().next()?;

    let date = iso_date(take(&mut rest, 6)?)?;
    // The entry date is optional and is four digits where the mark would otherwise be.
    if rest.len() >= 4 && rest[..4].chars().all(|c| c.is_ascii_digit()) {
        take(&mut rest, 4)?;
    }

    // `RC` and `RD` are reversals: money went the other way from the mark that follows.
    let reversal = rest.starts_with('R');
    if reversal {
        take(&mut rest, 1)?;
    }
    let credit = match take(&mut rest, 1)? {
        "C" => !reversal,
        "D" => reversal,
        _ => return None,
    };

    // An optional funds code sits between the mark and the amount: one letter, and only when a
    // digit follows it.
    let funds_code = {
        let mut chars = rest.chars();
        matches!((chars.next(), chars.next()), (Some(a), Some(b)) if a.is_ascii_alphabetic() && b.is_ascii_digit())
    };
    if funds_code {
        take(&mut rest, 1)?;
    }

    let digits = rest
        .find(|c: char| !c.is_ascii_digit() && c != ',')
        .unwrap_or(rest.len());
    let amount = take(&mut rest, digits)?.replace(',', ".");
    if amount.is_empty() || amount == "." {
        return None;
    }

    // `NTRF`, `NCHG`, `NINT`: the leading letter says who assigned the code, the three after it
    // say what happened.
    let code = rest
        .get(..4)
        .filter(|c| c.len() == 4)
        .map(|c| c[1..].to_ascii_uppercase())
        .unwrap_or_default();
    let reference = rest.get(4..).unwrap_or_default();

    Some(Row {
        date,
        kind: kind_of(&code, credit),
        amount,
        currency: currency.to_string(),
        // The bank's own name for the entry, after the `//`. `NONREF` is the format's way of
        // saying there is none, so it is not one.
        external_id: reference
            .split_once("//")
            .map(|(_, bank)| bank.trim().to_string())
            .filter(|id| !id.is_empty() && id != "NONREF"),
        note: None,
    })
}

/// What the entry is, as far as MT940 says. Everything it does not name is money in or out —
/// which for a bank account is the honest reading, and the wizard lets the user say otherwise.
fn kind_of(code: &str, credit: bool) -> &'static str {
    match (code, credit) {
        ("CHG", true) => "FEE_REFUND",
        ("CHG", false) => "FEE",
        ("INT", true) => "INTEREST",
        ("INT", false) => "INTEREST_CHARGE",
        (_, true) => "DEPOSIT",
        (_, false) => "WITHDRAWAL",
    }
}

/// `240102` — a two-digit year, which this format has never had room for more of.
fn iso_date(value: &str) -> Option<String> {
    if value.len() != 6 || !value.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(format!("20{}-{}-{}", &value[..2], &value[2..4], &value[4..]))
}

fn take<'a>(rest: &mut &'a str, count: usize) -> Option<&'a str> {
    if !rest.is_char_boundary(count) {
        return None;
    }
    let (head, tail) = rest.split_at(count);
    *rest = tail;
    Some(head)
}

/// A description arrives wrapped over several lines and is one sentence.
fn collapse(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The app's own transaction file. Written by hand rather than with a serialiser: the guest is
/// shipped as a binary and every dependency is in it.
fn document(rows: &[Row]) -> String {
    let mut out = String::from("{\"format\":\"stonqs.transactions\",\"version\":1,\"rows\":[");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        field(&mut out, "date", &row.date, true);
        field(&mut out, "kind", row.kind, false);
        field(&mut out, "amount", &row.amount, false);
        field(&mut out, "currency", &row.currency, false);
        if let Some(id) = &row.external_id {
            field(&mut out, "external_id", id, false);
        }
        if let Some(note) = &row.note {
            field(&mut out, "note", note, false);
        }
        out.push('}');
    }
    out.push_str("]}");
    out
}

fn field(out: &mut String, name: &str, value: &str, first: bool) {
    if !first {
        out.push(',');
    }
    out.push('"');
    out.push_str(name);
    out.push_str("\":\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out.push('"');
}
