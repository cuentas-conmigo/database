//! `GET /cuenta/entrar`

use crate::error::ApiError;
use crate::services as db;
use actix_web::{HttpResponse, get, http::StatusCode, web};
use sea_orm::DatabaseConnection;

use super::responder_cuenta;
use crate::routes::comun::Credenciales;

#[get("/entrar")]
pub(crate) async fn entrar_cuenta_navegador(
	db: web::Data<DatabaseConnection>,
	query: web::Query<Credenciales>,
) -> Result<HttpResponse, ApiError> {
	let entrada = db::entrar_con_clave(db.get_ref(), &query.usuario, &query.clave)
		.await
		.map_err(ApiError::db)?;
	responder_cuenta(db.get_ref(), &entrada, StatusCode::OK).await
}
