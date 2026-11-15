//! Control de acceso: cuenta del tallerista (cookie `sesion`) y administración (Basic).

use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::OnceLock;
use std::task::{Context, Poll};

use actix_web::body::{BoxBody, EitherBody};
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::Method;
use actix_web::http::header::{ACCEPT, AUTHORIZATION, LOCATION, WWW_AUTHENTICATE};
use actix_web::{Error, HttpMessage, HttpResponse, web};
use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use base64::Engine;
use sea_orm::DatabaseConnection;

use crate::services as db;

pub struct ControlAcceso;

impl<S, B> Transform<S, ServiceRequest> for ControlAcceso
where
	S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
	S::Future: 'static,
	B: 'static,
{
	type Response = ServiceResponse<EitherBody<B, BoxBody>>;
	type Error = Error;
	type InitError = ();
	type Transform = ControlAccesoSvc<S>;
	type Future = Ready<Result<Self::Transform, Self::InitError>>;

	fn new_transform(&self, service: S) -> Self::Future {
		ready(Ok(ControlAccesoSvc {
			service: Rc::new(service),
		}))
	}
}

pub struct ControlAccesoSvc<S> {
	service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for ControlAccesoSvc<S>
where
	S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
	S::Future: 'static,
	B: 'static,
{
	type Response = ServiceResponse<EitherBody<B, BoxBody>>;
	type Error = Error;
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

	fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		self.service.poll_ready(cx)
	}

	fn call(&self, req: ServiceRequest) -> Self::Future {
		let service = Rc::clone(&self.service);
		Box::pin(async move {
			let ruta = ruta_normal(req.path());
			if ruta == "/"
				|| ruta == "/entrar"
				|| ruta == "/crear"
				|| ruta == "/salir"
				|| ruta == "/cuenta/crear"
				|| ruta == "/cuenta/entrar"
				|| es_publica(ruta)
			{
				let respuesta = service.call(req).await?;
				return Ok(respuesta.map_into_left_body());
			}
			if es_peticion_del_navegador(ruta) {
				return Ok(
					req
						.into_response(HttpResponse::NotFound().finish())
						.map_into_right_body(),
				);
			}

			let es_cuenta = ruta == "/cuenta" || ruta.starts_with("/cuenta/");
			let Some(db) = req.app_data::<web::Data<DatabaseConnection>>().cloned() else {
				return Ok(req.into_response(error_interno()).map_into_right_body());
			};

			if es_de_la_app(req.method(), ruta) {
				let encabezado = req
					.headers()
					.get(AUTHORIZATION)
					.and_then(|valor| valor.to_str().ok())
					.map(str::to_owned);
				match crate::services::acceso::tallerista_basic(db.get_ref(), encabezado.as_deref()).await {
					Ok(Some(sesion)) => {
						req.extensions_mut().insert(sesion);
					}
					Ok(None) => {
						return Ok(
							req
								.into_response(HttpResponse::Unauthorized().json(serde_json::json!({
										"code": "unauthorized",
										"message": "wrong or missing tallerista login"
								})))
								.map_into_right_body(),
						);
					}
					Err(_) => {
						return Ok(req.into_response(error_interno()).map_into_right_body());
					}
				}
			} else if es_cuenta {
				let token = req.cookie("sesion").map(|cookie| cookie.value().to_string());
				let sesion = match token {
					Some(token) => db::sesion_por_token(db.get_ref(), &token).await,
					None => Ok(None),
				};
				match sesion {
					Ok(Some(sesion)) => {
						req.extensions_mut().insert(sesion);
					}
					Ok(None) => {
						if pide_html(&req) {
							return Ok(
								req
									.into_response(HttpResponse::SeeOther().insert_header((LOCATION, "/")).finish())
									.map_into_right_body(),
							);
						}
						return Ok(req.into_response(sin_codigo()).map_into_right_body());
					}
					Err(_) => {
						return Ok(req.into_response(error_interno()).map_into_right_body());
					}
				}
			} else {
				let encabezado = req
					.headers()
					.get(AUTHORIZATION)
					.and_then(|valor| valor.to_str().ok())
					.map(str::to_owned);
				let Some((usuario, clave)) = encabezado.as_deref().and_then(credenciales_basic) else {
					return Ok(req.into_response(no_autorizado("administracion")).map_into_right_body());
				};
				match db::clave_admin_valida(db.get_ref(), &usuario, &clave).await {
					Ok(true) => {}
					Ok(false) => {
						return Ok(req.into_response(no_autorizado("administracion")).map_into_right_body());
					}
					Err(_) => {
						return Ok(req.into_response(error_interno()).map_into_right_body());
					}
				}
			}

			let respuesta = service.call(req).await?;
			Ok(respuesta.map_into_left_body())
		})
	}
}

