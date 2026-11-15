//! `POST /v1/talleres/{id}/alumnos/{alumno}/aprobar`

use actix_web::{HttpResponse, post};

use crate::error::{AppError, AppResult, ErrorBody};

/// Gives a waiting student a place. Amanda's schema has no pending enrollments yet.
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_aprobar",
	params(
		("id" = String, Path, description = "Workshop id"),
		("alumno" = String, Path, description = "Student id"),
	),
	responses(
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 501, description = "Not in the schema yet", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[post("/{id}/alumnos/{alumno}/aprobar")]
pub(super) async fn aprobar_alumno() -> AppResult<HttpResponse> {
	Err(AppError::not_implemented("pending enrollments"))
}
