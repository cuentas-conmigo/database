//! `POST /v1/alumnos/{id}/unirse`

use actix_web::{HttpResponse, post};

use crate::error::{AppError, AppResult, ErrorBody};

/// Asks for a place with an enrollment code. Amanda's schema has no enrollment codes yet.
#[utoipa::path(
	tag = "alumnos",
	operation_id = "alumnos_unirse",
	params(("id" = String, Path, description = "Student id")),
	responses(
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 501, description = "Not in the schema yet", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[post("/{id}/unirse")]
pub(super) async fn unirse() -> AppResult<HttpResponse> {
	Err(AppError::not_implemented("enrollment codes"))
}
