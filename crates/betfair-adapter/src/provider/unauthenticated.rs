use std::sync::Arc;
use std::time::Duration;

use backon::{BackoffBuilder, ExponentialBuilder};
use betfair_types::bot_login::{BotLoginResponse, LoginError};
use reqwest::{Client, header};
use tokio::task::JoinHandle;
use tokio::time::{Instant, interval, sleep};

use super::BetfairRpcClient;
use crate::secret::{self, SessionToken};
use crate::{
    ApiError, ApplicationKey, Authenticated, BetfairConfigBuilder, Identity, Unauthenticated, urls,
};

/// Represents an unauthenticated Betfair RPC provider.
impl BetfairRpcClient<Unauthenticated> {
    /// Creates a new instance of `UnauthenticatedBetfairRpcProvider` using the provided secret
    /// provider.
    pub fn new(secret_provider: secret::SecretProvider) -> Result<Self, ApiError> {
        let config = BetfairConfigBuilder::new_with_global_jurisdiction(secret_provider);
        Self::new_with_config(config)
    }

    /// Authenticates the user and returns an `AuthenticatedBetfairRpcProvider`.
    pub async fn authenticate(
        self,
    ) -> Result<
        (
            Arc<BetfairRpcClient<Authenticated>>,
            JoinHandle<Result<(), ApiError>>,
        ),
        ApiError,
    > {
        let (session_token, authenticated_client) = retry_bot_login(|| self.bot_log_in()).await?;

        let state = Authenticated {
            session_token,
            authenticated_client,
        };
        let client = Arc::new(BetfairRpcClient {
            state,
            bot_login_client: self.bot_login_client,
            rest_base: self.rest_base,
            keep_alive: self.keep_alive,
            bot_login: self.bot_login,
            logout: self.logout,
            login: self.login,
            stream: self.stream,
            secret_provider: self.secret_provider,
        });

        let keep_alive = tokio::spawn({
            let client = Arc::downgrade(&client);
            async move {
                const MINUTES_15: u64 = 60 * 15;
                let mut interval = interval(Duration::from_secs(MINUTES_15));

                loop {
                    interval.tick().await;
                    let Some(client) = client.upgrade() else {
                        // all references to the client have been dropped, we can exit the `keep-alive` loop
                        return Ok(());
                    };

                    let t1 = Instant::now();
                    let _res = client.keep_alive()?.execute().await?;
                    let t2 = Instant::now();
                    let diff = t2.saturating_duration_since(t1);
                    tracing::info!(?diff, "latency");
                }
            }
        });

        Ok((client, keep_alive))
    }

    /// Creates a new instance of `UnauthenticatedBetfairRpcProvider` with a specific configuration.
    pub fn new_with_config(
        config: BetfairConfigBuilder<
            impl urls::RetrieveUrl<urls::RestBase> + core::fmt::Debug,
            impl urls::RetrieveUrl<urls::KeepAlive> + core::fmt::Debug,
            impl urls::RetrieveUrl<urls::BotLogin> + core::fmt::Debug,
            impl urls::RetrieveUrl<urls::Logout> + core::fmt::Debug,
            impl urls::RetrieveUrl<urls::InteractiveLogin> + core::fmt::Debug,
            impl urls::RetrieveUrl<urls::Stream> + core::fmt::Debug,
        >,
    ) -> Result<Self, ApiError> {
        let rest_base = config.rest.url();
        let keep_alive = config.keep_alive.url();
        let bot_login = config.bot_login.url();
        let login = config.login.url();
        let logout = config.logout.url();
        let stream = config.stream.url();
        let secret_provider = config.secrets_provider;

        // Use this to get the session token
        let bot_login_client =
            login_client(&secret_provider.application_key, &secret_provider.identity)?;

        Ok(Self {
            state: Unauthenticated,
            bot_login_client,
            rest_base,
            keep_alive,
            bot_login,
            logout,
            login,
            stream,
            secret_provider,
        })
    }
}

impl<T> BetfairRpcClient<T> {
    /// Performs a non-interactive login to obtain a session token and authenticated client.
    #[tracing::instrument(skip(self), err)]
    pub(super) async fn bot_log_in(&self) -> Result<(SessionToken, reqwest::Client), ApiError> {
        let login_response = self
            .bot_login_client
            .post(self.bot_login.url().as_str())
            .form(&[
                ("username", self.secret_provider.username.0.expose_secret()),
                ("password", self.secret_provider.password.0.expose_secret()),
            ])
            .send()
            .await?
            .json::<BotLoginResponse>()
            .await?;
        let login_response = login_response.0.map_err(ApiError::BotLoginError)?;
        let session_token = SessionToken(login_response.session_token);
        let authenticated_client =
            logged_in_client(&self.secret_provider.application_key, &session_token)?;
        Ok((session_token, authenticated_client))
    }

