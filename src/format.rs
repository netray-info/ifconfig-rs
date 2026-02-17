use rocket::http::ContentType;
use rocket::request::FromParam;
use serde_json::Value;

pub enum OutputFormat {
    Json,
    Yaml,
    Toml,
    Csv,
}

impl<'a> FromParam<'a> for OutputFormat {
    type Error = &'a str;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        match param {
            "json" => Ok(OutputFormat::Json),
            "yaml" => Ok(OutputFormat::Yaml),
            "toml" => Ok(OutputFormat::Toml),
            "csv" => Ok(OutputFormat::Csv),
            _ => Err(param),
        }
    }
}

impl OutputFormat {
    pub fn serialize(&self, value: &Value) -> Option<(ContentType, String)> {
        match self {
            OutputFormat::Json => serde_json::to_string_pretty(value).ok().map(|s| (ContentType::JSON, s)),
            OutputFormat::Yaml => serde_yaml::to_string(value)
                .ok()
                .map(|s| (ContentType::new("application", "yaml"), s)),
            OutputFormat::Toml => {
                let cleaned = strip_nulls(value.clone());
                toml::to_string_pretty(&cleaned)
                    .ok()
                    .map(|s| (ContentType::new("application", "toml"), s))
            }
            OutputFormat::Csv => {
                let csv = json_to_csv(value);
                Some((ContentType::new("text", "csv"), csv))
            }
        }
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

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
