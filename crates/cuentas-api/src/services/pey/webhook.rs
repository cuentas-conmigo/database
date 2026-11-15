//! Standard Webhooks verification (https://www.standardwebhooks.com), as Pey uses.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

/// Accepted distance between `webhook-timestamp` and now; Pey does not document one.
pub const TOLERANCE_SECONDS: i64 = 300;

#[derive(Debug, PartialEq, Eq)]
pub enum VerifyError {
	MissingHeaders,
	BadSecret,
	StaleTimestamp,
	BadSignature,
}

/// Checks `webhook-signature` against `"{id}.{timestamp}.{raw body}"` signed with HMAC-SHA256.
pub fn verify(
	secret: &str,
	id: Option<&str>,
	timestamp: Option<&str>,
	signature: Option<&str>,
	body: &[u8],
	now_unix: i64,
) -> Result<(), VerifyError> {
	let (Some(id), Some(timestamp), Some(signature)) = (id, timestamp, signature) else {
		return Err(VerifyError::MissingHeaders);
	};
	let sent: i64 = timestamp.trim().parse().map_err(|_| VerifyError::MissingHeaders)?;
	if (now_unix - sent).abs() > TOLERANCE_SECONDS {
		return Err(VerifyError::StaleTimestamp);
	}
	let key = STANDARD
		.decode(secret.trim().trim_start_matches("whsec_"))
		.map_err(|_| VerifyError::BadSecret)?;
	let mut signed = Vec::with_capacity(id.len() + timestamp.len() + body.len() + 2);
	signed.extend_from_slice(id.as_bytes());
	signed.push(b'.');
	signed.extend_from_slice(timestamp.trim().as_bytes());
	signed.push(b'.');
	signed.extend_from_slice(body);

	for entry in signature.split_whitespace() {
		let Some(("v1", encoded)) = entry.split_once(',') else {
			continue;
		};
		let Ok(expected) = STANDARD.decode(encoded) else {
			continue;
		};
		let mut mac = Hmac::<Sha256>::new_from_slice(&key).map_err(|_| VerifyError::BadSecret)?;
		mac.update(&signed);
		if mac.verify_slice(&expected).is_ok() {
			return Ok(());
		}
	}
	Err(VerifyError::BadSignature)
}

/// The fields of `codi.qr.payment_received` and `codi.pn.payment_received` we use.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentEvent {
	#[serde(rename = "type")]
	pub kind: String,
	pub amount: Option<f64>,
	pub codi_qr_id: Option<String>,
	pub transaction_id: Option<String>,
	pub date_paid: Option<i64>,
	pub metadata: Option<EventMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventMetadata {
	pub external_metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl PaymentEvent {
	pub fn is_payment(&self) -> bool {
		matches!(
			self.kind.as_str(),
			"codi.qr.payment_received" | "codi.pn.payment_received"
		)
	}

	/// The `pagoId` we attached when creating the charge, if Pey sends it back.
	pub fn payment_id(&self) -> Option<String> {
		self
			.metadata
			.as_ref()?
			.external_metadata
			.as_ref()?
			.get("pagoId")
			.and_then(|value| value.as_str())
			.map(str::to_string)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const SECRET: &str = "whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw";

	fn sign(id: &str, timestamp: &str, body: &[u8]) -> String {
		let key = STANDARD.decode(SECRET.trim_start_matches("whsec_")).unwrap();
		let mut mac = Hmac::<Sha256>::new_from_slice(&key).unwrap();
		mac.update(format!("{id}.{timestamp}.").as_bytes());
		mac.update(body);
		format!("v1,{}", STANDARD.encode(mac.finalize().into_bytes()))
	}

	#[test]
	fn accepts_a_valid_signature_among_several() {
		let body = br#"{"type":"codi.qr.payment_received","amount":1}"#;
		let signature = format!("v1,bm90LWl0 {}", sign("msg_1", "1790724818", body));
		assert_eq!(
			verify(
				SECRET,
				Some("msg_1"),
				Some("1790724818"),
				Some(&signature),
				body,
				1790724900
			),
			Ok(())
		);
	}

	#[test]
	fn rejects_tampering_old_timestamps_and_missing_headers() {
		let body = br#"{"type":"codi.qr.payment_received","amount":1}"#;
		let signature = sign("msg_1", "1790724818", body);
		let tampered = br#"{"type":"codi.qr.payment_received","amount":999}"#;
		assert_eq!(
			verify(
				SECRET,
				Some("msg_1"),
				Some("1790724818"),
				Some(&signature),
				tampered,
				1790724818
			),
			Err(VerifyError::BadSignature)
		);
		assert_eq!(
			verify(
				SECRET,
				Some("msg_1"),
				Some("1790724818"),
				Some(&signature),
				body,
				1790724818 + 301
			),
			Err(VerifyError::StaleTimestamp)
		);
		assert_eq!(
			verify(SECRET, None, Some("1790724818"), Some(&signature), body, 1790724818),
			Err(VerifyError::MissingHeaders)
		);
	}

	#[test]
	fn reads_pey_qr_events_and_our_payment_id() {
		let event: PaymentEvent = serde_json::from_str(
			r#"{"amount":150,"apiKeyId":"01M3","codiQrId":"35a13c183a","concept":"x","datePaid":1790724818,
            "metadata":{"clabe":"000000000000000000","externalMetadata":{"pagoId":"142"}},
            "numRef":"34567","transactionId":"01M3QR","type":"codi.qr.payment_received"}"#,
		)
		.unwrap();
		assert!(event.is_payment());
		assert_eq!(event.payment_id().as_deref(), Some("142"));
		assert_eq!(event.codi_qr_id.as_deref(), Some("35a13c183a"));
	}
}
