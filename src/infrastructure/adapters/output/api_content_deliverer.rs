/*
Api Content Deliverer

This output adapter is responsible for delivering content through an API endpoint.
It implements the ContentDeliveryService trait, which defines the methods for delivering content.
It can be used to deliver content to clients or other systems via HTTP requests.
It is designed to be used in a web application context, where content needs to be delivered
via an API endpoint.
It can be used to deliver notifications, alerts, or any other type of content that needs to be delivered via an API.
*/

use crate::application::ports::output::content_delivery_port::{
    ContentDeliveryService, BatchContentDeliveryService, DeliveryRequest, DeliveryResponse,
    DeliveryMethod, ContentPayload, DeliveryStatus, DeliveryStats, ContentDeliveryError,
    DeliveryPriority
};
use actix_web::{web, HttpResponse, Responder, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::Utc;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone)]
pub struct ApiContentDeliverer {
    http_client: reqwest::Client,
    delivery_history: Arc<Mutex<HashMap<Uuid, DeliveryResponse>>>,
    max_retries: u32,
    retry_delay_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiDeliveryRequest {
    pub content: serde_json::Value,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub priority: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiDeliveryResponse {
    pub success: bool,
    pub message: String,
    pub delivery_id: Option<String>,
}

impl ApiContentDeliverer {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            delivery_history: Arc::new(Mutex::new(HashMap::new())),
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }

    pub fn with_retry_config(mut self, max_retries: u32, retry_delay_ms: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_delay_ms = retry_delay_ms;
        self
    }

    async fn deliver_http(&self, endpoint: &str, headers: &Option<HashMap<String, String>>, payload: &ContentPayload) -> Result<(), ContentDeliveryError> {
        let mut request_builder = self.http_client.post(endpoint);

        // Add custom headers
        if let Some(headers) = headers {
            for (key, value) in headers {
                request_builder = request_builder.header(key, value);
            }
        }

        // Serialize payload
        let json_payload = serde_json::to_value(payload)
            .map_err(|e| ContentDeliveryError::SerializationError(e.to_string()))?;

        let api_request = ApiDeliveryRequest {
            content: json_payload,
            metadata: None,
            priority: "normal".to_string(),
        };

        let response = request_builder
            .json(&api_request)
            .send()
            .await
            .map_err(|e| ContentDeliveryError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ContentDeliveryError::DeliveryFailed(
                format!("HTTP {} - {}", response.status(), response.status().canonical_reason().unwrap_or("Unknown error"))
            ))
        }
    }

