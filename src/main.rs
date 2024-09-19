use actix_web::{web, App, HttpServer};
use actix_cors::Cors;
use dotenv::dotenv;
use env_logger;
use log::info;

mod features;
mod utils;

use crate::features::dify::handlers::openai::chat_completion::chat_completion;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let host = "0.0.0.0";
    let port = 8080;

    info!("Starting server at http://{}:{}", host, port);

    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .service(
                web::resource("/v1/chat/completions")
                    .route(web::post().to(chat_completion))
            )
    })
    .bind((host, port))?
    .run()
    .await
}