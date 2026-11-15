//! `GET /entrar`

use crate::routes::comun::*;
use crate::web::pagina;
use actix_web::{Responder, get};

#[get("")]
pub(crate) async fn pagina_entrar() -> impl Responder {
	html(pagina::entrar(None))
}
