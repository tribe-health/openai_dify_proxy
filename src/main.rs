use actix_web::{App, HttpServer, web};
use dotenv::dotenv;
use std::env;
use env_logger::Env;
use log::info;
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::web::Data;

mod features;
mod utils;

use crate::features::app::app_state::AppState;
use crate::features::dify::handlers::openai::{
    chat_completion::chat_completion,
    image::{create_image, replicate_webhook},
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    // Load configuration variables with better error handling
    let dify_api_url = env::var("DIFY_API_URL").expect("DIFY_API_URL must be set");
    let replicate_api_key = env::var("REPLICATE_API_KEY").expect("REPLICATE_API_KEY must be set");
    let ipfs_url = env::var("IPFS_URL").unwrap_or_else(|_| "https://ipfs.tribemedia.io".to_string());
    let public_url = env::var("PUBLIC_URL").expect("PUBLIC_URL must be set");
    let supabase_url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let supabase_key = env::var("SUPABASE_KEY").expect("SUPABASE_KEY must be set");
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8223".to_string());

    #[cfg(feature = "logging")]
    {
        env_logger::init_from_env(
            Env::default()
                .filter_or(env_logger::DEFAULT_FILTER_ENV, "debug")
                .write_style_or(env_logger::DEFAULT_WRITE_STYLE_ENV, "always"),
        );
    }

    info!("Dify server URL: {}", dify_api_url);
    let server_addr = format!("{}:{}", host, port);

    println!("Starting server at http://{}", server_addr);

    // Create and clone the app state
    let app_state = AppState::new(
        dify_api_url.clone(),
        replicate_api_key.clone(),
        ipfs_url.clone(),
        public_url.clone(),
        supabase_url.clone(),
        supabase_key.clone(),
    );
    let app_data = Data::new(app_state);

    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .wrap(Cors::default().allow_any_origin())
            .wrap(Logger::default())
            .service(
                web::scope("/v1")
                    .route("/chat/completions", web::post().to(chat_completion))
                    .route("/images/generations", web::post().to(create_image))
                    .route("/webhook/replicate/{task_id}", web::post().to(replicate_webhook))
            )
    })
    .bind(server_addr)?
    .run()
    .await
}
