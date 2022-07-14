#![feature(let_else, try_blocks, unzip_option)]

mod archive_store;

mod conductor;

mod config;

mod errors;

mod init;

mod invoker;

mod message {
    pub(crate) mod c2i;
    pub(crate) mod i2c;
}

mod polygon {
    pub(crate) mod converter;
    pub(crate) mod parser;
    pub(crate) mod tests;
}

mod problem {
    pub(crate) mod config;
    pub(crate) mod program;
    pub(crate) mod strategy;
    pub(crate) mod strategy_format;
}

mod verdict;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    init::main().await
}
