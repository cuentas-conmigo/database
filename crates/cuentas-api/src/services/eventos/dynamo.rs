use std::collections::HashMap;

use async_trait::async_trait;
use aws_sdk_dynamodb::Client;
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::types::AttributeValue;

use super::{ChargeRecord, EventStore, EventsError, charge_key, webhook_key};

/// One DynamoDB table with string hash key `eventId`.
pub struct DynamoEvents {
	client: Client,
	table: String,
}

impl DynamoEvents {
	pub fn new(client: Client, table: String) -> Self {
		Self { client, table }
	}
}

fn text(value: &str) -> AttributeValue {
	AttributeValue::S(value.to_string())
}

fn read(item: &HashMap<String, AttributeValue>, name: &str) -> Option<String> {
	item.get(name).and_then(|value| value.as_s().ok()).cloned()
}

fn failed(action: &str, error: impl std::fmt::Display) -> EventsError {
	EventsError(format!("dynamodb {action}: {error}"))
}

#[async_trait]
impl EventStore for DynamoEvents {
	async fn save_charge(&self, charge: &ChargeRecord) -> Result<(), EventsError> {
		let mut request = self
			.client
			.put_item()
			.table_name(&self.table)
			.item("eventId", text(&charge_key(charge.pago_id)))
			.item("pagoId", AttributeValue::N(charge.pago_id.to_string()))
			.item("workshopId", AttributeValue::N(charge.workshop_id.to_string()))
			.item("optionId", text(&charge.option_id))
			.item("method", text(&charge.method))
			.item("createdAt", text(&charge.created_at));
		if let Some(qr_string) = &charge.qr_string {
			request = request.item("qrString", text(qr_string));
		}
		request.send().await.map_err(|error| failed("put charge", error))?;
		Ok(())
	}

	async fn charge(&self, pago_id: i32) -> Result<Option<ChargeRecord>, EventsError> {
		let output = self
			.client
			.get_item()
			.table_name(&self.table)
			.key("eventId", text(&charge_key(pago_id)))
			.consistent_read(true)
			.send()
			.await
			.map_err(|error| failed("get charge", error))?;
		let Some(item) = output.item else {
			return Ok(None);
		};
		let number = |name: &str| {
			item
				.get(name)
				.and_then(|value| value.as_n().ok())
				.and_then(|value| value.parse::<i32>().ok())
		};
		Ok(Some(ChargeRecord {
			pago_id,
			workshop_id: number("workshopId").unwrap_or_default(),
			option_id: read(&item, "optionId").unwrap_or_default(),
			method: read(&item, "method").unwrap_or_default(),
			created_at: read(&item, "createdAt").unwrap_or_default(),
			qr_string: read(&item, "qrString"),
		}))
	}

	async fn first_delivery(&self, webhook_id: &str, kind: &str) -> Result<bool, EventsError> {
		let result = self
			.client
			.put_item()
			.table_name(&self.table)
			.item("eventId", text(&webhook_key(webhook_id)))
			.item("kind", text(kind))
			.item("receivedAt", text(&chrono::Utc::now().to_rfc3339()))
			.condition_expression("attribute_not_exists(eventId)")
			.send()
			.await;
		match result {
			Ok(_) => Ok(true),
			Err(SdkError::ServiceError(error)) if error.err().is_conditional_check_failed_exception() => Ok(false),
			Err(error) => Err(failed("put webhook", error)),
		}
	}
}
