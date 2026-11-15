//! `POST /v1/talleres/{id}/alumnos/{alumno}/aprobar`

use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use crate::config::AppState;
use crate::error::{AppError, AppResult, ErrorBody};
use crate::routes::comun::id;
use crate::services::{Sesion, contactos};

/// Gives a waiting person a place: their request becomes an active enrollment.
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_aprobar",
	params(
		("id" = String, Path, description = "Workshop id"),
		("alumno" = String, Path, description = "Student id"),
	),
	responses(
		(status = 204, description = "Enrolled"),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "No request from this student", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[post("/{id}/alumnos/{alumno}/aprobar")]
pub(super) async fn aprobar_alumno(
	conn: web::Data<DatabaseConnection>,
	state: web::Data<AppState>,
	sesion: web::ReqData<Sesion>,
	path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
	let (workshop_id, student_id) = path.into_inner();
	let row = state
		.payments
		.workshop(&conn, sesion.id, id(&workshop_id, "workshop")?)
		.await?;
	if !contactos::aprobar(&conn, row.id, id(&student_id, "student")?).await? {
		return Err(AppError::not_found("request"));
	}
	Ok(HttpResponse::NoContent().finish())
}
