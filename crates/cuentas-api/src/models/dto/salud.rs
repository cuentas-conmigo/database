//! `GET /v1/salud`.

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthDto {
	pub ok: bool,
	/// `live` or `simulate`.
	#[schema(value_type = String, example = "simulate")]
	pub pey_mode: &'static str,
}
