use rocket::http::ContentType;
use rocket::request::FromParam;
use serde_json::Value;
use std::borrow::Cow;

pub enum OutputFormat {
    Json,
    Yaml,
    Toml,
    Csv,
}

impl OutputFormat {
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "json" => Some(OutputFormat::Json),
            "yaml" => Some(OutputFormat::Yaml),
            "toml" => Some(OutputFormat::Toml),
            "csv" => Some(OutputFormat::Csv),
            _ => None,
        }
    }

    pub fn mime_type(&self) -> (&'static str, &'static str) {
        match self {
            OutputFormat::Json => ("application", "json"),
            OutputFormat::Yaml => ("application", "yaml"),
            OutputFormat::Toml => ("application", "toml"),
            OutputFormat::Csv => ("text", "csv"),
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

    pub fn serialize(&self, value: &Value) -> Option<(ContentType, String)> {
        let body = self.serialize_body(value)?;
        let (top, sub) = self.mime_type();
        Some((ContentType::new(top, sub), body))
    }
}

impl<'a> FromParam<'a> for OutputFormat {
    type Error = &'a str;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        Self::from_name(param).ok_or(param)
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
    fn format_from_param() {
        assert!(OutputFormat::from_param("json").is_ok());
        assert!(OutputFormat::from_param("yaml").is_ok());
        assert!(OutputFormat::from_param("toml").is_ok());
        assert!(OutputFormat::from_param("csv").is_ok());
        assert!(OutputFormat::from_param("xml").is_err());
        assert!(OutputFormat::from_param("").is_err());
    }

    #[test]
    fn mime_type_values() {
        assert_eq!(OutputFormat::Json.mime_type(), ("application", "json"));
        assert_eq!(OutputFormat::Yaml.mime_type(), ("application", "yaml"));
        assert_eq!(OutputFormat::Toml.mime_type(), ("application", "toml"));
        assert_eq!(OutputFormat::Csv.mime_type(), ("text", "csv"));
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
        assert_eq!(ct, ContentType::JSON);
        assert!(body.contains("1.2.3.4"));
    }

    #[test]
    fn serialize_yaml() {
        let val = json!({"ip": "1.2.3.4"});
        let (ct, body) = OutputFormat::Yaml.serialize(&val).unwrap();
        assert_eq!(ct, ContentType::new("application", "yaml"));
        assert!(body.contains("ip: 1.2.3.4"));
    }

    #[test]
    fn serialize_toml_strips_nulls() {
        let val = json!({"ip": "1.2.3.4", "host": null});
        let (ct, body) = OutputFormat::Toml.serialize(&val).unwrap();
        assert_eq!(ct, ContentType::new("application", "toml"));
        assert!(body.contains("ip"));
        assert!(!body.contains("host"));
    }

    #[test]
    fn serialize_csv_flat() {
        let val = json!({"addr": "1.2.3.4", "version": "4"});
        let (ct, body) = OutputFormat::Csv.serialize(&val).unwrap();
        assert_eq!(ct, ContentType::new("text", "csv"));
        assert!(body.starts_with("key,value\n"));
        assert!(body.contains("addr,1.2.3.4"));
        assert!(body.contains("version,4"));
    }

    #[test]
    fn serialize_csv_nested() {
        let val = json!({"ip": {"addr": "1.2.3.4", "version": "4"}});
        let (ct, body) = OutputFormat::Csv.serialize(&val).unwrap();
        assert_eq!(ct, ContentType::new("text", "csv"));
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
}
