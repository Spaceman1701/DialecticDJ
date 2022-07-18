use core::DialecticDj::SearchResult;
use std::{
    fmt::{Display, Write},
    io::Error,
};

use anyhow::Result;
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

    pub fn search(&self, query: &str) -> Result<SearchResult> {
        let res = self
            .client
            .post(format!("http://{}/search", self.address))
            .body(query.to_owned())
            .send()?;

        let json = res.json()?;

        return Ok(json);
    }

    pub fn play_track(&self, track: &str) -> Result<()>{
        let res = self.client.post(format!("http://{}/play/{}", self.address, track)).send()?;
        return Ok(());
    }

    pub fn add_track_to_queue(&self, track: &str) -> Result<()> {
        let res = self.client.post(format!("http://{}/queue/{}", self.address, track)).send()?;
        return Ok(());
    }
}
