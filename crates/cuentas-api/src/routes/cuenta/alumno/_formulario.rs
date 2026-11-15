//! `POST /cuenta/alumno`

use crate::error::ApiError;
use crate::routes::comun::*;
use crate::services as db;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::AlumnoDeCuenta;
use crate::routes::cuenta::cuenta_con_error;

#[post("")]
pub(crate) async fn alumno_cuenta_formulario(
	db: web::Data<DatabaseConnection>,
	sesion: web::ReqData<db::Sesion>,
	form: web::Form<AlumnoDeCuenta>,
) -> Result<HttpResponse, ApiError> {
	match db::guardar_alumno_cuenta(db.get_ref(), sesion.id, &form.nombre, form.taller.as_deref()).await {
		Ok(()) => Ok(ir_a_cuenta(None)),
		Err(error) => cuenta_con_error(db.get_ref(), &sesion, error).await,
	}
}
