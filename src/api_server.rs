use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::sync::Mutex;
use serde::{Serialize, Deserialize};

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

pub async fn run_server() -> std::io::Result<()> {
    let data = web::Data::new(AppState {
        summaries: Mutex::new(Vec::new()),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/content", web::get().to(get_content))
    })
    .bind("127.0.0.1:8080")?
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
