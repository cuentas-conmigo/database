//! `POST /v1/webhooks/pey`
//!
//! Pey tells us a CoDi payment arrived. Signed with Standard Webhooks; each `webhook-id` is handled once.

use actix_web::{HttpRequest, HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use crate::config::AppState;
use crate::error::{AppError, AppResult, ErrorBody};
use crate::models::dto::WebhookAckDto;
use crate::services::pey::webhook::{PaymentEvent, verify};

fn header<'a>(req: &'a HttpRequest, name: &str) -> Option<&'a str> {
	req.headers().get(name).and_then(|value| value.to_str().ok())
}

/// A CoDi payment arrived; settles the matching `pagos` row once.
#[utoipa::path(
	tag = "webhooks",
	operation_id = "webhooks_pey",
	request_body(content = Object, description = "Pey event, signed with Standard Webhooks", content_type = "application/json"),
	responses(
		(status = 200, description = "Accepted; `handled` says whether it settled a payment", body = WebhookAckDto),
		(status = 400, description = "Not a Pey event", body = ErrorBody),
		(status = 401, description = "Bad signature", body = ErrorBody),
		(status = 503, description = "No webhook secret configured", body = ErrorBody),
	),
)]
#[post("/pey")]
pub(super) async fn webhook_pey(
	req: HttpRequest,
	conn: web::Data<DatabaseConnection>,
	state: web::Data<AppState>,
	body: web::Bytes,
) -> AppResult<HttpResponse> {
	let Some(secret) = state.webhook_secret.as_deref() else {
		return Err(AppError::new(
			actix_web::http::StatusCode::SERVICE_UNAVAILABLE,
			"webhook-not-configured",
			"no webhook secret configured",
		));
	};
	let webhook_id = header(&req, "webhook-id");
	verify(
		secret,
		webhook_id,
		header(&req, "webhook-timestamp"),
		header(&req, "webhook-signature"),
		&body,
		chrono::Utc::now().timestamp(),
	)
	.map_err(|error| {
		tracing::warn!(?error, "rejected a Pey webhook");
		AppError::unauthorized()
	})?;
	let event: PaymentEvent = serde_json::from_slice(&body).map_err(|_| AppError::bad_request("not a Pey event"))?;
	// Settling is idempotent, so a retry after a failure here is safe; the delivery is
	// recorded afterwards for the audit trail and to skip exact repeats.
	let matched = event.is_payment() && state.payments.settle_from_webhook(&conn, &event).await?;
	if event.is_payment() && !matched {
		tracing::warn!(kind = %event.kind, "Pey webhook matched no pago");
	}
	let first = state
		.payments
		.events
		.first_delivery(webhook_id.unwrap_or_default(), &event.kind)
		.await
		.map_err(AppError::internal)?;
	if !first {
		return Ok(HttpResponse::Ok().json(WebhookAckDto {
			ok: true,
			handled: false,
		}));
	}
	Ok(HttpResponse::Ok().json(WebhookAckDto {
		ok: true,
		handled: matched,
	}))
}
