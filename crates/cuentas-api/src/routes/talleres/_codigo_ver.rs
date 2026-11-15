//! `GET /v1/talleres/{id}/codigo`

use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use crate::config::AppState;
use crate::error::{AppResult, ErrorBody};
use crate::models::dto::CodeDto;
use crate::routes::comun::id;
use crate::services::{Sesion, contactos};

/// The code people type to ask for a place in the workshop.
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_codigo_ver",
	params(("id" = String, Path, description = "Workshop id")),
	responses(
		(status = 200, description = "The current code", body = CodeDto),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "Not a workshop of this tallerista", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[get("/{id}/codigo")]
pub(super) async fn ver_codigo(
	conn: web::Data<DatabaseConnection>,
	state: web::Data<AppState>,
	sesion: web::ReqData<Sesion>,
	path: web::Path<String>,
) -> AppResult<HttpResponse> {
	let row = state
		.payments
		.workshop(&conn, sesion.id, id(&path, "workshop")?)
		.await?;
	let code = contactos::codigo(&conn, row.id, &row.nombre).await?;
	Ok(HttpResponse::Ok().json(CodeDto { code }))
}
