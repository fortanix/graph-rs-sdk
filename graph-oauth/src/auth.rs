use crate::access_token::MsalTokenResponse;
use crate::grants::{GrantRequest, GrantType};
use crate::id_token::IdToken;
use crate::identity::{AsQuery, Authority, AzureCloudInstance, Prompt};
use crate::oauth::ResponseType;
use crate::oauth_error::OAuthError;
use crate::strum::IntoEnumIterator;
use base64::Engine;
use graph_error::{AuthorizationFailure, AuthorizationResult, GraphFailure, GraphResult, AF};
use ring::rand::SecureRandom;
use std::collections::btree_map::{BTreeMap, Entry};
use std::collections::{BTreeSet, HashMap};
use std::default::Default;
use std::fmt;
use std::marker::PhantomData;
use url::form_urlencoded::Serializer;
use url::Url;

/// Fields that represent common OAuth credentials.
#[derive(
    Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, EnumIter,
)]
pub enum OAuthParameter {
    ClientId,
    ClientSecret,
    AuthorizationUrl,
    AccessTokenUrl,
    RefreshTokenUrl,
    RedirectUri,
    AuthorizationCode,
    AccessToken,
    RefreshToken,
    ResponseMode,
    State,
    SessionState,
    ResponseType,
    GrantType,
    Nonce,
    Prompt,
    IdToken,
    Resource,
    DomainHint,
    Scope,
    LoginHint,
    ClientAssertion,
    ClientAssertionType,
    CodeVerifier,
    CodeChallenge,
    CodeChallengeMethod,
    PostLogoutRedirectURI,
    LogoutURL,
    AdminConsent,
    Username,
    Password,
    DeviceCode,
}

impl OAuthParameter {
    pub fn alias(self) -> &'static str {
        match self {
            OAuthParameter::ClientId => "client_id",
            OAuthParameter::ClientSecret => "client_secret",
            OAuthParameter::AuthorizationUrl => "authorization_url",
            OAuthParameter::AccessTokenUrl => "access_token_url",
            OAuthParameter::RefreshTokenUrl => "refresh_token_url",
            OAuthParameter::RedirectUri => "redirect_uri",
            OAuthParameter::AuthorizationCode => "code",
            OAuthParameter::AccessToken => "access_token",
            OAuthParameter::RefreshToken => "refresh_token",
            OAuthParameter::ResponseMode => "response_mode",
            OAuthParameter::ResponseType => "response_type",
            OAuthParameter::State => "state",
            OAuthParameter::SessionState => "session_state",
            OAuthParameter::GrantType => "grant_type",
            OAuthParameter::Nonce => "nonce",
            OAuthParameter::Prompt => "prompt",
            OAuthParameter::IdToken => "id_token",
            OAuthParameter::Resource => "resource",
            OAuthParameter::DomainHint => "domain_hint",
            OAuthParameter::Scope => "scope",
            OAuthParameter::LoginHint => "login_hint",
            OAuthParameter::ClientAssertion => "client_assertion",
            OAuthParameter::ClientAssertionType => "client_assertion_type",
            OAuthParameter::CodeVerifier => "code_verifier",
            OAuthParameter::CodeChallenge => "code_challenge",
            OAuthParameter::CodeChallengeMethod => "code_challenge_method",
            OAuthParameter::LogoutURL => "logout_url",
            OAuthParameter::PostLogoutRedirectURI => "post_logout_redirect_uri",
            OAuthParameter::AdminConsent => "admin_consent",
            OAuthParameter::Username => "username",
            OAuthParameter::Password => "password",
            OAuthParameter::DeviceCode => "device_code",
        }
    }

    fn is_debug_redacted(&self) -> bool {
        matches!(
            self,
            OAuthParameter::ClientId
                | OAuthParameter::ClientSecret
                | OAuthParameter::AccessToken
                | OAuthParameter::RefreshToken
                | OAuthParameter::IdToken
                | OAuthParameter::CodeVerifier
                | OAuthParameter::CodeChallenge
                | OAuthParameter::Password
                | OAuthParameter::AuthorizationCode
        )
    }
}

impl ToString for OAuthParameter {
    fn to_string(&self) -> String {
        self.alias().to_string()
    }
}

impl AsRef<str> for OAuthParameter {
    fn as_ref(&self) -> &'static str {
        self.alias()
    }
}

/// Serializer for query/x-www-form-urlencoded OAuth requests.
///
/// OAuth Serializer for query/form serialization that supports the OAuth 2.0 and OpenID
/// Connect protocols on Microsoft identity platform.
///
/// # Example
/// ```
/// use graph_oauth::oauth::OAuthSerializer;
/// let oauth = OAuthSerializer::new();
/// ```
#[derive(Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct OAuthSerializer {
    access_token: Option<MsalTokenResponse>,
    scopes: BTreeSet<String>,
    credentials: BTreeMap<String, String>,
}

impl OAuthSerializer {
    /// Create a new OAuth instance.
    ///
    /// # Example
    /// ```
    /// use graph_oauth::oauth::{OAuthSerializer, GrantType};
    ///
    /// let mut oauth = OAuthSerializer::new();
    /// ```
    pub fn new() -> OAuthSerializer {
        OAuthSerializer {
            access_token: None,
            scopes: BTreeSet::new(),
            credentials: BTreeMap::new(),
        }
    }

    /// Insert oauth credentials using the OAuthParameter enum.
    /// This method is used internally for each of the setter methods.
    /// Callers can optionally use this method to set credentials instead
    /// of the individual setter methods.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # use graph_oauth::oauth::OAuthParameter;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.insert(OAuthParameter::AuthorizationUrl, "https://example.com");
    /// assert!(oauth.contains(OAuthParameter::AuthorizationUrl));
    /// println!("{:#?}", oauth.get(OAuthParameter::AuthorizationUrl));
    /// ```
    pub fn insert<V: ToString>(&mut self, oac: OAuthParameter, value: V) -> &mut OAuthSerializer {
        let v = value.to_string();
        match oac {
            OAuthParameter::RefreshTokenUrl
            | OAuthParameter::PostLogoutRedirectURI
            | OAuthParameter::AccessTokenUrl
            | OAuthParameter::AuthorizationUrl
            | OAuthParameter::LogoutURL => {
                Url::parse(v.as_ref())
                    .map_err(|_| AF::msg_internal_err("authorization_url | refresh_token_url"))
                    .unwrap();
            }
            _ => {}
        }

        self.credentials.insert(oac.to_string(), v);
        self
    }

    /// Insert and OAuth credential using the entry trait and
    /// returning the credential. This internally calls
    /// `entry.(OAuthParameter).or_insret_with(value)`.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # use graph_oauth::oauth::OAuthParameter;
    /// # let mut oauth = OAuthSerializer::new();
    /// let entry = oauth.entry_with(OAuthParameter::AuthorizationUrl, "https://example.com");
    /// assert_eq!(entry, "https://example.com")
    /// ```
    pub fn entry_with<V: ToString>(&mut self, oac: OAuthParameter, value: V) -> &mut String {
        let v = value.to_string();
        match oac {
            OAuthParameter::RefreshTokenUrl
            | OAuthParameter::PostLogoutRedirectURI
            | OAuthParameter::AccessTokenUrl
            | OAuthParameter::AuthorizationUrl
            | OAuthParameter::LogoutURL => {
                Url::parse(v.as_ref())
                    .map_err(|_| AF::msg_internal_err("authorization_url | refresh_token_url"))
                    .unwrap();
            }
            _ => {}
        }

        self.credentials
            .entry(oac.alias().to_string())
            .or_insert_with(|| v)
    }

    /// A view into a single entry in a map, which may either be vacant or occupied.
    ///
    /// This `enum` is constructed from the [`entry`] method on [`BTreeMap`].
    ///
    /// [`entry`]: BTreeMap::entry
    pub fn entry<V: ToString>(&mut self, oac: OAuthParameter) -> Entry<String, String> {
        self.credentials.entry(oac.alias().to_string())
    }

    /// Get a previously set credential.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # use graph_oauth::oauth::OAuthParameter;
    /// # let mut oauth = OAuthSerializer::new();
    /// let a = oauth.get(OAuthParameter::AuthorizationUrl);
    /// ```
    pub fn get(&self, oac: OAuthParameter) -> Option<String> {
        self.credentials.get(oac.alias()).cloned()
    }

    /// Check if an OAuth credential has already been set.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # use graph_oauth::oauth::OAuthParameter;
    /// # let mut oauth = OAuthSerializer::new();
    /// println!("{:#?}", oauth.contains(OAuthParameter::Nonce));
    /// ```
    pub fn contains(&self, t: OAuthParameter) -> bool {
        if t == OAuthParameter::Scope {
            return !self.scopes.is_empty();
        }
        self.credentials.contains_key(t.alias())
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.credentials.contains_key(key)
    }

