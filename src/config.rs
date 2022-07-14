use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub listen: ListenConfig,
    pub data: DataConfig,
}

#[derive(Deserialize)]
pub struct ListenConfig {
    pub invokers: String,
}

#[derive(Deserialize)]
pub struct DataConfig {
    pub problems: String,
}
