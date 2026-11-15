//! `GET /cuenta/taller`

use crate::error::ApiError;
use crate::services as db;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::TallerDeCuenta;

#[get("")]
pub(crate) async fn taller_cuenta_navegador(
	db: web::Data<DatabaseConnection>,
	sesion: web::ReqData<db::Sesion>,
	query: web::Query<TallerDeCuenta>,
) -> Result<HttpResponse, ApiError> {
	let cuota = query.cuota_mensual_mxn.unwrap_or(db::MENSUALIDAD_MXN);
	db::guardar_taller_cuenta(db.get_ref(), sesion.id, &query.nombre, cuota)
		.await
		.map_err(ApiError::db)?;
	let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(cuerpo))
}
