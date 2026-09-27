//! Running a plugin's assistant tool (ADR-0085): the model's arguments and the declared reads in,
//! one JSON answer out, in the same sandbox as the reader and the writer — no filesystem, no
//! network, a frozen clock. What the tool is shown and when it may run is the assistant's
//! business (`ai::tools::plugin`); this file only runs a module and checks a package.

use super::sandbox;
use super::widget::Read;
use crate::error::{UiError, UiResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

mod wit {
    wasmtime::component::bindgen!({
        path: "wit/tool.wit",
        world: "tool",
    });
}

/// The largest answer a tool may give. It is read by a model on every later step of the turn,
/// so a runaway answer costs the user money as well as context.
pub const ANSWER_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDef {
    pub id: String,
    /// What the consent card and the plugin list call it. The plugin's own words.
    pub name: String,
    /// Written for the model: what question it answers and when to prefer it.
    pub description: String,
    /// The component, as a `.wasm` file inside the package.
    pub file: String,
    /// A JSON Schema file for the arguments, in the strict subset every provider accepts.
    pub schema: String,
    #[serde(default)]
    pub reads: Vec<Read>,
    /// `{ "args": …, "data": … }` — a call the package vouches for.
    pub sample: String,
    /// What `sample` must answer, compared as JSON.
    pub expected: String,
}

impl ToolDef {
    /// Whether a period is asked for: the host adds it to the schema, resolves it like every
    /// other tool's (ADR-0018), and the model never writes a date.
    pub fn periodic(&self) -> bool {
        self.reads
            .iter()
            .any(|r| matches!(r, Read::Performance | Read::Transactions))
    }
}

/// Runs one call. Every failure — a module that does not start, a trap, the deadline, the memory
/// ceiling, the tool's own refusal, an answer that is not JSON or is too long — is one `Err`.
pub fn call(module: &Path, args: &Value, data: &Value) -> Result<Value, String> {
    let (args, data) = (args.to_string(), data.to_string());
    let answer = sandbox::run(module, |store, component, linker| {
        let tool = wit::Tool::instantiate(&mut *store, component, linker)
            .map_err(|e| format!("the module did not start: {e}"))?;
        tool.call_call(&mut *store, &args, &data)
            .map_err(|e| format!("the module stopped: {e}"))
    })??;
    if answer.len() > ANSWER_LIMIT {
        return Err(format!(
            "the answer is {} bytes; the limit is {ANSWER_LIMIT}",
            answer.len()
        ));
    }
    serde_json::from_str(&answer).map_err(|e| format!("the answer is not JSON: {e}"))
}

/// The strict subset of JSON Schema every provider accepts: an object whose every property is
/// required, nothing else allowed, and properties that are plain values. A schema outside it
/// would not fail this tool alone — a provider refuses the whole request, and with it every chat.
pub fn check_schema(id: &str, schema: &Value) -> UiResult<()> {
    let refuse = |why: &str| Err(UiError::invalid(format!("tool {id}: its schema {why}")));
    if schema["type"] != "object" {
        return refuse("is not an object");
    }
    if schema["additionalProperties"] != false {
        return refuse("does not say additionalProperties: false");
    }
    let Some(properties) = schema["properties"].as_object() else {
        return refuse("has no properties object");
    };
    let required: Vec<&str> = schema["required"]
        .as_array()
        .map(|r| r.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    for (name, property) in properties {
        if name == "reason" || name == "period" {
            return refuse(&format!("names {name:?}, which the app adds itself"));
        }
        if !required.contains(&name.as_str()) {
            return refuse(&format!("leaves {name:?} out of required"));
        }
        let plain = ["string", "number", "integer", "boolean"];
        if !property["type"].as_str().is_some_and(|t| plain.contains(&t)) {
            return refuse(&format!(
                "gives {name:?} a type other than string, number, integer or boolean"
            ));
        }
    }
    Ok(())
}

/// What a tool must prove before its package installs: a schema every provider accepts, and its
/// own sample answered exactly as the package says — compared as JSON, like a reader's, because
/// the answer is a document the model parses, not bytes another program judges.
pub fn check(def: &ToolDef, module: &Path, schema: &[u8], sample: &[u8], expected: &[u8]) -> UiResult<()> {
    let id = &def.id;
    let json = |what: &str, bytes: &[u8]| -> UiResult<Value> {
        serde_json::from_slice(bytes)
            .map_err(|e| UiError::invalid(format!("tool {id}: {what} is not JSON: {e}")))
    };
    check_schema(id, &json("its schema", schema)?)?;
    let sample = json("its sample", sample)?;
    let expected = json("its expectation", expected)?;
    let answer = call(module, &sample["args"], &sample["data"])
        .map_err(|e| UiError::invalid(format!("tool {id} fails its own sample: {e}")))?;
    if answer != expected {
        return Err(UiError::invalid(format!(
            "tool {id} does not answer its own sample the way the package says it does"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_schema_outside_the_strict_subset_is_refused() {
        let good = json!({
            "type": "object",
            "additionalProperties": false,
            "properties": { "top": { "type": "integer" } },
            "required": ["top"]
        });
        assert!(check_schema("t", &good).is_ok());

        let mut loose = good.clone();
        loose["additionalProperties"] = json!(true);
        assert!(
            check_schema("t", &loose).is_err(),
            "strict mode would refuse every chat"
        );

        let mut optional = good.clone();
        optional["required"] = json!([]);
        assert!(check_schema("t", &optional).is_err());

        let mut nested = good.clone();
        nested["properties"]["top"] = json!({ "type": "object" });
        assert!(check_schema("t", &nested).is_err());

        let mut own_period = good;
        own_period["properties"]["period"] = json!({ "type": "string" });
        own_period["required"] = json!(["top", "period"]);
        assert!(
            check_schema("t", &own_period).is_err(),
            "the app adds the period itself"
        );
    }
}
