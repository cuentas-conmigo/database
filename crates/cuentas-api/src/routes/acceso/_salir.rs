//! `POST /salir`

use crate::routes::comun::*;
use actix_web::{HttpResponse, Responder, post};

#[post("")]
pub(crate) async fn salir() -> impl Responder {
	let mut galleta = cookie_sesion("");
	galleta.make_removal();
	HttpResponse::SeeOther()
		.insert_header((actix_web::http::header::LOCATION, "/"))
		.cookie(galleta)
		.finish()
}
