use core::marker::PhantomData;

use betfair_types::keep_alive;
use betfair_types::types::{BetfairRpcRequest, BetfairRpcTransport};
use serde::Deserialize;
use tracing::instrument;

use crate::{ApiError, Authenticated, BetfairRpcClient};

impl BetfairRpcClient<Authenticated> {
    /// Sends a request and returns the response or an error.
    ///
    /// # Parameters
    /// - `request`: The request to be sent.
    ///
    /// # Returns
    /// A result containing either the response or an `ApiError`.
    #[tracing::instrument(skip_all, ret, err, fields(req = ?request))]
    pub async fn send_request<T>(&self, request: T) -> Result<T::Res, ApiError>
    where
        T: BetfairRpcRequest + serde::Serialize + core::fmt::Debug,
        T::Res: serde::de::DeserializeOwned + core::fmt::Debug,
        T::Error: serde::de::DeserializeOwned,
        ApiError: From<<T as BetfairRpcRequest>::Error>,
    {
        match self.build_request(request)?.execute().await?.json().await? {
            Ok(res) => Ok(res),
            Err(err) => Err(err.into()),
        }
    }

    /// Create a request
    ///
    /// # Parameters
    /// - `request`: The request to be sent.
    ///
    /// # Returns
    /// A result containing either the response or an `ApiError`.
    pub fn build_request<T>(&self, request: T) -> Result<BetfairRequest<T::Res, T::Error>, ApiError>
    where
        T: BetfairRpcRequest + serde::Serialize + core::fmt::Debug,
        T::Res: serde::de::DeserializeOwned + core::fmt::Debug,
        T::Error: serde::de::DeserializeOwned,
    {
        let endpoint = self.endpoint_for_request::<T>()?;
        let client = self.state.authenticated_client.clone();
        let request_builder = client.request(reqwest::Method::POST, endpoint.as_str());
        let reqwest_req = match T::transport() {
            BetfairRpcTransport::Rest => request_builder.json(&request).build()?,
            BetfairRpcTransport::JsonRpc => request_builder
                .json(&[JsonRpcRequest::new(T::method(), &request)])
                .build()?,
        };

        Ok(BetfairRequest {
            request: reqwest_req,
            client,
            result: PhantomData,
            err: PhantomData,
            transport: T::transport(),
        })
    }

    fn endpoint_for_request<T>(&self) -> Result<url::Url, ApiError>
    where
        T: BetfairRpcRequest,
    {
        match T::transport() {
            BetfairRpcTransport::Rest => Ok(self.rest_base.url().join(T::method())?),
            BetfairRpcTransport::JsonRpc => {
                endpoint_from_path(self.rest_base.url(), T::endpoint_path())
            }
        }
    }

    /// You can use Keep Alive to extend the session timeout period. The minimum session time is
    /// currently 20 minutes (Italian Exchange). On the international (.com) Exchange the current
    /// session time is 24 hours. Therefore, you should request Keep Alive within this time to
    /// prevent session expiry. If you don't call Keep Alive within the specified timeout period,
    /// the session will expire. Session times aren't determined or extended based on API activity.
    #[tracing::instrument(skip_all, ret, err)]
    pub fn keep_alive(&self) -> Result<BetfairRequest<keep_alive::Response, ()>, ApiError> {
        let endpoint = self.keep_alive.url();
        let client = self.state.authenticated_client.clone();
        let reqwest_req = client
            .request(reqwest::Method::GET, endpoint.as_str())
            .build()?;

        Ok(BetfairRequest {
            request: reqwest_req,
            client,
            result: PhantomData,
            err: PhantomData,
            transport: BetfairRpcTransport::Rest,
        })
    }

