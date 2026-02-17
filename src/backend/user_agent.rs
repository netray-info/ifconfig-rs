use serde::{Deserialize, Serialize};
use std::convert::From;

pub struct UserAgentParser {
    parser: uaparser::UserAgentParser,
}

impl From<uaparser::UserAgentParser> for UserAgentParser {
    fn from(parser: uaparser::UserAgentParser) -> Self {
        Self { parser }
    }
}

impl UserAgentParser {
    pub fn from_yaml(yaml: &str) -> Result<UserAgentParser, uaparser::Error> {
        uaparser::UserAgentParser::from_yaml(yaml).map(UserAgentParser::from)
    }
    pub fn parse(&self, user_agent_header: &'_ str) -> UserAgent {
        use uaparser::Parser;
        self.parser.parse(user_agent_header).into()
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct UserAgent {
    pub device: Device,
    pub os: OS,
    pub browser: Browser,
}

impl From<uaparser::Client<'_>> for UserAgent {
    fn from(ua: uaparser::Client<'_>) -> Self {
        Self {
            device: ua.device.into(),
            os: ua.os.into(),
            browser: ua.user_agent.into(),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Device {
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

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct OS {
    pub family: String,
    pub major: Option<String>,
    pub minor: Option<String>,
    pub patch: Option<String>,
    pub patch_minor: Option<String>,
    pub version: String,
}

impl From<uaparser::OS<'_>> for OS {
    fn from(value: uaparser::OS<'_>) -> Self {
        let major = value.major.as_ref().map(|s| s.as_ref());
        let minor = value.minor.as_ref().map(|s| s.as_ref());
        let patch = value.patch.as_ref().map(|s| s.as_ref());
        let patch_minor = value.patch_minor.as_ref().map(|s| s.as_ref());
        let version: String = vec![major, minor, patch, patch_minor]
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

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Browser {
    pub family: String,
    pub major: Option<String>,
    pub minor: Option<String>,
    pub patch: Option<String>,
    pub version: String,
}

impl<'a> From<uaparser::UserAgent<'a>> for Browser {
    fn from(value: uaparser::UserAgent<'a>) -> Self {
        let major = value.major.as_ref().map(|s| s.as_ref());
        let minor = value.minor.as_ref().map(|s| s.as_ref());
        let patch = value.patch.as_ref().map(|s| s.as_ref());
        let version: String = vec![major, minor, patch]
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