    async fn deliver_webhook(&self, url: &str, method: &str, payload: &ContentPayload) -> Result<(), ContentDeliveryError> {
        let json_payload = serde_json::to_value(payload)
            .map_err(|e| ContentDeliveryError::SerializationError(e.to_string()))?;

        let request_builder = match method.to_uppercase().as_str() {
            "POST" => self.http_client.post(url),
            "PUT" => self.http_client.put(url),
            "PATCH" => self.http_client.patch(url),
            _ => return Err(ContentDeliveryError::InvalidDeliveryMethod(format!("Unsupported HTTP method: {}", method))),
        };

        let response = request_builder
            .json(&json_payload)
            .send()
            .await
            .map_err(|e| ContentDeliveryError::NetworkError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ContentDeliveryError::DeliveryFailed(
                format!("Webhook delivery failed: HTTP {}", response.status())
            ))
        }
    }

    async fn execute_delivery(&self, request: &DeliveryRequest) -> Result<(), ContentDeliveryError> {
        match &request.delivery_method {
            DeliveryMethod::Http { endpoint, headers } => {
                self.deliver_http(endpoint, headers, &request.content_payload).await
            },
            DeliveryMethod::Webhook { url, method } => {
                self.deliver_webhook(url, method, &request.content_payload).await
            },
            DeliveryMethod::Email { .. } => {
                // Email delivery would be implemented here
                Err(ContentDeliveryError::DeliveryFailed("Email delivery not implemented".to_string()))
            },
            DeliveryMethod::Push { .. } => {
                // Push notification delivery would be implemented here
                Err(ContentDeliveryError::DeliveryFailed("Push notification delivery not implemented".to_string()))
            },
            DeliveryMethod::Sms { .. } => {
                // SMS delivery would be implemented here
                Err(ContentDeliveryError::DeliveryFailed("SMS delivery not implemented".to_string()))
            },
            DeliveryMethod::WebSocket { .. } => {
                // WebSocket delivery would be implemented here
                Err(ContentDeliveryError::DeliveryFailed("WebSocket delivery not implemented".to_string()))
            },
        }
    }

    async fn delivery_with_retry(&self, request: &DeliveryRequest) -> Result<u32, ContentDeliveryError> {
        let mut last_error = None;
        
        for attempt in 1..=self.max_retries {
            match self.execute_delivery(request).await {
                Ok(()) => return Ok(attempt),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.max_retries {
                        sleep(Duration::from_millis(self.retry_delay_ms * attempt as u64)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(ContentDeliveryError::DeliveryFailed("Unknown error".to_string())))
    }

    fn create_delivery_response(&self, delivery_id: Uuid, status: DeliveryStatus, attempt_count: u32, error: Option<&ContentDeliveryError>) -> DeliveryResponse {
        DeliveryResponse {
            delivery_id,
            status,
            delivered_at: if matches!(status, DeliveryStatus::Delivered) { Some(Utc::now()) } else { None },
            attempt_count,
            error_message: error.map(|e| e.to_string()),
            metadata: None,
        }
    }

    fn store_delivery_history(&self, response: DeliveryResponse) {
        if let Ok(mut history) = self.delivery_history.lock() {
            history.insert(response.delivery_id, response);
        }
    }
}

impl Default for ApiContentDeliverer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentDeliveryService for ApiContentDeliverer {
    fn deliver_content(&self, request: DeliveryRequest) -> Result<DeliveryResponse, ContentDeliveryError> {
        let delivery_id = Uuid::new_v4();
        
        // For synchronous trait, we need to use a runtime
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ContentDeliveryError::ServiceUnavailable)?;

        let result = rt.block_on(async {
            self.delivery_with_retry(&request).await
        });

        let (status, attempt_count, error) = match result {
            Ok(attempts) => (DeliveryStatus::Delivered, attempts, None),
            Err(ref e) => (DeliveryStatus::Failed, self.max_retries, Some(e)),
        };

        let response = self.create_delivery_response(delivery_id, status, attempt_count, error);
        self.store_delivery_history(response.clone());

        match result {
            Ok(_) => Ok(response),
            Err(e) => Err(e),
        }
    }

    fn schedule_delivery(&self, request: DeliveryRequest) -> Result<DeliveryResponse, ContentDeliveryError> {
        let delivery_id = Uuid::new_v4();
        
        // For demo purposes, we'll just mark it as scheduled
        // In a real implementation, you'd integrate with a job scheduler
        let response = self.create_delivery_response(delivery_id, DeliveryStatus::Scheduled, 0, None);
        self.store_delivery_history(response.clone());
        
        // TODO: Integrate with actual scheduler (e.g., tokio-cron-scheduler)
        Ok(response)
    }

    fn cancel_delivery(&self, delivery_id: Uuid) -> Result<(), ContentDeliveryError> {
        if let Ok(mut history) = self.delivery_history.lock() {
            if let Some(mut delivery) = history.get(&delivery_id).cloned() {
                delivery.status = DeliveryStatus::Cancelled;
                history.insert(delivery_id, delivery);
                Ok(())
            } else {
                Err(ContentDeliveryError::RecipientNotFound(delivery_id.to_string()))
            }
        } else {
            Err(ContentDeliveryError::ServiceUnavailable)
        }
    }

    fn get_delivery_status(&self, delivery_id: Uuid) -> Result<DeliveryResponse, ContentDeliveryError> {
        if let Ok(history) = self.delivery_history.lock() {
            history.get(&delivery_id)
                .cloned()
                .ok_or_else(|| ContentDeliveryError::RecipientNotFound(delivery_id.to_string()))
        } else {
            Err(ContentDeliveryError::ServiceUnavailable)
        }
    }

    fn list_deliveries(&self, recipient_id: &str, limit: Option<u32>) -> Result<Vec<DeliveryResponse>, ContentDeliveryError> {
        if let Ok(history) = self.delivery_history.lock() {
            let mut deliveries: Vec<DeliveryResponse> = history.values()
                .cloned()
                .collect();
            
            // Sort by most recent first
            deliveries.sort_by(|a, b| {
                b.delivered_at.unwrap_or(Utc::now()).cmp(&a.delivered_at.unwrap_or(Utc::now()))
            });

            if let Some(limit) = limit {
                deliveries.truncate(limit as usize);
            }

            Ok(deliveries)
        } else {
            Err(ContentDeliveryError::ServiceUnavailable)
        }
    }

    fn get_delivery_stats(&self, _recipient_id: Option<&str>) -> Result<DeliveryStats, ContentDeliveryError> {
        if let Ok(history) = self.delivery_history.lock() {
            let total = history.len() as u64;
            let successful = history.values()
                .filter(|d| matches!(d.status, DeliveryStatus::Delivered))
                .count() as u64;
            let failed = history.values()
                .filter(|d| matches!(d.status, DeliveryStatus::Failed))
                .count() as u64;
            let pending = history.values()
                .filter(|d| matches!(d.status, DeliveryStatus::Pending | DeliveryStatus::InProgress | DeliveryStatus::Scheduled))
                .count() as u64;

            Ok(DeliveryStats {
                total_deliveries: total,
                successful_deliveries: successful,
                failed_deliveries: failed,
                pending_deliveries: pending,
                average_delivery_time_ms: None, // Would calculate from actual delivery times
            })
        } else {
            Err(ContentDeliveryError::ServiceUnavailable)
        }
    }

    fn validate_delivery_method(&self, method: &DeliveryMethod) -> Result<(), ContentDeliveryError> {
        match method {
            DeliveryMethod::Http { endpoint, .. } => {
                if endpoint.is_empty() {
                    Err(ContentDeliveryError::InvalidDeliveryMethod("HTTP endpoint cannot be empty".to_string()))
                } else if !endpoint.starts_with("http") {
                    Err(ContentDeliveryError::InvalidDeliveryMethod("HTTP endpoint must start with http:// or https://".to_string()))
                } else {
                    Ok(())
                }
            },
            DeliveryMethod::Webhook { url, method } => {
                if url.is_empty() {
                    Err(ContentDeliveryError::InvalidDeliveryMethod("Webhook URL cannot be empty".to_string()))
                } else if !["GET", "POST", "PUT", "PATCH", "DELETE"].contains(&method.to_uppercase().as_str()) {
                    Err(ContentDeliveryError::InvalidDeliveryMethod(format!("Unsupported HTTP method: {}", method)))
                } else {
                    Ok(())
                }
            },
            DeliveryMethod::Email { to, .. } => {
                if to.is_empty() || !to.contains('@') {
                    Err(ContentDeliveryError::InvalidDeliveryMethod("Invalid email address".to_string()))
                } else {
                    Ok(())
                }
            },
            _ => Ok(()), // Other methods would have their own validation
        }
    }
}

impl BatchContentDeliveryService for ApiContentDeliverer {
    fn batch_deliver(&self, requests: Vec<DeliveryRequest>) -> Result<Vec<DeliveryResponse>, ContentDeliveryError> {
        let mut responses = Vec::new();
        
        for request in requests {
            let response = self.deliver_content(request)?;
            responses.push(response);
        }
        
        Ok(responses)
    }

    fn get_batch_status(&self, _batch_id: Uuid) -> Result<Vec<DeliveryResponse>, ContentDeliveryError> {
        // In a real implementation, you'd track batch IDs
        Err(ContentDeliveryError::DeliveryFailed("Batch status tracking not implemented".to_string()))
    }
}

// Actix Web handlers for API endpoints
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/delivery")
            .route("/deliver", web::post().to(deliver_content_handler))
            .route("/status/{delivery_id}", web::get().to(get_delivery_status_handler))
            .route("/stats", web::get().to(get_delivery_stats_handler))
    );
}

