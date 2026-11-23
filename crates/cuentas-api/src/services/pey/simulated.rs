//! Pey without network, for demos and tests.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::{ChargeRefs, ChargeRequest, Paid, PeyClient, PeyError, PushCharge, QrCharge};

/// Clock used to decide when a simulated payment is paid; replaceable in tests.
pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// No network: QR strings start with `DEMO-` (the app labels them "no es CoDi") and a payment
/// counts as paid `settle_after` after it was created, like Pey's webhook would.
pub struct SimulatedPey {
	settle_after: Duration,
	clock: Clock,
}

impl SimulatedPey {
	pub fn new(settle_after: Duration, clock: Clock) -> Self {
		Self { settle_after, clock }
	}
}

#[async_trait]
impl PeyClient for SimulatedPey {
	fn is_live(&self) -> bool {
		false
	}

	async fn create_qr(&self, request: &ChargeRequest) -> Result<QrCharge, PeyError> {
		Ok(QrCharge {
			qr_id: format!("demo-{}", request.num_ref),
			qr_string: format!("DEMO-NO-ES-CODI|{}|{}", request.payment_id, request.amount),
		})
	}

	async fn create_push(&self, request: &ChargeRequest, _phone: &str) -> Result<PushCharge, PeyError> {
		Ok(PushCharge {
			transaction_id: format!("demo-pn-{}", request.num_ref),
		})
	}

	async fn paid(&self, refs: &ChargeRefs<'_>) -> Result<Option<Paid>, PeyError> {
		let Some(created_at) = refs.created_at else {
			return Ok(None);
		};
		let now = (self.clock)();
		let elapsed = now.signed_duration_since(created_at);
		let settle = chrono::Duration::from_std(self.settle_after).unwrap_or_default();
		Ok((elapsed >= settle).then_some(Paid {
			at: now,
			transaction_id: None,
		}))
	}

	fn webhook_secret(&self) -> Option<&str> {
		None
	}
}
