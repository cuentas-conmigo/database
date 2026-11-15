//! Ayudas compartidas por las rutas: respuestas HTML, validaciones y borrado.

use crate::error::ApiError;
use actix_web::{HttpRequest, HttpResponse, http::StatusCode};
use sea_orm::{DatabaseConnection, DbErr, EntityTrait};
use serde::Deserialize;

pub(crate) fn cookie_sesion(token: &str) -> actix_web::cookie::Cookie<'static> {
	actix_web::cookie::Cookie::build("sesion", token.to_owned())
		.path("/")
		.http_only(true)
		.same_site(actix_web::cookie::SameSite::Lax)
		.finish()
}

pub(crate) fn html(cuerpo: String) -> HttpResponse {
	html_estado(StatusCode::OK, cuerpo)
}

pub(crate) fn html_estado(status: StatusCode, cuerpo: String) -> HttpResponse {
	HttpResponse::build(status)
		.content_type("text/html; charset=utf-8")
		.body(cuerpo)
}

pub(crate) fn ir_a_cuenta(token: Option<&str>) -> HttpResponse {
	let mut respuesta = HttpResponse::SeeOther();
	respuesta.insert_header((actix_web::http::header::LOCATION, "/cuenta"));
	if let Some(token) = token {
		respuesta.cookie(cookie_sesion(token));
	}
	respuesta.finish()
}

pub(crate) fn pide_html(req: &HttpRequest) -> bool {
	req
		.headers()
		.get(actix_web::http::header::ACCEPT)
		.and_then(|valor| valor.to_str().ok())
		.is_some_and(|valor| valor.contains("text/html"))
}

pub(crate) fn texto_aviso(error: DbErr) -> String {
	let texto = error.to_string();
	texto
		.split_once("aviso: ")
		.map(|(_, aviso)| aviso.to_string())
		.unwrap_or_else(|| "No se pudo guardar. Inténtalo de nuevo.".to_string())
}

#[derive(Deserialize)]
pub(crate) struct ConId {
	pub(crate) id: i32,
}

pub(crate) async fn borrar_en_navegador<E>(
	db: &DatabaseConnection,
	id: i32,
	nombre: &str,
) -> Result<HttpResponse, ApiError>
where
	E: EntityTrait,
	E::PrimaryKey: sea_orm::PrimaryKeyTrait<ValueType = i32>,
{
	eliminar::<E>(db, id, nombre).await?;
	Ok(HttpResponse::Ok().json(serde_json::json!({
			"borrado": nombre,
			"id": id
	})))
}

pub(crate) async fn eliminar<E>(db: &DatabaseConnection, id: i32, nombre: &str) -> Result<HttpResponse, ApiError>
where
	E: EntityTrait,
	E::PrimaryKey: sea_orm::PrimaryKeyTrait<ValueType = i32>,
{
	let resultado = E::delete_by_id(id).exec(db).await.map_err(ApiError::db)?;
	if resultado.rows_affected == 0 {
		return Err(ApiError::no(nombre, id));
	}
	Ok(HttpResponse::NoContent().finish())
}

pub(crate) fn exigir_texto(valor: &str, campo: &str) -> Result<(), ApiError> {
	if valor.trim().is_empty() {
		Err(ApiError::mensaje(
			StatusCode::BAD_REQUEST,
			format!("{campo} no puede estar vacío"),
		))
	} else {
		Ok(())
	}
}

pub(crate) fn exigir_monto(monto: i32) -> Result<(), ApiError> {
	if monto <= 0 {
		Err(ApiError::mensaje(
			StatusCode::BAD_REQUEST,
			"el monto debe ser mayor que 0",
		))
	} else {
		Ok(())
	}
}

pub(crate) fn exigir_cara(cara: &str) -> Result<(), ApiError> {
	if cara == "cobro" || cara == "inscripcion" {
		Ok(())
	} else {
		Err(ApiError::mensaje(
			StatusCode::BAD_REQUEST,
			"la cara debe ser cobro o inscripcion",
		))
	}
}

pub(crate) fn exigir_tipo(tipo: &str) -> Result<(), ApiError> {
	if tipo == "mensualidad" || tipo == "inscripcion_anual" {
		Ok(())
	} else {
		Err(ApiError::mensaje(
			StatusCode::BAD_REQUEST,
			"el tipo debe ser mensualidad o inscripcion_anual",
		))
	}
}

pub(crate) fn exigir_canal(canal: &str) -> Result<(), ApiError> {
	if canal == "boton" || canal == "nfc" || canal == "panel" {
		Ok(())
	} else {
		Err(ApiError::mensaje(
			StatusCode::BAD_REQUEST,
			"el canal debe ser boton, nfc o panel",
		))
	}
}

pub(crate) fn exigir_estado(estado: &str) -> Result<(), ApiError> {
	if estado == "pendiente" || estado == "pagado" || estado == "fallido" {
		Ok(())
	} else {
		Err(ApiError::mensaje(
			StatusCode::BAD_REQUEST,
			"el estado debe ser pendiente, pagado o fallido",
		))
	}
}

pub(crate) fn marca_ms() -> u128 {
	std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|duracion| duracion.as_millis())
		.unwrap_or(0)
}

/// Usuario y clave del tallerista, del formulario o de la dirección.
#[derive(Deserialize)]
pub(crate) struct Credenciales {
	pub(crate) usuario: String,
	pub(crate) clave: String,
}

/// Id numérico de la ruta; lo demás es un 404 de `what`.
pub(crate) fn id(value: &str, what: &str) -> crate::error::AppResult<i32> {
	value
		.trim()
		.parse()
		.map_err(|_| crate::error::AppError::not_found(what))
}
