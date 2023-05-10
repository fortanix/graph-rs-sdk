use crate::jwt::{JsonWebToken, JwtParser};
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::{Debug, Formatter};

use std::str::FromStr;
use url::form_urlencoded;

#[derive(Default, Clone, Eq, PartialEq, Serialize)]
pub struct IdToken {
    code: Option<String>,
    id_token: String,
    state: Option<String>,
    session_state: Option<String>,
    #[serde(flatten)]
    additional_fields: HashMap<String, Value>,
    #[serde(skip)]
    log_pii: bool,
}

impl IdToken {
    pub fn new(id_token: &str, code: &str, state: &str, session_state: &str) -> IdToken {
        IdToken {
            code: Some(code.into()),
            id_token: id_token.into(),
            state: Some(state.into()),
            session_state: Some(session_state.into()),
            additional_fields: Default::default(),
            log_pii: false,
        }
    }

    pub fn id_token(&mut self, id_token: &str) {
        self.id_token = id_token.into();
    }

    pub fn jwt(&self) -> Option<JsonWebToken> {
        JwtParser::parse(self.id_token.as_str()).ok()
    }

    pub fn code(&mut self, code: &str) {
        self.code = Some(code.into());
    }

    pub fn state(&mut self, state: &str) {
        self.state = Some(state.into());
    }

    pub fn session_state(&mut self, session_state: &str) {
        self.session_state = Some(session_state.into());
    }

    /// Enable or disable logging of personally identifiable information such
    /// as logging the id_token. This is disabled by default. When log_pii is enabled
    /// passing an [IdToken] to logging or print functions will log id_token field.
    /// By default this does not get logged.
    pub fn enable_pii_logging(&mut self, log_pii: bool) {
        self.log_pii = log_pii;
    }

    pub fn get_id_token(&self) -> String {
        self.id_token.clone()
    }

    pub fn get_code(&self) -> Option<String> {
        self.code.clone()
    }

    pub fn get_state(&self) -> Option<String> {
        self.state.clone()
    }

    pub fn get_session_state(&self) -> Option<String> {
        self.session_state.clone()
    }
}

impl TryFrom<String> for IdToken {
    type Error = std::io::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let id_token: IdToken = IdToken::from_str(value.as_str())?;
        Ok(id_token)
    }
}

impl TryFrom<&str> for IdToken {
    type Error = std::io::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let id_token: IdToken = IdToken::from_str(value)?;
        Ok(id_token)
    }
}

impl Debug for IdToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.log_pii {
            f.debug_struct("IdToken")
                .field("code", &self.code)
                .field("id_token", &self.id_token)
                .field("session_state", &self.session_state)
                .field("additional_fields", &self.additional_fields)
                .finish()
        } else {
            f.debug_struct("IdToken")
                .field("code", &self.code)
                .field("id_token", &"[REDACTED]")
                .field("session_state", &self.session_state)
                .field("additional_fields", &self.additional_fields)
                .finish()
        }
    }
}

struct IdTokenVisitor;

impl<'de> Deserialize<'de> for IdToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        impl<'de> Visitor<'de> for IdTokenVisitor {
            type Value = IdToken;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("`code`, `id_token`, `state`, and `session_state`")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let vec: Vec<(Cow<str>, Cow<str>)> = form_urlencoded::parse(v).collect();

                if vec.is_empty() {
                    return serde_json::from_slice(v)
                        .map_err(|err| serde::de::Error::custom(err.to_string()));
                }

                let mut id_token = IdToken::default();
                for (key, value) in vec.iter() {
                    match key.as_bytes() {
                        b"code" => id_token.code(value.as_ref()),
                        b"id_token" => id_token.id_token(value.as_ref()),
                        b"state" => id_token.state(value.as_ref()),
                        b"session_state" => id_token.session_state(value.as_ref()),
                        _ => {
                            id_token
                                .additional_fields
                                .insert(key.to_string(), Value::String(value.to_string()));
                        }
                    }
                }
                Ok(id_token)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                IdToken::from_str(v).map_err(|err| Error::custom(err))
            }
        }
        deserializer.deserialize_identifier(IdTokenVisitor)
    }
}

impl FromStr for IdToken {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vec: Vec<(Cow<str>, Cow<str>)> = form_urlencoded::parse(s.as_bytes()).collect();
        if vec.is_empty() {
            return serde_json::from_slice(s.as_bytes());
        }
        let mut id_token = IdToken::default();
        for (key, value) in vec.iter() {
            match key.as_bytes() {
                b"code" => id_token.code(value.as_ref()),
                b"id_token" => id_token.id_token(value.as_ref()),
                b"state" => id_token.state(value.as_ref()),
                b"session_state" => id_token.session_state(value.as_ref()),
                _ => {
                    id_token
                        .additional_fields
                        .insert(key.to_string(), Value::String(value.to_string()));
                }
            }
        }
        Ok(id_token)
    }
}