    /// You can use Logout to terminate your existing session.
    #[tracing::instrument(skip_all, ret, err)]
    pub fn logout(&self) -> Result<BetfairRequest<keep_alive::Response, ()>, ApiError> {
        let endpoint = self.logout.url();
        let client = self.state.authenticated_client.clone();
        let reqwest_req = client
            .request(reqwest::Method::GET, endpoint.as_str())
            .build()?;

        Ok(BetfairRequest {
            request: reqwest_req,
            client,
            result: PhantomData,
            err: PhantomData,
            transport: BetfairRpcTransport::Rest,
        })
    }
}

/// Encalpsulated HTTP request for the Betfair API
#[derive(Debug)]
pub struct BetfairRequest<T, E> {
    request: reqwest::Request,
    client: reqwest::Client,
    result: PhantomData<T>,
    err: PhantomData<E>,
    transport: BetfairRpcTransport,
}

impl<T, E> BetfairRequest<T, E> {
    /// execute an Betfair API request
    #[instrument(name = "execute_request", skip(self), fields(method = %self.request.method(), url = %self.request.url()))]
    pub async fn execute(self) -> Result<BetfairResponse<T, E>, ApiError> {
        let response = self.client.execute(self.request).await?;

        // Capture the current span
        let span = tracing::Span::current();

        Ok(BetfairResponse {
            response,
            result: PhantomData,
            err: PhantomData,
            span,
            transport: self.transport,
        })
    }
}

/// The raw response of the Betfair API request
#[derive(Debug)]
pub struct BetfairResponse<T, E> {
    response: reqwest::Response,
    result: PhantomData<T>,
    err: PhantomData<E>,
    // this span carries the context of the `BetfairRequest`
    span: tracing::Span,
    transport: BetfairRpcTransport,
}

impl<T, E> BetfairResponse<T, E> {
    /// Only check if the returtned HTTP response is of error type; don't parse the data
    ///
    /// Useful when you don't care about the actual response besides if it was an error.
    #[instrument(name = "response_ok", skip(self), err, parent = &self.span)]
    pub fn ok(self) -> Result<(), ApiError> {
        self.response.error_for_status()?;
        Ok(())
    }

    /// Check if the returned HTTP result is an error;
    /// Only parse the error type if we received an error.
    ///
    /// Useful when you don't care about the actual response besides if it was an error.
    #[instrument(name = "parse_response_json_err", skip(self), err, parent = &self.span)]
    pub async fn json_err(self) -> Result<Result<(), E>, ApiError>
    where
        E: serde::de::DeserializeOwned,
    {
        let status = self.response.status();
        let transport = self.transport;
        if status.is_success() {
            if transport == BetfairRpcTransport::Rest {
                Ok(Ok(()))
            } else {
                let bytes = self.response.bytes().await?;
                if bytes.is_empty() {
                    return Ok(Ok(()));
                }
                let res = parse_json_rpc_response::<serde_json::Value, E>(&bytes)?;
                Ok(res.map(|_| ()))
            }
        } else {
            let bytes = self.response.bytes().await?;
            let res = parse_betfair_error::<E>(&bytes, status, transport)?;
            Ok(Err(res))
        }
    }

