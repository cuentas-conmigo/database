//! `POST /v1/talleres/{id}/codigo`

use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use crate::config::AppState;
use crate::error::{AppResult, ErrorBody};
use crate::models::dto::CodeDto;
use crate::routes::comun::id;
use crate::services::{Sesion, contactos};

/// Replaces the enrollment code; the old one stops working at once.
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_codigo_cambiar",
	params(("id" = String, Path, description = "Workshop id")),
	responses(
		(status = 200, description = "The new code", body = CodeDto),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "Not a workshop of this tallerista", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[post("/{id}/codigo")]
pub(super) async fn cambiar_codigo(
	conn: web::Data<DatabaseConnection>,
	state: web::Data<AppState>,
	sesion: web::ReqData<Sesion>,
	path: web::Path<String>,
) -> AppResult<HttpResponse> {
	let row = state
		.payments
		.workshop(&conn, sesion.id, id(&path, "workshop")?)
		.await?;
	let code = contactos::cambiar_codigo(&conn, row.id, &row.nombre).await?;
	Ok(HttpResponse::Ok().json(CodeDto { code }))
}
