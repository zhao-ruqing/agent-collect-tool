use anyhow::Result;
use std::env;

#[derive(Clone)]
pub struct BackendConfig {
    pub database_url: String,
    pub port: u16,
    pub salt: String,
    pub jwt_secret: String,
    pub agent_api_secret: String,
}

impl BackendConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        Ok(Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            salt: env::var("SALT").unwrap_or_else(|_| "default_salt".to_string()),
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            agent_api_secret: env::var("AGENT_API_SECRET")
                .unwrap_or_else(|_| String::new()),
        })
    }
}
