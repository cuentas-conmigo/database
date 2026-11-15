use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use async_trait::async_trait;

use super::{ChargeRecord, EventStore, EventsError, webhook_key};

/// For tests and local runs without DynamoDB; forgets everything on restart.
#[derive(Default)]
pub struct MemoryEvents {
	charges: Mutex<HashMap<i32, ChargeRecord>>,
	webhooks: Mutex<HashSet<String>>,
}

#[async_trait]
impl EventStore for MemoryEvents {
	async fn save_charge(&self, charge: &ChargeRecord) -> Result<(), EventsError> {
		let mut charges = self.charges.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		charges.insert(charge.pago_id, charge.clone());
		Ok(())
	}

	async fn charge(&self, pago_id: i32) -> Result<Option<ChargeRecord>, EventsError> {
		let charges = self.charges.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		Ok(charges.get(&pago_id).cloned())
	}

	async fn first_delivery(&self, webhook_id: &str, _kind: &str) -> Result<bool, EventsError> {
		let mut webhooks = self.webhooks.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		Ok(webhooks.insert(webhook_key(webhook_id)))
	}
}
