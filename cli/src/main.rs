use clap::clap_derive::*;
use clap::Parser;

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

    match args.command {
        Subcommands::Search { query } => {
            println!("search query is: {}", query);
        }
    }
}
