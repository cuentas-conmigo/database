//! `/v1/talleres/{id}`: el taller, sus alumnos, lo que se puede cobrar y las solicitudes.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::dto::alumnos::StudentDto;
use crate::services::cobros::ChargeOption;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopDto {
	pub id: String,
	pub name: String,
	pub instructor: String,
	pub schedule: String,
	pub monthly_fee: i32,
	pub capacity: u32,
}

#[derive(Serialize, ToSchema)]
pub struct RosterEntryDto {
	pub student: StudentDto,
	/// `paid` or `pending`, for the current month's fee.
	#[schema(value_type = String, example = "pending")]
	pub status: &'static str,
}

#[derive(Serialize, ToSchema)]
pub struct ReviewEntryDto {
	pub student: StudentDto,
	/// `pending` or `waitlist`.
	pub status: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ChargeOptionDto {
	pub id: String,
	pub label: String,
	pub detail: String,
	pub amount: i32,
}

impl From<ChargeOption> for ChargeOptionDto {
	fn from(option: ChargeOption) -> Self {
		Self {
			id: option.id,
			label: option.label,
			detail: option.detail,
			amount: option.amount,
		}
	}
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewStudentBody {
	pub name: String,
	/// WhatsApp, 10 digits.
	pub phone: String,
	/// Whoever helps them pay, 10 digits.
	#[serde(default)]
	pub helper_phone: Option<String>,
	/// Kept only when `photo_consent` is true.
	#[serde(default)]
	pub photo_uri: Option<String>,
	#[serde(default)]
	pub photo_consent: bool,
}

#[derive(Serialize, ToSchema)]
pub struct CodeDto {
	pub code: String,
}
