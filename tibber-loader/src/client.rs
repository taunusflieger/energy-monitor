use crate::{config::Config, consts, errors::TibberLoaderError};
use anyhow::Result;
use graphql_client::GraphQLQuery;
use graphql_client::Response as GraphQLResponse;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use std::time::Duration;

pub fn connect(config: &Config) -> Result<Client, TibberLoaderError> {
    let mut headers = HeaderMap::new();

    headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_str("application/json")?,
    );
    headers.insert(
        reqwest::header::AUTHORIZATION,
        HeaderValue::from_str(format!("Bearer {}", config.token).as_str())?,
    );

    let client = Client::builder()
        .user_agent(consts::get_user_agent())
        .default_headers(headers)
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();
    Ok(client)
}

pub async fn post_graphql<Q: GraphQLQuery, U: reqwest::IntoUrl>(
    client: &reqwest::Client,
    url: U,
    variables: Q::Variables,
) -> Result<Q::ResponseData, TibberLoaderError> {
    let body = Q::build_query(variables);
    let res: GraphQLResponse<Q::ResponseData> =
        client.post(url).json(&body).send().await?.json().await?;

    if let Some(errors) = res.errors {
        if errors[0].message.to_lowercase().contains("not authorized") {
            Err(TibberLoaderError::Unauthorized)
        } else {
            Err(TibberLoaderError::GraphQLError(errors[0].message.clone()))
        }
    } else if let Some(data) = res.data {
        Ok(data)
    } else {
        Err(TibberLoaderError::MissingResponseData)
    }
}
