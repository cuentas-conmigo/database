//! What Amanda's schema has no column for: the QR to draw, how a payment was started, and
//! which Pey webhooks were already handled. Stored in DynamoDB in production, in memory in tests.

mod dynamo;
mod memoria;

use async_trait::async_trait;

pub use dynamo::DynamoEvents;
pub use memoria::MemoryEvents;

/// Extra data of one Pey charge, keyed by the `pagos.id` it belongs to.
#[derive(Clone, Debug, PartialEq)]
pub struct ChargeRecord {
	pub pago_id: i32,
	pub workshop_id: i32,
	pub option_id: String,
	/// `qr`, `helper` or `cash`.
	pub method: String,
	/// RFC 3339, UTC.
	pub created_at: String,
	pub qr_string: Option<String>,
}

#[derive(Debug)]
pub struct EventsError(pub String);

impl std::fmt::Display for EventsError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}

impl std::error::Error for EventsError {}

#[async_trait]
pub trait EventStore: Send + Sync {
	async fn save_charge(&self, charge: &ChargeRecord) -> Result<(), EventsError>;
	async fn charge(&self, pago_id: i32) -> Result<Option<ChargeRecord>, EventsError>;
	/// Records a webhook delivery; `false` when that `webhook-id` was already handled.
	async fn first_delivery(&self, webhook_id: &str, kind: &str) -> Result<bool, EventsError>;
}

pub(crate) fn charge_key(pago_id: i32) -> String {
	format!("charge#{pago_id}")
}

pub(crate) fn webhook_key(webhook_id: &str) -> String {
	format!("webhook#{webhook_id}")
}
