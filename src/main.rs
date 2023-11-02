use clap::Parser;

#[derive(Parser)]
struct Cli {
    postcode: String,
}

fn main() {
    let args = Cli::parse();
    println!("{}", &args.postcode);
}