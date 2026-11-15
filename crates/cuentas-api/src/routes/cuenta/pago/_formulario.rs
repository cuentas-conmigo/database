//! `POST /cuenta/pago`

use crate::error::ApiError;
use crate::routes::comun::*;
use crate::services as db;
use actix_web::{HttpResponse, post, web};
use sea_orm::DatabaseConnection;

use super::PagoDeCuenta;
use crate::routes::cuenta::cuenta_con_error;

#[post("")]
pub(crate) async fn pago_cuenta_formulario(
	db: web::Data<DatabaseConnection>,
	sesion: web::ReqData<db::Sesion>,
	form: web::Form<PagoDeCuenta>,
) -> Result<HttpResponse, ApiError> {
	let estado = form.estado.clone().unwrap_or_else(|| "pagado".to_string());
	match db::marcar_pago_cuenta(db.get_ref(), sesion.id, &form.alumno, form.taller.as_deref(), &estado).await {
		Ok(()) => Ok(ir_a_cuenta(None)),
		Err(error) => cuenta_con_error(db.get_ref(), &sesion, error).await,
	}
}
