use std::collections::HashSet;

use log::trace;
use serde_json::Value;

pub fn filter(value: &Value, include_keys: &HashSet<String>) -> Value {
    trace!("Filtering value {} to include {:?}", value, include_keys);

    match value {
        Value::Null => value.clone(),
        Value::Bool(_) => value.clone(),
        Value::Number(_) => value.clone(),
        Value::String(_) => value.clone(),
        Value::Array(arr) => {
            let filtered_arr: Vec<Value> = arr
                .iter()
                .map(|v| filter(v, include_keys))
                .filter(|v| !v.is_null())
                .collect();
            Value::Array(filtered_arr)
        }
        Value::Object(map) => {
            let filtered_map = map
                .iter()
                .filter(|(k, _)| {
                    let inc = include_keys.contains(k.as_str());
                    inc
                })
                .map(|(k, v)| (k.clone(), filter(v, include_keys)))
                .filter(|(_, v)| !v.is_null())
                .collect();

            trace!("Filtered Map {:?}", filtered_map);

            Value::Object(filtered_map)
        }
    }
}
