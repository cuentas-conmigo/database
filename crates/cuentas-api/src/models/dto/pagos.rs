//! `/v1/pagos`: cobros por CoDi o en efectivo.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::services::cobros::Payment;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewPaymentBody {
	pub workshop_id: String,
	pub student_id: String,
	/// One of the workshop's charge options, or `custom` with `amount`.
	pub option_id: String,
	/// `qr`, `helper` or `cash`.
	pub method: String,
	#[serde(default)]
	pub amount: f64,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDto {
	pub id: String,
	pub workshop_id: String,
	pub student_id: String,
	pub option_id: String,
	pub method: String,
	pub amount: i32,
	pub concept: String,
	/// `paid` or `pending`.
	#[schema(value_type = String, example = "pending")]
	pub status: &'static str,
	pub folio: String,
	pub created_at: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub paid_at: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub qr_string: Option<String>,
}

impl From<Payment> for PaymentDto {
	fn from(payment: Payment) -> Self {
		let status = if payment.is_paid() { "paid" } else { "pending" };
		let folio = payment.folio();
		let paid_at = payment.paid_at();
		let charge = payment.charge;
		let row = payment.row;
		let method = charge.as_ref().map(|item| item.method.clone()).unwrap_or_else(|| {
			if row.canal == "panel" {
				"cash".into()
			} else {
				"qr".into()
			}
		});
		Self {
			id: row.id.to_string(),
			workshop_id: row
				.taller_id
				.or(charge.as_ref().map(|item| item.workshop_id))
				.map(|id| id.to_string())
				.unwrap_or_default(),
			student_id: row.alumno_id.to_string(),
			option_id: charge.as_ref().map(|item| item.option_id.clone()).unwrap_or_default(),
			method,
			amount: row.monto_mxn,
			concept: row.concepto,
			status,
			folio,
			created_at: charge
				.as_ref()
				.map(|item| item.created_at.clone())
				.or_else(|| paid_at.clone())
				.unwrap_or_default(),
			paid_at,
			qr_string: charge.and_then(|item| item.qr_string),
		}
	}
}
