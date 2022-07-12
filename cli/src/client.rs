use reqwest::blocking::Client;

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

    pub fn search(&self, query: &str) {
        let res = self
            .client
            .post(format!("http://{}/search", self.address))
            .body(query.to_owned())
            .send();

        if let Err(err) = res {
            println!("failed to send request: {}", err);
        } else {
            println!("{:#?} is the response", res.unwrap().bytes());
        }
    }
}
