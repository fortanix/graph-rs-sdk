use crate::auth::{OAuth, OAuthCredential};
use crate::grants::GrantType;
use crate::identity::form_credential::FormCredential;
use crate::identity::{
    Authority, AuthorizationCodeAuthorizationUrl, AuthorizationSerializer, AzureAuthorityHost,
};
use graph_error::{AuthorizationFailure, AuthorizationResult, GraphFailure, GraphResult};
use std::collections::HashMap;

use url::Url;

#[derive(Clone)]
pub struct AuthorizationCodeCredential {
    /// The authorization code obtained from a call to authorize. The code should be obtained with all required scopes.
    pub(crate) authorization_code: Option<String>,
    /// The refresh token needed to make an access token request using a refresh token.
    /// Do not include an authorization code when using a refresh token.
    pub(crate) refresh_token: Option<String>,
    /// The client (application) ID of the service principal
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
    pub(crate) redirect_uri: String,
    pub(crate) scopes: Vec<String>,
    /// The Azure Active Directory tenant (directory) Id of the service principal.
    pub(crate) authority: Authority,
    pub(crate) code_verifier: Option<String>,
    serializer: OAuth,
}

/*
   pub(crate) client_id: String,
   pub(crate) redirect_uri: String,
   pub(crate) authority: Authority,
   pub(crate) response_mode: ResponseMode,
   pub(crate) response_type: String,
   pub(crate) nonce: Option<String>,
   pub(crate) state: Option<String>,
   pub(crate) scopes: Vec<String>,
   pub(crate) prompt: Option<Prompt>,
   pub(crate) domain_hint: Option<String>,
   pub(crate) login_hint: Option<String>,
   pub(crate) code_challenge: Option<String>,
   pub(crate) code_challenge_method: Option<String>,

   authority

*/

impl AuthorizationCodeCredential {
    pub fn new<T: AsRef<str>>(
        client_id: T,
        client_secret: T,
        authorization_code: T,
        redirect_uri: T,
    ) -> AuthorizationCodeCredential {
        AuthorizationCodeCredential {
            authorization_code: Some(authorization_code.as_ref().to_owned()),
            refresh_token: None,
            client_id: client_id.as_ref().to_owned(),
            client_secret: client_secret.as_ref().to_owned(),
            redirect_uri: redirect_uri.as_ref().to_owned(),
            scopes: vec![],
            authority: Default::default(),
            code_verifier: None,
            serializer: OAuth::new(),
        }
    }

    pub fn grant_type(&self) -> GrantType {
        GrantType::AuthorizationCode
    }

    pub fn builder() -> AuthorizationCodeCredentialBuilder {
        AuthorizationCodeCredentialBuilder::new()
    }
}

impl AuthorizationSerializer for AuthorizationCodeCredential {
    fn uri(&mut self, azure_authority_host: &AzureAuthorityHost) -> GraphResult<Url> {
        self.serializer
            .authority(azure_authority_host, &self.authority);

        let uri = self
            .serializer
            .get_or_else(OAuthCredential::AccessTokenUrl)?;
        Url::parse(uri.as_str()).map_err(GraphFailure::from)
    }

    fn form(&mut self) -> AuthorizationResult<HashMap<String, String>> {
        if self.authorization_code.is_some() && self.refresh_token.is_some() {
            return AuthorizationFailure::required_value(
                &format!(
                    "{} or {}",
                    OAuthCredential::AuthorizationCode.alias(),
                    OAuthCredential::RefreshToken.alias()
                ),
                Some("Authorization code and refresh token cannot be set at the same time - choose one or the other"),
            );
        }

        if self.client_id.trim().is_empty() {
            return AuthorizationFailure::required_value(OAuthCredential::ClientId.alias(), None);
        }

        if self.client_secret.trim().is_empty() {
            return AuthorizationFailure::required_value(
                OAuthCredential::ClientSecret.alias(),
                None,
            );
        }

        self.serializer
            .client_id(self.client_id.as_str())
            .client_secret(self.client_secret.as_str())
            .extend_scopes(self.scopes.clone());

        if let Some(refresh_token) = self.refresh_token.as_ref() {
            if refresh_token.trim().is_empty() {
                return AuthorizationFailure::required_value(
                    OAuthCredential::RefreshToken.alias(),
                    Some("Either authorization code or refresh token is required"),
                );
            }

            self.serializer
                .grant_type("refresh_token")
                .refresh_token(refresh_token.as_ref());

            return self.serializer.authorization_form(vec![
                FormCredential::Required(OAuthCredential::ClientId),
                FormCredential::Required(OAuthCredential::ClientSecret),
                FormCredential::Required(OAuthCredential::RefreshToken),
                FormCredential::Required(OAuthCredential::GrantType),
                FormCredential::NotRequired(OAuthCredential::Scopes),
            ]);
        } else if let Some(authorization_code) = self.authorization_code.as_ref() {
            if authorization_code.trim().is_empty() {
                return AuthorizationFailure::required_value(
                    OAuthCredential::RefreshToken.alias(),
                    Some("Either authorization code or refresh token is required"),
                );
            }

            self.serializer
                .authorization_code(authorization_code.as_ref())
                .grant_type("authorization_code")
                .redirect_uri(self.redirect_uri.as_str());

            if let Some(code_verifier) = self.code_verifier.as_ref() {
                self.serializer.code_verifier(code_verifier.as_str());
            }

            return self.serializer.authorization_form(vec![
                FormCredential::Required(OAuthCredential::ClientId),
                FormCredential::Required(OAuthCredential::ClientSecret),
                FormCredential::Required(OAuthCredential::RedirectUri),
                FormCredential::Required(OAuthCredential::AuthorizationCode),
                FormCredential::Required(OAuthCredential::GrantType),
                FormCredential::NotRequired(OAuthCredential::Scopes),
                FormCredential::NotRequired(OAuthCredential::CodeVerifier),
            ]);
        }

        AuthorizationFailure::required_value(
            &format!(
                "{} or {}",
                OAuthCredential::AuthorizationCode.alias(),
                OAuthCredential::RefreshToken.alias()
            ),
            Some("Either authorization code or refresh token is required"),
        )
    }
}

