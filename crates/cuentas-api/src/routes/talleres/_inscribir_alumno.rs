//! `POST /v1/talleres/{id}/alumnos`

use actix_web::{HttpResponse, post, web};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::config::AppState;
use crate::error::{AppError, AppResult, ErrorBody};
use crate::models::dto::{NewStudentBody, StudentDto};
use crate::models::entities::alumno;
use crate::routes::comun::{id, student_dto};
use crate::services::{self as db, Sesion};

/// Enrolls a new student, with the same rule as Amanda's `/cuenta/alumno` (`guardar_alumno_cuenta`).
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_inscribir_alumno",
	params(("id" = String, Path, description = "Workshop id")),
	request_body = NewStudentBody,
	responses(
		(status = 201, description = "The new student", body = StudentDto),
		(status = 400, description = "Amanda's validation message", body = ErrorBody),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "Not a workshop of this tallerista", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[post("/{id}/alumnos")]
pub(super) async fn inscribir_alumno(
	conn: web::Data<DatabaseConnection>,
	state: web::Data<AppState>,
	sesion: web::ReqData<Sesion>,
	path: web::Path<String>,
	body: web::Json<NewStudentBody>,
) -> AppResult<HttpResponse> {
	let row = state
		.payments
		.workshop(&conn, sesion.id, id(&path, "workshop")?)
		.await?;
	let name = body.name.trim();
	db::guardar_alumno_cuenta(&conn, sesion.id, name, Some(&row.nombre))
		.await
		.map_err(|error| match error.to_string().split_once("aviso: ") {
			Some((_, aviso)) => AppError::bad_request(aviso.to_string()),
			None => AppError::internal(error),
		})?;
	let created = alumno::Entity::find()
		.filter(alumno::Column::Nombre.eq(name))
		.order_by_desc(alumno::Column::Id)
		.one(conn.get_ref())
		.await?
		.ok_or_else(|| AppError::not_found("student"))?;
	Ok(HttpResponse::Created().json(student_dto(created)))
}
