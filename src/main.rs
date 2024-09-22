use std::env;
use actix_web::{web, App, HttpServer};
use actix_cors::Cors;
use actix_web::middleware::Logger;
use dotenv::dotenv;

#[cfg(feature = "logging")]
use env_logger::Env;
use log::info;
use crate::features::app::app_state::AppState;

mod features;
mod utils;

use crate::features::dify::handlers::openai::chat_completion::chat_completion;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let dify_api_url = env::var("DIFY_API_URL").expect("DIFY_API_URL must be set");

    #[cfg(feature = "logging")]
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    info!("Dify server URL: {}", dify_api_url);

    let host = "0.0.0.0";
    let port = 8223;

    info!("Starting server at http(s)://{}:{}", host, port);

    let app_state = web::Data::new(AppState {
        dify_api_url: dify_api_url.clone(),
    });

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(Logger::default())
            .service(
                web::resource("/v1/chat{tail:.*}")
                    .route(web::post().to(chat_completion))
            )
    })
    .bind((host, port))?
    .run()
    .await
}