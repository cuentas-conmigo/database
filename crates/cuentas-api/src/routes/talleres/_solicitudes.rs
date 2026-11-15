//! `GET /v1/talleres/{id}/solicitudes`

use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use crate::config::AppState;
use crate::error::{AppResult, ErrorBody};
use crate::models::dto::ReviewEntryDto;
use crate::routes::comun::id;
use crate::services::{Sesion, contactos};

/// People asking for a place, oldest first: `pending` or `waitlist`.
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_solicitudes",
	params(("id" = String, Path, description = "Workshop id")),
	responses(
		(status = 200, description = "People waiting for approval", body = [ReviewEntryDto]),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "Not a workshop of this tallerista", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[get("/{id}/solicitudes")]
pub(super) async fn solicitudes(
	conn: web::Data<DatabaseConnection>,
	state: web::Data<AppState>,
	sesion: web::ReqData<Sesion>,
	path: web::Path<String>,
) -> AppResult<HttpResponse> {
	let row = state
		.payments
		.workshop(&conn, sesion.id, id(&path, "workshop")?)
		.await?;
	let mut lista = Vec::new();
	for (alumno, estado) in contactos::solicitudes(&conn, row.id).await? {
		lista.push(ReviewEntryDto {
			student: contactos::perfil(&conn, alumno).await?,
			status: estado,
		});
	}
	Ok(HttpResponse::Ok().json(lista))
}
