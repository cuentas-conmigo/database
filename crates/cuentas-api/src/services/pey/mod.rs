//! Pey CoDi client: a simulated one for demos and a live one for real charges.
//! API reference: <https://docs.pey.mx>.

mod live;
mod simulated;
pub mod webhook;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

pub use live::{LivePey, PeyCredentials};
pub use simulated::SimulatedPey;

/// What the backend asks Pey for one charge.
#[derive(Clone, Debug)]
pub struct ChargeRequest {
	pub payment_id: String,
	/// Amount in MXN.
	pub amount: f64,
	pub concept: String,
	/// Up to 7 digits; we use the folio number.
	pub num_ref: String,
	pub created_at: DateTime<Utc>,
}

/// A CoDi QR created by Pey.
#[derive(Clone, Debug)]
pub struct QrCharge {
	/// Banxico IDC.
	pub qr_id: String,
	/// Payload to draw as a QR.
	pub qr_string: String,
}

/// A CoDi push sent to a phone.
#[derive(Clone, Debug)]
pub struct PushCharge {
	pub transaction_id: String,
}

/// References to look a charge up again at Pey.
#[derive(Clone, Debug, Default)]
pub struct ChargeRefs<'a> {
	pub payment_id: &'a str,
	pub qr_id: Option<&'a str>,
	pub transaction_id: Option<&'a str>,
	pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct PeyError(pub String);

impl std::fmt::Display for PeyError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}

impl std::error::Error for PeyError {}

#[async_trait]
pub trait PeyClient: Send + Sync {
	/// Whether this client moves real money.
	fn is_live(&self) -> bool;
	async fn create_qr(&self, request: &ChargeRequest) -> Result<QrCharge, PeyError>;
	async fn create_push(&self, request: &ChargeRequest, phone: &str) -> Result<PushCharge, PeyError>;
	/// Returns when it was paid if Pey (or the simulation) says so; `None` while pending.
	async fn paid_at(&self, refs: &ChargeRefs<'_>) -> Result<Option<DateTime<Utc>>, PeyError>;
	/// Secret to verify webhooks; `None` means webhooks cannot be trusted.
	fn webhook_secret(&self) -> Option<&str>;
}

/// Keeps a CoDi concept short; Pey does not document a maximum.
pub fn short_concept(concept: &str) -> String {
	concept.chars().take(40).collect()
}
