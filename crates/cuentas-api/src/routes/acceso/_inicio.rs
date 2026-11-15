//! `GET /`

use crate::routes::comun::*;
use crate::web::pagina;
use actix_web::{Responder, get};

#[get("/")]
pub(crate) async fn inicio() -> impl Responder {
	html(pagina::eleccion())
}
