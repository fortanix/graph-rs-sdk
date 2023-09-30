use graph_error::GraphFailure;
use serde::{Deserialize, Deserializer};
use serde_aux::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::ops::Add;

use crate::token::IdToken;
use std::str::FromStr;
use time::OffsetDateTime;

fn deserialize_scope<'de, D>(scope: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let scope_string: Result<String, D::Error> = serde::Deserialize::deserialize(scope);
    if let Ok(scope) = scope_string {
        Ok(scope.split(' ').map(|scope| scope.to_owned()).collect())
    } else {
        Ok(vec![])
    }
}

// Used to set timestamp based on expires in
// which can only be done after deserialization.
#[derive(Clone, Serialize, Deserialize)]
struct PhantomMsalToken {
    access_token: String,
    token_type: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    expires_in: i64,
    /// Legacy version of expires_in
    ext_expires_in: Option<i64>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_scope")]
    scope: Vec<String>,
    refresh_token: Option<String>,
    user_id: Option<String>,
    id_token: Option<String>,
    state: Option<String>,
    correlation_id: Option<String>,
    client_info: Option<String>,
    #[serde(flatten)]
    additional_fields: HashMap<String, Value>,
}

/// An access token is a security token issued by an authorization server as part of an OAuth 2.0 flow.
/// It contains information about the user and the resource for which the token is intended.
/// The information can be used to access web APIs and other protected resources.
/// Resources validate access tokens to grant access to a client application.
/// For more information, see [Access tokens in the Microsoft Identity Platform](https://learn.microsoft.com/en-us/azure/active-directory/develop/access-tokens)
///
/// Create a new AccessToken.
/// # Example
/// ```
/// # use graph_extensions::token::MsalToken;
/// let token_response = MsalToken::new("Bearer", 3600, "ASODFIUJ34KJ;LADSK", vec!["User.Read"]);
/// ```
/// The [MsalToken::jwt] method attempts to parse the access token as a JWT.
/// Tokens returned for personal microsoft accounts that use legacy MSA
/// are encrypted and cannot be parsed. This bearer token may still be
/// valid but the jwt() method will return None.
/// For more info see:
/// [Microsoft identity platform access tokens](https://docs.microsoft.com/en-us/azure/active-directory/develop/access-tokens)
/// ```
#[derive(Clone, Eq, PartialEq, Serialize)]
pub struct MsalToken {
    pub access_token: String,
    pub token_type: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub expires_in: i64,
    /// Legacy version of expires_in
    pub ext_expires_in: Option<i64>,
    #[serde(default)]
    #[serde(deserialize_with = "deserialize_scope")]
    pub scope: Vec<String>,

    /// Because access tokens are valid for only a short period of time,
    /// authorization servers sometimes issue a refresh token at the same
    /// time the access token is issued. The client application can then
    /// exchange this refresh token for a new access token when needed.
    /// For more information, see
    /// [Refresh tokens in the Microsoft identity platform.](https://learn.microsoft.com/en-us/azure/active-directory/develop/refresh-tokens)
    pub refresh_token: Option<String>,
    pub user_id: Option<String>,
    pub id_token: Option<String>,
    pub state: Option<String>,
    pub correlation_id: Option<String>,
    pub client_info: Option<String>,
    pub timestamp: Option<time::OffsetDateTime>,
    pub expires_on: Option<time::OffsetDateTime>,
    /// Any extra returned fields for AccessToken.
    #[serde(flatten)]
    pub additional_fields: HashMap<String, Value>,
    #[serde(skip)]
    log_pii: bool,
}

impl MsalToken {
    pub fn new<T: ToString, I: IntoIterator<Item = T>>(
        token_type: &str,
        expires_in: i64,
        access_token: &str,
        scope: I,
    ) -> MsalToken {
        let timestamp = time::OffsetDateTime::now_utc();
        let expires_on = timestamp.add(time::Duration::seconds(expires_in));

        MsalToken {
            token_type: token_type.into(),
            ext_expires_in: None,
            expires_in,
            scope: scope.into_iter().map(|s| s.to_string()).collect(),
            access_token: access_token.into(),
            refresh_token: None,
            user_id: None,
            id_token: None,
            state: None,
            correlation_id: None,
            client_info: None,
            timestamp: Some(timestamp),
            expires_on: Some(expires_on),
            additional_fields: Default::default(),
            log_pii: false,
        }
    }

