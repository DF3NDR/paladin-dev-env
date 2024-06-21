use crate::domain::services::content_delivery_service::ContentDeliveryService;
use actix_web::{web, HttpResponse, Responder, Scope};
pub struct HttpContentDeliverer;

impl ContentDeliveryService for HttpContentDeliverer {
    fn deliver_content(&self, content: &str) -> Result<(), String> {
        // Implementation here
        Ok(())
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/deliver")
        .route(web::get().to(deliver_content)));
}

async fn deliver_content() -> impl Responder {
    HttpResponse::Ok().body("Content delivered")
}
