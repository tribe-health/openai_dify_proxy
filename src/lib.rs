pub mod features;
pub mod utils;

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;
    use features::app::app_state::AppState;
    use features::dify::handlers::openai::image::{create_image, replicate_webhook};
    use serde_json::json;

    #[actix_web::test]
    async fn test_basic_setup() {
        let app_state = AppState::new(
            "test_url".to_string(),
            "test_key".to_string(),
            "test_ipfs".to_string(),
            "test_public".to_string(),
            "test_supabase".to_string(),
            "test_key".to_string(),
        );
        assert!(app_state.dify_api_url == "test_url");
    }
}