/// Rutas de la app que no piden login: la salud, el aviso de Pey (que trae su firma) y la sesión,
/// que revisa el login por sí misma.
fn es_publica(ruta: &str) -> bool {
	matches!(ruta, "/v1/salud" | "/v1/webhooks/pey" | "/v1/sesion")
}

/// Rutas de la app, que entran con el login del tallerista. El resto de `/v1` es administración.
fn es_de_la_app(metodo: &Method, ruta: &str) -> bool {
	let partes: Vec<&str> = ruta.trim_start_matches('/').split('/').collect();
	let numero = |parte: &str| !parte.is_empty() && parte.bytes().all(|byte| byte.is_ascii_digit());
	match (metodo.as_str(), partes.as_slice()) {
		("GET", ["v1", "talleres", id]) => numero(id),
		("GET", ["v1", "talleres", id, "alumnos" | "cobros" | "solicitudes" | "codigo"]) => numero(id),
		("POST", ["v1", "talleres", id, "alumnos" | "codigo"]) => numero(id),
		("POST", ["v1", "talleres", id, "alumnos", alumno, "aprobar" | "baja"]) => numero(id) && numero(alumno),
		("GET", ["v1", "alumnos", id]) => numero(id),
		("DELETE", ["v1", "alumnos", id, "foto"]) => numero(id),
		("GET", ["v1", "alumnos", id, "tarjeta"]) => numero(id),
		("POST", ["v1", "alumnos", id, "unirse"]) => numero(id),
		("POST", ["v1", "pagos", "cobrar"]) => true,
		("GET", ["v1", "pagos", id]) => numero(id),
		_ => false,
	}
}

fn ruta_normal(ruta: &str) -> &str {
	if ruta.len() > 1 {
		ruta.trim_end_matches('/')
	} else {
		ruta
	}
}

fn es_peticion_del_navegador(ruta: &str) -> bool {
	matches!(
		ruta,
		"/favicon.ico" | "/robots.txt" | "/apple-touch-icon.png" | "/apple-touch-icon-precomposed.png"
	)
}

fn pide_html(req: &ServiceRequest) -> bool {
	req
		.headers()
		.get(ACCEPT)
		.and_then(|valor| valor.to_str().ok())
		.is_some_and(|valor| valor.contains("text/html"))
}

fn sin_codigo() -> HttpResponse {
	HttpResponse::Unauthorized().json(serde_json::json!({
			"error": "Para entrar, usa tu usuario y tu contraseña.",
			"entrar": "/entrar",
			"crear": "/crear"
	}))
}

fn no_autorizado(realm: &str) -> HttpResponse {
	let mensaje = if realm == "cuenta del tallerista" {
		"inicia sesión con el usuario y la contraseña del tallerista"
	} else {
		"inicia sesión con el usuario y la contraseña de administración"
	};
	HttpResponse::Unauthorized()
		.insert_header((WWW_AUTHENTICATE, format!("Basic realm=\"{realm}\", charset=\"UTF-8\"")))
		.json(serde_json::json!({ "error": mensaje }))
}

fn error_interno() -> HttpResponse {
	HttpResponse::InternalServerError().json(serde_json::json!({
			"error": "no se pudo comprobar la sesión"
	}))
}

