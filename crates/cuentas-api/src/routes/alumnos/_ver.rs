//! `GET /v1/alumnos/{id}`

use actix_web::{HttpResponse, get, web};
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::error::{AppError, AppResult, ErrorBody};
use crate::models::dto::StudentDto;
use crate::models::entities::alumno;
use crate::routes::comun::{id, student_dto};

/// One student.
#[utoipa::path(
	tag = "alumnos",
	operation_id = "alumnos_ver",
	params(("id" = String, Path, description = "Student id")),
	responses(
		(status = 200, description = "The student", body = StudentDto),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "No such student", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[get("/{id}")]
pub(super) async fn ver_alumno(
	conn: web::Data<DatabaseConnection>,
	path: web::Path<String>,
) -> AppResult<HttpResponse> {
	let row = alumno::Entity::find_by_id(id(&path, "student")?)
		.one(conn.get_ref())
		.await?
		.ok_or_else(|| AppError::not_found("student"))?;
	Ok(HttpResponse::Ok().json(student_dto(row)))
}
