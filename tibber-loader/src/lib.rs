use crate::{
    client::{connect, post_graphql},
    config::Config,
    errors::TibberLoaderError,
    gql::queries::{self, PriceInfo},
};
use reqwest::Client;

pub mod client;
pub mod config;
pub mod consts;
pub mod errors;
pub mod gql;

#[derive(Debug, Clone)]
/// ID used to represent a house / home
pub struct HomeId(pub String);

#[derive(Debug, Clone)]
pub struct Session {
    url: String,
    pub user_id: String,
    /// Only a single home is supported
    home_id: HomeId,
    client: Client,
}

#[derive(Debug, Clone)]
struct User {
    /// User id
    pub user_id: String,
    /// Only a single home is supported
    pub home_id: HomeId,
}

impl Session {
    pub async fn new(config: Config) -> Result<Self, TibberLoaderError> {
        let url = config.url.clone();
        let client = connect(&config)?;
        let user = Session::get_user(&client, config).await?;
        Ok(Session {
            url,
            user_id: user.user_id,
            home_id: user.home_id,
            client,
        })
    }

    async fn get_user(client: &Client, config: Config) -> Result<User, TibberLoaderError> {
        let viewer = post_graphql::<queries::Viewer, _>(
            client,
            config.url.clone(),
            queries::viewer::Variables {},
        )
        .await?
        .viewer;

        let homes: Vec<HomeId> = viewer
            .homes
            .into_iter()
            .flatten()
            .map(|h| HomeId(h.id))
            .collect();
        if homes.len() != 1 {
            Err(TibberLoaderError::OnlyOneHomeSupported)
        } else {
            Ok(User {
                user_id: viewer.login.ok_or(TibberLoaderError::MissingUserId)?,
                home_id: homes[0].to_owned(),
            })
        }
    }

    pub async fn get_current_price(&self) -> Result<Option<PriceInfo>, TibberLoaderError> {
        let price = post_graphql::<queries::Price, _>(
            &self.client,
            &self.url.clone(),
            queries::price::Variables {
                id: self.home_id.0.clone(),
            },
        )
        .await?
        .viewer;

        Ok(PriceInfo::new(
            price
                .home
                .current_subscription
                .ok_or(TibberLoaderError::NoSubscription)?
                .price_info
                .ok_or(TibberLoaderError::NoPriceInfo)?
                .current
                .ok_or(TibberLoaderError::NoCurrentPrice)?,
        ))
    }
}
