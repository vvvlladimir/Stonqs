//! A widget or screen: one ES module served as its own origin-less page, fed only its declared
//! reads (ADR-0083/0084/0088).

use crate::error::{UiError, UiResult};
use serde::{Deserialize, Serialize};

/// What a widget may be handed. A closed list per plugin API: a read is added, never changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Read {
    /// Value, cash and results of the tile's scope on the board's date.
    Valuation,
    /// The positions of that scope on that date.
    Positions,
    /// Return, flows and the value series over the tile's period.
    Performance,
    /// The operations of the app's scope over the period. A screen's only: the operation list has
    /// no data source of its own, and a widget has one (ADR-0084).
    Transactions,
}

/// A size on the board's grid: width in twelfths, height in rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

/// The tallest tile the board offers a new widget; a manifest asking for more is a typo.
const MAX_ROWS: u32 = 40;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WidgetDef {
    pub id: String,
    /// What the palette and the tile call it. The plugin's own words.
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The module, a single self-contained `.js` file: the page allows no script but its own.
    pub file: String,
    #[serde(default)]
    pub reads: Vec<Read>,
    /// Whether it reads over a period, and so offers one in its settings.
    #[serde(default)]
    pub periodic: bool,
    pub size: Size,
    pub min: Size,
}

/// A whole screen: a larger widget that follows the app's lenses and may keep one document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub file: String,
    #[serde(default)]
    pub reads: Vec<Read>,
    #[serde(default)]
    pub periodic: bool,
    /// Whether it keeps a document in the profile (`plugin_state`).
    #[serde(default)]
    pub storage: bool,
}

fn check_module(id: &str, file: &str, module: &[u8]) -> UiResult<()> {
    if !file.ends_with(".js") {
        return Err(UiError::invalid(format!("{id}: {file} is not a .js module")));
    }
    if std::str::from_utf8(module).is_err() {
        return Err(UiError::invalid(format!("{id}: {file} is not UTF-8 text")));
    }
    Ok(())
}

fn check_period(id: &str, reads: &[Read], periodic: bool) -> UiResult<()> {
    match reads
        .iter()
        .find(|r| matches!(r, Read::Performance | Read::Transactions))
    {
        Some(read) if !periodic => Err(UiError::invalid(format!(
            "{id}: reads {read:?}, which is over a period, without being periodic"
        ))),
        _ => Ok(()),
    }
}

/// What a screen must be before its package installs: the widget's check without the grid.
pub fn check_screen(def: &ScreenDef, module: &[u8]) -> UiResult<()> {
    let id = format!("screen {}", def.id);
    check_module(&id, &def.file, module)?;
    check_period(&id, &def.reads, def.periodic)
}

/// What a widget must be before its package installs. Structural only: a drawing has no expected
/// answer to compare against, which is why ADR-0083 names this the one kind without a fixture.
pub fn check(def: &WidgetDef, module: &[u8]) -> UiResult<()> {
    let id = format!("widget {}", def.id);
    let refuse = |why: String| Err(UiError::invalid(format!("{id}: {why}")));
    check_module(&id, &def.file, module)?;
    let fits = |s: Size| (1..=12).contains(&s.w) && (1..=MAX_ROWS).contains(&s.h);
    if !fits(def.size) || !fits(def.min) {
        return refuse(format!(
            "sizes must be 1–12 twelfths wide and 1–{MAX_ROWS} rows tall"
        ));
    }
    if def.min.w > def.size.w || def.min.h > def.size.h {
        return refuse("its smallest size is larger than its size".into());
    }
    if def.reads.contains(&Read::Transactions) {
        return refuse("transactions are a screen's read: a widget has a data source of its own".into());
    }
    check_period(&id, &def.reads, def.periodic)
}

const SHIM: &str = include_str!("../widget_shim.js");

/// Every `</script`, whatever its case, as `<\/script` with the case it had.
fn escape_script_end(module: &str) -> String {
    const END: &[u8] = b"</script";
    let bytes = module.as_bytes();
    let mut out = String::with_capacity(module.len());
    let mut from = 0;
    let mut i = 0;
    while i + END.len() <= bytes.len() {
        if bytes[i..i + END.len()].eq_ignore_ascii_case(END) {
            out.push_str(&module[from..i]);
            out.push_str("<\\");
            from = i + 1;
            i += END.len();
        } else {
            i += 1;
        }
    }
    out.push_str(&module[from..]);
    out
}

/// The page a widget or a screen is served as, and the policy it is served under. `nonce` is new per response,
/// so the only scripts that run are the two written here.
pub fn page(module: &str, nonce: &str) -> (String, String) {
    // `</script` in any case would end the element; `<\/` means the same in a string or regex.
    let module = escape_script_end(module);
    let html = format!(
        // DNS prefetching is not a request the policy can refuse, and a looked-up name is a
        // message to whoever answers for its domain.
        "<!doctype html><html><head><meta charset=\"utf-8\">\
         <meta http-equiv=\"x-dns-prefetch-control\" content=\"off\">\
         <style>html,body{{margin:0;height:100%;background:transparent;overflow:auto}}</style>\
         <script nonce=\"{nonce}\">{SHIM}</script>\
         <script type=\"module\" nonce=\"{nonce}\">{module}</script>\
         </head><body></body></html>"
    );
    let csp = format!(
        "default-src 'none'; script-src 'nonce-{nonce}'; style-src 'unsafe-inline'; \
         img-src data:; font-src data:; base-uri 'none'; form-action 'none'"
    );
    (html, csp)
}

#[cfg(test)]
mod tests;
