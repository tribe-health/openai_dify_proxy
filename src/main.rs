use actix_web::{App, HttpServer, web};
use dotenv::dotenv;
use std::env;
use env_logger::Env;
use log::info;
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::web::Data;

mod features;

use crate::features::app::app_state::AppState;
use crate::features::dify::handlers::openai::chat_completion::chat_completion;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    // Load configuration variables with better error handling
    let dify_api_url = env::var("DIFY_API_URL").expect("DIFY_API_URL must be set");
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
    let app_state = AppState {
        dify_api_url: dify_api_url.clone(),
    };
    let app_data = Data::new(app_state);

    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .wrap(Cors::default().allow_any_origin())
            .wrap(Logger::default())
            .service(
                web::scope("/v1")
                    .route("/chat/completions", web::post().to(chat_completion))
                // Uncomment and implement if needed
                // .route("/images/generations", web::post().to(generate_image))
            )
    })
    .bind(server_addr)?
    .run()
    .await
}
