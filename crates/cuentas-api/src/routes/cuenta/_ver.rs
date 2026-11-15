//! `GET /cuenta`

use crate::error::ApiError;
use crate::routes::comun::*;
use crate::services as db;
use crate::web::pagina;
use actix_web::{HttpRequest, HttpResponse, get, web};
use sea_orm::DatabaseConnection;

#[get("")]
pub(crate) async fn cuenta(
	req: HttpRequest,
	db: web::Data<DatabaseConnection>,
	sesion: web::ReqData<db::Sesion>,
) -> Result<HttpResponse, ApiError> {
	let cuerpo = db::cuenta(db.get_ref(), &sesion).await.map_err(ApiError::db)?;
	if pide_html(&req) {
		Ok(html(pagina::cuenta(&cuerpo, None)))
	} else {
		Ok(HttpResponse::Ok().json(cuerpo))
	}
}
