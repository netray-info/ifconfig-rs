use serde::{Deserialize, Serialize};

const MAX_USER_AGENT_LENGTH: usize = 2000;

pub struct UserAgentParser {
    parser: uaparser::UserAgentParser,
}

impl From<uaparser::UserAgentParser> for UserAgentParser {
    fn from(parser: uaparser::UserAgentParser) -> Self {
        Self { parser }
    }
}

impl UserAgentParser {
    pub async fn from_yaml(path: &str) -> Result<UserAgentParser, uaparser::Error> {
        let bytes = tokio::fs::read(path)
            .await
            .map_err(uaparser::Error::IO)?;
        uaparser::UserAgentParser::from_bytes(&bytes).map(UserAgentParser::from)
    }
    pub fn parse(&self, user_agent_header: &'_ str) -> UserAgent {
        use uaparser::Parser;
        let truncated = &user_agent_header[..user_agent_header.len().min(MAX_USER_AGENT_LENGTH)];
        self.parser.parse(truncated).into()
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UserAgent {
    #[schema(example = "curl/8.7.1")]
    pub raw: Option<String>,
    pub device: Device,
    pub os: OS,
    pub browser: Browser,
}

impl From<uaparser::Client<'_>> for UserAgent {
    fn from(ua: uaparser::Client<'_>) -> Self {
        Self {
            raw: None,
            device: ua.device.into(),
            os: ua.os.into(),
            browser: ua.user_agent.into(),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Device {
    #[schema(example = "Other")]
    pub family: String,
    pub brand: Option<String>,
    pub model: Option<String>,
}

impl From<uaparser::Device<'_>> for Device {
    fn from(value: uaparser::Device<'_>) -> Self {
        Self {
            family: value.family.to_string(),
            brand: value.brand.map(String::from),
            model: value.model.map(String::from),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct OS {
    #[schema(example = "Mac OS X")]
    pub family: String,
    #[schema(example = "14")]
    pub major: Option<String>,
    #[schema(example = "0")]
    pub minor: Option<String>,
    pub patch: Option<String>,
    pub patch_minor: Option<String>,
    #[schema(example = "14.0")]
    pub version: String,
}

impl From<uaparser::OS<'_>> for OS {
    fn from(value: uaparser::OS<'_>) -> Self {
        let version: String = [
            value.major.as_deref(),
            value.minor.as_deref(),
            value.patch.as_deref(),
            value.patch_minor.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(".");

        Self {
            family: value.family.to_string(),
            major: value.major.map(String::from),
            minor: value.minor.map(String::from),
            patch: value.patch.map(String::from),
            patch_minor: value.patch_minor.map(String::from),
            version,
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Browser {
    #[schema(example = "Chrome")]
    pub family: String,
    #[schema(example = "120")]
    pub major: Option<String>,
    #[schema(example = "0")]
    pub minor: Option<String>,
    pub patch: Option<String>,
    #[schema(example = "120.0")]
    pub version: String,
}

impl From<uaparser::UserAgent<'_>> for Browser {
    fn from(value: uaparser::UserAgent<'_>) -> Self {
        let version: String = [value.major.as_deref(), value.minor.as_deref(), value.patch.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(".");

        Self {
            family: value.family.to_string(),
            major: value.major.map(String::from),
            minor: value.minor.map(String::from),
            patch: value.patch.map(String::from),
            version,
        }
    }
}
