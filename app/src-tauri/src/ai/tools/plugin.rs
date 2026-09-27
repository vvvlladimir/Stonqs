//! A plugin's assistant tool (ADR-0085): a WASM body handed the reads `plugins::reads` builds,
//! with the same schema rules, card and fence as the shipped tools.

use super::args::{period_property, period_summary, resolve_period};
use super::{AiResult, Params, ToolContext, add_required, tool, with_reason};
use crate::plugins::LoadedTool;
use serde_json::{Value, json};

/// What the model is offered. The description says whose tool it is, because its figures are
/// the plugin's own (ADR-0082) and the model must not present them as the app's.
pub fn definition(t: &LoadedTool) -> (String, String, Value) {
    let mut schema = t.schema.clone();
    if t.periodic {
        add_required(&mut schema, "period", period_property());
    }
    let description = format!(
        "From the {} plugin, not the app: its figures are the plugin's own, say so when you use \
         them. {}",
        t.info.plugin_name, t.description
    );
    (t.model_name.clone(), description, with_reason(schema))
}

/// The values behind the consent card: whose code runs, and over which window.
pub fn summary(t: &LoadedTool, context: &ToolContext, args: &Value) -> Params {
    let mut params = if t.periodic {
        period_summary(context, args)
    } else {
        Params::new()
    };
    params.insert("plugin".into(), t.info.plugin_name.clone());
    params.insert("tool".into(), t.info.name.clone());
    params
}

pub fn run(t: &LoadedTool, context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = if t.periodic {
        Some(resolve_period(context, args)?)
    } else {
        None
    };
    // The module is handed what the model asked for, minus what the app added for itself.
    let mut own = args.clone();
    if let Some(map) = own.as_object_mut() {
        map.remove(super::REASON);
        map.remove("period");
    }
    let data =
        crate::plugins::reads::project(context.store, context.scope, &t.info.reads, context.today, range)
            .map_err(tool)?;
    let answer = crate::plugins::tool::call(&t.module, &own, &data).map_err(tool)?;
    Ok(json!({ "plugin": t.info.plugin_name, "tool": t.info.name, "answer": answer }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::Read;

    /// A package installed before the install check demanded `required` still reaches the model
    /// with every property required — one left out and a strict provider refuses every chat.
    #[test]
    fn a_schema_without_required_still_requires_what_the_app_adds() {
        let loaded = LoadedTool {
            info: crate::plugins::ToolInfo {
                key: "p/t".into(),
                name: "T".into(),
                plugin: "p".into(),
                plugin_name: "P".into(),
                reads: vec![Read::Performance],
            },
            model_name: "plugin_p_t".into(),
            description: String::new(),
            schema: json!({ "type": "object", "additionalProperties": false, "properties": {} }),
            periodic: true,
            module: std::path::PathBuf::new(),
        };
        let (_, _, schema) = definition(&loaded);
        let mut required: Vec<&str> = schema["required"]
            .as_array()
            .expect("required is there")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        required.sort();
        let mut properties: Vec<&str> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        properties.sort();
        assert_eq!(required, properties);
        assert_eq!(required, ["period", "reason"]);
    }
}