pub fn hash_clave(clave: &str) -> Result<String, sea_orm::DbErr> {
	let salt = salt_aleatorio()?;
	Argon2::default()
		.hash_password(clave.as_bytes(), &salt)
		.map(|hash| hash.to_string())
		.map_err(|error| sea_orm::DbErr::Custom(error.to_string()))
}

fn salt_aleatorio() -> Result<SaltString, sea_orm::DbErr> {
	let mut bytes = [0u8; 16];
	let mut origen = std::fs::File::open("/dev/urandom").map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?;
	std::io::Read::read_exact(&mut origen, &mut bytes).map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?;
	SaltString::encode_b64(&bytes).map_err(|error| sea_orm::DbErr::Custom(error.to_string()))
}

pub fn clave_correcta(hash: &str, clave: &str) -> bool {
	PasswordHash::new(hash)
		.ok()
		.and_then(|parsed| Argon2::default().verify_password(clave.as_bytes(), &parsed).ok())
		.is_some()
}

pub fn hash_falso() -> &'static str {
	static HASH: OnceLock<String> = OnceLock::new();
	HASH.get_or_init(|| {
		hash_clave("no-es-la-clave")
			.unwrap_or_else(|_| "$argon2id$v=19$m=19456,t=2,p=1$bm9lc2xhc2FsdA$bm9lc2VsSGFzaA".to_owned())
	})
}

pub fn credenciales_basic(valor: &str) -> Option<(String, String)> {
	let (esquema, encoded) = valor.trim().split_once(' ')?;
	if !esquema.eq_ignore_ascii_case("basic") {
		return None;
	}
	let bytes = base64::engine::general_purpose::STANDARD.decode(encoded.trim()).ok()?;
	let texto = String::from_utf8(bytes).ok()?;
	let (usuario, clave) = texto.split_once(':')?;
	let usuario = usuario.trim();
	let clave = clave.trim();
	if usuario.is_empty() || clave.is_empty() {
		return None;
	}
	Some((usuario.to_owned(), clave.to_owned()))
}

pub fn normalizar_usuario(valor: &str) -> String {
	valor
		.trim()
		.chars()
		.filter_map(|caracter| {
			let caracter = match caracter {
				'á' | 'à' | 'ä' | 'Á' | 'À' | 'Ä' => 'a',
				'é' | 'è' | 'ë' | 'É' | 'È' | 'Ë' => 'e',
				'í' | 'ì' | 'ï' | 'Í' | 'Ì' | 'Ï' => 'i',
				'ó' | 'ò' | 'ö' | 'Ó' | 'Ò' | 'Ö' => 'o',
				'ú' | 'ù' | 'ü' | 'Ú' | 'Ù' | 'Ü' => 'u',
				'ñ' | 'Ñ' => 'n',
				otro => otro.to_ascii_lowercase(),
			};
			if caracter.is_whitespace() { None } else { Some(caracter) }
		})
		.collect()
}

pub fn encabezado_basic(usuario: &str, clave: &str) -> String {
	let token = base64::engine::general_purpose::STANDARD.encode(format!("{usuario}:{clave}"));
	format!("Basic {token}")
}

pub fn token_opaco() -> Result<String, sea_orm::DbErr> {
	let mut bytes = [0u8; 32];
	llenar_azar(&mut bytes)?;
	Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

pub fn resumen_codigo(sal: &str, codigo: &str) -> String {
	use sha2::{Digest, Sha256};
	let mut resumen = Sha256::new();
	resumen.update(sal.as_bytes());
	resumen.update(b":");
	resumen.update(codigo.as_bytes());
	format!("{:x}", resumen.finalize())
}

fn llenar_azar(bytes: &mut [u8]) -> Result<(), sea_orm::DbErr> {
	let mut origen = std::fs::File::open("/dev/urandom").map_err(|error| sea_orm::DbErr::Custom(error.to_string()))?;
	std::io::Read::read_exact(&mut origen, bytes).map_err(|error| sea_orm::DbErr::Custom(error.to_string()))
}
