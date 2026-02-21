use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashSet;

pub enum OutputFormat {
    Json,
    Yaml,
    Toml,
    Csv,
}

impl OutputFormat {
    #[cfg(test)]
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "json" => Some(OutputFormat::Json),
            "yaml" => Some(OutputFormat::Yaml),
            "toml" => Some(OutputFormat::Toml),
            "csv" => Some(OutputFormat::Csv),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn mime_type(&self) -> (&'static str, &'static str) {
        match self {
            OutputFormat::Json => ("application", "json"),
            OutputFormat::Yaml => ("application", "yaml"),
            OutputFormat::Toml => ("application", "toml"),
            OutputFormat::Csv => ("text", "csv"),
        }
    }

    pub fn content_type(&self) -> &'static str {
        match self {
            OutputFormat::Json => "application/json",
            OutputFormat::Yaml => "application/yaml",
            OutputFormat::Toml => "application/toml",
            OutputFormat::Csv => "text/csv",
        }
    }

    pub fn serialize_body(&self, value: &Value) -> Option<String> {
        match self {
            OutputFormat::Json => serde_json::to_string_pretty(value).ok(),
            OutputFormat::Yaml => serde_yaml::to_string(value).ok(),
            OutputFormat::Toml => {
                let cleaned = strip_nulls(value.clone());
                toml::to_string_pretty(&cleaned).ok()
            }
            OutputFormat::Csv => Some(json_to_csv(value)),
        }
    }

    pub fn serialize(&self, value: &Value) -> Option<(&'static str, String)> {
        let body = self.serialize_body(value)?;
        Some((self.content_type(), body))
    }
}

/// Parse `?fields=ip,location,isp` from a URI string into a set of field names.
pub fn parse_fields_param(uri: &str) -> Option<HashSet<String>> {
    let query = uri.split('?').nth(1)?;
    query
        .split('&')
        .find_map(|p| p.strip_prefix("fields="))
        .map(|f| f.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

/// Keep only the specified top-level keys from a JSON object.
pub fn filter_fields(value: Value, fields: &HashSet<String>) -> Value {
    match value {
        Value::Object(map) => {
            let filtered = map.into_iter().filter(|(k, _)| fields.contains(k)).collect();
            Value::Object(filtered)
        }
        other => other,
    }
}

fn strip_nulls(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let cleaned: serde_json::Map<String, Value> = map
                .into_iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k, strip_nulls(v)))
                .collect();
            Value::Object(cleaned)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(strip_nulls).collect()),
        other => other,
    }
}

fn json_to_csv(value: &Value) -> String {
    let mut rows = Vec::new();
    rows.push("key,value".to_string());
    flatten_json(value, String::new(), &mut rows);
    rows.join("\n") + "\n"
}

fn flatten_json(value: &Value, prefix: String, rows: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_json(v, key, rows);
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let key = format!("{}.{}", prefix, i);
                flatten_json(v, key, rows);
            }
        }
        Value::Null => {
            rows.push(format!("{},", csv_escape(&prefix)));
        }
        Value::Bool(b) => {
            rows.push(format!("{},{}", csv_escape(&prefix), b));
        }
        Value::Number(n) => {
            rows.push(format!("{},{}", csv_escape(&prefix), n));
        }
        Value::String(s) => {
            rows.push(format!("{},{}", csv_escape(&prefix), csv_escape(s)));
        }
    }
}

/// Render a JSON array of objects as tabular CSV (one row per item).
/// Column names are derived from flattened dot-notation keys of the first valid item.
/// Error objects (those with an "error" key) are included with empty data columns.
pub fn json_array_to_csv(array: &[Value]) -> String {
    // Collect columns from the first non-error item
    let columns: Vec<String> = array
        .iter()
        .find(|v| v.get("error").is_none())
        .map(|first| {
            let mut cols = Vec::new();
            collect_keys(first, String::new(), &mut cols);
            cols
        })
        .unwrap_or_default();

    if columns.is_empty() {
        // All errors or empty array — fall back to error-only output
        let mut rows = vec!["error,input".to_string()];
        for item in array {
            let err = item.get("error").and_then(|v| v.as_str()).unwrap_or("");
            let input = item.get("input").and_then(|v| v.as_str()).unwrap_or("");
            rows.push(format!("{},{}", csv_escape(err), csv_escape(input)));
        }
        return rows.join("\n") + "\n";
    }

    let mut rows = Vec::with_capacity(array.len() + 1);
    rows.push(columns.iter().map(|c| csv_escape(c).into_owned()).collect::<Vec<_>>().join(","));

    for item in array {
        if item.get("error").is_some() {
            // Error row: fill with empty values, but put error in first column
            let err = item.get("error").and_then(|v| v.as_str()).unwrap_or("");
            let mut vals = vec![csv_escape(err).into_owned()];
            vals.extend(std::iter::repeat_n(String::new(), columns.len().saturating_sub(1)));
            rows.push(vals.join(","));
        } else {
            let vals: Vec<String> = columns
                .iter()
                .map(|col| {
                    let val = get_nested_value(item, col);
                    match val {
                        Some(Value::String(s)) => csv_escape(s).into_owned(),
                        Some(Value::Null) | None => String::new(),
                        Some(v) => v.to_string(),
                    }
                })
                .collect();
            rows.push(vals.join(","));
        }
    }

    rows.join("\n") + "\n"
}

