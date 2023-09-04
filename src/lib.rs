// Copyright 2023 Salesforce, Inc. All rights reserved.
mod generated;

use anyhow::Result;

use pdk::api::hl::*;

use crate::generated::config::Config;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

pub enum FilterError {
    Unexpected,
    NoToken,
    InactiveToken,
    ExpiredToken,
    NotYetActive,
    ClientError(HttpClientError),
    NonParsableIntrospectionBody(serde_json::Error),
}

#[derive(Deserialize)]
pub struct IntrospectionResponse {
    pub active: bool,
    pub exp: Option<u64>,
    pub nbf: Option<u64>,
}

async fn introspect_token(
    token: &str,
    config: &Config,
    client: HttpClient,
) -> Result<IntrospectionResponse, FilterError> {
    let body =
        serde_urlencoded::to_string([("token", token)]).map_err(|_| FilterError::Unexpected)?;

    let headers = vec![
        ("content-type", "application/x-www-form-urlencoded"),
        ("Authorization", config.authorization.as_str()),
    ];

    let response = client
        .request(config.upstream.as_str(), config.host.as_str())
        .path(config.path.as_str())
        .headers(headers)
        .body(body.as_bytes())
        .post()
        .await
        .map_err(FilterError::ClientError)?;

    if response.status_code() == 200 {
        serde_json::from_slice(response.body()).map_err(FilterError::NonParsableIntrospectionBody)
    } else {
        Err(FilterError::InactiveToken)
    }
}

async fn do_filter(
    request: impl HeadersHandler,
    config: &Config,
    client: HttpClient,
) -> Result<(), FilterError> {
    //Extract the token from the request

    let result = config
        .token_extractor
        .resolve_on_headers(&request)
        .map_err(|_| FilterError::NoToken)?;

    let token = result.as_str().ok_or(FilterError::NoToken)?;

    let response = introspect_token(token, config, client).await?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| FilterError::Unexpected)?
        .as_secs();

    if !response.active {
        return Err(FilterError::InactiveToken);
    }

    //validates if token has expired
    if response.exp.map(|exp| now > exp).unwrap_or_default() {
        return Err(FilterError::ExpiredToken);
    }

    //validates if token has started its validity period
    if response.nbf.map(|nbf| now < nbf).unwrap_or_default() {
        return Err(FilterError::NotYetActive);
    }

    Ok(())
}

/// Generates a standard early response that indicates the token validation failed
fn unauthorized_response() -> Flow<()> {
    Flow::Break(Response::new(401).with_headers(vec![(
        "WWW-Authenticate".to_string(),
        "Bearer realm=\"oauth2\"".to_string(),
    )]))
}

/// Generates a standard early response that indicates that there was an unexpected error
fn server_error_response() -> Flow<()> {
    Flow::Break(Response::new(500))
}

/// Defines a filter function that works as a wrapper for the real filter function that enables simplified error handling
async fn request_filter(state: RequestState, client: HttpClient, config: &Config) -> Flow<()> {
    let state = state.into_headers_state().await;

    let guess: String = String::from("Hello");

    match do_filter(state, config, client).await {
        Ok(_) => Flow::Continue(()),
        Err(err) => match err {
            FilterError::Unexpected => {
                logger::warn!("Unexpected error occurred while processing the request.");
                server_error_response()
            }
            FilterError::NoToken => {
                logger::debug!("No authorization token was provided.");
                unauthorized_response()
            }
            FilterError::InactiveToken => {
                logger::debug!("Token is marked as inactive by the introspection endpoint.");
                unauthorized_response()
            }
            FilterError::ExpiredToken => {
                logger::debug!("Expiration time on the token has been exceeded.");
                unauthorized_response()
            }
            FilterError::NotYetActive => {
                logger::debug!(
                    "Token is not yet valid, since time set in the nbf claim has not been reached."
                );
                unauthorized_response()
            }
            FilterError::ClientError(err) => {
                logger::warn!(
                    "Error sending the request to the introspection endpoint. {:?}.",
                    err
                );
                server_error_response()
            }
            FilterError::NonParsableIntrospectionBody(err) => {
                logger::warn!(
                    "Error parsing the response from the introspection endpoint. {}.",
                    err
                );
                server_error_response()
            }
        },
    }
}

#[entrypoint]
async fn configure(launcher: Launcher, Configuration(bytes): Configuration) -> Result<()> {
    let config = serde_json::from_slice(&bytes)?;
    let filter = on_request(|request, client| request_filter(request, client, &config));
    launcher.launch(filter).await?;
    Ok(())
}
