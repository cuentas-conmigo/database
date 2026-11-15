//! `POST /cuenta/horario`

use crate::error::ApiError;
use crate::routes::comun::*;
use crate::services as db;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::HorarioDeCuenta;
use crate::routes::cuenta::cuenta_con_error;

#[post("")]
pub(crate) async fn horario_cuenta_formulario(
	db: web::Data<DatabaseConnection>,
	sesion: web::ReqData<db::Sesion>,
	form: web::Form<HorarioDeCuenta>,
) -> Result<HttpResponse, ApiError> {
	match db::guardar_horario_cuenta(
		db.get_ref(),
		sesion.id,
		form.taller.as_deref(),
		&form.dia,
		&form.de,
		&form.a,
	)
	.await
	{
		Ok(()) => Ok(ir_a_cuenta(None)),
		Err(error) => cuenta_con_error(db.get_ref(), &sesion, error).await,
	}
}