    /// Set the token type.
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::MsalToken;
    ///
    /// let mut access_token = MsalToken::default();
    /// access_token.with_token_type("Bearer");
    /// ```
    pub fn with_token_type(&mut self, s: &str) -> &mut Self {
        self.token_type = s.into();
        self
    }

    /// Set the expies in time. This should usually be done in seconds.
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::MsalToken;
    ///
    /// let mut access_token = MsalToken::default();
    /// access_token.with_expires_in(3600);
    /// ```
    pub fn with_expires_in(&mut self, expires_in: i64) -> &mut Self {
        self.expires_in = expires_in;
        let timestamp = time::OffsetDateTime::now_utc();
        self.expires_on = Some(timestamp.add(time::Duration::seconds(self.expires_in.clone())));
        self.timestamp = Some(timestamp);
        self
    }

    /// Set the scope.
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::MsalToken;
    ///
    /// let mut access_token = MsalToken::default();
    /// access_token.with_scope(vec!["User.Read"]);
    /// ```
    pub fn with_scope<T: ToString, I: IntoIterator<Item = T>>(&mut self, scope: I) -> &mut Self {
        self.scope = scope.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the access token.
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::MsalToken;
    ///
    /// let mut access_token = MsalToken::default();
    /// access_token.with_access_token("ASODFIUJ34KJ;LADSK");
    /// ```
    pub fn with_access_token(&mut self, s: &str) -> &mut Self {
        self.access_token = s.into();
        self
    }

    /// Set the refresh token.
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::MsalToken;
    ///
    /// let mut access_token = MsalToken::default();
    /// access_token.with_refresh_token("#ASOD323U5342");
    /// ```
    pub fn with_refresh_token(&mut self, s: &str) -> &mut Self {
        self.refresh_token = Some(s.to_string());
        self
    }

    /// Set the user id.
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::MsalToken;
    ///
    /// let mut access_token = MsalToken::default();
    /// access_token.with_user_id("user_id");
    /// ```
    pub fn with_user_id(&mut self, s: &str) -> &mut Self {
        self.user_id = Some(s.to_string());
        self
    }

    /// Set the id token.
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::{MsalToken, IdToken};
    ///
    /// let mut access_token = MsalToken::default();
    /// access_token.set_id_token("id_token");
    /// ```
    pub fn set_id_token(&mut self, s: &str) -> &mut Self {
        self.id_token = Some(s.to_string());
        self
    }

    /// Set the id token.
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::{MsalToken, IdToken};
    ///
    /// let mut access_token = MsalToken::default();
    /// access_token.with_id_token(IdToken::new("id_token", "code", "state", "session_state"));
    /// ```
    pub fn with_id_token(&mut self, id_token: IdToken) {
        self.id_token = Some(id_token.id_token);
    }

    pub fn parse_id_token(&mut self) -> Option<Result<IdToken, serde::de::value::Error>> {
        self.id_token.clone().map(|s| IdToken::from_str(s.as_str()))
    }

    /// Set the state.
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::MsalToken;
    /// # use graph_extensions::token::IdToken;
    ///
    /// let mut access_token = MsalToken::default();
    /// access_token.with_state("state");
    /// ```
    pub fn with_state(&mut self, s: &str) -> &mut Self {
        self.state = Some(s.to_string());
        self
    }

    /// Enable or disable logging of personally identifiable information such
    /// as logging the id_token. This is disabled by default. When log_pii is enabled
    /// passing [MsalToken] to logging or print functions will log both the bearer
    /// access token value, the refresh token value if any, and the id token value.
    /// By default these do not get logged.
    pub fn enable_pii_logging(&mut self, log_pii: bool) {
        self.log_pii = log_pii;
    }

    /// Timestamp field is used to tell whether the access token is expired.
    /// This method is mainly used internally as soon as the access token
    /// is deserialized from the api response for an accurate reading
    /// on when the access token expires.
    ///
    /// You most likely do not want to use this method unless you are deserializing
    /// the access token using custom deserialization or creating your own access tokens
    /// manually.
    ///
    /// This method resets the access token timestamp based on the expires_in field
    /// which is the total seconds that the access token is valid for starting
    /// from when the token was first retrieved.
    ///
    /// This will reset the the timestamp from Utc Now + expires_in. This means
    /// that if calling [MsalToken::gen_timestamp] will only be reliable if done
    /// when the access token is first retrieved.
    ///
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::MsalToken;
    ///
    /// let mut access_token = MsalToken::default();
    /// access_token.expires_in = 86999;
    /// access_token.gen_timestamp();
    /// println!("{:#?}", access_token.timestamp);
    /// // The timestamp is in UTC.
    /// ```
    pub fn gen_timestamp(&mut self) {
        let timestamp = time::OffsetDateTime::now_utc();
        let expires_on = timestamp.add(time::Duration::seconds(self.expires_in.clone()));
        self.timestamp = Some(timestamp);
        self.expires_on = Some(expires_on);
    }

    /// Check whether the access token is expired. Uses the expires_in
    /// field to check time elapsed since token was first deserialized.
    /// This is done using a Utc timestamp set when the [MsalToken] is
    /// deserialized from the api response
    ///
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::MsalToken;
    ///
    /// let mut access_token = MsalToken::default();
    /// println!("{:#?}", access_token.is_expired());
    /// ```
    pub fn is_expired(&self) -> bool {
        if let Some(expires_on) = self.expires_on.as_ref() {
            expires_on.lt(&OffsetDateTime::now_utc())
        } else {
            false
        }
    }

    /// Get the time left in seconds until the access token expires.
    /// See the HumanTime crate. If you just need to know if the access token
    /// is expired then use the is_expired() message which returns a boolean
    /// true for the token has expired and false otherwise.
    ///
    /// # Example
    /// ```
    /// # use graph_extensions::token::MsalToken;
    ///
    /// let mut access_token = MsalToken::default();
    /// println!("{:#?}", access_token.elapsed());
    /// ```
    pub fn elapsed(&self) -> Option<time::Duration> {
        Some(self.expires_on? - self.timestamp?)
    }
}

impl Default for MsalToken {
    fn default() -> Self {
        MsalToken {
            token_type: String::new(),
            expires_in: 0,
            ext_expires_in: None,
            scope: vec![],
            access_token: String::new(),
            refresh_token: None,
            user_id: None,
            id_token: None,
            state: None,
            correlation_id: None,
            client_info: None,
            timestamp: Some(time::OffsetDateTime::now_utc()),
            expires_on: Some(
                time::OffsetDateTime::from_unix_timestamp(0)
                    .unwrap_or(time::OffsetDateTime::UNIX_EPOCH),
            ),
            additional_fields: Default::default(),
            log_pii: false,
        }
    }
}

impl TryFrom<&str> for MsalToken {
    type Error = GraphFailure;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(serde_json::from_str(value)?)
    }
}