    /// Remove a field from OAuth.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # use graph_oauth::oauth::OAuthParameter;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.client_id("client_id");
    ///
    /// assert_eq!(oauth.contains(OAuthParameter::ClientId), true);
    /// oauth.remove(OAuthParameter::ClientId);
    ///
    /// assert_eq!(oauth.contains(OAuthParameter::ClientId), false);
    /// ```
    pub fn remove(&mut self, oac: OAuthParameter) -> &mut OAuthSerializer {
        self.credentials.remove(oac.alias());
        self
    }

    /// Set the client id for an OAuth request.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # use graph_oauth::oauth::OAuthParameter;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.client_id("client_id");
    /// ```
    pub fn client_id(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::ClientId, value)
    }

    /// Set the state for an OAuth request.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # use graph_oauth::oauth::OAuthParameter;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.state("1234");
    /// ```
    pub fn state(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::State, value)
    }

    /// Set the client secret for an OAuth request.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.client_secret("client_secret");
    /// ```
    pub fn client_secret(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::ClientSecret, value)
    }

    /// Set the authorization URL.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.authorization_url("https://example.com/authorize");
    /// ```
    pub fn authorization_url(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::AuthorizationUrl, value)
    }

    /// Set the access token url of a request for OAuth
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.access_token_url("https://example.com/token");
    /// ```
    pub fn access_token_url(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::AccessTokenUrl, value)
    }

    /// Set the refresh token url of a request for OAuth
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.refresh_token_url("https://example.com/token");
    /// ```
    pub fn refresh_token_url(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::RefreshTokenUrl, value)
    }

    /// Set the authorization, access token, and refresh token URL
    /// for OAuth based on a tenant id.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.tenant_id("tenant_id");
    /// ```
    pub fn tenant_id(&mut self, value: &str) -> &mut OAuthSerializer {
        let token_url = format!("https://login.microsoftonline.com/{value}/oauth2/v2.0/token",);
        let auth_url = format!("https://login.microsoftonline.com/{value}/oauth2/v2.0/authorize",);

        self.authorization_url(&auth_url)
            .access_token_url(&token_url)
            .refresh_token_url(&token_url)
    }

    /// Set the authorization, access token, and refresh token URL
    /// for OAuth based on a tenant id.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.tenant_id("tenant_id");
    /// ```
    pub fn authority(
        &mut self,
        host: &AzureCloudInstance,
        authority: &Authority,
    ) -> &mut OAuthSerializer {
        if host.eq(&AzureCloudInstance::OneDriveAndSharePoint) {
            return self.legacy_authority();
        }

        let token_url = format!("{}/{}/oauth2/v2.0/token", host.as_ref(), authority.as_ref());
        let auth_url = format!(
            "{}/{}/oauth2/v2.0/authorize",
            host.as_ref(),
            authority.as_ref()
        );

        self.authorization_url(&auth_url)
            .access_token_url(&token_url)
            .refresh_token_url(&token_url)
    }

    pub fn authority_admin_consent(
        &mut self,
        host: &AzureCloudInstance,
        authority: &Authority,
    ) -> &mut OAuthSerializer {
        let token_url = format!("{}/{}/oauth2/v2.0/token", host.as_ref(), authority.as_ref());
        let auth_url = format!("{}/{}/adminconsent", host.as_ref(), authority.as_ref());

        self.authorization_url(&auth_url)
            .access_token_url(&token_url)
            .refresh_token_url(&token_url)
    }

    pub fn legacy_authority(&mut self) -> &mut OAuthSerializer {
        self.authorization_url(AzureCloudInstance::OneDriveAndSharePoint.as_ref());
        self.access_token_url(AzureCloudInstance::OneDriveAndSharePoint.as_ref());
        self.refresh_token_url(AzureCloudInstance::OneDriveAndSharePoint.as_ref())
    }

    /// Set the redirect url of a request
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.redirect_uri("https://localhost:8888/redirect");
    /// ```
    pub fn redirect_uri(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::RedirectUri, value)
    }

    /// Set the access code.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.authorization_code("LDSF[POK43");
    /// ```
    pub fn authorization_code(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::AuthorizationCode, value)
    }

    /// Set the response mode.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.response_mode("query");
    /// ```
    pub fn response_mode(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::ResponseMode, value)
    }

    /// Set the response type.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.response_type("token");
    /// ```
    pub fn response_type<T: ToString>(&mut self, value: T) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::ResponseType, value)
    }

    pub fn response_types(
        &mut self,
        value: std::collections::btree_set::Iter<'_, ResponseType>,
    ) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::ResponseType, value.as_query())
    }

    /// Set the nonce.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    ///
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.nonce("1234");
    /// ```
    pub fn nonce(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::Nonce, value)
    }

    /// Set the prompt for open id.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    ///
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.prompt("login");
    /// ```
    pub fn prompt(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::Prompt, value)
    }

    pub fn prompts(&mut self, value: &[Prompt]) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::Prompt, value.to_vec().as_query())
    }

    /// Set id token for open id.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::{OAuthSerializer, IdToken};
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.id_token(IdToken::new("1345", "code", "state", "session_state"));
    /// ```
    pub fn id_token(&mut self, value: IdToken) -> &mut OAuthSerializer {
        if let Some(code) = value.get_code() {
            self.authorization_code(code.as_str());
        }
        if let Some(state) = value.get_state() {
            let _ = self.entry_with(OAuthParameter::State, state.as_str());
        }
        if let Some(session_state) = value.get_session_state() {
            self.session_state(session_state.as_str());
        }
        self.insert(OAuthParameter::IdToken, value.get_id_token().as_str())
    }

    /// Set the session state.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.session_state("session-state");
    /// ```
    pub fn session_state(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::SessionState, value)
    }

    /// Set the grant_type.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.grant_type("token");
    /// ```
    pub fn grant_type(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::GrantType, value)
    }

    /// Set the resource.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.resource("resource");
    /// ```
    pub fn resource(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::Resource, value)
    }

    /// Set the code verifier.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.code_verifier("code_verifier");
    /// ```
    pub fn code_verifier(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::CodeVerifier, value)
    }

    /// Set the domain hint.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.domain_hint("domain_hint");
    /// ```
    pub fn domain_hint(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::DomainHint, value)
    }

    /// Set the code challenge.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.code_challenge("code_challenge");
    /// ```
    pub fn code_challenge(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::CodeChallenge, value)
    }

    /// Set the code challenge method.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.code_challenge_method("code_challenge_method");
    /// ```
    pub fn code_challenge_method(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::CodeChallengeMethod, value)
    }

    /// Generate a code challenge and code verifier for the
    /// authorization code grant flow using proof key for
    /// code exchange (PKCE) and SHA256.
    ///
    /// This method automatically sets the code_verifier,
    /// code_challenge, and code_challenge_method fields.
    ///
    /// For authorization, the code_challenge_method parameter in the request body
    /// is automatically set to 'S256'.
    ///
    /// Internally this method uses the Rust ring cyrpto library to
    /// generate a secure random 32-octet sequence that is base64 URL
    /// encoded (no padding). This sequence is hashed using SHA256 and
    /// base64 URL encoded (no padding) resulting in a 43-octet URL safe string.
    ///
    ///
    /// For more info on PKCE and entropy see: <https://tools.ietf.org/html/rfc7519#section-7.2>
    ///
    /// # Example
    /// ```
    /// # use base64::Engine;
    /// use graph_oauth::oauth::OAuthSerializer;
    /// use graph_oauth::oauth::OAuthParameter;
    ///
    /// let mut oauth = OAuthSerializer::new();
    /// oauth.generate_sha256_challenge_and_verifier().unwrap();
    ///
    /// # assert!(oauth.contains(OAuthParameter::CodeChallenge));
    /// # assert!(oauth.contains(OAuthParameter::CodeVerifier));
    /// # assert!(oauth.contains(OAuthParameter::CodeChallengeMethod));
    /// println!("Code Challenge: {:#?}", oauth.get(OAuthParameter::CodeChallenge));
    /// println!("Code Verifier: {:#?}", oauth.get(OAuthParameter::CodeVerifier));
    /// println!("Code Challenge Method: {:#?}", oauth.get(OAuthParameter::CodeChallengeMethod));
    ///
    /// # let challenge = oauth.get(OAuthParameter::CodeChallenge).unwrap();
    /// # let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    /// # context.update(oauth.get(OAuthParameter::CodeVerifier).unwrap().as_bytes());
    /// # let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(context.finish().as_ref());
    /// # assert_eq!(challenge, verifier);
    /// ```
    pub fn generate_sha256_challenge_and_verifier(&mut self) -> Result<(), GraphFailure> {
        let mut buf = [0; 32];
        let rng = ring::rand::SystemRandom::new();
        rng.fill(&mut buf)?;
        let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf);
        let mut context = ring::digest::Context::new(&ring::digest::SHA256);
        context.update(verifier.as_bytes());
        let code_challenge =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(context.finish().as_ref());

        self.code_verifier(&verifier);
        self.code_challenge(&code_challenge);
        self.code_challenge_method("S256");
        Ok(())
    }

    /// Set the login hint.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.login_hint("login_hint");
    /// ```
    pub fn login_hint(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::LoginHint, value)
    }

    /// Set the client assertion.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.client_assertion("client_assertion");
    /// ```
    pub fn client_assertion(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::ClientAssertion, value)
    }

    /// Set the client assertion type.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.client_assertion_type("client_assertion_type");
    /// ```
    pub fn client_assertion_type(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::ClientAssertionType, value)
    }

    /// Set the url to send a post request that will log out the user.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.logout_url("https://example.com/logout?");
    /// ```
    pub fn logout_url(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::LogoutURL, value)
    }

    /// Set the redirect uri that user will be redirected to after logging out.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.post_logout_redirect_uri("http://localhost:8080");
    /// ```
    pub fn post_logout_redirect_uri(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::PostLogoutRedirectURI, value)
    }

    /// Set the redirect uri that user will be redirected to after logging out.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::{OAuthSerializer, OAuthParameter};
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.username("user");
    /// assert!(oauth.contains(OAuthParameter::Username))
    /// ```
    pub fn username(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::Username, value)
    }

    /// Set the redirect uri that user will be redirected to after logging out.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::{OAuthSerializer, OAuthParameter};
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.password("user");
    /// assert!(oauth.contains(OAuthParameter::Password))
    /// ```
    pub fn password(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::Password, value)
    }

    pub fn refresh_token(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::RefreshToken, value)
    }

    /// Set the device code for the device authorization flow.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::{OAuthSerializer, OAuthParameter};
    /// # let mut oauth = OAuthSerializer::new();
    /// oauth.device_code("device_code");
    /// assert!(oauth.contains(OAuthParameter::DeviceCode))
    /// ```
    pub fn device_code(&mut self, value: &str) -> &mut OAuthSerializer {
        self.insert(OAuthParameter::DeviceCode, value)
    }

    /// Add a scope' for the OAuth URL.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    ///
    /// oauth.add_scope("Sites.Read")
    ///     .add_scope("Sites.ReadWrite")
    ///     .add_scope("Sites.ReadWrite.All");
    /// assert_eq!(oauth.join_scopes(" "), "Sites.Read Sites.ReadWrite Sites.ReadWrite.All");
    /// ```
    pub fn add_scope<T: ToString>(&mut self, scope: T) -> &mut OAuthSerializer {
        self.scopes.insert(scope.to_string());
        self
    }

    /// Get the scopes.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// let mut oauth = OAuthSerializer::new();
    /// oauth.add_scope("Files.Read");
    /// oauth.add_scope("Files.ReadWrite");
    ///
    /// let scopes = oauth.get_scopes();
    /// assert!(scopes.contains("Files.Read"));
    /// assert!(scopes.contains("Files.ReadWrite"));
    /// ```
    pub fn get_scopes(&self) -> &BTreeSet<String> {
        &self.scopes
    }

    /// Join scopes.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    ///
    /// // the scopes take a separator just like Vec join.
    ///  let s = oauth.join_scopes(" ");
    /// println!("{:#?}", s);
    /// ```
    pub fn join_scopes(&self, sep: &str) -> String {
        self.scopes
            .iter()
            .map(|s| &**s)
            .collect::<Vec<&str>>()
            .join(sep)
    }

    /// Extend scopes.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # use std::collections::HashSet;
    /// # let mut oauth = OAuthSerializer::new();
    ///
    /// let scopes = vec!["Files.Read", "Files.ReadWrite"];
    /// oauth.extend_scopes(&scopes);
    ///
    /// assert_eq!(oauth.join_scopes(" "), "Files.Read Files.ReadWrite");
    /// ```
    pub fn extend_scopes<T: ToString, I: IntoIterator<Item = T>>(&mut self, iter: I) -> &mut Self {
        self.scopes.extend(iter.into_iter().map(|s| s.to_string()));
        self
    }

    /// Check if OAuth contains a specific scope.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    ///
    /// oauth.add_scope("Files.Read");
    /// assert_eq!(oauth.contains_scope("Files.Read"), true);
    ///
    /// // Or using static scopes
    /// oauth.add_scope("File.ReadWrite");
    /// assert!(oauth.contains_scope("File.ReadWrite"));
    /// ```
    pub fn contains_scope<T: ToString>(&self, scope: T) -> bool {
        self.scopes.contains(&scope.to_string())
    }

    /// Remove a previously added scope.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    ///
    /// oauth.add_scope("scope");
    /// # assert!(oauth.contains_scope("scope"));
    /// oauth.remove_scope("scope");
    /// # assert!(!oauth.contains_scope("scope"));
    /// ```
    pub fn remove_scope<T: AsRef<str>>(&mut self, scope: T) {
        self.scopes.remove(scope.as_ref());
    }

    /// Remove all scopes.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    ///
    /// oauth.add_scope("Files.Read").add_scope("Files.ReadWrite");
    /// assert_eq!("Files.Read Files.ReadWrite", oauth.join_scopes(" "));
    ///
    /// oauth.clear_scopes();
    /// assert!(oauth.get_scopes().is_empty());
    /// ```
    pub fn clear_scopes(&mut self) {
        self.scopes.clear();
    }

    /// Set the access token.
    ///
    /// # Example
    /// ```
    /// use graph_oauth::oauth::OAuthSerializer;
    /// use graph_oauth::oauth::MsalTokenResponse;
    /// let mut oauth = OAuthSerializer::new();
    /// let access_token = MsalTokenResponse::default();
    /// oauth.access_token(access_token);
    /// ```
    pub fn access_token(&mut self, ac: MsalTokenResponse) {
        if let Some(refresh_token) = ac.refresh_token.as_ref() {
            self.refresh_token(refresh_token.as_str());
        }
        self.access_token.replace(ac);
    }

    /// Get the access token.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # use graph_oauth::oauth::MsalTokenResponse;
    /// # let access_token = MsalTokenResponse::default();
    /// # let mut oauth = OAuthSerializer::new();
    /// # oauth.access_token(access_token);
    /// let access_token = oauth.get_access_token().unwrap();
    /// println!("{:#?}", access_token);
    /// ```
    pub fn get_access_token(&self) -> Option<MsalTokenResponse> {
        self.access_token.clone()
    }

    /// Get the refrsh token. This method returns the current refresh
    /// token stored in OAuth and does not make a request for a refresh
    /// token.
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # use graph_oauth::oauth::MsalTokenResponse;
    /// # let mut oauth = OAuthSerializer::new();
    /// let mut  access_token = MsalTokenResponse::default();
    /// access_token.set_refresh_token("refresh_token");
    /// oauth.access_token(access_token);
    ///
    /// let refresh_token = oauth.get_refresh_token().unwrap();
    /// println!("{:#?}", refresh_token);
    /// ```
    pub fn get_refresh_token(&self) -> GraphResult<String> {
        if let Some(refresh_token) = self.get(OAuthParameter::RefreshToken) {
            return Ok(refresh_token);
        }

        match self.get_access_token() {
            Some(token) => match token.refresh_token {
                Some(t) => Ok(t),
                None => OAuthError::error_from::<String>(OAuthParameter::RefreshToken),
            },
            None => OAuthError::error_from::<String>(OAuthParameter::RefreshToken),
        }
    }

    pub fn build(&mut self) -> GrantSelector<AccessTokenGrant> {
        GrantSelector {
            oauth: self.clone(),
            t: PhantomData,
        }
    }

    pub fn build_async(&mut self) -> GrantSelector<AsyncAccessTokenGrant> {
        GrantSelector {
            oauth: self.clone(),
            t: PhantomData,
        }
    }
}

