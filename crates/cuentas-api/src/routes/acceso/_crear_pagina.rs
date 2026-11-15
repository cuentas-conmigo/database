//! `GET /crear`

use crate::routes::comun::*;
use crate::web::pagina;
use actix_web::{Responder, get};

#[get("")]
pub(crate) async fn pagina_crear() -> impl Responder {
	html(pagina::crear(None))
}
