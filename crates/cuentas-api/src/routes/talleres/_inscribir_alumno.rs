//! `POST /v1/talleres/{id}/alumnos`

use actix_web::{HttpResponse, post, web};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use crate::config::AppState;
use crate::error::{AppError, AppResult, ErrorBody};
use crate::models::dto::{NewStudentBody, StudentDto};
use crate::models::entities::alumno;
use crate::routes::comun::id;
use crate::services::{self as db, Sesion, contactos};

/// Enrolls a new student with Amanda's rule (`guardar_alumno_cuenta`), plus WhatsApp, helper and a consented photo.
#[utoipa::path(
	tag = "talleres",
	operation_id = "talleres_inscribir_alumno",
	params(("id" = String, Path, description = "Workshop id")),
	request_body = NewStudentBody,
	responses(
		(status = 201, description = "The new student", body = StudentDto),
		(status = 400, description = "Amanda's validation message, or a phone without 10 digits", body = ErrorBody),
		(status = 409, description = "That WhatsApp is already registered (`duplicate-phone`)", body = ErrorBody),
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
	let phone = contactos::digitos(&body.phone);
	if phone.len() != 10 {
		return Err(AppError::bad_request("the WhatsApp needs 10 digits"));
	}
	let helper = body
		.helper_phone
		.as_deref()
		.map(contactos::digitos)
		.filter(|value| !value.is_empty());
	if helper.as_ref().is_some_and(|value| value.len() != 10) {
		return Err(AppError::bad_request("the helper's WhatsApp needs 10 digits"));
	}
	if contactos::telefono_registrado(&conn, &phone).await? {
		return Err(AppError::duplicate_phone());
	}
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
	contactos::guardar_contacto(&conn, created.id, &phone, helper.as_deref()).await?;
	if body.photo_consent
		&& let Some(uri) = body.photo_uri.as_deref().filter(|uri| !uri.is_empty())
	{
		contactos::guardar_foto(&conn, created.id, uri, name).await?;
	}
	Ok(HttpResponse::Created().json(contactos::perfil(&conn, created).await?))
}
