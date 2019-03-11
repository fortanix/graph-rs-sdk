use serde;

use crate::oauth::OAuthError;

pub trait WellKnown {
    fn signing_keys<T>(url: &str) -> Result<T, OAuthError>
    where
        T: serde::Serialize,
        for<'de> T: serde::Deserialize<'de>;
}

#[derive(Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Commons;

impl WellKnown for Commons {
    fn signing_keys<T>(url: &str) -> Result<T, OAuthError>
    where
        T: serde::Serialize,
        for<'de> T: serde::Deserialize<'de>,
    {
        let client = reqwest::Client::builder().build()?;
        let response = client.get(url).send();

        match response {
            Ok(mut t) => {
                let keys: T = t.json()?;
                Ok(keys)
            },
            Err(e) => Err(OAuthError::from(e)),
        }
    }
}