impl OAuthSerializer {
    pub fn get_or_else(&self, c: OAuthParameter) -> GraphResult<String> {
        self.get(c).ok_or_else(|| OAuthError::credential_error(c))
    }

    pub fn ok_or(&self, oac: &OAuthParameter) -> AuthorizationResult<String> {
        self.get(*oac).ok_or(AuthorizationFailure::required(oac))
    }

    pub fn try_as_tuple(&self, oac: &OAuthParameter) -> AuthorizationResult<(String, String)> {
        if oac.eq(&OAuthParameter::Scope) {
            if self.scopes.is_empty() {
                return Err(AuthorizationFailure::required(oac));
            }
            Ok((oac.alias().to_owned(), self.join_scopes(" ")))
        } else {
            Ok((
                oac.alias().to_owned(),
                self.get(*oac).ok_or(AuthorizationFailure::required(oac))?,
            ))
        }
    }

    pub fn form_encode_credentials(
        &mut self,
        pairs: Vec<OAuthParameter>,
        encoder: &mut Serializer<String>,
    ) {
        pairs
            .iter()
            .filter(|oac| (self.contains_key(oac.alias()) || oac.alias().eq("scope")))
            .for_each(|oac| {
                if oac.alias().eq("scope") && !self.scopes.is_empty() {
                    encoder.append_pair("scope", self.join_scopes(" ").as_str());
                } else if let Some(val) = self.get(*oac) {
                    encoder.append_pair(oac.alias(), val.as_str());
                }
            });
    }