#[derive(Clone)]
pub struct AuthorizationCodeCredentialBuilder {
    authorization_code_credential: AuthorizationCodeCredential,
}

impl AuthorizationCodeCredentialBuilder {
    fn new() -> AuthorizationCodeCredentialBuilder {
        Self {
            authorization_code_credential: AuthorizationCodeCredential {
                authorization_code: None,
                refresh_token: None,
                client_id: String::new(),
                client_secret: String::new(),
                redirect_uri: String::new(),
                scopes: vec![],
                authority: Default::default(),
                code_verifier: None,
                serializer: OAuth::new(),
            },
        }
    }

    pub fn with_authorization_code<T: AsRef<str>>(
        &mut self,
        authorization_code: T,
    ) -> &mut AuthorizationCodeCredentialBuilder {
        self.authorization_code_credential.authorization_code =
            Some(authorization_code.as_ref().to_owned());
        self
    }

    pub fn with_refresh_token<T: AsRef<str>>(
        &mut self,
        refresh_token: T,
    ) -> &mut AuthorizationCodeCredentialBuilder {
        self.authorization_code_credential.authorization_code = None;
        self.authorization_code_credential.refresh_token = Some(refresh_token.as_ref().to_owned());
        self
    }

    pub fn with_redirect_uri<T: AsRef<str>>(
        &mut self,
        redirect_uri: T,
    ) -> &mut AuthorizationCodeCredentialBuilder {
        self.authorization_code_credential.redirect_uri = redirect_uri.as_ref().to_owned();
        self
    }

    pub fn with_client_id<T: AsRef<str>>(
        &mut self,
        client_id: T,
    ) -> &mut AuthorizationCodeCredentialBuilder {
        self.authorization_code_credential.client_id = client_id.as_ref().to_owned();
        self
    }

    pub fn with_client_secret<T: AsRef<str>>(
        &mut self,
        client_secret: T,
    ) -> &mut AuthorizationCodeCredentialBuilder {
        self.authorization_code_credential.client_secret = client_secret.as_ref().to_owned();
        self
    }

    /// Convenience method. Same as calling [with_authority(Authority::TenantId("tenant_id"))]
    pub fn with_tenant<T: AsRef<str>>(&mut self, tenant: T) -> &mut Self {
        self.authorization_code_credential.authority =
            Authority::TenantId(tenant.as_ref().to_owned());
        self
    }

    pub fn with_authority<T: Into<Authority>>(
        &mut self,
        authority: T,
    ) -> &mut AuthorizationCodeCredentialBuilder {
        self.authorization_code_credential.authority = authority.into();
        self
    }

    pub fn with_code_verifier<T: AsRef<str>>(
        &mut self,
        code_verifier: T,
    ) -> &mut AuthorizationCodeCredentialBuilder {
        self.authorization_code_credential.code_verifier = Some(code_verifier.as_ref().to_owned());
        self
    }

    pub fn with_scopes<T: ToString, I: IntoIterator<Item = T>>(
        &mut self,
        scopes: I,
    ) -> &mut AuthorizationCodeCredentialBuilder {
        self.authorization_code_credential.scopes =
            scopes.into_iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn build(&self) -> AuthorizationCodeCredential {
        self.authorization_code_credential.clone()
    }
}

impl From<AuthorizationCodeAuthorizationUrl> for AuthorizationCodeCredentialBuilder {
    fn from(value: AuthorizationCodeAuthorizationUrl) -> Self {
        let mut builder = AuthorizationCodeCredentialBuilder::new();
        builder
            .with_scopes(value.scopes)
            .with_client_id(value.client_id)
            .with_redirect_uri(value.redirect_uri)
            .with_authority(value.authority);

        if let Some(code_verifier) = value.code_verifier.as_ref() {
            builder.with_code_verifier(code_verifier);
        }

        builder
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn with_tenant_id_common() {
        let credential = AuthorizationCodeCredential::builder()
            .with_authority(Authority::TenantId("common".into()))
            .build();

        assert_eq!(credential.authority, Authority::TenantId("common".into()))
    }

    #[test]
    fn with_tenant_id_adfs() {
        let credential = AuthorizationCodeCredential::builder()
            .with_authority(Authority::AzureDirectoryFederatedServices)
            .build();

        assert_eq!(credential.authority.as_ref(), "adfs");
    }

    #[test]
    #[should_panic]
    fn authorization_code_missing_required_value() {
        let mut credential_builder = AuthorizationCodeCredential::builder();
        credential_builder
            .with_redirect_uri("https://localhost:8080")
            .with_client_id("client_id")
            .with_client_secret("client_secret")
            .with_scopes(vec!["scope"])
            .with_tenant("tenant_id");
        let mut credential = credential_builder.build();
        let _ = credential.form().unwrap();
    }

    #[test]
    #[should_panic]
    fn required_value_missing_client_id() {
        let mut credential_builder = AuthorizationCodeCredential::builder();
        credential_builder
            .with_authorization_code("code")
            .with_refresh_token("token");
        let mut credential = credential_builder.build();
        let _ = credential.form().unwrap();
    }
}
