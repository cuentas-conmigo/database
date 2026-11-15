//! Pruebas de las rutas de Amanda.

use actix_web::{http::StatusCode, web};

use super::config;
use crate::services as db;
use actix_web::{App, test};

macro_rules! abrir_sesion {
	($servicio:expr, $usuario:expr, $clave:expr) => {{
		let entrada = test::call_service(
			$servicio,
			test::TestRequest::get()
				.uri(&format!("/cuenta/entrar?usuario={}&clave={}", $usuario, $clave))
				.to_request(),
		)
		.await;
		assert!(entrada.status().is_success());
		galleta_de(&entrada)
	}};
}

#[actix_web::test]
async fn la_pagina_guia_la_cuenta_nueva() {
	let db = db::base_de_prueba().await;
	let servicio = test::init_service(
		App::new()
			.app_data(web::Data::new(db))
			.wrap(crate::middleware::auth::ControlAcceso)
			.configure(config),
	)
	.await;

	let portada = test::call_service(&servicio, test::TestRequest::get().uri("/").to_request()).await;
	assert!(portada.status().is_success());
	let texto = test::read_body(portada).await;
	let texto = String::from_utf8(texto.to_vec()).unwrap();
	assert!(texto.contains("Iniciar sesión"));
	assert!(texto.contains("Crear una cuenta"));

	#[derive(serde::Serialize)]
	struct Forma {
		usuario: &'static str,
		clave: &'static str,
	}
	let alta = test::call_service(
		&servicio,
		test::TestRequest::post()
			.uri("/crear")
			.set_form(Forma {
				usuario: "nila",
				clave: "ceramica-2026",
			})
			.to_request(),
	)
	.await;
	assert_eq!(alta.status(), StatusCode::SEE_OTHER);
	let sesion = galleta_de(&alta);

	let paso = test::call_service(
		&servicio,
		con_sesion(
			&sesion,
			test::TestRequest::get()
				.uri("/cuenta")
				.insert_header((actix_web::http::header::ACCEPT, "text/html")),
		)
		.to_request(),
	)
	.await;
	assert!(paso.status().is_success());
	let texto = String::from_utf8(test::read_body(paso).await.to_vec()).unwrap();
	assert!(texto.contains("Tu nombre y tu oficio"));

	#[derive(serde::Serialize)]
	struct Datos {
		nombre: &'static str,
		oficio: &'static str,
	}
	let datos = test::call_service(
		&servicio,
		con_sesion(
			&sesion,
			test::TestRequest::post().uri("/cuenta/datos").set_form(Datos {
				nombre: "Nila",
				oficio: "ceramica",
			}),
		)
		.to_request(),
	)
	.await;
	assert_eq!(datos.status(), StatusCode::SEE_OTHER);
}