    pub fn encode_query(
        &mut self,
        optional_fields: Vec<OAuthParameter>,
        required_fields: Vec<OAuthParameter>,
        encoder: &mut Serializer<String>,
    ) -> AuthorizationResult<()> {
        for parameter in required_fields {
            if parameter.alias().eq("scope") {
                if self.scopes.is_empty() {
                    return AuthorizationFailure::result::<()>(parameter.alias());
                } else {
                    encoder.append_pair("scope", self.join_scopes(" ").as_str());
                }
            } else {
                let value = self
                    .get(parameter)
                    .ok_or(AuthorizationFailure::required(parameter))?;

                encoder.append_pair(parameter.alias(), value.as_str());
            }
        }

        for parameter in optional_fields {
            if parameter.alias().eq("scope") && !self.scopes.is_empty() {
                encoder.append_pair("scope", self.join_scopes(" ").as_str());
            } else if let Some(val) = self.get(parameter) {
                encoder.append_pair(parameter.alias(), val.as_str());
            }
        }

        Ok(())
    }

    pub fn params(&mut self, pairs: Vec<OAuthParameter>) -> GraphResult<HashMap<String, String>> {
        let mut map: HashMap<String, String> = HashMap::new();
        for oac in pairs.iter() {
            if oac.eq(&OAuthParameter::RefreshToken) {
                if let Some(val) = self.get(*oac) {
                    map.insert(oac.to_string(), val);
                } else {
                    map.insert("refresh_token".into(), self.get_refresh_token()?);
                }
            } else if oac.alias().eq("scope") && !self.scopes.is_empty() {
                map.insert("scope".into(), self.join_scopes(" "));
            } else if let Some(val) = self.get(*oac) {
                map.insert(oac.to_string(), val);
            }
        }
        Ok(map)
    }

    pub fn as_credential_map(
        &mut self,
        optional_fields: Vec<OAuthParameter>,
        required_fields: Vec<OAuthParameter>,
    ) -> AuthorizationResult<HashMap<String, String>> {
        let mut required_map = required_fields
            .iter()
            .map(|oac| self.try_as_tuple(oac))
            .collect::<AuthorizationResult<HashMap<String, String>>>()?;

        let optional_map: HashMap<String, String> = optional_fields
            .iter()
            .flat_map(|oac| self.try_as_tuple(oac))
            .collect();

        required_map.extend(optional_map);
        Ok(required_map)
    }

    pub fn encode_uri(
        &mut self,
        grant: GrantType,
        request_type: GrantRequest,
    ) -> GraphResult<String> {
        let mut encoder = Serializer::new(String::new());
        match grant {
			GrantType::TokenFlow =>
				match request_type {
					GrantRequest::Authorization => {
						let _ = self.entry_with(OAuthParameter::ResponseType, "token");
						self.form_encode_credentials(GrantType::TokenFlow.available_credentials(GrantRequest::Authorization), &mut encoder);
						let mut url = self.get_or_else(OAuthParameter::AuthorizationUrl)?;
						if !url.ends_with('?') {
							url.push('?');
						}
						url.push_str(encoder.finish().as_str());
						Ok(url)
					}
					GrantRequest::AccessToken | GrantRequest::RefreshToken => {
						OAuthError::grant_error(
							GrantType::TokenFlow,
							GrantRequest::AccessToken,
							"Grant type does not use request type. Please use OAuth::request_authorization() for browser requests"
						)
					}
				}
			GrantType::CodeFlow =>
				match request_type {
					GrantRequest::Authorization => {
						let _ = self.entry_with(OAuthParameter::ResponseType, "code");
						let _ = self.entry_with(OAuthParameter::ResponseMode, "query");
						self.form_encode_credentials(GrantType::CodeFlow.available_credentials(GrantRequest::Authorization), &mut encoder);

						let mut url = self.get_or_else(OAuthParameter::AuthorizationUrl)?;
						if !url.ends_with('?') {
							url.push('?');
						}
						url.push_str(encoder.finish().as_str());
						Ok(url)
					}
					GrantRequest::AccessToken => {
						let _ = self.entry_with(OAuthParameter::ResponseType, "token");
						let _ = self.entry_with(OAuthParameter::GrantType, "authorization_code");
						self.form_encode_credentials(GrantType::CodeFlow.available_credentials(GrantRequest::AccessToken), &mut encoder);
						Ok(encoder.finish())
					}
					GrantRequest::RefreshToken => {
						let _ = self.entry_with(OAuthParameter::GrantType, "refresh_token");
						self.form_encode_credentials(GrantType::CodeFlow.available_credentials(GrantRequest::RefreshToken), &mut encoder);
						Ok(encoder.finish())
					}
				}
			GrantType::AuthorizationCode =>
				match request_type {
					GrantRequest::Authorization => {
						let _ = self.entry_with(OAuthParameter::ResponseType, "code");
						let _ = self.entry_with(OAuthParameter::ResponseMode, "query");
						self.form_encode_credentials(GrantType::AuthorizationCode.available_credentials(GrantRequest::Authorization), &mut encoder);
						let mut url = self.get_or_else(OAuthParameter::AuthorizationUrl)?;
						if !url.ends_with('?') {
							url.push('?');
						}
						url.push_str(encoder.finish().as_str());
						Ok(url)
					}
					GrantRequest::AccessToken | GrantRequest::RefreshToken => {
						if request_type == GrantRequest::AccessToken {
							let _ = self.entry_with(OAuthParameter::GrantType, "authorization_code");
						} else {
							let _ = self.entry_with(OAuthParameter::GrantType, "refresh_token");
						}
						self.form_encode_credentials(GrantType::AuthorizationCode.available_credentials(request_type), &mut encoder);
						Ok(encoder.finish())
					}
				}
			GrantType::Implicit =>
				match request_type {
					GrantRequest::Authorization => {
						if !self.scopes.is_empty() {
							let _ = self.entry_with(OAuthParameter::ResponseType, "token");
						}
						self.form_encode_credentials(GrantType::Implicit.available_credentials(GrantRequest::Authorization), &mut encoder);
						let mut url = self.get_or_else(OAuthParameter::AuthorizationUrl)?;
						if !url.ends_with('?') {
							url.push('?');
						}
						url.push_str(encoder.finish().as_str());
						Ok(url)
					}
					GrantRequest::AccessToken | GrantRequest::RefreshToken => {
						OAuthError::grant_error(
							GrantType::Implicit,
							GrantRequest::AccessToken,
							"Grant type does not use request type. Please use OAuth::request_authorization() for browser requests"
						)
					}
				}
            GrantType::DeviceCode =>
                match request_type {
                    GrantRequest::Authorization => {
                        self.form_encode_credentials(GrantType::DeviceCode.available_credentials(GrantRequest::Authorization), &mut encoder);

                        let mut url = self.get_or_else(OAuthParameter::AuthorizationUrl)?;
                        if !url.ends_with('?') {
                            url.push('?');
                        }
                        url.push_str(encoder.finish().as_str());
                        Ok(url)
                    }
                    GrantRequest::AccessToken => {
                        let _ = self.entry_with(OAuthParameter::GrantType, "urn:ietf:params:oauth:grant-type:device_code");
                        self.form_encode_credentials(GrantType::DeviceCode.available_credentials(GrantRequest::AccessToken), &mut encoder);
                        Ok(encoder.finish())
                    }
                    GrantRequest::RefreshToken => {
                        let _ = self.entry_with(OAuthParameter::GrantType, "refresh_token");
                        self.form_encode_credentials(GrantType::DeviceCode.available_credentials(GrantRequest::AccessToken), &mut encoder);
                        Ok(encoder.finish())
                    }
                }
			GrantType::OpenId =>
				match request_type {
					GrantRequest::Authorization => {
						self.form_encode_credentials(GrantType::OpenId.available_credentials(GrantRequest::Authorization), &mut encoder);

						let mut url = self.get_or_else(OAuthParameter::AuthorizationUrl)?;
						if !url.ends_with('?') {
							url.push('?');
						}
						url.push_str(encoder.finish().as_str());
						Ok(url)
					}
					GrantRequest::AccessToken => {
						let _ = self.entry_with(OAuthParameter::GrantType, "authorization_code");
						self.form_encode_credentials(GrantType::OpenId.available_credentials(GrantRequest::AccessToken), &mut encoder);
						Ok(encoder.finish())
					}
					GrantRequest::RefreshToken => {
						let _ = self.entry_with(OAuthParameter::GrantType, "refresh_token");
						self.form_encode_credentials(GrantType::OpenId.available_credentials(GrantRequest::RefreshToken), &mut encoder);
						Ok(encoder.finish())
					}
				}
			GrantType::ClientCredentials =>
				match request_type {
					GrantRequest::Authorization => {
						self.form_encode_credentials(GrantType::ClientCredentials.available_credentials(GrantRequest::Authorization), &mut encoder);
						let mut url = self.get_or_else(OAuthParameter::AuthorizationUrl)?;
						if !url.ends_with('?') {
							url.push('?');
						}
						url.push_str(encoder.finish().as_str());
						Ok(url)
					}
					GrantRequest::AccessToken | GrantRequest::RefreshToken => {
						self.pre_request_check(GrantType::ClientCredentials, request_type);
						self.form_encode_credentials(GrantType::ClientCredentials.available_credentials(request_type), &mut encoder);
						Ok(encoder.finish())
					}
				}
			GrantType::ResourceOwnerPasswordCredentials => {
				self.pre_request_check(GrantType::ResourceOwnerPasswordCredentials, request_type);
				self.form_encode_credentials(GrantType::ResourceOwnerPasswordCredentials.available_credentials(request_type), &mut encoder);
				Ok(encoder.finish())
			}
		}
    }

