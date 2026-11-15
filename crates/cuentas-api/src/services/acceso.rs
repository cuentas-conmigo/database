//! Tallerista login for the app's routes with HTTP Basic, reusing Amanda's argon2 helpers. Unlike
//! `/cuenta/entrar`, it opens no session row: the app sends the login on every request.

use base64::Engine;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};

use crate::middleware::auth::{clave_correcta, credenciales_basic, hash_falso};
use crate::services::{Sesion, usuario_valido};

/// The tallerista behind `usuario`/`clave`, or `None` when they do not match.
pub async fn tallerista(db: &DatabaseConnection, usuario: &str, clave: &str) -> Result<Option<Sesion>, DbErr> {
	let clave = clave.trim();
	let Ok(usuario) = usuario_valido(usuario) else {
		let _ = clave_correcta(hash_falso(), clave);
		return Ok(None);
	};
	let fila = db
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT id, nombre, usuario, clave_hash FROM talleristas WHERE usuario = $1",
			[usuario.into()],
		))
		.await?;
	let Some(fila) = fila else {
		let _ = clave_correcta(hash_falso(), clave);
		return Ok(None);
	};
	let hash: Option<String> = fila.try_get("", "clave_hash")?;
	if !hash.as_deref().is_some_and(|hash| clave_correcta(hash, clave)) {
		return Ok(None);
	}
	Ok(Some(Sesion {
		id: fila.try_get("", "id")?,
		nombre: fila.try_get("", "nombre")?,
		usuario: fila.try_get("", "usuario")?,
	}))
}

/// Same as [`tallerista`], from an `Authorization: Basic` header value.
pub async fn tallerista_basic(db: &DatabaseConnection, encabezado: Option<&str>) -> Result<Option<Sesion>, DbErr> {
	match encabezado.and_then(credenciales_basic) {
		Some((usuario, clave)) => tallerista(db, &usuario, &clave).await,
		None => Ok(None),
	}
}

/// `Basic` header for tests and scripts.
pub fn basic(usuario: &str, clave: &str) -> String {
	format!(
		"Basic {}",
		base64::engine::general_purpose::STANDARD.encode(format!("{usuario}:{clave}"))
	)
}
