//! `DELETE /v1/alumnos/{id}/foto`

use actix_web::{HttpResponse, delete};

use crate::error::{AppError, AppResult, ErrorBody};

/// Deletes the photo and its consent record. Amanda's schema has no photos yet.
#[utoipa::path(
	tag = "alumnos",
	operation_id = "alumnos_quitar_foto",
	params(("id" = String, Path, description = "Student id")),
	responses(
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 501, description = "Not in the schema yet", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[delete("/{id}/foto")]
pub(super) async fn quitar_foto() -> AppResult<HttpResponse> {
	Err(AppError::not_implemented("photos"))
}