    fn pre_request_check(&mut self, grant: GrantType, request_type: GrantRequest) {
        match grant {
            GrantType::TokenFlow => {
                if request_type.eq(&GrantRequest::Authorization) {
                    let _ = self.entry_with(OAuthParameter::ResponseType, "token");
                }
            }
            GrantType::CodeFlow => match request_type {
                GrantRequest::Authorization => {
                    let _ = self.entry_with(OAuthParameter::ResponseType, "code");
                    let _ = self.entry_with(OAuthParameter::ResponseMode, "query");
                }
                GrantRequest::AccessToken => {
                    let _ = self.entry_with(OAuthParameter::ResponseType, "token");
                    let _ = self.entry_with(OAuthParameter::GrantType, "authorization_code");
                }
                GrantRequest::RefreshToken => {
                    let _ = self.entry_with(OAuthParameter::GrantType, "refresh_token");
                }
            },
            GrantType::AuthorizationCode => match request_type {
                GrantRequest::Authorization => {
                    let _ = self.entry_with(OAuthParameter::ResponseType, "code");
                    let _ = self.entry_with(OAuthParameter::ResponseMode, "query");
                }
                GrantRequest::AccessToken | GrantRequest::RefreshToken => {
                    if request_type == GrantRequest::AccessToken {
                        let _ = self.entry_with(OAuthParameter::GrantType, "authorization_code");
                    } else {
                        let _ = self.entry_with(OAuthParameter::GrantType, "refresh_token");
                    }
                }
            },
            GrantType::Implicit => {
                if request_type.eq(&GrantRequest::Authorization) && !self.scopes.is_empty() {
                    let _ = self.entry_with(OAuthParameter::ResponseType, "token");
                }
            }
            GrantType::DeviceCode => {
                if request_type.eq(&GrantRequest::AccessToken) {
                    let _ = self.entry_with(
                        OAuthParameter::GrantType,
                        "urn:ietf:params:oauth:grant-type:device_code",
                    );
                } else if request_type.eq(&GrantRequest::RefreshToken) {
                    let _ = self.entry_with(OAuthParameter::GrantType, "refresh_token");
                }
            }
            GrantType::OpenId => {
                if request_type.eq(&GrantRequest::AccessToken) {
                    let _ = self.entry_with(OAuthParameter::GrantType, "authorization_code");
                } else if request_type.eq(&GrantRequest::RefreshToken) {
                    let _ = self.entry_with(OAuthParameter::GrantType, "refresh_token");
                }
            }
            GrantType::ClientCredentials => {
                if request_type.eq(&GrantRequest::AccessToken)
                    || request_type.eq(&GrantRequest::RefreshToken)
                {
                    let _ = self.entry_with(OAuthParameter::GrantType, "client_credentials");
                }
            }
            GrantType::ResourceOwnerPasswordCredentials => {
                if request_type.eq(&GrantRequest::RefreshToken) {
                    let _ = self.entry_with(OAuthParameter::GrantType, "refresh_token");
                } else {
                    let _ = self.entry_with(OAuthParameter::GrantType, "password");
                }
            }
        }
    }
}

/// Extend the OAuth credentials.
///
/// # Example
/// ```
/// # use graph_oauth::oauth::{OAuthSerializer, OAuthParameter};
/// # use std::collections::HashMap;
/// # let mut oauth = OAuthSerializer::new();
/// let mut map: HashMap<OAuthParameter, &str> = HashMap::new();
/// map.insert(OAuthParameter::ClientId, "client_id");
/// map.insert(OAuthParameter::ClientSecret, "client_secret");
///
/// oauth.extend(map);
/// # assert_eq!(oauth.get(OAuthParameter::ClientId), Some("client_id".to_string()));
/// # assert_eq!(oauth.get(OAuthParameter::ClientSecret), Some("client_secret".to_string()));
/// ```
impl<V: ToString> Extend<(OAuthParameter, V)> for OAuthSerializer {
    fn extend<I: IntoIterator<Item = (OAuthParameter, V)>>(&mut self, iter: I) {
        iter.into_iter().for_each(|entry| {
            self.insert(entry.0, entry.1);
        });
    }
}

pub struct GrantSelector<T> {
    oauth: OAuthSerializer,
    t: PhantomData<T>,
}

impl GrantSelector<AccessTokenGrant> {
    /// Create a new instance for token flow.
    ///
    /// # See
    /// [Microsoft Token Flow Authorization](https://docs.microsoft.com/en-us/onedrive/developer/rest-api/getting-started/msa-oauth?view=odsp-graph-online)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().token_flow();
    /// ```
    pub fn token_flow(self) -> ImplicitGrant {
        ImplicitGrant {
            oauth: self.oauth,
            grant: GrantType::TokenFlow,
        }
    }

    /// Create a new instance for code flow.
    ///
    /// # See
    /// [Microsoft Code Flow Authorization](https://docs.microsoft.com/en-us/onedrive/developer/rest-api/getting-started/msa-oauth?view=odsp-graph-online)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().code_flow();
    /// ```
    pub fn code_flow(self) -> AccessTokenGrant {
        AccessTokenGrant {
            oauth: self.oauth,
            grant: GrantType::CodeFlow,
        }
    }

    /// Create a new instance for the implicit grant.
    ///
    /// # See
    /// [Implicit Grant for OAuth 2.0](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-oauth2-implicit-grant-flow)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().implicit_grant();
    /// ```
    pub fn implicit_grant(self) -> ImplicitGrant {
        ImplicitGrant {
            oauth: self.oauth,
            grant: GrantType::Implicit,
        }
    }

    /// Create a new instance for authorization code grant.
    ///
    /// # See
    /// [Authorization Code Grant for OAuth 2.0](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-oauth2-auth-code-flow)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().authorization_code_grant();
    /// ```
    pub fn authorization_code_grant(self) -> AccessTokenGrant {
        AccessTokenGrant {
            oauth: self.oauth,
            grant: GrantType::AuthorizationCode,
        }
    }

    /// Create a new instance for device authorization code grant.
    ///
    /// # See
    /// [Microsoft identity platform and the OAuth 2.0 device authorization grant flow](https://learn.microsoft.com/en-us/azure/active-directory/develop/v2-oauth2-device-code)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let device_code_handler = oauth.build().device_code();
    /// ```
    pub fn device_code(self) -> DeviceCodeGrant {
        DeviceCodeGrant {
            oauth: self.oauth,
            grant: GrantType::DeviceCode,
        }
    }

    /// Create a new instance for the open id connect grant.
    ///
    /// # See
    /// [Microsoft Open ID Connect](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-protocols-oidc)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().open_id_connect();
    /// ```
    pub fn open_id_connect(self) -> AccessTokenGrant {
        AccessTokenGrant {
            oauth: self.oauth,
            grant: GrantType::OpenId,
        }
    }

    /// Create a new instance for the client credentials grant.
    ///
    /// # See
    /// [Microsoft Client Credentials](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-oauth2-client-creds-grant-flow)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().client_credentials();
    /// ```
    pub fn client_credentials(self) -> AccessTokenGrant {
        AccessTokenGrant {
            oauth: self.oauth,
            grant: GrantType::ClientCredentials,
        }
    }

