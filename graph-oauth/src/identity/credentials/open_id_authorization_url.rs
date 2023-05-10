use crate::identity::{
    Authority, AuthorizationUrl, AzureAuthorityHost, Crypto, Prompt, ResponseMode, ResponseType,
};
use graph_error::{AuthorizationFailure, AuthorizationResult};
use url::Url;

/// OpenID Connect (OIDC) extends the OAuth 2.0 authorization protocol for use as an additional
/// authentication protocol. You can use OIDC to enable single sign-on (SSO) between your
/// OAuth-enabled applications by using a security token called an ID token.
/// https://learn.microsoft.com/en-us/azure/active-directory/develop/v2-protocols-oidc
#[derive(Clone, Debug)]
pub struct OpenIdAuthorizationUrl {
    /// Required
    /// The Application (client) ID that the Azure portal – App registrations experience
    /// assigned to your app.
    pub(crate) client_id: String,
    /// Required
    /// The redirect URI of your app, where authentication responses can be sent and received
    /// by your app. It must exactly match one of the redirect URIs you registered in the portal,
    /// except that it must be URL-encoded. If not present, the endpoint will pick one registered
    /// redirect_uri at random to send the user back to.
    pub(crate) redirect_uri: String,
    /// Required
    /// Must include code for OpenID Connect sign-in.
    pub(crate) response_type: Vec<ResponseType>,
    /// Optional
    /// Specifies how the identity platform should return the requested token to your app.
    ///
    /// Supported values:
    ///
    /// - query: Default when requesting an access token. Provides the code as a query string
    /// parameter on your redirect URI. The query parameter isn't supported when requesting an
    /// ID token by using the implicit flow.
    /// - fragment: Default when requesting an ID token by using the implicit flow.
    /// Also supported if requesting only a code.
    /// - form_post: Executes a POST containing the code to your redirect URI.
    /// Supported when requesting a code.
    pub(crate) response_mode: Option<ResponseMode>,
    /// Optional
    /// A value generated and sent by your app in its request for an ID token. The same nonce
    /// value is included in the ID token returned to your app by the Microsoft identity platform.
    /// To mitigate token replay attacks, your app should verify the nonce value in the ID token
    /// is the same value it sent when requesting the token. The value is typically a unique,
    /// random string.
    pub(crate) nonce: String,
    /// Required
    /// A value included in the request that also will be returned in the token response.
    /// It can be a string of any content you want. A randomly generated unique value typically
    /// is used to prevent cross-site request forgery attacks. The state also is used to encode
    /// information about the user's state in the app before the authentication request occurred,
    /// such as the page or view the user was on.
    pub(crate) state: Option<String>,
    /// Required - the openid scope is already included
    /// A space-separated list of scopes. For OpenID Connect, it must include the scope openid,
    /// which translates to the Sign you in permission in the consent UI. You might also include
    /// other scopes in this request for requesting consent.
    pub(crate) scope: Vec<String>,
    /// Optional
    /// Indicates the type of user interaction that is required. The only valid values at
    /// this time are login, none, consent, and select_account.
    ///
    /// The [Prompt::Login] claim forces the user to enter their credentials on that request,
    /// which negates single sign-on.
    ///
    /// The [Prompt::None] parameter is the opposite, and should be paired with a login_hint to
    /// indicate which user must be signed in. These parameters ensure that the user isn't
    /// presented with any interactive prompt at all. If the request can't be completed silently
    /// via single sign-on, the Microsoft identity platform returns an error. Causes include no
    /// signed-in user, the hinted user isn't signed in, or multiple users are signed in but no
    /// hint was provided.
    ///
    /// The [Prompt::Consent] claim triggers the OAuth consent dialog after the
    /// user signs in. The dialog asks the user to grant permissions to the app.
    ///
    /// Finally, [Prompt::SelectAccount] shows the user an account selector, negating silent SSO but
    /// allowing the user to pick which account they intend to sign in with, without requiring
    /// credential entry. You can't use both login_hint and select_account.
    pub(crate) prompt: Vec<Prompt>,
    /// Optional
    /// The realm of the user in a federated directory. This skips the email-based discovery
    /// process that the user goes through on the sign-in page, for a slightly more streamlined
    /// user experience. For tenants that are federated through an on-premises directory
    /// like AD FS, this often results in a seamless sign-in because of the existing login session.
    pub(crate) domain_hint: Option<String>,
    /// Optional
    /// You can use this parameter to pre-fill the username and email address field of the
    /// sign-in page for the user, if you know the username ahead of time. Often, apps use
    /// this parameter during reauthentication, after already extracting the login_hint
    /// optional claim from an earlier sign-in.
    pub(crate) login_hint: Option<String>,
    pub(crate) authority: Authority,
}

