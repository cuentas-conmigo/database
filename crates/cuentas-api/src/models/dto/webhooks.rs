//! `POST /v1/webhooks/pey`.

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct WebhookAckDto {
	pub ok: bool,
	/// Whether this delivery matched a `pagos` row for the first time.
	pub handled: bool,
}
