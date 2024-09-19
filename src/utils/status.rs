// src/utils/status.rs

use reqwest::StatusCode as ReqwestStatusCode;
use actix_web::http::StatusCode as ActixStatusCode;

pub fn reqwest_to_actix_status(status: ReqwestStatusCode) -> ActixStatusCode {
    ActixStatusCode::from_u16(status.as_u16())
        .unwrap_or(ActixStatusCode::INTERNAL_SERVER_ERROR)
}