//! Conexión a Postgres, migraciones y datos iniciales.
//!
//! El esquema vive en `migrations/*.sql` (se aplica al arrancar, incluido en el binario al compilar)
//! y los datos de ejemplo en `seeds/*.sql`. Aquí solo queda lo que necesita código: las contraseñas,
//! que se guardan con argon2.

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, DbErr, Statement};

/// Datos de ejemplo de Amanda: sus cuatro talleres con alumnos y pagos.
const SEED_EJEMPLO: &str = include_str!("../../seeds/0001_ejemplo_amanda.sql");

/// Datos de la demo de la app: María González y su Taller de tejido.
const SEED_DEMO: &str = include_str!("../../seeds/0002_demo_app.sql");

/// Tallerista de la demo en `seeds/0002_demo_app.sql`.
const DEMO_PEY_API_KEY_ID: &str = "demo_pey_maria_gonzalez";

/// Qué sembrar al arrancar, según las variables de entorno.
#[derive(Clone, Debug, Default)]
pub struct Siembra {
	/// Corre `seeds/0001_ejemplo_amanda.sql` (`SEED=1`), y `seeds/0002_demo_app.sql` si hay `demo`.
	pub datos: bool,
	/// Contraseña de la cuenta `admin` (`ADMIN_CLAVE`); sin ella no se crea la cuenta.
	pub admin_clave: Option<String>,
	/// Cuenta de la tallerista de la demo como (usuario, clave) (`DEMO_USUARIO`, `DEMO_CLAVE`).
	pub demo: Option<(String, String)>,
}

/// Abre la base y aplica las migraciones pendientes.
pub async fn conectar(url: &str) -> Result<DatabaseConnection, DbErr> {
	let db = Database::connect(url).await?;
	sqlx::migrate!("./migrations")
		.run(db.get_postgres_connection_pool())
		.await
		.map_err(|error| DbErr::Custom(format!("migraciones: {error}")))?;
	Ok(db)
}

/// Siembra los datos y las cuentas que pide `siembra`. Se puede correr en cada arranque.
pub async fn sembrar(db: &DatabaseConnection, siembra: &Siembra) -> Result<(), DbErr> {
	if siembra.datos {
		correr_sql(db, SEED_EJEMPLO).await?;
		if let Some((usuario, clave)) = &siembra.demo {
			correr_sql(db, SEED_DEMO).await?;
			asegurar_demo(db, usuario, clave).await?;
			crate::services::cobros::aprobacion::approve_by_pey_key(db, DEMO_PEY_API_KEY_ID).await?;
		}
	}
	asegurar_accesos(db, siembra.admin_clave.as_deref()).await
}

async fn correr_sql(db: &DatabaseConnection, sql: &'static str) -> Result<(), DbErr> {
	sqlx::raw_sql(sql)
		.execute(db.get_postgres_connection_pool())
		.await
		.map(|_| ())
		.map_err(|error| DbErr::Custom(format!("datos iniciales: {error}")))
}

/// Da usuario y contraseña a la tallerista de la demo, solo la primera vez.
async fn asegurar_demo(db: &DatabaseConnection, usuario: &str, clave: &str) -> Result<(), DbErr> {
	let usuario = crate::services::usuario_valido(usuario)?;
	let hash = crate::middleware::auth::hash_clave(clave.trim())?;
	db.execute_raw(Statement::from_sql_and_values(
		DbBackend::Postgres,
		"UPDATE talleristas SET usuario = $1, clave_hash = $2 WHERE pey_api_key_id = $3 AND clave_hash IS NULL",
		[usuario.into(), hash.into(), DEMO_PEY_API_KEY_ID.into()],
	))
	.await?;
	Ok(())
}

/// Base nueva para una prueba: migrada y con los datos de ejemplo, en el servidor de
/// `TEST_DATABASE_URL` (por omisión, el Postgres de `docker-compose.yml`).
#[doc(hidden)]
pub async fn base_de_prueba() -> DatabaseConnection {
	let admin_url = std::env::var("TEST_DATABASE_URL")
		.unwrap_or_else(|_| "postgres://cuentas:cuentas@localhost:5432/cuentas".to_string());
	let url = base_temporal(&admin_url).await.expect("base temporal");
	let db = conectar(&url).await.expect("migraciones");
	sembrar(
		&db,
		&Siembra {
			datos: true,
			admin_clave: Some(ADMIN_CLAVE_PRUEBAS.to_string()),
			demo: None,
		},
	)
	.await
	.expect("datos de ejemplo");
	db
}

/// Crea una base vacía en el servidor de `admin_url` y devuelve su URL. Solo para pruebas.
#[doc(hidden)]
pub async fn base_temporal(admin_url: &str) -> Result<String, DbErr> {
	static SIGUIENTE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
	let nombre = format!(
		"prueba_{}_{}_{}",
		std::process::id(),
		SIGUIENTE.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|duracion| duracion.as_nanos())
			.unwrap_or(0)
	);
	let admin = Database::connect(admin_url).await?;
	admin.execute_unprepared(&format!("CREATE DATABASE {nombre}")).await?;
	admin.close().await?;
	let (base, _) = admin_url
		.rsplit_once('/')
		.ok_or_else(|| DbErr::Custom("DATABASE_URL sin base".into()))?;
	Ok(format!("{base}/{nombre}"))
}

pub const ADMIN_USUARIO: &str = "admin";

/// Contraseña de `admin` en las pruebas; en producción llega por `ADMIN_CLAVE`.
#[doc(hidden)]
pub const ADMIN_CLAVE_PRUEBAS: &str = "cuentas-2026";

pub(crate) struct AccesoInicial {
	id: i32,
	usuario: &'static str,
	clave: &'static str,
}

pub(crate) const ACCESOS: &[AccesoInicial] = &[
	AccesoInicial {
		id: 1,
		usuario: "marichuy",
		clave: "telar-2026",
	},
	AccesoInicial {
		id: 2,
		usuario: "pepe",
		clave: "color-2026",
	},
	AccesoInicial {
		id: 3,
		usuario: "maria",
		clave: "suma-2026",
	},
	AccesoInicial {
		id: 4,
		usuario: "juan",
		clave: "meta-2026",
	},
];

pub(crate) async fn asegurar_accesos(db: &DatabaseConnection, admin_clave: Option<&str>) -> Result<(), DbErr> {
	for acceso in ACCESOS {
		let pendiente = db
			.query_one_raw(Statement::from_sql_and_values(
				DbBackend::Postgres,
				"SELECT id FROM talleristas WHERE id = $1 AND clave_hash IS NULL",
				[acceso.id.into()],
			))
			.await?;
		if pendiente.is_none() {
			continue;
		}
		let hash = crate::middleware::auth::hash_clave(acceso.clave)?;
		db.execute_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"UPDATE talleristas SET usuario = $1, clave_hash = $2 WHERE id = $3 AND clave_hash IS NULL",
			[acceso.usuario.into(), hash.into(), acceso.id.into()],
		))
		.await?;
	}

	let Some(admin_clave) = admin_clave else {
		return Ok(());
	};
	let admin = db
		.query_one_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"SELECT id FROM administradores WHERE usuario = $1",
			[ADMIN_USUARIO.into()],
		))
		.await?;
	if admin.is_none() {
		let hash = crate::middleware::auth::hash_clave(admin_clave)?;
		db.execute_raw(Statement::from_sql_and_values(
			DbBackend::Postgres,
			"INSERT INTO administradores (usuario, clave_hash) VALUES ($1, $2)",
			[ADMIN_USUARIO.into(), hash.into()],
		))
		.await?;
	}
	Ok(())
}
