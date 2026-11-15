//! `POST /cuenta/datos`

use crate::error::ApiError;
use crate::routes::comun::*;
use crate::services as db;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::DatosCuenta;
use crate::routes::cuenta::cuenta_con_error;

#[post("")]
pub(crate) async fn datos_cuenta_formulario(
	db: web::Data<DatabaseConnection>,
	sesion: web::ReqData<db::Sesion>,
	form: web::Form<DatosCuenta>,
) -> Result<HttpResponse, ApiError> {
	match db::guardar_datos_cuenta(db.get_ref(), sesion.id, &form.nombre, &form.oficio).await {
		Ok(()) => Ok(ir_a_cuenta(None)),
		Err(error) => cuenta_con_error(db.get_ref(), &sesion, error).await,
	}
}