impl TryFrom<reqwest::blocking::RequestBuilder> for MsalToken {
    type Error = GraphFailure;

    fn try_from(value: reqwest::blocking::RequestBuilder) -> Result<Self, Self::Error> {
        let response = value.send()?;
        MsalToken::try_from(response)
    }
}

impl TryFrom<Result<reqwest::blocking::Response, reqwest::Error>> for MsalToken {
    type Error = GraphFailure;

    fn try_from(
        value: Result<reqwest::blocking::Response, reqwest::Error>,
    ) -> Result<Self, Self::Error> {
        let response = value?;
        MsalToken::try_from(response)
    }
}

impl TryFrom<reqwest::blocking::Response> for MsalToken {
    type Error = GraphFailure;

    fn try_from(value: reqwest::blocking::Response) -> Result<Self, Self::Error> {
        Ok(value.json::<MsalToken>()?)
    }
}

impl fmt::Debug for MsalToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.log_pii {
            f.debug_struct("MsalAccessToken")
                .field("bearer_token", &self.access_token)
                .field("refresh_token", &self.refresh_token)
                .field("token_type", &self.token_type)
                .field("expires_in", &self.expires_in)
                .field("scope", &self.scope)
                .field("user_id", &self.user_id)
                .field("id_token", &self.id_token)
                .field("state", &self.state)
                .field("timestamp", &self.timestamp)
                .field("expires_on", &self.expires_on)
                .field("additional_fields", &self.additional_fields)
                .finish()
        } else {
            f.debug_struct("MsalAccessToken")
                .field(
                    "bearer_token",
                    &"[REDACTED]  - call enable_pii_logging(true) to log value",
                )
                .field(
                    "refresh_token",
                    &"[REDACTED] - call enable_pii_logging(true) to log value",
                )
                .field("token_type", &self.token_type)
                .field("expires_in", &self.expires_in)
                .field("scope", &self.scope)
                .field("user_id", &self.user_id)
                .field(
                    "id_token",
                    &"[REDACTED] - call enable_pii_logging(true) to log value",
                )
                .field("state", &self.state)
                .field("timestamp", &self.timestamp)
                .field("expires_on", &self.expires_on)
                .field("additional_fields", &self.additional_fields)
                .finish()
        }
    }
}

