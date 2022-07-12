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
            client.search(&query);
            println!("search query is: {}", query);


        }
    }
}
