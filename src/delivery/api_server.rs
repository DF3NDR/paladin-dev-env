use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::sync::Mutex;
use serde::{Serialize, Deserialize};
use crate::config::Settings;

#[derive(Serialize, Deserialize)]
pub struct SummaryResponse {
    pub summaries: Vec<String>,
}

pub async fn get_content(data: web::Data<AppState>) -> impl Responder {
    let summaries = data.summaries.lock().unwrap();
    HttpResponse::Ok().json(SummaryResponse {
        summaries: summaries.clone(),
    })
}

pub struct AppState {
    pub summaries: Mutex<Vec<String>>,
}

pub async fn run_server(config: Settings) -> std::io::Result<()> {
    let data = web::Data::new(AppState {
        summaries: Mutex::new(Vec::new()),
    });

    let server_config = config.server.clone();
    let config = std::sync::Arc::new(config);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .app_data(web::Data::new(config.clone()))
            .configure(crate::delivery::router::configure)
    })
    .bind(format!("{}:{}", server_config.host, server_config.port))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_get_content() {
        let data = web::Data::new(AppState {
            summaries: Mutex::new(vec!["Test summary".to_string()]),
        });

        let mut app = test::init_service(App::new().app_data(data.clone()).route("/content", web::get().to(get_content))).await;
        let req = test::TestRequest::with_uri("/content").to_request();
        let resp: SummaryResponse = test::call_and_read_body_json(&mut app, req).await;

        assert_eq!(resp.summaries, vec!["Test summary".to_string()]);
    }
}
