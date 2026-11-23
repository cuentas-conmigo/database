//! `POST /v1/cuenta`

use actix_web::{HttpResponse, http::StatusCode, post, web};
use sea_orm::DatabaseConnection;

use crate::error::{AppError, AppResult, ErrorBody};
use crate::models::dto::{NewAccountBody, SessionDto};
use crate::services::{Registro, RegistroError, registrar};

/// Creates an instructor account with its first workshop; the app then signs in with the same login.
#[utoipa::path(
	tag = "cuenta",
	operation_id = "cuenta_crear",
	request_body = NewAccountBody,
	responses(
		(status = 201, description = "The new instructor and their workshop", body = SessionDto),
		(status = 400, description = "A field to fix, in plain Spanish in `message`", body = ErrorBody),
		(status = 409, description = "The username is taken (`username-taken`)", body = ErrorBody),
	),
)]
#[post("")]
pub(super) async fn crear_cuenta(
	conn: web::Data<DatabaseConnection>,
	body: web::Json<NewAccountBody>,
) -> AppResult<HttpResponse> {
	let body = body.into_inner();
	let registro = Registro {
		usuario: &body.usuario,
		clave: &body.clave,
		nombre: &body.name,
		oficio: &body.trade,
		taller: &body.workshop,
		cuota: body.monthly_fee,
	};
	let nueva = registrar(&conn, registro).await.map_err(|error| match error {
		RegistroError::Taken => AppError::username_taken(),
		RegistroError::Invalid(message) => AppError::bad_request(message),
		RegistroError::Db(error) => AppError::internal(error),
	})?;
	Ok(HttpResponse::build(StatusCode::CREATED).json(SessionDto {
		name: nueva.sesion.nombre,
		role: "instructor",
		workshop_id: Some(nueva.taller_id.to_string()),
	}))
}