#[actix_web::test]
async fn el_servidor_lista_y_modifica_alumnos() {
	let db = db::base_de_prueba().await;
	let servicio = test::init_service(
		App::new()
			.app_data(web::Data::new(db))
			.wrap(crate::middleware::auth::ControlAcceso)
			.configure(config),
	)
	.await;

	let sin_sesion = test::call_service(&servicio, test::TestRequest::get().uri("/v1/resumen").to_request()).await;
	assert_eq!(sin_sesion.status(), StatusCode::UNAUTHORIZED);

	let respuesta = test::call_service(
		&servicio,
		admin(test::TestRequest::get().uri("/v1/resumen")).to_request(),
	)
	.await;
	assert!(respuesta.status().is_success());
	let cuerpo: serde_json::Value = test::read_body_json(respuesta).await;
	assert_eq!(cuerpo["alumnos"], 10);
	assert_eq!(cuerpo["inscripcion_anual_mxn"], 125);

	let alta = test::call_service(
		&servicio,
		admin(
			test::TestRequest::post()
				.uri("/v1/alumnos")
				.set_json(serde_json::json!({
						"nombre": "Lucía Méndez",
						"cartera_ref": "cartera_lucia"
				})),
		)
		.to_request(),
	)
	.await;
	assert_eq!(alta.status(), StatusCode::CREATED);
	let creado: serde_json::Value = test::read_body_json(alta).await;
	let id = creado["id"].as_i64().unwrap();

	let baja = test::call_service(
		&servicio,
		admin(test::TestRequest::delete().uri(&format!("/v1/alumnos/{id}"))).to_request(),
	)
	.await;
	assert_eq!(baja.status(), StatusCode::NO_CONTENT);

	let bloqueado = test::call_service(
		&servicio,
		admin(test::TestRequest::delete().uri("/v1/alumnos/1")).to_request(),
	)
	.await;
	assert_eq!(bloqueado.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn la_cuenta_muestra_solo_los_datos_del_tallerista() {
	let db = db::base_de_prueba().await;
	let servicio = test::init_service(
		App::new()
			.app_data(web::Data::new(db))
			.wrap(crate::middleware::auth::ControlAcceso)
			.configure(config),
	)
	.await;

	let cerrada = test::call_service(&servicio, test::TestRequest::get().uri("/cuenta").to_request()).await;
	let icono = test::call_service(&servicio, test::TestRequest::get().uri("/favicon.ico").to_request()).await;
	assert_eq!(icono.status(), StatusCode::NOT_FOUND);
	assert!(icono.headers().get(actix_web::http::header::WWW_AUTHENTICATE).is_none());

	assert_eq!(cerrada.status(), StatusCode::UNAUTHORIZED);
	assert!(
		cerrada
			.headers()
			.get(actix_web::http::header::WWW_AUTHENTICATE)
			.is_none()
	);

	let sesion_marichuy = abrir_sesion!(&servicio, "marichuy", "telar-2026");
	let sesion_pepe = abrir_sesion!(&servicio, "pepe", "color-2026");

	let ajena = test::call_service(
		&servicio,
		con_sesion(&sesion_marichuy, test::TestRequest::get().uri("/v1/alumnos")).to_request(),
	)
	.await;
	assert_eq!(ajena.status(), StatusCode::UNAUTHORIZED);

	let respuesta = test::call_service(
		&servicio,
		con_sesion(&sesion_marichuy, test::TestRequest::get().uri("/cuenta")).to_request(),
	)
	.await;
	assert!(respuesta.status().is_success());
	let cuerpo: serde_json::Value = test::read_body_json(respuesta).await;
	assert_eq!(cuerpo["tallerista"], "Marichuy");
	assert_eq!(cuerpo["usuario"], "marichuy");
	let talleres = cuerpo["talleres"].as_array().unwrap();
	assert_eq!(talleres.len(), 1);
	assert_eq!(talleres[0]["nombre"], "Tejido");
	let dias: Vec<&str> = talleres[0]["horarios"]
		.as_array()
		.unwrap()
		.iter()
		.map(|horario| horario["dia"].as_str().unwrap())
		.collect();
	assert_eq!(dias, ["martes", "jueves"]);
	let alumnos = talleres[0]["alumnos"].as_array().unwrap();
	let camila = alumnos
		.iter()
		.find(|alumno| alumno["nombre"] == "Camila Flores")
		.unwrap();
	assert_eq!(camila["mensualidad"], "pendiente");
	let ana = alumnos.iter().find(|alumno| alumno["nombre"] == "Ana López").unwrap();
	assert_eq!(ana["mensualidad"], "pagado");

	let pepe = test::call_service(
		&servicio,
		con_sesion(&sesion_pepe, test::TestRequest::get().uri("/cuenta")).to_request(),
	)
	.await;
	let cuerpo: serde_json::Value = test::read_body_json(pepe).await;
	let nombres: Vec<&str> = cuerpo["talleres"][0]["alumnos"]
		.as_array()
		.unwrap()
		.iter()
		.map(|alumno| alumno["nombre"].as_str().unwrap())
		.collect();
	assert!(nombres.contains(&"Luis Hernández"));
	assert!(!nombres.contains(&"Camila Flores"));
}

#[actix_web::test]
async fn un_tallerista_nuevo_crea_su_cuenta_y_la_completa() {
	let db = db::base_de_prueba().await;
	let servicio = test::init_service(
		App::new()
			.app_data(web::Data::new(db))
			.wrap(crate::middleware::auth::ControlAcceso)
			.configure(config),
	)
	.await;

	let corta = test::call_service(
		&servicio,
		test::TestRequest::get()
			.uri("/cuenta/crear?usuario=rosa&clave=123")
			.to_request(),
	)
	.await;
	assert_eq!(corta.status(), StatusCode::BAD_REQUEST);

	let alta = test::call_service(
		&servicio,
		test::TestRequest::get()
			.uri("/cuenta/crear?usuario=rosa&clave=ceramica-2026")
			.to_request(),
	)
	.await;
	assert_eq!(alta.status(), StatusCode::CREATED);
	assert!(alta.headers().get(actix_web::http::header::SET_COOKIE).is_some());
	let creada: serde_json::Value = test::read_body_json(alta).await;
	assert_eq!(creada["usuario"], "rosa");
	assert!(creada.get("clave").is_none());

	let repetida = test::call_service(
		&servicio,
		test::TestRequest::get()
			.uri("/cuenta/crear?usuario=rosa&clave=ceramica-2026")
			.to_request(),
	)
	.await;
	assert_eq!(repetida.status(), StatusCode::BAD_REQUEST);

	let mal = test::call_service(
		&servicio,
		test::TestRequest::get()
			.uri("/cuenta/entrar?usuario=rosa&clave=otra-clave")
			.to_request(),
	)
	.await;
	assert_eq!(mal.status(), StatusCode::BAD_REQUEST);

	let entrada = test::call_service(
		&servicio,
		test::TestRequest::get()
			.uri("/cuenta/entrar?usuario=rosa&clave=ceramica-2026")
			.to_request(),
	)
	.await;
	assert!(entrada.status().is_success());
	let sesion_rosa = galleta_de(&entrada);

	let vacia = test::call_service(
		&servicio,
		con_sesion(&sesion_rosa, test::TestRequest::get().uri("/cuenta")).to_request(),
	)
	.await;
	let cuerpo: serde_json::Value = test::read_body_json(vacia).await;
	assert_eq!(cuerpo["listo"], false);
	assert!(cuerpo["oficio"].as_str().unwrap().is_empty());

	let datos = test::call_service(
		&servicio,
		con_sesion(
			&sesion_rosa,
			test::TestRequest::get().uri("/cuenta/datos?nombre=Rosa&oficio=ceramica"),
		)
		.to_request(),
	)
	.await;
	assert!(datos.status().is_success());

	let taller = test::call_service(
		&servicio,
		con_sesion(
			&sesion_rosa,
			test::TestRequest::get().uri("/cuenta/taller?nombre=Ceramica&cuota_mensual_mxn=30"),
		)
		.to_request(),
	)
	.await;
	assert!(taller.status().is_success());

	let horario = test::call_service(
		&servicio,
		con_sesion(
			&sesion_rosa,
			test::TestRequest::get().uri("/cuenta/horario?dia=martes&de=16:00&a=18:00"),
		)
		.to_request(),
	)
	.await;
	assert!(horario.status().is_success());

	let alumno = test::call_service(
		&servicio,
		con_sesion(
			&sesion_rosa,
			test::TestRequest::get().uri("/cuenta/alumno?nombre=Nila%20Prueba"),
		)
		.to_request(),
	)
	.await;
	assert!(alumno.status().is_success());
	let cuerpo: serde_json::Value = test::read_body_json(alumno).await;
	assert_eq!(cuerpo["listo"], true);
	assert_eq!(cuerpo["tallerista"], "Rosa");
	assert_eq!(cuerpo["oficio"], "ceramica");
	assert_eq!(cuerpo["talleres"][0]["nombre"], "Ceramica");
	assert_eq!(cuerpo["talleres"][0]["horarios"][0]["dia"], "martes");
	assert_eq!(cuerpo["talleres"][0]["alumnos"][0]["nombre"], "Nila Prueba");
	assert_eq!(cuerpo["talleres"][0]["alumnos"][0]["mensualidad"], "pendiente");

	let pago = test::call_service(
		&servicio,
		con_sesion(
			&sesion_rosa,
			test::TestRequest::get().uri("/cuenta/pago?alumno=Nila%20Prueba&estado=pagado"),
		)
		.to_request(),
	)
	.await;
	let cuerpo: serde_json::Value = test::read_body_json(pago).await;
	assert_eq!(cuerpo["talleres"][0]["alumnos"][0]["mensualidad"], "pagado");

	let sesion_marichuy = abrir_sesion!(&servicio, "marichuy", "telar-2026");
	let ajena = test::call_service(
		&servicio,
		con_sesion(&sesion_marichuy, test::TestRequest::get().uri("/cuenta")).to_request(),
	)
	.await;
	let cuerpo: serde_json::Value = test::read_body_json(ajena).await;
	assert_eq!(cuerpo["talleres"][0]["nombre"], "Tejido");
}

fn admin(peticion: actix_web::test::TestRequest) -> actix_web::test::TestRequest {
	tallerista(db::ADMIN_USUARIO, db::ADMIN_CLAVE_PRUEBAS, peticion)
}

fn tallerista(usuario: &str, clave: &str, peticion: actix_web::test::TestRequest) -> actix_web::test::TestRequest {
	peticion.insert_header((
		actix_web::http::header::AUTHORIZATION,
		crate::middleware::auth::encabezado_basic(usuario, clave),
	))
}

fn con_sesion(sesion: &str, peticion: actix_web::test::TestRequest) -> actix_web::test::TestRequest {
	peticion.insert_header((actix_web::http::header::COOKIE, sesion))
}

fn galleta_de(respuesta: &actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>) -> String {
	respuesta
		.headers()
		.get(actix_web::http::header::SET_COOKIE)
		.unwrap()
		.to_str()
		.unwrap()
		.split(';')
		.next()
		.unwrap()
		.to_string()
}