impl OpenIdAuthorizationUrl {
    pub fn new<T: AsRef<str>>(
        client_id: T,
        redirect_uri: T,
    ) -> anyhow::Result<OpenIdAuthorizationUrl> {
        Ok(OpenIdAuthorizationUrl {
            client_id: client_id.as_ref().to_owned(),
            redirect_uri: redirect_uri.as_ref().to_owned(),
            response_type: vec![ResponseType::Code],
            response_mode: None,
            nonce: Crypto::secure_random_string()?,
            state: None,
            scope: vec!["openid".to_owned()],
            prompt: vec![],
            domain_hint: None,
            login_hint: None,
            authority: Authority::default(),
        })
    }

    pub fn builder() -> anyhow::Result<OpenIdAuthorizationUrlBuilder> {
        OpenIdAuthorizationUrlBuilder::new()
    }

    pub fn url(&self) -> AuthorizationResult<Url> {
        self.url_with_host(&AzureAuthorityHost::default())
    }

    pub fn url_with_host(
        &self,
        azure_authority_host: &AzureAuthorityHost,
    ) -> AuthorizationResult<Url> {
        self.authorization_url_with_host(azure_authority_host)
    }
}

impl AuthorizationUrl for OpenIdAuthorizationUrl {
    fn redirect_uri(&self) -> AuthorizationResult<Url> {
        Url::parse(self.redirect_uri.as_str()).map_err(AuthorizationFailure::from)
    }

    fn authorization_url(&self) -> AuthorizationResult<Url> {
        self.authorization_url_with_host(&AzureAuthorityHost::default())
    }

    fn authorization_url_with_host(
        &self,
        _azure_authority_host: &AzureAuthorityHost,
    ) -> AuthorizationResult<Url> {
        unimplemented!()
    }
}

pub struct OpenIdAuthorizationUrlBuilder {
    auth_url_parameters: OpenIdAuthorizationUrl,
}

impl OpenIdAuthorizationUrlBuilder {
    fn new() -> anyhow::Result<OpenIdAuthorizationUrlBuilder> {
        Ok(OpenIdAuthorizationUrlBuilder {
            auth_url_parameters: OpenIdAuthorizationUrl::new(String::new(), String::new())?,
        })
    }

    pub fn with_redirect_uri<T: AsRef<str>>(&mut self, redirect_uri: T) -> &mut Self {
        self.auth_url_parameters.redirect_uri = redirect_uri.as_ref().to_owned();
        self
    }

    pub fn with_client_id<T: AsRef<str>>(&mut self, client_id: T) -> &mut Self {
        self.auth_url_parameters.client_id = client_id.as_ref().to_owned();
        self
    }

    /// Convenience method. Same as calling [with_authority(Authority::TenantId("tenant_id"))]
    pub fn with_tenant<T: AsRef<str>>(&mut self, tenant: T) -> &mut Self {
        self.auth_url_parameters.authority = Authority::TenantId(tenant.as_ref().to_owned());
        self
    }

    pub fn with_authority<T: Into<Authority>>(&mut self, authority: T) -> &mut Self {
        self.auth_url_parameters.authority = authority.into();
        self
    }

    /// Default is code. Must include code for the authorization code flow.
    /// Can also include id_token or token if using the hybrid flow.
    pub fn with_response_type<I: IntoIterator<Item = ResponseType>>(
        &mut self,
        response_type: I,
    ) -> &mut Self {
        self.auth_url_parameters.response_type = response_type.into_iter().collect();
        self
    }

    /// Specifies how the identity platform should return the requested token to your app.
    ///
    /// Supported values:
    ///
    /// - **query**: Default when requesting an access token. Provides the code as a query string
    ///     parameter on your redirect URI. The query parameter is not supported when requesting an
    ///     ID token by using the implicit flow.
    /// - **fragment**: Default when requesting an ID token by using the implicit flow.
    ///     Also supported if requesting only a code.
    /// - **form_post**: Executes a POST containing the code to your redirect URI.
    ///     Supported when requesting a code.
    pub fn with_response_mode(&mut self, response_mode: ResponseMode) -> &mut Self {
        self.auth_url_parameters.response_mode = Some(response_mode);
        self
    }