fn collect_keys(value: &Value, prefix: String, keys: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() { k.clone() } else { format!("{}.{}", prefix, k) };
                match v {
                    Value::Object(_) => collect_keys(v, key, keys),
                    _ => keys.push(key),
                }
            }
        }
        _ => {
            if !prefix.is_empty() {
                keys.push(prefix);
            }
        }
    }
}

fn get_nested_value<'a>(value: &'a Value, dotted_key: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in dotted_key.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn csv_escape(s: &str) -> Cow<'_, str> {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        Cow::Owned(format!("\"{}\"", s.replace('"', "\"\"")))
    } else {
        Cow::Borrowed(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn from_name_valid() {
        assert!(OutputFormat::from_name("json").is_some());
        assert!(OutputFormat::from_name("yaml").is_some());
        assert!(OutputFormat::from_name("toml").is_some());
        assert!(OutputFormat::from_name("csv").is_some());
    }

    #[test]
    fn from_name_invalid() {
        assert!(OutputFormat::from_name("xml").is_none());
        assert!(OutputFormat::from_name("").is_none());
    }

    #[test]
    fn mime_type_values() {
        assert_eq!(OutputFormat::Json.mime_type(), ("application", "json"));
        assert_eq!(OutputFormat::Yaml.mime_type(), ("application", "yaml"));
        assert_eq!(OutputFormat::Toml.mime_type(), ("application", "toml"));
        assert_eq!(OutputFormat::Csv.mime_type(), ("text", "csv"));
    }

    #[test]
    fn content_type_values() {
        assert_eq!(OutputFormat::Json.content_type(), "application/json");
        assert_eq!(OutputFormat::Yaml.content_type(), "application/yaml");
        assert_eq!(OutputFormat::Toml.content_type(), "application/toml");
        assert_eq!(OutputFormat::Csv.content_type(), "text/csv");
    }

    #[test]
    fn serialize_body_json() {
        let val = json!({"ip": "1.2.3.4"});
        let body = OutputFormat::Json.serialize_body(&val).unwrap();
        assert!(body.contains("1.2.3.4"));
    }

    #[test]
    fn serialize_body_yaml() {
        let val = json!({"ip": "1.2.3.4"});
        let body = OutputFormat::Yaml.serialize_body(&val).unwrap();
        assert!(body.contains("ip: 1.2.3.4"));
    }

    #[test]
    fn serialize_body_toml_strips_nulls() {
        let val = json!({"ip": "1.2.3.4", "host": null});
        let body = OutputFormat::Toml.serialize_body(&val).unwrap();
        assert!(body.contains("ip"));
        assert!(!body.contains("host"));
    }

    #[test]
    fn serialize_body_csv() {
        let val = json!({"addr": "1.2.3.4"});
        let body = OutputFormat::Csv.serialize_body(&val).unwrap();
        assert!(body.starts_with("key,value\n"));
        assert!(body.contains("addr,1.2.3.4"));
    }

    #[test]
    fn serialize_json() {
        let val = json!({"ip": "1.2.3.4"});
        let (ct, body) = OutputFormat::Json.serialize(&val).unwrap();
        assert_eq!(ct, "application/json");
        assert!(body.contains("1.2.3.4"));
    }

    #[test]
    fn serialize_yaml() {
        let val = json!({"ip": "1.2.3.4"});
        let (ct, body) = OutputFormat::Yaml.serialize(&val).unwrap();
        assert_eq!(ct, "application/yaml");
        assert!(body.contains("ip: 1.2.3.4"));
    }

    #[test]
    fn serialize_toml_strips_nulls() {
        let val = json!({"ip": "1.2.3.4", "host": null});
        let (ct, body) = OutputFormat::Toml.serialize(&val).unwrap();
        assert_eq!(ct, "application/toml");
        assert!(body.contains("ip"));
        assert!(!body.contains("host"));
    }

    #[test]
    fn serialize_csv_flat() {
        let val = json!({"addr": "1.2.3.4", "version": "4"});
        let (ct, body) = OutputFormat::Csv.serialize(&val).unwrap();
        assert_eq!(ct, "text/csv");
        assert!(body.starts_with("key,value\n"));
        assert!(body.contains("addr,1.2.3.4"));
        assert!(body.contains("version,4"));
    }

    #[test]
    fn serialize_csv_nested() {
        let val = json!({"ip": {"addr": "1.2.3.4", "version": "4"}});
        let (ct, body) = OutputFormat::Csv.serialize(&val).unwrap();
        assert_eq!(ct, "text/csv");
        assert!(body.contains("ip.addr,1.2.3.4"));
        assert!(body.contains("ip.version,4"));
    }

    #[test]
    fn strip_nulls_removes_null_fields() {
        let val = json!({"a": 1, "b": null, "c": {"d": null, "e": 2}});
        let cleaned = strip_nulls(val);
        assert_eq!(cleaned, json!({"a": 1, "c": {"e": 2}}));
    }

    #[test]
    fn csv_escape_quotes() {
        assert_eq!(csv_escape("hello"), "hello");
        assert_eq!(csv_escape("hello,world"), "\"hello,world\"");
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn parse_fields_param_basic() {
        let fields = parse_fields_param("/all/json?fields=ip,location").unwrap();
        assert_eq!(fields.len(), 2);
        assert!(fields.contains("ip"));
        assert!(fields.contains("location"));
    }

    #[test]
    fn parse_fields_param_with_other_params() {
        let fields = parse_fields_param("/all/json?ip=8.8.8.8&fields=ip,isp&dns=true").unwrap();
        assert_eq!(fields.len(), 2);
        assert!(fields.contains("ip"));
        assert!(fields.contains("isp"));
    }

    #[test]
    fn parse_fields_param_missing() {
        assert!(parse_fields_param("/all/json").is_none());
        assert!(parse_fields_param("/all/json?ip=8.8.8.8").is_none());
    }

    #[test]
    fn parse_fields_param_empty_value() {
        let fields = parse_fields_param("/all/json?fields=").unwrap();
        assert!(fields.is_empty());
    }

    #[test]
    fn filter_fields_keeps_selected() {
        let val = json!({"ip": {"addr": "1.2.3.4"}, "location": {"city": "Test"}, "isp": {"name": "ISP"}});
        let fields: HashSet<String> = ["ip", "location"].iter().map(|s| s.to_string()).collect();
        let filtered = filter_fields(val, &fields);
        assert!(filtered["ip"].is_object());
        assert!(filtered["location"].is_object());
        assert!(filtered.get("isp").is_none());
    }

    #[test]
    fn filter_fields_non_object() {
        let val = json!("just a string");
        let fields: HashSet<String> = ["ip"].iter().map(|s| s.to_string()).collect();
        let filtered = filter_fields(val.clone(), &fields);
        assert_eq!(filtered, val);
    }

    #[test]
    fn json_array_to_csv_basic() {
        let items = vec![
            json!({"ip": {"addr": "8.8.8.8", "version": "4"}, "tcp": {"port": 0}}),
            json!({"ip": {"addr": "1.1.1.1", "version": "4"}, "tcp": {"port": 0}}),
        ];
        let csv = json_array_to_csv(&items);
        let lines: Vec<&str> = csv.trim().split('\n').collect();
        assert_eq!(lines[0], "ip.addr,ip.version,tcp.port");
        assert!(lines[1].starts_with("8.8.8.8,4,"));
        assert!(lines[2].starts_with("1.1.1.1,4,"));
    }

    #[test]
    fn json_array_to_csv_with_errors() {
        let items = vec![
            json!({"ip": {"addr": "8.8.8.8", "version": "4"}}),
            json!({"error": "private IP address not allowed", "input": "10.0.0.1"}),
            json!({"ip": {"addr": "1.1.1.1", "version": "4"}}),
        ];
        let csv = json_array_to_csv(&items);
        let lines: Vec<&str> = csv.trim().split('\n').collect();
        assert_eq!(lines.len(), 4); // header + 3 rows
        assert!(lines[0].starts_with("ip.addr,"));
        assert!(lines[2].starts_with("private IP address not allowed"));
    }

    #[test]
    fn json_array_to_csv_all_errors() {
        let items = vec![
            json!({"error": "invalid", "input": "foo"}),
            json!({"error": "invalid", "input": "bar"}),
        ];
        let csv = json_array_to_csv(&items);
        assert!(csv.starts_with("error,input\n"));
    }

    #[test]
    fn json_array_to_csv_empty() {
        let items: Vec<serde_json::Value> = vec![];
        let csv = json_array_to_csv(&items);
        assert!(csv.starts_with("error,input\n"));
    }
}
