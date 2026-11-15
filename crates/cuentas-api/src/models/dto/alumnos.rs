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

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StudentDto {
	pub id: String,
	pub name: String,
	/// Amanda's `alumnos` has no phone yet.
	pub phone: String,
	pub helpers: Vec<HelperDto>,
}