    // TODO implement interactive login
}

// Retry once a minute so an almost-expired ban does not add a full ban-length wait.
const TEMPORARY_BAN_RETRY_DELAY: Duration = Duration::from_secs(60);

async fn retry_bot_login<T, F, Fut>(mut login: F) -> Result<T, ApiError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiError>>,
{
    let mut backoff = ExponentialBuilder::new().build();
    loop {
        match login().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                let Some(delay) = backoff.next() else {
                    return Err(error);
                };
                let delay = if matches!(
                    error,
                    ApiError::BotLoginError(LoginError::TemporaryBanTooManyRequests)
                ) {
                    delay.max(TEMPORARY_BAN_RETRY_DELAY)
                } else {
                    delay
                };
                tracing::warn!(?delay, %error, "Bot login failed; waiting before retrying");
                sleep(delay).await;
            }
        }
    }
}

/// Creates a logged-in HTTP client with the specified application key and session token.
fn logged_in_client(
    app_key: &ApplicationKey,
    session_token: &SessionToken,
) -> eyre::Result<reqwest::Client> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "X-Application",
        header::HeaderValue::from_str(app_key.0.expose_secret().as_str())?,
    );
    headers.insert(
        "Accept",
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        "Accept-Encoding",
        header::HeaderValue::from_static("gzip, deflate"),
    );
    headers.insert(
        "Content-Type",
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        "X-Authentication",
        header::HeaderValue::from_str(session_token.0.expose_secret().as_str())?,
    );
    Ok(reqwest::Client::builder()
        .use_rustls_tls()
        .default_headers(headers)
        .build()?)
}

/// Creates a login client with the specified application key and identity.
fn login_client(application_key: &ApplicationKey, identity: &Identity) -> Result<Client, ApiError> {
    const KEEP_ALIVE_INTERVAL: core::time::Duration = core::time::Duration::from_secs(15);
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "X-Application",
        header::HeaderValue::from_str(application_key.0.expose_secret().as_str()).unwrap(),
    );
    headers.insert(
        "Accept",
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        "Content-Type",
        header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    let temp_client = reqwest::Client::builder()
        .use_rustls_tls()
        .identity(identity.0.expose_secret().clone())
        .default_headers(headers)
        .http2_keep_alive_interval(KEEP_ALIVE_INTERVAL)
        .http2_keep_alive_while_idle(true)
        .build()?;
    Ok(temp_client)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::ready;

    #[tokio::test(start_paused = true)]
    async fn successful_login_does_not_wait_or_retry() {
        let start = Instant::now();
        let mut attempts = 0;
        let result = retry_bot_login(|| {
            attempts += 1;
            ready(Ok::<_, ApiError>(42))
        })
        .await
        .unwrap();

        assert_eq!(result, 42);
        assert_eq!(attempts, 1);
        assert_eq!(start.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn failed_logins_back_off_and_return_the_last_error() {
        let start = Instant::now();
        let mut attempts = Vec::new();
        let error = retry_bot_login(|| {
            attempts.push(start.elapsed());
            ready(Err::<(), _>(ApiError::EyreError(eyre::eyre!(
                "login failure {}",
                attempts.len()
            ))))
        })
        .await
        .unwrap_err();

        assert_eq!(attempts, [0, 1, 3, 7].map(Duration::from_secs));
        assert_eq!(error.to_string(), "login failure 4");
    }

    #[tokio::test(start_paused = true)]
    async fn temporary_bans_wait_one_minute_before_each_retry() {
        let start = Instant::now();
        let mut attempts = Vec::new();
        retry_bot_login(|| {
            attempts.push(start.elapsed());
            ready(if attempts.len() < 3 {
                Err(ApiError::BotLoginError(
                    LoginError::TemporaryBanTooManyRequests,
                ))
            } else {
                Ok(())
            })
        })
        .await
        .unwrap();

        assert_eq!(attempts, [0, 60, 120].map(Duration::from_secs));
    }

    #[tokio::test(start_paused = true)]
    async fn repeated_temporary_bans_exhaust_retries_and_preserve_the_error() {
        let start = Instant::now();
        let mut attempts = Vec::new();
        let error = retry_bot_login(|| {
            attempts.push(start.elapsed());
            ready(Err::<(), _>(ApiError::BotLoginError(
                LoginError::TemporaryBanTooManyRequests,
            )))
        })
        .await
        .unwrap_err();

        assert_eq!(attempts, [0, 60, 120, 180].map(Duration::from_secs));
        assert!(matches!(
            error,
            ApiError::BotLoginError(LoginError::TemporaryBanTooManyRequests)
        ));
    }
}