async fn deliver_content_handler(
    payload: web::Json<DeliveryRequest>,
    deliverer: web::Data<ApiContentDeliverer>,
) -> ActixResult<HttpResponse> {
    match deliverer.deliver_content(payload.into_inner()) {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": e.to_string()
        }))),
    }
}

async fn get_delivery_status_handler(
    path: web::Path<String>,
    deliverer: web::Data<ApiContentDeliverer>,
) -> ActixResult<HttpResponse> {
    let delivery_id_str = path.into_inner();
    
    match Uuid::parse_str(&delivery_id_str) {
        Ok(delivery_id) => {
            match deliverer.get_delivery_status(delivery_id) {
                Ok(response) => Ok(HttpResponse::Ok().json(response)),
                Err(e) => Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "error": e.to_string()
                }))),
            }
        },
        Err(_) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid delivery ID format"
        }))),
    }
}

async fn get_delivery_stats_handler(
    deliverer: web::Data<ApiContentDeliverer>,
) -> ActixResult<HttpResponse> {
    match deliverer.get_delivery_stats(None) {
        Ok(stats) => Ok(HttpResponse::Ok().json(stats)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e.to_string()
        }))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::container::content::{ContentItem, ContentType, TextContent};
    use mockito::Server;

    #[tokio::test]
    async fn test_http_delivery_success() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/webhook")
            .with_status(200)
            .with_body(r#"{"success": true, "message": "Delivered"}"#)
            .create_async()
            .await;

        let deliverer = ApiContentDeliverer::new();
        let text_content = TextContent::new(None, Some("Test content".to_string())).unwrap();
        let content_item = ContentItem::new(ContentType::Text(text_content)).unwrap();

        let request = DeliveryRequest {
            recipient_id: "test_user".to_string(),
            delivery_method: DeliveryMethod::Http {
                endpoint: format!("{}/webhook", server.url()),
                headers: None,
            },
            content_payload: ContentPayload::SingleItem(content_item),
            priority: DeliveryPriority::Normal,
            scheduled_time: None,
            metadata: None,
        };

        let result = deliverer.deliver_content(request);
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, DeliveryStatus::Delivered);

        mock.assert_async().await;
    }

    #[test]
    fn test_validate_delivery_method_http() {
        let deliverer = ApiContentDeliverer::new();
        
        let valid_method = DeliveryMethod::Http {
            endpoint: "https://example.com/webhook".to_string(),
            headers: None,
        };
        assert!(deliverer.validate_delivery_method(&valid_method).is_ok());

        let invalid_method = DeliveryMethod::Http {
            endpoint: "".to_string(),
            headers: None,
        };
        assert!(deliverer.validate_delivery_method(&invalid_method).is_err());
    }

    #[test]
    fn test_delivery_stats() {
        let deliverer = ApiContentDeliverer::new();
        let stats = deliverer.get_delivery_stats(None).unwrap();
        
        assert_eq!(stats.total_deliveries, 0);
        assert_eq!(stats.successful_deliveries, 0);
        assert_eq!(stats.failed_deliveries, 0);
        assert_eq!(stats.pending_deliveries, 0);
    }
}