    /// Create a new instance for the resource owner password credentials grant.
    ///
    /// # See
    /// [Microsoft Resource Owner Password Credentials](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-oauth-ropc)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().resource_owner_password_credentials();
    /// ```
    pub fn resource_owner_password_credentials(self) -> AccessTokenGrant {
        AccessTokenGrant {
            oauth: self.oauth,
            grant: GrantType::ResourceOwnerPasswordCredentials,
        }
    }
}

impl GrantSelector<AsyncAccessTokenGrant> {
    /// Create a new instance for token flow.
    ///
    /// # See
    /// [Microsoft Token Flow Authorization](https://docs.microsoft.com/en-us/onedrive/developer/rest-api/getting-started/msa-oauth?view=odsp-graph-online)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().token_flow();
    /// ```
    pub fn token_flow(self) -> ImplicitGrant {
        ImplicitGrant {
            oauth: self.oauth,
            grant: GrantType::TokenFlow,
        }
    }

    /// Create a new instance for code flow.
    ///
    /// # See
    /// [Microsoft Code Flow Authorization](https://docs.microsoft.com/en-us/onedrive/developer/rest-api/getting-started/msa-oauth?view=odsp-graph-online)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().code_flow();
    /// ```
    pub fn code_flow(self) -> AsyncAccessTokenGrant {
        AsyncAccessTokenGrant {
            oauth: self.oauth,
            grant: GrantType::CodeFlow,
        }
    }

    /// Create a new instance for the implicit grant.
    ///
    /// # See
    /// [Implicit Grant for OAuth 2.0](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-oauth2-implicit-grant-flow)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().implicit_grant();
    /// ```
    pub fn implicit_grant(self) -> ImplicitGrant {
        ImplicitGrant {
            oauth: self.oauth,
            grant: GrantType::Implicit,
        }
    }

    /// Create a new instance for authorization code grant.
    ///
    /// # See
    /// [Authorization Code Grant for OAuth 2.0](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-oauth2-auth-code-flow)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().authorization_code_grant();
    /// ```
    pub fn authorization_code_grant(self) -> AsyncAccessTokenGrant {
        AsyncAccessTokenGrant {
            oauth: self.oauth,
            grant: GrantType::AuthorizationCode,
        }
    }

    /// Create a new instance for device authorization code grant.
    ///
    /// # See
    /// [Microsoft identity platform and the OAuth 2.0 device authorization grant flow](https://learn.microsoft.com/en-us/azure/active-directory/develop/v2-oauth2-device-code)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let device_code_handler = oauth.build().device_code();
    /// ```
    pub fn device_code(self) -> AsyncDeviceCodeGrant {
        AsyncDeviceCodeGrant {
            oauth: self.oauth,
            grant: GrantType::DeviceCode,
        }
    }

    /// Create a new instance for the open id connect grant.
    ///
    /// # See
    /// [Microsoft Open ID Connect](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-protocols-oidc)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().open_id_connect();
    /// ```
    pub fn open_id_connect(self) -> AsyncAccessTokenGrant {
        AsyncAccessTokenGrant {
            oauth: self.oauth,
            grant: GrantType::OpenId,
        }
    }

    /// Create a new instance for the open id connect grant.
    ///
    /// # See
    /// [Microsoft Client Credentials](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-oauth2-client-creds-grant-flow)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().client_credentials();
    /// ```
    pub fn client_credentials(self) -> AsyncAccessTokenGrant {
        AsyncAccessTokenGrant {
            oauth: self.oauth,
            grant: GrantType::ClientCredentials,
        }
    }

    /// Create a new instance for the resource owner password credentials grant.
    ///
    /// # See
    /// [Microsoft Resource Owner Password Credentials](https://docs.microsoft.com/en-us/azure/active-directory/develop/v2-oauth-ropc)
    ///
    /// # Example
    /// ```
    /// # use graph_oauth::oauth::OAuthSerializer;
    /// # let mut oauth = OAuthSerializer::new();
    /// let open_id = oauth.build().resource_owner_password_credentials();
    /// ```
    pub fn resource_owner_password_credentials(self) -> AsyncAccessTokenGrant {
        AsyncAccessTokenGrant {
            oauth: self.oauth,
            grant: GrantType::ResourceOwnerPasswordCredentials,
        }
    }
}

#[derive(Debug)]
pub struct AuthorizationRequest {
    uri: String,
    error: Option<GraphFailure>,
}

impl AuthorizationRequest {
    pub fn open(self) -> GraphResult<()> {
        if self.error.is_some() {
            return Err(self.error.unwrap_or_default());
        }

        webbrowser::open(self.uri.as_str()).map_err(GraphFailure::from)
    }
}

#[derive(Debug)]
pub struct AccessTokenRequest {
    uri: String,
    params: HashMap<String, String>,
    error: Option<GraphFailure>,
}

impl AccessTokenRequest {
    /// Send the request for an access token. If successful, the Response body
    /// should be an access token which you can convert to [MsalTokenResponse]
    /// and pass back to [OAuthSerializer] to use to get refresh tokens.
    ///
    /// # Example
    /// ```rust,ignore
    /// # use graph_oauth::oauth::{OAuth, AccessToken};
    /// let mut oauth: OAuth = OAuth::new();
    ///
    /// // As an example create a random access token.
    /// let mut access_token = AccessToken::default();
    /// access_token.access_token("12345");
    /// // Store the token in OAuth if the access token has a refresh token.
    /// // The refresh token can be later used to request more access tokens.
    /// oauth.access_token(access_token);
    /// // You can get the actual bearer token if needed:
    /// println!("{:#?}", oauth.get_access_token().unwrap().bearer_token());
    /// ```
    ///
    /// Request an access token.
    /// # Example
    /// ```rust,ignore
    /// use graph_oauth::oauth::{AccessToken, OAuth};
    /// let mut oauth: OAuth = OAuth::new();
    ///
    /// // This assumes the user has been authenticated and
    /// // the access_code from the request has been given:
    /// oauth.access_code("access_code");
    ///
    /// // To get an access token a access_token_url is needed and the grant_type
    /// // should be set to token.
    /// // There are other parameters that may need to be included depending on the
    /// // authorization flow chosen.
    /// // The url below is for the v1.0 drive API. You can also use the Graph URLs as well.
    /// oauth.access_token_url("https://login.live.com/oauth20_token.srf")
    ///     .response_type("token")
    ///     .grant_type("authorization_code");
    ///
    /// // Make a request for an access token.
    /// let mut request = oauth.build().authorization_code_grant();
    /// let response = request.access_token().send()?;
    /// println!("{response:#?}");
    ///
    /// if response.status().is_success() {
    ///     let mut access_token: AccessToken = response.json()?;
    ///
    ///     let jwt = access_token.jwt();
    ///     println!("{jwt:#?}");
    ///
    ///     // Store in OAuth for getting refresh tokens.
    ///     oauth.access_token(access_token);
    /// } else {
    ///     // See if Microsoft Graph returned an error in the Response body
    ///     let result: reqwest::Result<serde_json::Value> = response.json();
    ///     println!("{:#?}", result);
    /// }
    /// ```
    pub fn send(self) -> GraphResult<reqwest::blocking::Response> {
        if self.error.is_some() {
            return Err(self.error.unwrap_or_default());
        }

        let client = reqwest::blocking::Client::new();
        client
            .post(self.uri.as_str())
            .form(&self.params)
            .send()
            .map_err(GraphFailure::from)
    }
}

#[derive(Debug)]
pub struct AsyncAccessTokenRequest {
    uri: String,
    params: HashMap<String, String>,
    error: Option<GraphFailure>,
}

