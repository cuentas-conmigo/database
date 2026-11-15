//! `POST /v1/talleres/{id}/codigo`

use actix_web::{HttpResponse, post};

use crate::error::{AppError, AppResult, ErrorBody};

/// Replaces the workshop's enrollment code. Amanda's schema has no enrollment codes yet.
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_codigo_cambiar",
	params(("id" = String, Path, description = "Workshop id")),
	responses(
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 501, description = "Not in the schema yet", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[post("/{id}/codigo")]
pub(super) async fn cambiar_codigo() -> AppResult<HttpResponse> {
	Err(AppError::not_implemented("enrollment codes"))
}
