use core::DDJ::SearchResult;
use std::{io::Error, fmt::{Display, Write}};

use reqwest::blocking::Client;
use thiserror::Error;

pub struct DialecticDjClient {
    address: String,
    client: Client,
}

impl DialecticDjClient {
    pub fn new(address: &str) -> DialecticDjClient {
        return DialecticDjClient {
            address: address.to_owned(),
            client: Client::new(),
        };
    }


    pub fn search(&self, query: &str) -> Result<SearchResult, DJServiceError> {
        let res = self
            .client
            .post(format!("http://{}/search", self.address))
            .body(query.to_owned())
            .send()?;


        let json = res.json()?;

        return Ok(json);

    }
}

#[derive(Error, Debug)]
pub enum DJServiceError {
    RequestFailed(reqwest::Error),
}

impl From<reqwest::Error> for DJServiceError {
    fn from(cause: reqwest::Error) -> Self {
        return Self::RequestFailed(cause);
    }
}

impl Display for DJServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("error using DJ service backend:")?;
        match self {
            DJServiceError::RequestFailed(inner) => {
                inner.fmt(f)
            },
        }
    }
}
