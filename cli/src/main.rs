use clap::clap_derive::*;
use clap::Parser;
use client::DialecticDjClient;

mod client;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct TopLevel {
    #[clap(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand, Debug)]
enum Subcommands {
    Search {
        #[clap(short, long, value_parser)]
        /// Spotify search query
        query: String,
    },
}

fn main() {
    let args = TopLevel::parse();

    let client = DialecticDjClient::new("127.0.0.1:8000");

    match args.command {
        Subcommands::Search { query } => {
            let res = client.search(&query);
            if let Err(err) = res {
                println!("SEARCH FAILED: {}", err);
                return;
            }

            match res.unwrap().0 {
                rspotify::model::SearchResult::Tracks(tracks) => {
                    if let Some(track) = tracks.items.first() {
                        println!("track: {}", track.name);
                        println!("link: {:#?}", track.external_urls.get("spotify").unwrap());
                    }
                }
                _ => {}
            }

        }
    }
}
