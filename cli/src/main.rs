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

            if let Err(err) = res {
                panic!("failed: {}", err);
            }
            let unwrapped = res.unwrap();
            let first = unwrapped.tracks.first();
            match first {
                Some(track) => {
                    let name = &track.name;
                    let author = track.album.artists.first().map(|artist| &artist.name[..]).unwrap_or("unknown");
                    let duration = &track.duration.as_secs();

                    println!("--- Top Result ---");
                    println!("Name:     {}", name);
                    println!("By:       {}", author);
                    println!("Duration: {} seconds", duration);
                    println!("Link:     {}", track.external_urls.get("spotify").unwrap())
                }

                None => {
                    println!("no track found");
                }
            }

        }
    }
}
