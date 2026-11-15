//! `POST /v1/talleres/{id}/alumnos/{alumno}/baja`

use actix_web::{HttpResponse, post, web};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::config::AppState;
use crate::error::{AppError, AppResult, ErrorBody};
use crate::models::entities::inscripcion;
use crate::routes::comun::id;
use crate::services::{Sesion, contactos};

/// Removes a request, or frees an enrolled student's place (`inscripciones_taller.activa = 0`); payments stay.
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_baja",
	params(
		("id" = String, Path, description = "Workshop id"),
		("alumno" = String, Path, description = "Student id"),
	),
	responses(
		(status = 204, description = "Released"),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "No request and no active enrollment", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[post("/{id}/alumnos/{alumno}/baja")]
pub(super) async fn dar_de_baja(
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
	let student_id = id(&student_id, "student")?;
	if contactos::quitar_solicitud(&conn, row.id, student_id).await? {
		return Ok(HttpResponse::NoContent().finish());
	}
	let enrolled = inscripcion::Entity::find()
		.filter(inscripcion::Column::TallerId.eq(row.id))
		.filter(inscripcion::Column::AlumnoId.eq(student_id))
		.filter(inscripcion::Column::Activa.eq(1))
		.all(conn.get_ref())
		.await?;
	if enrolled.is_empty() {
		return Err(AppError::not_found("enrollment"));
	}
	for item in enrolled {
		let mut cambio: inscripcion::ActiveModel = item.into();
		cambio.activa = Set(0);
		cambio.update(conn.get_ref()).await?;
	}
	Ok(HttpResponse::NoContent().finish())
}
