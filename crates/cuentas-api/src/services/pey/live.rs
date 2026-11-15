//! Pey over HTTPS with the API key from Secrets Manager.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::json;

use super::{ChargeRefs, ChargeRequest, PeyClient, PeyError, PushCharge, QrCharge, short_concept};

/// API key and webhook secret, read once from Secrets Manager. Never logged.
#[derive(Clone)]
pub struct PeyCredentials {
	pub api_key: String,
	pub webhook_secret: Option<String>,
}

impl std::fmt::Debug for PeyCredentials {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PeyCredentials")
			.field("api_key", &"<redacted>")
			.field("webhook_secret", &self.webhook_secret.as_ref().map(|_| "<redacted>"))
			.finish()
	}
}

impl PeyCredentials {
	/// Accepts a JSON object (`apiKey`/`api_key`/`key`, `webhookSecret`/`webhook_secret`)
	/// or the bare API key as plain text.
	pub fn parse(secret: &str) -> Result<Self, PeyError> {
		let secret = secret.trim();
		if secret.is_empty() {
			return Err(PeyError("the Pey secret is empty".into()));
		}
		if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(secret) {
			let pick = |names: &[&str]| {
				names.iter().find_map(|name| {
					map
						.get(*name)
						.and_then(|value| value.as_str())
						.map(str::trim)
						.filter(|value| !value.is_empty())
						.map(str::to_string)
				})
			};
			let api_key =
				pick(&["apiKey", "api_key", "key"]).ok_or_else(|| PeyError("the Pey secret JSON has no apiKey".into()))?;
			return Ok(Self {
				api_key,
				webhook_secret: pick(&["webhookSecret", "webhook_secret"]),
			});
		}
		Ok(Self {
			api_key: secret.to_string(),
			webhook_secret: None,
		})
	}
}

/// Talks to `https://api.pey.mx` with a Bearer key.
pub struct LivePey {
	http: reqwest::Client,
	base_url: String,
	credentials: PeyCredentials,
	last_check: Mutex<HashMap<String, Instant>>,
}

/// Minimum time between status lookups for the same payment.
const CHECK_EVERY: Duration = Duration::from_secs(5);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QrResponse {
	id: String,
	qr_string: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushResponse {
	transaction_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transaction {
	status: String,
	date_paid: Option<i64>,
}

#[derive(Deserialize)]
struct TransactionPage {
	items: Vec<Transaction>,
}

impl LivePey {
	pub fn new(base_url: String, credentials: PeyCredentials) -> Result<Self, PeyError> {
		let http = reqwest::Client::builder()
			.timeout(Duration::from_secs(15))
			.build()
			.map_err(|error| PeyError(format!("http client: {error}")))?;
		Ok(Self {
			http,
			base_url: base_url.trim_end_matches('/').to_string(),
			credentials,
			last_check: Mutex::new(HashMap::new()),
		})
	}

	async fn post<T: for<'de> Deserialize<'de>>(&self, path: &str, body: serde_json::Value) -> Result<T, PeyError> {
		let response = self
			.http
			.post(format!("{}{path}", self.base_url))
			.bearer_auth(&self.credentials.api_key)
			.json(&body)
			.send()
			.await
			.map_err(|error| PeyError(format!("POST {path}: {error}")))?;
		Self::read(path, response).await
	}

	async fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, PeyError> {
		let response = self
			.http
			.get(format!("{}{path}", self.base_url))
			.bearer_auth(&self.credentials.api_key)
			.send()
			.await
			.map_err(|error| PeyError(format!("GET {path}: {error}")))?;
		Self::read(path, response).await
	}

	async fn read<T: for<'de> Deserialize<'de>>(path: &str, response: reqwest::Response) -> Result<T, PeyError> {
		let status = response.status();
		let body = response
			.text()
			.await
			.map_err(|error| PeyError(format!("{path}: {error}")))?;
		if !status.is_success() {
			let detail: String = body.chars().take(300).collect();
			return Err(PeyError(format!("{path} answered {status}: {detail}")));
		}
		serde_json::from_str(&body).map_err(|error| PeyError(format!("{path}: {error}")))
	}

	fn should_check(&self, payment_id: &str) -> bool {
		let mut checks = self.last_check.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
		let now = Instant::now();
		match checks.get(payment_id) {
			Some(last) if now.duration_since(*last) < CHECK_EVERY => false,
			_ => {
				checks.insert(payment_id.to_string(), now);
				true
			}
		}
	}
}

fn charge_body(request: &ChargeRequest) -> serde_json::Value {
	json!({
			"amount": request.amount,
			"concept": short_concept(&request.concept),
			"numRef": request.num_ref,
			"metadata": { "pagoId": request.payment_id },
	})
}

fn paid_time(transaction: &Transaction) -> Option<DateTime<Utc>> {
	(transaction.status == "PAID").then(|| {
		transaction
			.date_paid
			.and_then(|seconds| Utc.timestamp_opt(seconds, 0).single())
			.unwrap_or_else(Utc::now)
	})
}

#[async_trait]
impl PeyClient for LivePey {
	fn is_live(&self) -> bool {
		true
	}

	async fn create_qr(&self, request: &ChargeRequest) -> Result<QrCharge, PeyError> {
		let response: QrResponse = self.post("/codi/qr", charge_body(request)).await?;
		Ok(QrCharge {
			qr_id: response.id,
			qr_string: response.qr_string,
		})
	}

	async fn create_push(&self, request: &ChargeRequest, phone: &str) -> Result<PushCharge, PeyError> {
		let mut body = charge_body(request);
		body["phoneNumber"] = json!(phone);
		let response: PushResponse = self.post("/codi/pn", body).await?;
		Ok(PushCharge {
			transaction_id: response.transaction_id,
		})
	}

	async fn paid_at(&self, refs: &ChargeRefs<'_>) -> Result<Option<DateTime<Utc>>, PeyError> {
		if !self.should_check(refs.payment_id) {
			return Ok(None);
		}
		if let Some(qr_id) = refs.qr_id {
			let page: TransactionPage = self.get(&format!("/transaction/codi/qr/{qr_id}")).await?;
			return Ok(page.items.iter().find_map(paid_time));
		}
		if let Some(transaction_id) = refs.transaction_id {
			let transaction: Transaction = self.get(&format!("/transaction/{transaction_id}")).await?;
			return Ok(paid_time(&transaction));
		}
		Ok(None)
	}

	fn webhook_secret(&self) -> Option<&str> {
		self.credentials.webhook_secret.as_deref()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reads_json_or_plain_secrets_and_never_prints_them() {
		let json = PeyCredentials::parse(r#"{"apiKey":"k-123","webhookSecret":"whsec_abc"}"#).expect("json secret");
		assert_eq!(json.api_key, "k-123");
		assert_eq!(json.webhook_secret.as_deref(), Some("whsec_abc"));
		let printed = format!("{json:?}");
		assert!(!printed.contains("k-123") && !printed.contains("whsec_abc"));

		let plain = PeyCredentials::parse("  k-456 ").expect("plain secret");
		assert_eq!(plain.api_key, "k-456");
		assert!(plain.webhook_secret.is_none());

		assert!(PeyCredentials::parse(" ").is_err());
		assert!(PeyCredentials::parse(r#"{"webhookSecret":"x"}"#).is_err());
	}
}