impl AsyncAccessTokenRequest {
    /// Send the request for an access token. If successful, the Response body
    /// should be an access token which you can convert to [MsalTokenResponse]
    /// and pass back to [OAuthSerializer] to use to get refresh tokens.
    ///
    /// # Example
    /// ```rust,ignore
    /// # use graph_oauth::oauth::{OAuth, AccessToken};
    /// let mut oauth: OAuth = OAuth::new();
    ///
    /// // As an example create a random access token.
    /// let mut access_token = AccessToken::default();
    /// access_token.access_token("12345");
    /// // Store the token in OAuth if the access token has a refresh token.
    /// // The refresh token can be later used to request more access tokens.
    /// oauth.access_token(access_token);
    /// // You can get the actual bearer token if needed:
    /// println!("{:#?}", oauth.get_access_token().unwrap().bearer_token());
    /// ```
    ///
    /// Request an access token.
    /// # Example
    /// ```rust,ignore
    /// use graph_oauth::oauth::{AccessToken, OAuth};
    /// let mut oauth: OAuth = OAuth::new();
    ///
    /// // This assumes the user has been authenticated and
    /// // the access_code from the request has been given:
    /// oauth.access_code("access_code");
    ///
    /// // To get an access token a access_token_url is needed and the grant_type
    /// // should be set to token.
    /// // There are other parameters that may need to be included depending on the
    /// // authorization flow chosen.
    /// // The url below is for the v1.0 drive API. You can also use the Graph URLs as well.
    /// oauth.access_token_url("https://login.live.com/oauth20_token.srf")
    ///     .response_type("token")
    ///     .grant_type("authorization_code");
    ///
    /// // Make a request for an access token.
    /// let mut request = oauth.build().authorization_code_grant();
    /// let response = request.access_token().send().await?;
    /// println!("{response:#?}");
    ///
    /// if response.status().is_success() {
    ///     let mut access_token: AccessToken = response.json().await?;
    ///
    ///     let jwt = access_token.jwt();
    ///     println!("{jwt:#?}");
    ///
    ///     // Store in OAuth for getting refresh tokens.
    ///     oauth.access_token(access_token);
    /// } else {
    ///     // See if Microsoft Graph returned an error in the Response body
    ///     let result: reqwest::Result<serde_json::Value> = response.json().await;
    ///     println!("{:#?}", result);
    /// }
    /// ```
    pub async fn send(self) -> GraphResult<reqwest::Response> {
        if self.error.is_some() {
            return Err(self.error.unwrap_or_default());
        }

        let client = reqwest::Client::new();
        client
            .post(self.uri.as_str())
            .form(&self.params)
            .send()
            .await
            .map_err(GraphFailure::from)
    }
}

#[derive(Debug)]
pub struct ImplicitGrant {
    oauth: OAuthSerializer,
    grant: GrantType,
}

impl ImplicitGrant {
    pub fn url(&mut self) -> GraphResult<Url> {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::Authorization);
        Ok(Url::parse(
            self.oauth
                .get_or_else(OAuthParameter::AuthorizationUrl)?
                .as_str(),
        )?)
    }

    pub fn browser_authorization(&mut self) -> AuthorizationRequest {
        let params = self.oauth.params(
            self.grant
                .available_credentials(GrantRequest::Authorization),
        );

        if let Err(e) = params {
            return AuthorizationRequest {
                uri: Default::default(),
                error: Some(e),
            };
        }

        let url_result = self.url();

        if let Err(e) = url_result {
            return AuthorizationRequest {
                uri: Default::default(),
                error: Some(e),
            };
        }

        let mut url = url_result.unwrap();
        url.query_pairs_mut().extend_pairs(&params.unwrap());
        AuthorizationRequest {
            uri: url.to_string(),
            error: None,
        }
    }
}

impl From<ImplicitGrant> for OAuthSerializer {
    fn from(token_grant: ImplicitGrant) -> Self {
        token_grant.oauth
    }
}

impl AsRef<OAuthSerializer> for ImplicitGrant {
    fn as_ref(&self) -> &OAuthSerializer {
        &self.oauth
    }
}

pub struct DeviceCodeGrant {
    oauth: OAuthSerializer,
    grant: GrantType,
}

