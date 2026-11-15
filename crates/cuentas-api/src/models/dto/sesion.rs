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