    /// A value included in the request, generated by the app, that is included in the
    /// resulting id_token as a claim. The app can then verify this value to mitigate token
    /// replay attacks. The value is typically a randomized, unique string that can be used
    /// to identify the origin of the request.
    ///
    /// Because openid requires a nonce as part of the OAuth flow a nonce is already included.
    /// The nonce is generated internally using the same requirements of generating a secure
    /// random string as is done when using proof key for code exchange (PKCE) in the
    /// authorization code grant. If you are unsure or unclear how the nonce works then it is
    /// recommended to stay with the generated nonce as it is cryptographically secure.
    pub fn with_nonce<T: AsRef<str>>(&mut self, nonce: T) -> &mut Self {
        self.auth_url_parameters.nonce = nonce.as_ref().to_owned();
        self
    }

    /// A value included in the request, generated by the app, that is included in the
    /// resulting id_token as a claim. The app can then verify this value to mitigate token
    /// replay attacks. The value is typically a randomized, unique string that can be used
    /// to identify the origin of the request.
    ///
    /// The nonce is generated in the same way as generating a PKCE.
    ///
    /// Internally this method uses the Rust ring cyrpto library to
    /// generate a secure random 32-octet sequence that is base64 URL
    /// encoded (no padding). This sequence is hashed using SHA256 and
    /// base64 URL encoded (no padding) resulting in a 43-octet URL safe string.
    #[doc(hidden)]
    fn with_nonce_generated(&mut self) -> anyhow::Result<&mut Self> {
        self.auth_url_parameters.nonce = Crypto::secure_random_string()?;
        Ok(self)
    }

    pub fn with_state<T: AsRef<str>>(&mut self, state: T) -> &mut Self {
        self.auth_url_parameters.state = Some(state.as_ref().to_owned());
        self
    }

    pub fn with_scope<T: ToString, I: IntoIterator<Item = T>>(&mut self, scopes: I) -> &mut Self {
        self.auth_url_parameters.scope = scopes.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// Automatically adds profile, email, id_token, and offline_access to the scope parameter.
    /// The openid scope is already included when using [OpenIdCredential]
    pub fn with_default_scope(&mut self) -> anyhow::Result<&mut Self> {
        self.with_nonce_generated()?;
        self.with_response_mode(ResponseMode::FormPost);
        self.with_response_type(vec![ResponseType::Code, ResponseType::IdToken]);
        self.with_scope(vec!["profile", "email", "id_token", "offline_access"]);
        Ok(self)
    }

    /// Indicates the type of user interaction that is required. Valid values are login, none,
    /// consent, and select_account.
    ///
    /// - **prompt=login** forces the user to enter their credentials on that request, negating single-sign on.
    /// - **prompt=none** is the opposite. It ensures that the user isn't presented with any interactive prompt.
    ///     If the request can't be completed silently by using single-sign on, the Microsoft identity platform returns an interaction_required error.
    /// - **prompt=consent** triggers the OAuth consent dialog after the user signs in, asking the user to
    ///     grant permissions to the app.
    /// - **prompt=select_account** interrupts single sign-on providing account selection experience
    ///     listing all the accounts either in session or any remembered account or an option to choose to use a different account altogether.
    pub fn with_prompt<I: IntoIterator<Item = Prompt>>(&mut self, prompt: I) -> &mut Self {
        self.auth_url_parameters.prompt = prompt.into_iter().collect();
        self
    }

    /// Optional
    /// The realm of the user in a federated directory. This skips the email-based discovery
    /// process that the user goes through on the sign-in page, for a slightly more streamlined
    /// user experience. For tenants that are federated through an on-premises directory
    /// like AD FS, this often results in a seamless sign-in because of the existing login session.
    pub fn with_domain_hint<T: AsRef<str>>(&mut self, domain_hint: T) -> &mut Self {
        self.auth_url_parameters.domain_hint = Some(domain_hint.as_ref().to_owned());
        self
    }

    /// Optional
    /// You can use this parameter to pre-fill the username and email address field of the
    /// sign-in page for the user, if you know the username ahead of time. Often, apps use
    /// this parameter during reauthentication, after already extracting the login_hint
    /// optional claim from an earlier sign-in.
    pub fn with_login_hint<T: AsRef<str>>(&mut self, login_hint: T) -> &mut Self {
        self.auth_url_parameters.login_hint = Some(login_hint.as_ref().to_owned());
        self
    }

    pub fn build(&self) -> OpenIdAuthorizationUrl {
        self.auth_url_parameters.clone()
    }

    pub fn url(&self) -> AuthorizationResult<Url> {
        self.auth_url_parameters.url()
    }
}
