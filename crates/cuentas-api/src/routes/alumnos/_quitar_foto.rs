//! `DELETE /v1/alumnos/{id}/foto`

use actix_web::{HttpResponse, delete, web};
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::error::{AppError, AppResult, ErrorBody};
use crate::models::dto::StudentDto;
use crate::models::entities::alumno;
use crate::routes::comun::id;
use crate::services::contactos;

/// Deletes the photo together with its consent record.
#[utoipa::path(
	tag = "alumnos",
	operation_id = "alumnos_quitar_foto",
	params(("id" = String, Path, description = "Student id")),
	responses(
		(status = 200, description = "The student, without photo", body = StudentDto),
		(status = 401, description = "Wrong or missing login", body = ErrorBody),
		(status = 404, description = "No such student", body = ErrorBody),
	),
	security(("basic" = [])),
)]
#[delete("/{id}/foto")]
pub(super) async fn quitar_foto(
	conn: web::Data<DatabaseConnection>,
	path: web::Path<String>,
) -> AppResult<HttpResponse> {
	let row = alumno::Entity::find_by_id(id(&path, "student")?)
		.one(conn.get_ref())
		.await?
		.ok_or_else(|| AppError::not_found("student"))?;
	contactos::quitar_foto(&conn, row.id).await?;
	Ok(HttpResponse::Ok().json(contactos::perfil(&conn, row).await?))
}
