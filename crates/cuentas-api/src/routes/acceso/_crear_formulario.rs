//! `POST /crear`

use crate::error::ApiError;
use crate::routes::comun::*;
use crate::services as db;
use crate::web::pagina;
use actix_web::{HttpResponse, http::StatusCode, post, web};
use sea_orm::DatabaseConnection;

use crate::routes::comun::Credenciales;

#[post("")]
pub(crate) async fn crear_formulario(
	db: web::Data<DatabaseConnection>,
	form: web::Form<Credenciales>,
) -> Result<HttpResponse, ApiError> {
	match db::crear_cuenta(db.get_ref(), &form.usuario, &form.clave).await {
		Ok(entrada) => Ok(ir_a_cuenta(Some(&entrada.token))),
		Err(error) => Ok(html_estado(
			StatusCode::BAD_REQUEST,
			pagina::crear(Some(&texto_aviso(error))),
		)),
	}
}