impl AsRef<str> for MsalToken {
    fn as_ref(&self) -> &str {
        self.access_token.as_str()
    }
}

impl<'de> Deserialize<'de> for MsalToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let phantom_access_token: PhantomMsalToken = Deserialize::deserialize(deserializer)?;

        let timestamp = time::OffsetDateTime::now_utc();
        let expires_on = timestamp.add(time::Duration::seconds(phantom_access_token.expires_in));

        Ok(MsalToken {
            access_token: phantom_access_token.access_token,
            token_type: phantom_access_token.token_type,
            expires_in: phantom_access_token.expires_in,
            ext_expires_in: phantom_access_token.ext_expires_in,
            scope: phantom_access_token.scope,
            refresh_token: phantom_access_token.refresh_token,
            user_id: phantom_access_token.user_id,
            id_token: phantom_access_token.id_token,
            state: phantom_access_token.state,
            correlation_id: phantom_access_token.correlation_id,
            client_info: phantom_access_token.client_info,
            timestamp: Some(timestamp),
            expires_on: Some(expires_on),
            additional_fields: phantom_access_token.additional_fields,
            log_pii: false,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn is_expired_test() {
        let mut access_token = MsalToken::default();
        access_token.with_expires_in(1);
        std::thread::sleep(std::time::Duration::from_secs(3));
        assert!(access_token.is_expired());

        let mut access_token = MsalToken::default();
        access_token.with_expires_in(8);
        std::thread::sleep(std::time::Duration::from_secs(4));
        assert!(!access_token.is_expired());
    }

    pub const ACCESS_TOKEN_INT: &str = r#"{
        "access_token": "fasdfasdfasfdasdfasfsdf",
        "token_type": "Bearer",
        "expires_in": 65874,
        "scope": null,
        "refresh_token": null,
        "user_id": "santa@north.pole.com",
        "id_token": "789aasdf-asdf",
        "state": null,
        "timestamp": "2020-10-27T16:31:38.788098400Z"
    }"#;

    pub const ACCESS_TOKEN_STRING: &str = r#"{
        "access_token": "fasdfasdfasfdasdfasfsdf",
        "token_type": "Bearer",
        "expires_in": "65874",
        "scope": null,
        "refresh_token": null,
        "user_id": "helpers@north.pole.com",
        "id_token": "789aasdf-asdf",
        "state": null,
        "timestamp": "2020-10-27T16:31:38.788098400Z"
    }"#;

    #[test]
    pub fn test_deserialize() {
        let _token: MsalToken = serde_json::from_str(ACCESS_TOKEN_INT).unwrap();
        let _token: MsalToken = serde_json::from_str(ACCESS_TOKEN_STRING).unwrap();
    }
}