    /// Parse the response json
    #[instrument(name = "parse_response_json", skip(self), err, parent = &self.span)]
    pub async fn json(self) -> Result<Result<T, E>, ApiError>
    where
        T: serde::de::DeserializeOwned,
        E: serde::de::DeserializeOwned,
    {
        let status = self.response.status();
        let transport = self.transport;
        let bytes = self.response.bytes().await?;
        if bytes.is_empty() {
            tracing::warn!("Received empty response body");
            return Err(ApiError::EmptyResponse);
        }
        if status.is_success() {
            let json = String::from_utf8_lossy(bytes.as_ref());
            tracing::debug!(response_body = %json, "Response JSON");

            match transport {
                BetfairRpcTransport::Rest => Ok(Ok(serde_json::from_slice::<T>(&bytes)?)),
                BetfairRpcTransport::JsonRpc => parse_json_rpc_response::<T, E>(&bytes),
            }
        } else {
            let res = parse_betfair_error::<E>(&bytes, status, transport)?;
            Ok(Err(res))
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct JsonRpcRequest<'a, T> {
    jsonrpc: &'static str,
    method: &'static str,
    params: &'a T,
    id: u64,
}

impl<'a, T> JsonRpcRequest<'a, T> {
    const fn new(method: &'static str, params: &'a T) -> Self {
        Self {
            jsonrpc: "2.0",
            method,
            params,
            id: 1,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonRpcPayload<T> {
    Batch(Vec<JsonRpcEnvelope<T>>),
    Single(JsonRpcEnvelope<T>),
}

#[derive(Debug, Deserialize)]
struct JsonRpcEnvelope<T> {
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct JsonRpcError {
    code: Option<i64>,
    message: Option<String>,
    data: Option<serde_json::Value>,
}

fn endpoint_from_path(base: &url::Url, endpoint_path: &str) -> Result<url::Url, ApiError> {
    if let Ok(url) = url::Url::parse(endpoint_path) {
        return Ok(url);
    }

    if endpoint_path.starts_with('/') {
        let mut endpoint = base.clone();
        endpoint.set_path(endpoint_path);
        endpoint.set_query(None);
        endpoint.set_fragment(None);
        Ok(endpoint)
    } else {
        Ok(base.join(endpoint_path)?)
    }
}

fn parse_betfair_error<E>(
    bytes: &[u8],
    status: reqwest::StatusCode,
    transport: BetfairRpcTransport,
) -> Result<E, ApiError>
where
    E: serde::de::DeserializeOwned,
{
    let json = String::from_utf8_lossy(bytes);
    tracing::error!(
        status = %status,
        body = %json,
        "Failed to execute request"
    );

    match transport {
        BetfairRpcTransport::Rest => Ok(serde_json::from_slice::<E>(bytes)?),
        BetfairRpcTransport::JsonRpc => parse_json_rpc_response::<serde_json::Value, E>(bytes)?
            .map_or_else(Ok, |_| {
                serde_json::from_slice::<E>(bytes).map_err(Into::into)
            }),
    }
}

fn parse_json_rpc_response<T, E>(bytes: &[u8]) -> Result<Result<T, E>, ApiError>
where
    T: serde::de::DeserializeOwned,
    E: serde::de::DeserializeOwned,
{
    let payload = serde_json::from_slice::<JsonRpcPayload<T>>(bytes)?;
    let envelope = match payload {
        JsonRpcPayload::Batch(mut batch) => {
            if batch.is_empty() {
                return Err(ApiError::EmptyResponse);
            }
            batch.remove(0)
        }
        JsonRpcPayload::Single(single) => single,
    };

    if let Some(result) = envelope.result {
        return Ok(Ok(result));
    }

    if let Some(error) = envelope.error {
        return parse_json_rpc_error(error).map(Err);
    }

    Err(ApiError::EmptyResponse)
}

fn parse_json_rpc_error<E>(error: JsonRpcError) -> Result<E, ApiError>
where
    E: serde::de::DeserializeOwned,
{
    if let Some(data) = error.data {
        return parse_error_value(data);
    }

    parse_error_value(serde_json::to_value(error)?)
}

fn parse_error_value<E>(value: serde_json::Value) -> Result<E, ApiError>
where
    E: serde::de::DeserializeOwned,
{
    // First try the value exactly as supplied. For JSON-RPC this is usually
    // `error.data`, which can already be the generated APING exception shape.
    match serde_json::from_value::<E>(value.clone()) {
        Ok(error) => Ok(error),
        Err(direct_err) => {
            // Some Betfair JSON-RPC errors wrap the APING exception one level
            // down, for example under a service-specific property. If the direct
            // parse failed, scan immediate object values for the expected error.
            if let serde_json::Value::Object(map) = value {
                for inner in map.into_values() {
                    if let Ok(error) = serde_json::from_value::<E>(inner) {
                        return Ok(error);
                    }
                }
            }
            // Preserve the original direct parse error; it points at the shape
            // the caller expected rather than at a fallback candidate.
            Err(direct_err.into())
        }
    }
}
