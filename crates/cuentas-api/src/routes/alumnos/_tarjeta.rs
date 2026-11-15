//! `GET /v1/alumnos/{id}/tarjeta`

use actix_web::{HttpResponse, get};

use crate::error::{AppError, AppResult, ErrorBody};

/// The student's single card across workshops. Amanda's schema has no student card yet.
#[utoipa::path(
	tag = "alumnos",
	operation_id = "alumnos_tarjeta",
	params(("id" = String, Path, description = "Student id")),
	responses(
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 501, description = "Not in the schema yet", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[get("/{id}/tarjeta")]
pub(super) async fn tarjeta() -> AppResult<HttpResponse> {
	Err(AppError::not_implemented("the student card"))
}
