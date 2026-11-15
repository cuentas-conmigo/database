//! Cuentas de tallerista, contraseñas y sesiones.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};

use super::*;

#[derive(Clone, Debug)]
pub struct Sesion {
	pub id: i32,
	pub nombre: String,
	pub usuario: String,
}

pub async fn clave_admin_valida(db: &DatabaseConnection, usuario: &str, clave: &str) -> Result<bool, DbErr> {
	let clave = clave.trim();
	let fila = db
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT clave_hash FROM administradores WHERE lower(usuario) = lower($1)",
			[usuario.trim().into()],
		))
		.await?;
	let Some(fila) = fila else {
		let _ = crate::middleware::auth::clave_correcta(crate::middleware::auth::hash_falso(), clave);
		return Ok(false);
	};
	let hash: String = fila.try_get("", "clave_hash")?;
	Ok(crate::middleware::auth::clave_correcta(&hash, clave))
}

pub(crate) fn usuario_valido(valor: &str) -> Result<String, DbErr> {
	let usuario = crate::middleware::auth::normalizar_usuario(valor);
	if (3..=30).contains(&usuario.len()) && usuario.chars().all(|caracter| caracter.is_ascii_alphanumeric()) {
		Ok(usuario)
	} else {
		Err(aviso("el usuario usa letras y números, entre 3 y 30, sin espacios"))
	}
}

pub(crate) fn clave_valida(clave: &str) -> Result<&str, DbErr> {
	let clave = clave.trim();
	if (6..=100).contains(&clave.len()) {
		Ok(clave)
	} else {
		Err(aviso("la contraseña debe tener entre 6 y 100 caracteres"))
	}
}

pub async fn crear_cuenta(db: &DatabaseConnection, usuario: &str, clave: &str) -> Result<Entrada, DbErr> {
	let usuario = usuario_valido(usuario)?;
	let clave = clave_valida(clave)?;
	let ya = db
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT id FROM talleristas WHERE usuario = $1",
			[usuario.clone().into()],
		))
		.await?;
	if ya.is_some() {
		return Err(aviso("ese usuario ya existe. Entra en /cuenta/entrar?usuario=&clave="));
	}
	let hash = crate::middleware::auth::hash_clave(clave)?;
	db.execute_raw(Statement::from_sql_and_values(
		DbBackend::Postgres,
		"INSERT INTO talleristas (nombre, oficio, pey_api_key_id, usuario, clave_hash)
         VALUES ($1, '', $2, $3, $4)",
		[
			usuario.clone().into(),
			format!("ejemplo_pey_{usuario}").into(),
			usuario.clone().into(),
			hash.into(),
		],
	))
	.await?;
	let fila = db
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT id, nombre, usuario FROM talleristas WHERE usuario = $1",
			[usuario.into()],
		))
		.await?
		.ok_or_else(|| DbErr::Custom("no se pudo crear la cuenta".to_owned()))?;
	abrir_sesion(
		db,
		Sesion {
			id: fila.try_get("", "id")?,
			nombre: fila.try_get("", "nombre")?,
			usuario: fila.try_get("", "usuario")?,
		},
	)
	.await
}

pub struct Entrada {
	pub sesion: Sesion,
	pub token: String,
}

pub async fn entrar_con_clave(db: &DatabaseConnection, usuario: &str, clave: &str) -> Result<Entrada, DbErr> {
	let clave = clave.trim();
	let usuario = match usuario_valido(usuario) {
		Ok(usuario) => usuario,
		Err(_) => {
			let _ = crate::middleware::auth::clave_correcta(crate::middleware::auth::hash_falso(), clave);
			return Err(aviso("usuario o contraseña incorrectos"));
		}
	};
	let fila = db
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT id, nombre, usuario, clave_hash FROM talleristas WHERE usuario = $1",
			[usuario.into()],
		))
		.await?;
	let Some(fila) = fila else {
		let _ = crate::middleware::auth::clave_correcta(crate::middleware::auth::hash_falso(), clave);
		return Err(aviso("usuario o contraseña incorrectos"));
	};
	let hash: Option<String> = fila.try_get("", "clave_hash")?;
	let valida = hash
		.as_deref()
		.is_some_and(|hash| crate::middleware::auth::clave_correcta(hash, clave));
	if !valida {
		return Err(aviso("usuario o contraseña incorrectos"));
	}
	abrir_sesion(
		db,
		Sesion {
			id: fila.try_get("", "id")?,
			nombre: fila.try_get("", "nombre")?,
			usuario: fila.try_get("", "usuario")?,
		},
	)
	.await
}

pub(crate) async fn abrir_sesion(db: &DatabaseConnection, sesion: Sesion) -> Result<Entrada, DbErr> {
	let token = crate::middleware::auth::token_opaco()?;
	let token_hash = crate::middleware::auth::resumen_codigo("sesion", &token);
	db.execute_raw(Statement::from_sql_and_values(
		DbBackend::Postgres,
		"INSERT INTO sesiones (tallerista_id, token_hash, creada_en, vence_en)
         VALUES ($1, $2, ahora_texto(), ahora_texto(INTERVAL '12 hours'))",
		[sesion.id.into(), token_hash.into()],
	))
	.await?;
	Ok(Entrada { sesion, token })
}

pub async fn sesion_por_token(db: &DatabaseConnection, token: &str) -> Result<Option<Sesion>, DbErr> {
	if token.is_empty() {
		return Ok(None);
	}
	let token_hash = crate::middleware::auth::resumen_codigo("sesion", token);
	let fila = db
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT t.id, t.nombre, t.usuario
             FROM sesiones s
             JOIN talleristas t ON t.id = s.tallerista_id
             WHERE s.token_hash = $1 AND s.vence_en > ahora_texto()",
			[token_hash.into()],
		))
		.await?;
	let Some(fila) = fila else {
		return Ok(None);
	};
	Ok(Some(Sesion {
		id: fila.try_get("", "id")?,
		nombre: fila.try_get("", "nombre")?,
		usuario: fila.try_get("", "usuario")?,
	}))
}
