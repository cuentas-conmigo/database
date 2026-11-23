//! `POST /v1/sesion`.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct SessionBody {
	pub usuario: Option<String>,
	pub clave: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
	pub name: String,
	/// Always `instructor` for now.
	#[schema(value_type = String, example = "instructor")]
	pub role: &'static str,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub workshop_id: Option<String>,
}

/// The app's sign-up form: the login, who teaches, and their first workshop.
#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewAccountBody {
	/// Letters and numbers, 3 to 30, no spaces.
	pub usuario: String,
	/// 6 to 100 characters.
	pub clave: String,
	pub name: String,
	/// What they teach, e.g. `tejido`.
	pub trade: String,
	/// Name of their first workshop.
	pub workshop: String,
	/// Monthly fee in MXN.
	pub monthly_fee: i32,
}
