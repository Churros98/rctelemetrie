use clap::Parser;

#[derive(Debug, Parser, Clone)]
pub struct Cli {
    pub uuid: String,
    pub db_url: String,
    pub db_username: String,
    pub db_password: String,
}