//! `/`, `/entrar`, `/crear` y `/salir` — página de inicio, entrar, crear cuenta y salir.

mod _crear_formulario;
mod _crear_pagina;
mod _entrar_formulario;
mod _entrar_pagina;
mod _inicio;
mod _salir;

use actix_web::web;

use _crear_formulario::crear_formulario;
use _crear_pagina::pagina_crear;
use _entrar_formulario::entrar_formulario;
use _entrar_pagina::pagina_entrar;
use _inicio::inicio;
use _salir::salir;

/// Registra la página de inicio y las páginas para entrar, crear cuenta y salir.
pub fn config(cfg: &mut web::ServiceConfig) {
	cfg
		.service(inicio)
		.service(web::scope("/entrar").service(pagina_entrar).service(entrar_formulario))
		.service(web::scope("/crear").service(pagina_crear).service(crear_formulario))
		.service(web::scope("/salir").service(salir));
}
