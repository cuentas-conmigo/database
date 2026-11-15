//! `POST /cuenta/taller`

use crate::error::ApiError;
use crate::routes::comun::*;
use crate::services as db;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::TallerDeCuenta;
use crate::routes::cuenta::cuenta_con_error;

#[post("")]
pub(crate) async fn taller_cuenta_formulario(
	db: web::Data<DatabaseConnection>,
	sesion: web::ReqData<db::Sesion>,
	form: web::Form<TallerDeCuenta>,
) -> Result<HttpResponse, ApiError> {
	let cuota = form.cuota_mensual_mxn.unwrap_or(db::MENSUALIDAD_MXN);
	match db::guardar_taller_cuenta(db.get_ref(), sesion.id, &form.nombre, cuota).await {
		Ok(()) => Ok(ir_a_cuenta(None)),
		Err(error) => cuenta_con_error(db.get_ref(), &sesion, error).await,
	}
}
