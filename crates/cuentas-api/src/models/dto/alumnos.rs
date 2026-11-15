//! `/v1/alumnos/{id}`: un alumno como lo ve la app.

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HelperDto {
	pub name: String,
	pub relation: String,
	pub phone: String,
}

/// A photo is only sent together with who allowed it and when.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PhotoDto {
	pub uri: String,
	pub granted_by: String,
	pub granted_at: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StudentDto {
	pub id: String,
	pub name: String,
	/// WhatsApp, 10 digits; empty when it was never registered.
	pub phone: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub photo: Option<PhotoDto>,
	pub helpers: Vec<HelperDto>,
}
