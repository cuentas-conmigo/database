//! `GET /cuenta/horario`

use crate::error::ApiError;
use crate::services as db;
use actix_web::{HttpResponse, get, web};
use sea_orm::DatabaseConnection;

use super::HorarioDeCuenta;

#[get("")]
pub(crate) async fn horario_cuenta_navegador(
	db: web::Data<DatabaseConnection>,
	sesion: web::ReqData<db::Sesion>,
	query: web::Query<HorarioDeCuenta>,
) -> Result<HttpResponse, ApiError> {
	db::guardar_horario_cuenta(
		db.get_ref(),
		sesion.id,
		query.taller.as_deref(),
		&query.dia,
		&query.de,
		&query.a,
	)
	.await
	.map_err(ApiError::db)?;
	let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
	Ok(HttpResponse::Ok().json(cuerpo))
}
