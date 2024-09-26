use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use dotenv::dotenv;
use std::env;
use env_logger::Env;
use log::info;
use actix_cors::Cors;
use actix_web::middleware::Logger;

pub mod features;
use features::{app::app_state::AppState, dify::handlers::openai::{
    chat_completion::chat_completion, image::generate_image
}};

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, OpenAI API emulator!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let dify_api_url = env::var("DIFY_API_URL").expect("DIFY_API_URL must be set");

    #[cfg(feature = "logging")]
    env_logger::init_from_env(Env::default().default_filter_or("debug"));

    info!("Dify server URL: {}", dify_api_url);

    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8223".to_string());
    let server_addr = format!("{}:{}", host, port);

    println!("Starting server at http://{}", server_addr);

    let app_state = AppState {
        dify_api_url: dify_api_url.clone(),
    };
    let app_data = web::Data::new(app_state);

    HttpServer::new(move || {
        App::new()
        .app_data(app_data.clone())
            .wrap(Cors::default().allow_any_origin())
            .wrap(Logger::default())
            .route("/", web::get().to(hello))
            .service(
                web::scope("/v1")
                    .route("/chat/completions", web::post().to(chat_completion))
                    .route("/images/generations", web::post().to(generate_image))
            )
    })
    .bind(server_addr)?
    .run()
    .await
}