impl DeviceCodeGrant {
    pub fn authorization_url(&mut self) -> Result<Url, GraphFailure> {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::Authorization);
        let params = self.oauth.params(
            self.grant
                .available_credentials(GrantRequest::Authorization),
        )?;
        let mut url = Url::parse(
            self.oauth
                .get_or_else(OAuthParameter::AuthorizationUrl)?
                .as_str(),
        )?;
        url.query_pairs_mut().extend_pairs(&params);
        Ok(url)
    }

    pub fn authorization(&mut self) -> AccessTokenRequest {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::Authorization);
        let uri = self.oauth.get_or_else(OAuthParameter::AuthorizationUrl);
        let params = self.oauth.params(
            self.grant
                .available_credentials(GrantRequest::Authorization),
        );

        if let Err(e) = uri {
            return AccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        if let Err(e) = params {
            return AccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        AccessTokenRequest {
            uri: uri.unwrap(),
            params: params.unwrap(),
            error: None,
        }
    }

    pub fn access_token(&mut self) -> AccessTokenRequest {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::AccessToken);
        let uri = self.oauth.get_or_else(OAuthParameter::AccessTokenUrl);
        let params = self
            .oauth
            .params(self.grant.available_credentials(GrantRequest::AccessToken));

        if let Err(e) = uri {
            return AccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        if let Err(e) = params {
            return AccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        AccessTokenRequest {
            uri: uri.unwrap(),
            params: params.unwrap(),
            error: None,
        }
    }

    pub fn refresh_token(&mut self) -> AccessTokenRequest {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::RefreshToken);
        let uri = self.oauth.get_or_else(OAuthParameter::RefreshTokenUrl);
        let params = self
            .oauth
            .params(self.grant.available_credentials(GrantRequest::RefreshToken));

        if let Err(e) = uri {
            return AccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        if let Err(e) = params {
            return AccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        AccessTokenRequest {
            uri: uri.unwrap(),
            params: params.unwrap(),
            error: None,
        }
    }
}

pub struct AsyncDeviceCodeGrant {
    oauth: OAuthSerializer,
    grant: GrantType,
}

impl AsyncDeviceCodeGrant {
    pub fn authorization_url(&mut self) -> Result<Url, GraphFailure> {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::Authorization);
        let params = self.oauth.params(
            self.grant
                .available_credentials(GrantRequest::Authorization),
        )?;
        let mut url = Url::parse(
            self.oauth
                .get_or_else(OAuthParameter::AuthorizationUrl)?
                .as_str(),
        )?;
        url.query_pairs_mut().extend_pairs(&params);
        Ok(url)
    }

    pub fn authorization(&mut self) -> AsyncAccessTokenRequest {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::Authorization);
        let uri = self.oauth.get_or_else(OAuthParameter::AuthorizationUrl);
        let params = self.oauth.params(
            self.grant
                .available_credentials(GrantRequest::Authorization),
        );

        if let Err(e) = uri {
            return AsyncAccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        if let Err(e) = params {
            return AsyncAccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        AsyncAccessTokenRequest {
            uri: uri.unwrap(),
            params: params.unwrap(),
            error: None,
        }
    }

    pub fn access_token(&mut self) -> AsyncAccessTokenRequest {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::AccessToken);
        let uri = self.oauth.get_or_else(OAuthParameter::AccessTokenUrl);
        let params = self
            .oauth
            .params(self.grant.available_credentials(GrantRequest::AccessToken));

        if let Err(e) = uri {
            return AsyncAccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        if let Err(e) = params {
            return AsyncAccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        AsyncAccessTokenRequest {
            uri: uri.unwrap(),
            params: params.unwrap(),
            error: None,
        }
    }

    pub fn refresh_token(&mut self) -> AsyncAccessTokenRequest {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::RefreshToken);
        let uri = self.oauth.get_or_else(OAuthParameter::RefreshTokenUrl);
        let params = self
            .oauth
            .params(self.grant.available_credentials(GrantRequest::RefreshToken));

        if let Err(e) = uri {
            return AsyncAccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        if let Err(e) = params {
            return AsyncAccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        AsyncAccessTokenRequest {
            uri: uri.unwrap(),
            params: params.unwrap(),
            error: None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccessTokenGrant {
    oauth: OAuthSerializer,
    grant: GrantType,
}

impl AccessTokenGrant {
    pub fn authorization_url(&mut self) -> Result<Url, GraphFailure> {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::Authorization);
        let params = self.oauth.params(
            self.grant
                .available_credentials(GrantRequest::Authorization),
        )?;
        let mut url = Url::parse(
            self.oauth
                .get_or_else(OAuthParameter::AuthorizationUrl)?
                .as_str(),
        )?;
        url.query_pairs_mut().extend_pairs(&params);
        Ok(url)
    }

    /// Make a request for authorization. The default browser for a user
    /// will be opened to the sign in page where the user will need to
    /// sign in and agree to any permissions that were set by the provided
    /// scopes.
    pub fn browser_authorization(&mut self) -> AuthorizationRequest {
        let uri = self.authorization_url();
        if let Err(e) = uri {
            return AuthorizationRequest {
                uri: Default::default(),
                error: Some(e),
            };
        }

        AuthorizationRequest {
            uri: uri.unwrap().to_string(),
            error: None,
        }
    }

    /// Make a request for an access token. The token is stored in OAuth and
    /// will be used to make for making requests for refresh tokens. The below
    /// example shows how access tokens are stored and retrieved for OAuth:
    /// # Example
    /// ```rust,ignore
    /// # use graph_oauth::oauth::{OAuth, AccessToken};
    /// let mut oauth: OAuth = OAuth::new();
    ///
    /// // As an example create a random access token.
    /// let mut access_token = AccessToken::default();
    /// access_token.access_token("12345");
    /// // Store the token in OAuth if the access token has a refresh token.
    /// // The refresh token can be later used to request more access tokens.
    /// oauth.access_token(access_token);
    /// // You can get the actual bearer token if needed:
    /// println!("{:#?}", oauth.get_access_token().unwrap().bearer_token());
    /// ```
    ///
    /// Request an access token.
    /// # Example
    /// ```rust,ignore
    /// use graph_oauth::oauth::{AccessToken, OAuth};
    /// let mut oauth: OAuth = OAuth::new();
    ///
    /// // This assumes the user has been authenticated and
    /// // the access_code from the request has been given:
    /// oauth.access_code("access_code");
    ///
    /// // To get an access token a access_token_url is needed and the grant_type
    /// // should be set to token.
    /// // There are other parameters that may need to be included depending on the
    /// // authorization flow chosen.
    /// // The url below is for the v1.0 drive API. You can also use the Graph URLs as well.
    /// oauth.access_token_url("https://login.live.com/oauth20_token.srf")
    ///     .response_type("token")
    ///     .grant_type("authorization_code");
    ///
    /// // Make a request for an access token.
    /// let mut request = oauth.build().authorization_code_grant();
    /// let response = request.access_token().send()?;
    /// println!("{response:#?}");
    ///
    /// if response.status().is_success() {
    ///     let mut access_token: AccessToken = response.json()?;
    ///
    ///     let jwt = access_token.jwt();
    ///     println!("{jwt:#?}");
    ///
    ///     // Store in OAuth for getting refresh tokens.
    ///     oauth.access_token(access_token);
    /// } else {
    ///     // See if Microsoft Graph returned an error in the Response body
    ///     let result: reqwest::Result<serde_json::Value> = response.json();
    ///     println!("{:#?}", result);
    /// }
    /// ```
    pub fn access_token(&mut self) -> AccessTokenRequest {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::AccessToken);
        let uri = self.oauth.get_or_else(OAuthParameter::AccessTokenUrl);
        let params = self
            .oauth
            .params(self.grant.available_credentials(GrantRequest::AccessToken));

        if let Err(e) = uri {
            return AccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        if let Err(e) = params {
            return AccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        AccessTokenRequest {
            uri: uri.unwrap(),
            params: params.unwrap(),
            error: None,
        }
    }

    /// Request a refresh token. Assumes an access token has already
    /// been retrieved.
    pub fn refresh_token(&mut self) -> AccessTokenRequest {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::RefreshToken);
        let uri = self.oauth.get_or_else(OAuthParameter::RefreshTokenUrl);
        let params = self
            .oauth
            .params(self.grant.available_credentials(GrantRequest::RefreshToken));

        if let Err(e) = uri {
            return AccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        if let Err(e) = params {
            return AccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        AccessTokenRequest {
            uri: uri.unwrap(),
            params: params.unwrap(),
            error: None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AsyncAccessTokenGrant {
    oauth: OAuthSerializer,
    grant: GrantType,
}

impl AsyncAccessTokenGrant {
    pub fn authorization_url(&mut self) -> Result<Url, GraphFailure> {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::Authorization);
        let params = self.oauth.params(
            self.grant
                .available_credentials(GrantRequest::Authorization),
        )?;
        let mut url = Url::parse(
            self.oauth
                .get_or_else(OAuthParameter::AuthorizationUrl)?
                .as_str(),
        )?;
        url.query_pairs_mut().extend_pairs(&params);
        Ok(url)
    }

    /// Make a request for authorization. The default browser for a user
    /// will be opened to the sign in page where the user will need to
    /// sign in and agree to any permissions that were set by the provided
    /// scopes.
    pub fn browser_authorization(&mut self) -> AuthorizationRequest {
        let uri = self.authorization_url();
        if let Err(e) = uri {
            return AuthorizationRequest {
                uri: Default::default(),
                error: Some(e),
            };
        }

        AuthorizationRequest {
            uri: uri.unwrap().to_string(),
            error: None,
        }
    }

    /// Make a request for an access token. The token is stored in OAuth and
    /// will be used to make for making requests for refresh tokens. The below
    /// example shows how access tokens are stored and retrieved for OAuth:
    ///
    /// # Example
    /// ```rust,ignore
    /// # use graph_oauth::oauth::{OAuth, AccessToken};
    /// let mut oauth: OAuth = OAuth::new();
    ///
    /// // As an example create a random access token.
    /// let mut access_token = AccessToken::default();
    /// access_token.access_token("12345");
    /// // Store the token in OAuth if the access token has a refresh token.
    /// // The refresh token can be later used to request more access tokens.
    /// oauth.access_token(access_token);
    /// // You can get the actual bearer token if needed:
    /// println!("{:#?}", oauth.get_access_token().unwrap().bearer_token());
    /// ```
    ///
    /// Request an access token.
    /// # Example
    /// ```rust,ignore
    /// use graph_oauth::oauth::{AccessToken, OAuth};
    /// let mut oauth: OAuth = OAuth::new();
    ///
    /// // This assumes the user has been authenticated and
    /// // the access_code from the request has been given:
    /// oauth.access_code("access_code");
    ///
    /// // To get an access token a access_token_url is needed and the grant_type
    /// // should be set to token.
    /// // There are other parameters that may need to be included depending on the
    /// // authorization flow chosen.
    /// // The url below is for the v1.0 drive API. You can also use the Graph URLs as well.
    /// oauth.access_token_url("https://login.live.com/oauth20_token.srf")
    ///     .response_type("token")
    ///     .grant_type("authorization_code");
    ///
    /// // Make a request for an access token.
    /// let mut request = oauth.build().authorization_code_grant();
    /// let response = request.access_token().send().await?;
    /// println!("{response:#?}");
    ///
    /// if response.status().is_success() {
    ///     let mut access_token: AccessToken = response.json().await?;
    ///
    ///     let jwt = access_token.jwt();
    ///     println!("{jwt:#?}");
    ///
    ///     // Store in OAuth for getting refresh tokens.
    ///     oauth.access_token(access_token);
    /// } else {
    ///     // See if Microsoft Graph returned an error in the Response body
    ///     let result: reqwest::Result<serde_json::Value> = response.json().await;
    ///     println!("{:#?}", result);
    /// }
    /// ```
    pub fn access_token(&mut self) -> AsyncAccessTokenRequest {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::AccessToken);
        let uri = self.oauth.get_or_else(OAuthParameter::AccessTokenUrl);
        let params = self
            .oauth
            .params(self.grant.available_credentials(GrantRequest::AccessToken));

        if let Err(e) = uri {
            return AsyncAccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        if let Err(e) = params {
            return AsyncAccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        AsyncAccessTokenRequest {
            uri: uri.unwrap(),
            params: params.unwrap(),
            error: None,
        }
    }

    /// Request a refresh token. Assumes an access token has already
    /// been retrieved.
    pub fn refresh_token(&mut self) -> AsyncAccessTokenRequest {
        self.oauth
            .pre_request_check(self.grant, GrantRequest::RefreshToken);
        let uri = self.oauth.get_or_else(OAuthParameter::RefreshTokenUrl);
        let params = self
            .oauth
            .params(self.grant.available_credentials(GrantRequest::RefreshToken));

        if let Err(e) = uri {
            return AsyncAccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        if let Err(e) = params {
            return AsyncAccessTokenRequest {
                uri: Default::default(),
                params: Default::default(),
                error: Some(e),
            };
        }

        AsyncAccessTokenRequest {
            uri: uri.unwrap(),
            params: params.unwrap(),
            error: None,
        }
    }
}

impl From<AccessTokenGrant> for OAuthSerializer {
    fn from(token_grant: AccessTokenGrant) -> Self {
        token_grant.oauth
    }
}

impl AsRef<OAuthSerializer> for AccessTokenGrant {
    fn as_ref(&self) -> &OAuthSerializer {
        &self.oauth
    }
}

impl AsMut<OAuthSerializer> for AccessTokenGrant {
    fn as_mut(&mut self) -> &mut OAuthSerializer {
        &mut self.oauth
    }
}

impl fmt::Debug for OAuthSerializer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map_debug: BTreeMap<&str, &str> = BTreeMap::new();
        for (key, value) in self.credentials.iter() {
            if let Some(oac) = OAuthParameter::iter()
                .find(|oac| oac.alias().eq(key.as_str()) && oac.is_debug_redacted())
            {
                map_debug.insert(oac.alias(), "[REDACTED]");
            } else {
                map_debug.insert(key.as_str(), value.as_str());
            }
        }

        f.debug_struct("AccessToken")
            .field("access_token", &"[REDACTED]".to_string())
            .field("credentials", &map_debug)
            .field("scopes", &self.scopes)
            .finish()
    }
}
