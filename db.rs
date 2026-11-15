use std::str::FromStr;

use sea_orm::sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, SqlxSqliteConnector, Statement,
    TransactionTrait,
};

use crate::entity;

pub const INSCRIPCION_ANUAL_MXN: i32 = 125;
pub const MENSUALIDAD_MXN: i32 = 25;

const SCHEMA: &str = "
PRAGMA foreign_keys = ON;

DROP TABLE IF EXISTS cuentas;

CREATE TABLE IF NOT EXISTS talleristas (
    id INTEGER PRIMARY KEY,
    nombre TEXT NOT NULL,
    oficio TEXT NOT NULL,
    pey_api_key_id TEXT NOT NULL UNIQUE,
    usuario TEXT UNIQUE,
    telefono TEXT UNIQUE,
    clave_hash TEXT
);

CREATE TABLE IF NOT EXISTS codigos_acceso (
    id INTEGER PRIMARY KEY,
    tallerista_id INTEGER NOT NULL REFERENCES talleristas(id),
    codigo_hash TEXT NOT NULL,
    sal TEXT NOT NULL,
    creado_en TEXT NOT NULL,
    vence_en TEXT NOT NULL,
    usado INTEGER NOT NULL DEFAULT 0 CHECK (usado IN (0, 1)),
    intentos INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS sesiones (
    id INTEGER PRIMARY KEY,
    tallerista_id INTEGER NOT NULL REFERENCES talleristas(id),
    token_hash TEXT NOT NULL UNIQUE,
    creada_en TEXT NOT NULL,
    vence_en TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS administradores (
    id INTEGER PRIMARY KEY,
    usuario TEXT NOT NULL UNIQUE,
    clave_hash TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS talleres (
    id INTEGER PRIMARY KEY,
    tallerista_id INTEGER NOT NULL REFERENCES talleristas(id),
    nombre TEXT NOT NULL,
    cuota_mensual_mxn INTEGER NOT NULL CHECK (cuota_mensual_mxn > 0),
    activo INTEGER NOT NULL DEFAULT 1 CHECK (activo IN (0, 1)),
    UNIQUE (tallerista_id, nombre)
);

CREATE TABLE IF NOT EXISTS horarios (
    id INTEGER PRIMARY KEY,
    taller_id INTEGER NOT NULL REFERENCES talleres(id),
    dia TEXT NOT NULL CHECK (
        dia IN ('lunes', 'martes', 'miercoles', 'jueves', 'viernes', 'sabado', 'domingo')
    ),
    hora_inicio TEXT NOT NULL,
    hora_fin TEXT NOT NULL,
    UNIQUE (taller_id, dia, hora_inicio)
);

CREATE TABLE IF NOT EXISTS cambios_cuota (
    id INTEGER PRIMARY KEY,
    taller_id INTEGER NOT NULL REFERENCES talleres(id),
    cuota_anterior_mxn INTEGER,
    cuota_nueva_mxn INTEGER NOT NULL CHECK (cuota_nueva_mxn > 0),
    vigente_desde TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS alumnos (
    id INTEGER PRIMARY KEY,
    nombre TEXT NOT NULL,
    cartera_ref TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS dispositivos_nfc (
    id INTEGER PRIMARY KEY,
    tallerista_id INTEGER NOT NULL REFERENCES talleristas(id),
    cara TEXT NOT NULL CHECK (cara IN ('cobro', 'inscripcion')),
    chip_uid TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS inscripciones_anuales (
    id INTEGER PRIMARY KEY,
    alumno_id INTEGER NOT NULL REFERENCES alumnos(id),
    anio INTEGER NOT NULL,
    monto_mxn INTEGER NOT NULL CHECK (monto_mxn > 0),
    UNIQUE (alumno_id, anio)
);

CREATE TABLE IF NOT EXISTS inscripciones_taller (
    id INTEGER PRIMARY KEY,
    alumno_id INTEGER NOT NULL REFERENCES alumnos(id),
    taller_id INTEGER NOT NULL REFERENCES talleres(id),
    anio INTEGER NOT NULL,
    activa INTEGER NOT NULL DEFAULT 1 CHECK (activa IN (0, 1)),
    UNIQUE (alumno_id, taller_id, anio)
);

CREATE TABLE IF NOT EXISTS pagos (
    id INTEGER PRIMARY KEY,
    alumno_id INTEGER NOT NULL REFERENCES alumnos(id),
    taller_id INTEGER REFERENCES talleres(id),
    tallerista_id INTEGER REFERENCES talleristas(id),
    tipo TEXT NOT NULL CHECK (tipo IN ('inscripcion_anual', 'mensualidad')),
    canal TEXT NOT NULL CHECK (canal IN ('boton', 'nfc', 'panel')),
    concepto TEXT NOT NULL,
    monto_mxn INTEGER NOT NULL CHECK (monto_mxn > 0),
    num_ref TEXT NOT NULL UNIQUE,
    codi_qr_id TEXT UNIQUE,
    transaction_id TEXT UNIQUE,
    tipo_evento TEXT CHECK (
        tipo_evento IN ('codi.qr.payment_received', 'codi.push.payment_received')
    ),
    estado TEXT NOT NULL CHECK (estado IN ('pendiente', 'pagado', 'fallido')),
    periodo TEXT NOT NULL,
    pagado_en TEXT,
    CHECK (
        (tipo = 'inscripcion_anual' AND taller_id IS NULL AND tallerista_id IS NULL)
        OR (tipo = 'mensualidad' AND taller_id IS NOT NULL AND tallerista_id IS NOT NULL)
    ),
    CHECK (
        (estado = 'pagado' AND transaction_id IS NOT NULL AND pagado_en IS NOT NULL)
        OR (estado <> 'pagado')
    )
);

CREATE INDEX IF NOT EXISTS idx_inscripciones_taller_taller
    ON inscripciones_taller (taller_id);
CREATE INDEX IF NOT EXISTS idx_pagos_tallerista
    ON pagos (tallerista_id, estado);
CREATE INDEX IF NOT EXISTS idx_pagos_alumno
    ON pagos (alumno_id);

DROP VIEW IF EXISTS ingresos_tallerista;
CREATE VIEW ingresos_tallerista AS
SELECT
    ta.id AS tallerista_id,
    ta.nombre,
    ta.oficio,
    COALESCE(SUM(p.monto_mxn), 0) AS total_cobrado_mxn
FROM talleristas ta
LEFT JOIN pagos p
    ON p.tallerista_id = ta.id
   AND p.estado = 'pagado'
GROUP BY ta.id;
";

const SEED: &str = "
INSERT INTO talleristas (id, nombre, oficio, pey_api_key_id) VALUES
    (1, 'Marichuy', 'tejido', 'ejemplo_pey_marichuy'),
    (2, 'Pepe', 'artes visuales', 'ejemplo_pey_pepe'),
    (3, 'María', 'matemáticas', 'ejemplo_pey_maria'),
    (4, 'Juan', 'deportes', 'ejemplo_pey_juan');

INSERT INTO talleres (id, tallerista_id, nombre, cuota_mensual_mxn) VALUES
    (1, 1, 'Tejido', 25),
    (2, 2, 'Artes visuales', 25),
    (3, 3, 'Matemáticas', 25),
    (4, 4, 'Deportes', 25);

INSERT INTO cambios_cuota (taller_id, cuota_anterior_mxn, cuota_nueva_mxn, vigente_desde) VALUES
    (1, NULL, 25, '2026-01-01'),
    (2, NULL, 25, '2026-01-01'),
    (3, NULL, 25, '2026-01-01'),
    (4, NULL, 25, '2026-01-01');

INSERT INTO alumnos (id, nombre, cartera_ref) VALUES
    (1, 'Ana López', 'cartera_ana_lopez'),
    (2, 'Luis Hernández', 'cartera_luis_hernandez'),
    (3, 'Sofía Ramírez', 'cartera_sofia_ramirez'),
    (4, 'Diego Torres', 'cartera_diego_torres'),
    (5, 'Valeria Gómez', 'cartera_valeria_gomez'),
    (6, 'Mateo Cruz', 'cartera_mateo_cruz'),
    (7, 'Camila Flores', 'cartera_camila_flores'),
    (8, 'Javier Morales', 'cartera_javier_morales'),
    (9, 'Regina Ortiz', 'cartera_regina_ortiz'),
    (10, 'Emiliano Vargas', 'cartera_emiliano_vargas');

INSERT INTO dispositivos_nfc (tallerista_id, cara, chip_uid) VALUES
    (1, 'cobro', 'nfc-marichuy-cobro'),
    (1, 'inscripcion', 'nfc-marichuy-inscripcion'),
    (4, 'cobro', 'nfc-juan-cobro'),
    (4, 'inscripcion', 'nfc-juan-inscripcion');

INSERT INTO inscripciones_anuales (alumno_id, anio, monto_mxn) VALUES
    (1, 2026, 125),
    (2, 2026, 125),
    (3, 2026, 125),
    (4, 2026, 125),
    (5, 2026, 125),
    (6, 2026, 125),
    (7, 2026, 125),
    (8, 2026, 125),
    (9, 2026, 125),
    (10, 2026, 125);

INSERT INTO inscripciones_taller (alumno_id, taller_id, anio) VALUES
    (1, 1, 2026),
    (1, 3, 2026),
    (2, 2, 2026),
    (4, 1, 2026),
    (5, 3, 2026),
    (5, 4, 2026),
    (6, 2, 2026),
    (7, 1, 2026),
    (8, 4, 2026),
    (9, 3, 2026),
    (10, 2, 2026),
    (10, 1, 2026),
    (3, 4, 2026);

INSERT INTO pagos (
    alumno_id, taller_id, tallerista_id, tipo, canal, concepto, monto_mxn,
    num_ref, codi_qr_id, transaction_id, tipo_evento, estado, periodo, pagado_en
) VALUES
    (1, NULL, NULL, 'inscripcion_anual', 'nfc', 'Inscripción anual 2026', 125,
        'ANUAL-2026-01', 'qr-anual-01', 'txn-anual-01', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-08 09:12:00'),
    (2, NULL, NULL, 'inscripcion_anual', 'boton', 'Inscripción anual 2026', 125,
        'ANUAL-2026-02', 'qr-anual-02', 'txn-anual-02', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-09 11:03:00'),
    (3, NULL, NULL, 'inscripcion_anual', 'nfc', 'Inscripción anual 2026', 125,
        'ANUAL-2026-03', 'qr-anual-03', 'txn-anual-03', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-10 16:40:00'),
    (4, NULL, NULL, 'inscripcion_anual', 'nfc', 'Inscripción anual 2026', 125,
        'ANUAL-2026-04', 'qr-anual-04', 'txn-anual-04', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-11 10:18:00'),
    (5, NULL, NULL, 'inscripcion_anual', 'boton', 'Inscripción anual 2026', 125,
        'ANUAL-2026-05', 'qr-anual-05', 'txn-anual-05', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-12 13:22:00'),
    (6, NULL, NULL, 'inscripcion_anual', 'panel', 'Inscripción anual 2026', 125,
        'ANUAL-2026-06', 'qr-anual-06', 'txn-anual-06', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-13 18:05:00'),
    (7, NULL, NULL, 'inscripcion_anual', 'nfc', 'Inscripción anual 2026', 125,
        'ANUAL-2026-07', 'qr-anual-07', 'txn-anual-07', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-14 09:47:00'),
    (8, NULL, NULL, 'inscripcion_anual', 'nfc', 'Inscripción anual 2026', 125,
        'ANUAL-2026-08', 'qr-anual-08', 'txn-anual-08', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-15 15:11:00'),
    (9, NULL, NULL, 'inscripcion_anual', 'panel', 'Inscripción anual 2026', 125,
        'ANUAL-2026-09', 'qr-anual-09', 'txn-anual-09', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-16 12:36:00'),
    (10, NULL, NULL, 'inscripcion_anual', 'boton', 'Inscripción anual 2026', 125,
        'ANUAL-2026-10', 'qr-anual-10', 'txn-anual-10', 'codi.qr.payment_received', 'pagado', '2026', '2026-01-17 17:28:00'),
    (1, 1, 1, 'mensualidad', 'nfc', 'Mensualidad Tejido 2026-11', 25,
        'MENS-2026-11-01', 'qr-mens-01', 'txn-mens-01', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-02 08:10:00'),
    (1, 3, 3, 'mensualidad', 'panel', 'Mensualidad Matemáticas 2026-11', 25,
        'MENS-2026-11-02', 'qr-mens-02', 'txn-mens-02', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-02 08:16:00'),
    (2, 2, 2, 'mensualidad', 'panel', 'Mensualidad Artes visuales 2026-11', 25,
        'MENS-2026-11-03', 'qr-mens-03', 'txn-mens-03', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-03 19:02:00'),
    (3, 4, 4, 'mensualidad', 'nfc', 'Mensualidad Deportes 2026-11', 25,
        'MENS-2026-11-04', 'qr-mens-04', 'txn-mens-04', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-03 07:45:00'),
    (4, 1, 1, 'mensualidad', 'nfc', 'Mensualidad Tejido 2026-11', 25,
        'MENS-2026-11-05', 'qr-mens-05', 'txn-mens-05', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-04 18:20:00'),
    (5, 3, 3, 'mensualidad', 'panel', 'Mensualidad Matemáticas 2026-11', 25,
        'MENS-2026-11-06', 'qr-mens-06', 'txn-mens-06', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-04 20:11:00'),
    (5, 4, 4, 'mensualidad', 'nfc', 'Mensualidad Deportes 2026-11', 25,
        'MENS-2026-11-07', 'qr-mens-07', 'txn-mens-07', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-05 06:55:00'),
    (6, 2, 2, 'mensualidad', 'panel', 'Mensualidad Artes visuales 2026-11', 25,
        'MENS-2026-11-08', 'qr-mens-08', 'txn-mens-08', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-05 21:14:00'),
    (7, 1, 1, 'mensualidad', 'nfc', 'Mensualidad Tejido 2026-11', 25,
        'MENS-2026-11-09', 'qr-mens-09', NULL, NULL, 'pendiente', '2026-11', NULL),
    (8, 4, 4, 'mensualidad', 'nfc', 'Mensualidad Deportes 2026-11', 25,
        'MENS-2026-11-10', 'qr-mens-10', NULL, NULL, 'pendiente', '2026-11', NULL),
    (9, 3, 3, 'mensualidad', 'panel', 'Mensualidad Matemáticas 2026-11', 25,
        'MENS-2026-11-11', 'qr-mens-11', 'txn-mens-11', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-06 11:09:00'),
    (10, 2, 2, 'mensualidad', 'panel', 'Mensualidad Artes visuales 2026-11', 25,
        'MENS-2026-11-12', 'qr-mens-12', 'txn-mens-12', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-06 17:33:00'),
    (10, 1, 1, 'mensualidad', 'nfc', 'Mensualidad Tejido 2026-11', 25,
        'MENS-2026-11-13', 'qr-mens-13', 'txn-mens-13', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-07 09:27:00');
";

pub async fn conectar(url: &str) -> Result<DatabaseConnection, DbErr> {
    let options = SqliteConnectOptions::from_str(url)
        .map_err(|error| DbErr::Custom(error.to_string()))?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .map_err(|error| DbErr::Custom(error.to_string()))?;
    let db = SqlxSqliteConnector::from_sqlx_sqlite_pool(pool);
    preparar(&db).await?;
    Ok(db)
}

pub const ADMIN_USUARIO: &str = "admin";
pub const ADMIN_CLAVE: &str = "cuentas-2026";

struct AccesoInicial {
    id: i32,
    usuario: &'static str,
    clave: &'static str,
}

const ACCESOS: &[AccesoInicial] = &[
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

const HORARIOS: &str = "
INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin)
SELECT 1, 'martes', '16:00', '18:00'
WHERE NOT EXISTS (
    SELECT 1 FROM horarios WHERE taller_id = 1 AND dia = 'martes' AND hora_inicio = '16:00'
);
INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin)
SELECT 1, 'jueves', '16:00', '18:00'
WHERE NOT EXISTS (
    SELECT 1 FROM horarios WHERE taller_id = 1 AND dia = 'jueves' AND hora_inicio = '16:00'
);
INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin)
SELECT 2, 'lunes', '17:00', '19:00'
WHERE NOT EXISTS (
    SELECT 1 FROM horarios WHERE taller_id = 2 AND dia = 'lunes' AND hora_inicio = '17:00'
);
INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin)
SELECT 2, 'miercoles', '17:00', '19:00'
WHERE NOT EXISTS (
    SELECT 1 FROM horarios WHERE taller_id = 2 AND dia = 'miercoles' AND hora_inicio = '17:00'
);
INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin)
SELECT 3, 'lunes', '10:00', '11:30'
WHERE NOT EXISTS (
    SELECT 1 FROM horarios WHERE taller_id = 3 AND dia = 'lunes' AND hora_inicio = '10:00'
);
INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin)
SELECT 3, 'miercoles', '10:00', '11:30'
WHERE NOT EXISTS (
    SELECT 1 FROM horarios WHERE taller_id = 3 AND dia = 'miercoles' AND hora_inicio = '10:00'
);
INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin)
SELECT 3, 'viernes', '10:00', '11:30'
WHERE NOT EXISTS (
    SELECT 1 FROM horarios WHERE taller_id = 3 AND dia = 'viernes' AND hora_inicio = '10:00'
);
INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin)
SELECT 4, 'sabado', '09:00', '11:00'
WHERE NOT EXISTS (
    SELECT 1 FROM horarios WHERE taller_id = 4 AND dia = 'sabado' AND hora_inicio = '09:00'
);
";

async fn preparar(db: &DatabaseConnection) -> Result<(), DbErr> {
    ejecutar_lote(db, SCHEMA).await?;
    if !tiene_columna(db, "talleristas", "usuario").await? {
        db.execute_unprepared("ALTER TABLE talleristas ADD COLUMN usuario TEXT")
            .await?;
    }
    if !tiene_columna(db, "talleristas", "clave_hash").await? {
        db.execute_unprepared("ALTER TABLE talleristas ADD COLUMN clave_hash TEXT")
            .await?;
    }
    if !tiene_columna(db, "talleristas", "telefono").await? {
        db.execute_unprepared("ALTER TABLE talleristas ADD COLUMN telefono TEXT")
            .await?;
    }
    db.execute_unprepared(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_talleristas_usuario ON talleristas(usuario)",
    )
    .await?;
    db.execute_unprepared(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_talleristas_telefono ON talleristas(telefono)",
    )
    .await?;
    use sea_orm::{EntityTrait, PaginatorTrait};
    let existentes = entity::tallerista::Entity::find().count(db).await?;
    if existentes == 0 {
        ejecutar_lote(db, SEED).await?;
    }
    asegurar_accesos(db).await?;
    ejecutar_lote(db, HORARIOS).await?;
    Ok(())
}

async fn tiene_columna(db: &DatabaseConnection, tabla: &str, columna: &str) -> Result<bool, DbErr> {
    let filas = db
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            format!("PRAGMA table_info({tabla})"),
        ))
        .await?;
    Ok(filas.into_iter().any(|fila| {
        fila.try_get::<String>("", "name")
            .ok()
            .as_deref()
            == Some(columna)
    }))
}

async fn asegurar_accesos(db: &DatabaseConnection) -> Result<(), DbErr> {
    for acceso in ACCESOS {
        let pendiente = db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT id FROM talleristas WHERE id = ? AND clave_hash IS NULL",
                [acceso.id.into()],
            ))
            .await?;
        if pendiente.is_none() {
            continue;
        }
        let hash = crate::auth::hash_clave(acceso.clave)?;
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "UPDATE talleristas SET usuario = ?, clave_hash = ? WHERE id = ? AND clave_hash IS NULL",
            [acceso.usuario.into(), hash.into(), acceso.id.into()],
        ))
        .await?;
    }

    let admin = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id FROM administradores WHERE usuario = ?",
            [ADMIN_USUARIO.into()],
        ))
        .await?;
    if admin.is_none() {
        let hash = crate::auth::hash_clave(ADMIN_CLAVE)?;
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO administradores (usuario, clave_hash) VALUES (?, ?)",
            [ADMIN_USUARIO.into(), hash.into()],
        ))
        .await?;
    }
    Ok(())
}

async fn ejecutar_lote(db: &DatabaseConnection, sql: &str) -> Result<(), DbErr> {
    for sentencia in sql.split(';') {
        let sentencia = sentencia.trim();
        if sentencia.is_empty() {
            continue;
        }
        db.execute_unprepared(sentencia).await?;
    }
    Ok(())
}

pub async fn ahora<C>(db: &C) -> Result<String, DbErr>
where
    C: ConnectionTrait,
{
    let fila = db
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT datetime('now') AS ahora".to_owned(),
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("no se pudo leer la fecha".to_owned()))?;
    fila.try_get("", "ahora")
}

pub async fn anio_actual(db: &DatabaseConnection) -> Result<i32, DbErr> {
    let fila = db
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT CAST(strftime('%Y', 'now') AS INTEGER) AS anio".to_owned(),
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("no se pudo leer el año".to_owned()))?;
    fila.try_get("", "anio")
}

#[derive(Debug, serde::Serialize)]
pub struct Ingreso {
    pub nombre: String,
    pub oficio: String,
    pub total_cobrado_mxn: i32,
}

pub async fn ingresos(db: &DatabaseConnection) -> Result<Vec<Ingreso>, DbErr> {
    let filas = db
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT nombre, oficio, total_cobrado_mxn
             FROM ingresos_tallerista
             ORDER BY tallerista_id"
                .to_owned(),
        ))
        .await?;
    filas
        .into_iter()
        .map(|fila| {
            Ok(Ingreso {
                nombre: fila.try_get("", "nombre")?,
                oficio: fila.try_get("", "oficio")?,
                total_cobrado_mxn: fila.try_get("", "total_cobrado_mxn")?,
            })
        })
        .collect()
}

#[derive(Clone, Debug)]
pub struct Sesion {
    pub id: i32,
    pub nombre: String,
    pub usuario: String,
}

pub async fn clave_admin_valida(
    db: &DatabaseConnection,
    usuario: &str,
    clave: &str,
) -> Result<bool, DbErr> {
    let clave = clave.trim();
    let fila = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT clave_hash FROM administradores WHERE lower(usuario) = lower(?)",
            [usuario.trim().into()],
        ))
        .await?;
    let Some(fila) = fila else {
        let _ = crate::auth::clave_correcta(crate::auth::hash_falso(), clave);
        return Ok(false);
    };
    let hash: String = fila.try_get("", "clave_hash")?;
    Ok(crate::auth::clave_correcta(&hash, clave))
}

#[derive(Debug, serde::Serialize)]
pub struct Cuenta {
    pub tallerista: String,
    pub usuario: String,
    pub telefono: String,
    pub oficio: String,
    pub listo: bool,
    pub falta: Vec<String>,
    pub siguientes: Siguientes,
    pub talleres: Vec<TallerCuenta>,
}

#[derive(Debug, serde::Serialize)]
pub struct Siguientes {
    pub datos: String,
    pub taller: String,
    pub horario: String,
    pub alumno: String,
    pub pago: String,
}

#[derive(Debug, serde::Serialize)]
pub struct TallerCuenta {
    pub nombre: String,
    pub cuota_mensual_mxn: i32,
    pub horarios: Vec<HorarioCuenta>,
    pub alumnos: Vec<AlumnoCuenta>,
}

#[derive(Debug, serde::Serialize)]
pub struct HorarioCuenta {
    pub dia: String,
    pub de: String,
    pub a: String,
}

#[derive(Debug, serde::Serialize)]
pub struct AlumnoCuenta {
    pub nombre: String,
    pub mensualidad: String,
    pub periodo: String,
}

pub async fn cuenta(db: &DatabaseConnection, sesion: &Sesion) -> Result<Cuenta, DbErr> {
    let fila = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT nombre, usuario, oficio, telefono FROM talleristas WHERE id = ?",
            [sesion.id.into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("el tallerista no existe".to_owned()))?;
    let nombre: String = fila.try_get("", "nombre")?;
    let usuario: String = fila.try_get("", "usuario")?;
    let oficio: String = fila.try_get("", "oficio")?;
    let telefono: Option<String> = fila.try_get("", "telefono")?;

    let talleres = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, nombre, cuota_mensual_mxn
             FROM talleres
             WHERE tallerista_id = ? AND activo = 1
             ORDER BY id",
            [sesion.id.into()],
        ))
        .await?;
    let horarios = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT h.taller_id, h.dia, h.hora_inicio, h.hora_fin
             FROM horarios h
             JOIN talleres t ON t.id = h.taller_id
             WHERE t.tallerista_id = ?
             ORDER BY h.taller_id,
                 CASE h.dia
                     WHEN 'lunes' THEN 1
                     WHEN 'martes' THEN 2
                     WHEN 'miercoles' THEN 3
                     WHEN 'jueves' THEN 4
                     WHEN 'viernes' THEN 5
                     WHEN 'sabado' THEN 6
                     ELSE 7
                 END,
                 h.hora_inicio",
            [sesion.id.into()],
        ))
        .await?;
    let alumnos = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT
                 i.taller_id,
                 a.nombre,
                 COALESCE(
                     (
                         SELECT p.estado
                         FROM pagos p
                         WHERE p.alumno_id = a.id
                           AND p.taller_id = i.taller_id
                           AND p.tipo = 'mensualidad'
                           AND p.periodo = strftime('%Y-%m', 'now')
                         ORDER BY p.id DESC
                         LIMIT 1
                     ),
                     'pendiente'
                 ) AS mensualidad,
                 strftime('%Y-%m', 'now') AS periodo
             FROM inscripciones_taller i
             JOIN alumnos a ON a.id = i.alumno_id
             JOIN talleres t ON t.id = i.taller_id
             WHERE t.tallerista_id = ?
               AND i.activa = 1
               AND i.anio = CAST(strftime('%Y', 'now') AS INTEGER)
             ORDER BY i.taller_id, a.nombre",
            [sesion.id.into()],
        ))
        .await?;

    let mut resultado = Vec::new();
    for taller in talleres {
        let taller_id: i32 = taller.try_get("", "id")?;
        let mut horarios_taller = Vec::new();
        for horario in &horarios {
            let id: i32 = horario.try_get("", "taller_id")?;
            if id != taller_id {
                continue;
            }
            let dia: String = horario.try_get("", "dia")?;
            horarios_taller.push(HorarioCuenta {
                dia: dia_visible(&dia).to_owned(),
                de: horario.try_get("", "hora_inicio")?,
                a: horario.try_get("", "hora_fin")?,
            });
        }
        let mut alumnos_taller = Vec::new();
        for alumno in &alumnos {
            let id: i32 = alumno.try_get("", "taller_id")?;
            if id != taller_id {
                continue;
            }
            alumnos_taller.push(AlumnoCuenta {
                nombre: alumno.try_get("", "nombre")?,
                mensualidad: alumno.try_get("", "mensualidad")?,
                periodo: alumno.try_get("", "periodo")?,
            });
        }
        resultado.push(TallerCuenta {
            nombre: taller.try_get("", "nombre")?,
            cuota_mensual_mxn: taller.try_get("", "cuota_mensual_mxn")?,
            horarios: horarios_taller,
            alumnos: alumnos_taller,
        });
    }

    let mut falta = Vec::new();
    if oficio.trim().is_empty() {
        falta.push(
            "Escribe tu nombre y tu oficio en /cuenta/datos?nombre=&oficio=".to_string(),
        );
    }
    if resultado.is_empty() {
        falta.push(
            "Crea el taller en /cuenta/taller?nombre=&cuota_mensual_mxn=25".to_string(),
        );
    } else if resultado.iter().any(|taller| taller.horarios.is_empty()) {
        falta.push(
            "Agrega el horario en /cuenta/horario?dia=martes&de=16:00&a=18:00".to_string(),
        );
    }
    let taller_ejemplo = resultado
        .first()
        .map(|taller| url_parte(&taller.nombre))
        .unwrap_or_else(|| "Ceramica".to_string());

    Ok(Cuenta {
        tallerista: nombre,
        usuario,
        telefono: telefono
            .as_deref()
            .map(enmascarar_telefono)
            .unwrap_or_default(),
        oficio,
        listo: falta.is_empty(),
        falta,
        siguientes: Siguientes {
            datos: "/cuenta/datos?nombre=&oficio=".to_string(),
            taller: "/cuenta/taller?nombre=&cuota_mensual_mxn=25".to_string(),
            horario: format!("/cuenta/horario?taller={taller_ejemplo}&dia=martes&de=16:00&a=18:00"),
            alumno: format!("/cuenta/alumno?nombre=&taller={taller_ejemplo}"),
            pago: format!("/cuenta/pago?alumno=&taller={taller_ejemplo}&estado=pagado"),
        },
        talleres: resultado,
    })
}

fn dia_visible(dia: &str) -> &str {
    match dia {
        "miercoles" => "miércoles",
        "sabado" => "sábado",
        otro => otro,
    }
}

fn aviso(mensaje: impl Into<String>) -> DbErr {
    DbErr::Custom(format!("aviso: {}", mensaje.into()))
}

fn url_parte(valor: &str) -> String {
    valor.replace(' ', "%20")
}

fn usuario_valido(valor: &str) -> Result<String, DbErr> {
    let usuario = crate::auth::normalizar_usuario(valor);
    if (3..=30).contains(&usuario.len())
        && usuario.chars().all(|caracter| caracter.is_ascii_alphanumeric())
    {
        Ok(usuario)
    } else {
        Err(aviso(
            "el usuario usa letras y números, entre 3 y 30, sin espacios",
        ))
    }
}

fn clave_valida(clave: &str) -> Result<&str, DbErr> {
    let clave = clave.trim();
    if (6..=100).contains(&clave.len()) {
        Ok(clave)
    } else {
        Err(aviso("la contraseña debe tener entre 6 y 100 caracteres"))
    }
}

pub async fn crear_cuenta(
    db: &DatabaseConnection,
    usuario: &str,
    clave: &str,
) -> Result<Entrada, DbErr> {
    let usuario = usuario_valido(usuario)?;
    let clave = clave_valida(clave)?;
    let ya = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id FROM talleristas WHERE usuario = ?",
            [usuario.clone().into()],
        ))
        .await?;
    if ya.is_some() {
        return Err(aviso(
            "ese usuario ya existe. Entra en /cuenta/entrar?usuario=&clave=",
        ));
    }
    let hash = crate::auth::hash_clave(clave)?;
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO talleristas (nombre, oficio, pey_api_key_id, usuario, clave_hash)
         VALUES (?, '', ?, ?, ?)",
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
            DbBackend::Sqlite,
            "SELECT id, nombre, usuario FROM talleristas WHERE usuario = ?",
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

pub async fn entrar_con_clave(
    db: &DatabaseConnection,
    usuario: &str,
    clave: &str,
) -> Result<Entrada, DbErr> {
    let clave = clave.trim();
    let usuario = match usuario_valido(usuario) {
        Ok(usuario) => usuario,
        Err(_) => {
            let _ = crate::auth::clave_correcta(crate::auth::hash_falso(), clave);
            return Err(aviso("usuario o contraseña incorrectos"));
        }
    };
    let fila = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, nombre, usuario, clave_hash FROM talleristas WHERE usuario = ?",
            [usuario.into()],
        ))
        .await?;
    let Some(fila) = fila else {
        let _ = crate::auth::clave_correcta(crate::auth::hash_falso(), clave);
        return Err(aviso("usuario o contraseña incorrectos"));
    };
    let hash: Option<String> = fila.try_get("", "clave_hash")?;
    let valida = hash
        .as_deref()
        .is_some_and(|hash| crate::auth::clave_correcta(hash, clave));
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

async fn abrir_sesion(db: &DatabaseConnection, sesion: Sesion) -> Result<Entrada, DbErr> {
    let token = crate::auth::token_opaco()?;
    let token_hash = crate::auth::resumen_codigo("sesion", &token);
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO sesiones (tallerista_id, token_hash, creada_en, vence_en)
         VALUES (?, ?, datetime('now'), datetime('now', '+12 hours'))",
        [sesion.id.into(), token_hash.into()],
    ))
    .await?;
    Ok(Entrada { sesion, token })
}

pub async fn sesion_por_token(
    db: &DatabaseConnection,
    token: &str,
) -> Result<Option<Sesion>, DbErr> {
    if token.is_empty() {
        return Ok(None);
    }
    let token_hash = crate::auth::resumen_codigo("sesion", token);
    let fila = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT t.id, t.nombre, t.usuario
             FROM sesiones s
             JOIN talleristas t ON t.id = s.tallerista_id
             WHERE s.token_hash = ? AND s.vence_en > datetime('now')",
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

fn enmascarar_telefono(telefono: &str) -> String {
    let visibles = telefono.chars().rev().take(4).collect::<String>();
    let visibles = visibles.chars().rev().collect::<String>();
    format!("******{visibles}")
}

pub async fn guardar_datos_cuenta(
    db: &DatabaseConnection,
    tallerista_id: i32,
    nombre: &str,
    oficio: &str,
) -> Result<(), DbErr> {
    let nombre = nombre.trim();
    let oficio = oficio.trim();
    if nombre.is_empty() || oficio.is_empty() {
        return Err(aviso("escribe nombre y oficio"));
    }
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "UPDATE talleristas SET nombre = ?, oficio = ? WHERE id = ?",
        [nombre.into(), oficio.into(), tallerista_id.into()],
    ))
    .await?;
    Ok(())
}

pub async fn guardar_taller_cuenta(
    db: &DatabaseConnection,
    tallerista_id: i32,
    nombre: &str,
    cuota: i32,
) -> Result<(), DbErr> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(aviso("escribe el nombre del taller"));
    }
    if cuota <= 0 {
        return Err(aviso("la cuota debe ser mayor que 0"));
    }
    let txn = db.begin().await?;
    let actual = txn
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id, cuota_mensual_mxn FROM talleres WHERE tallerista_id = ? AND nombre = ?",
            [tallerista_id.into(), nombre.into()],
        ))
        .await?;
    let ahora = ahora(&txn).await?;
    if let Some(fila) = actual {
        let id: i32 = fila.try_get("", "id")?;
        let anterior: i32 = fila.try_get("", "cuota_mensual_mxn")?;
        if anterior != cuota {
            txn.execute_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "UPDATE talleres SET cuota_mensual_mxn = ? WHERE id = ?",
                [cuota.into(), id.into()],
            ))
            .await?;
            txn.execute_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT INTO cambios_cuota (taller_id, cuota_anterior_mxn, cuota_nueva_mxn, vigente_desde)
                 VALUES (?, ?, ?, ?)",
                [id.into(), anterior.into(), cuota.into(), ahora.into()],
            ))
            .await?;
        }
    } else {
        txn.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO talleres (tallerista_id, nombre, cuota_mensual_mxn) VALUES (?, ?, ?)",
            [tallerista_id.into(), nombre.into(), cuota.into()],
        ))
        .await?;
        let id: i32 = txn
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT id FROM talleres WHERE tallerista_id = ? AND nombre = ?",
                [tallerista_id.into(), nombre.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::Custom("no se pudo crear el taller".to_owned()))?
            .try_get("", "id")?;
        txn.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO cambios_cuota (taller_id, cuota_anterior_mxn, cuota_nueva_mxn, vigente_desde)
             VALUES (?, NULL, ?, ?)",
            [id.into(), cuota.into(), ahora.into()],
        ))
        .await?;
    }
    txn.commit().await?;
    Ok(())
}

pub async fn guardar_horario_cuenta(
    db: &DatabaseConnection,
    tallerista_id: i32,
    taller: Option<&str>,
    dia: &str,
    inicio: &str,
    fin: &str,
) -> Result<(), DbErr> {
    let taller_id = buscar_taller(db, tallerista_id, taller).await?;
    let dia = dia_interno(dia)?;
    let inicio = hora(inicio)?;
    let fin = hora(fin)?;
    if inicio >= fin {
        return Err(aviso("la hora de inicio tiene que ser antes de la hora de fin"));
    }
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin)
         SELECT ?, ?, ?, ?
         WHERE NOT EXISTS (
             SELECT 1 FROM horarios WHERE taller_id = ? AND dia = ? AND hora_inicio = ?
         )",
        [
            taller_id.into(),
            dia.into(),
            inicio.clone().into(),
            fin.into(),
            taller_id.into(),
            dia.into(),
            inicio.into(),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn guardar_alumno_cuenta(
    db: &DatabaseConnection,
    tallerista_id: i32,
    nombre: &str,
    taller: Option<&str>,
) -> Result<(), DbErr> {
    let nombre = nombre.trim();
    if nombre.is_empty() {
        return Err(aviso("escribe el nombre del alumno"));
    }
    let taller_id = buscar_taller(db, tallerista_id, taller).await?;
    let txn = db.begin().await?;
    let alumno_id: i32 = match txn
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id FROM alumnos WHERE nombre = ?",
            [nombre.into()],
        ))
        .await?
    {
        Some(fila) => fila.try_get::<i32>("", "id")?,
        None => {
            let cartera = format!(
                "cartera_{}_{tallerista_id}",
                crate::auth::normalizar_usuario(nombre)
            );
            txn.execute_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "INSERT INTO alumnos (nombre, cartera_ref) VALUES (?, ?)",
                [nombre.into(), cartera.into()],
            ))
            .await?;
            txn.query_one_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT id FROM alumnos WHERE nombre = ?",
                [nombre.into()],
            ))
            .await?
            .ok_or_else(|| DbErr::Custom("no se pudo guardar al alumno".to_owned()))?
            .try_get::<i32>("", "id")?
        }
    };
    txn.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO inscripciones_taller (alumno_id, taller_id, anio)
         SELECT ?, ?, CAST(strftime('%Y', 'now') AS INTEGER)
         WHERE NOT EXISTS (
             SELECT 1 FROM inscripciones_taller
             WHERE alumno_id = ? AND taller_id = ? AND anio = CAST(strftime('%Y', 'now') AS INTEGER)
         )",
        [
            alumno_id.into(),
            taller_id.into(),
            alumno_id.into(),
            taller_id.into(),
        ],
    ))
    .await?;
    txn.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO inscripciones_anuales (alumno_id, anio, monto_mxn)
         SELECT ?, CAST(strftime('%Y', 'now') AS INTEGER), ?
         WHERE NOT EXISTS (
             SELECT 1 FROM inscripciones_anuales
             WHERE alumno_id = ? AND anio = CAST(strftime('%Y', 'now') AS INTEGER)
         )",
        [
            alumno_id.into(),
            INSCRIPCION_ANUAL_MXN.into(),
            alumno_id.into(),
        ],
    ))
    .await?;
    let num_ref = format!("alta-{tallerista_id}-{alumno_id}-{}", marca());
    txn.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO pagos (
             alumno_id, taller_id, tallerista_id, tipo, canal, concepto, monto_mxn,
             num_ref, estado, periodo
         )
         SELECT ?, ?, ?, 'mensualidad', 'panel',
                'Mensualidad ' || (SELECT nombre FROM talleres WHERE id = ?) || ' ' || strftime('%Y-%m', 'now'),
                (SELECT cuota_mensual_mxn FROM talleres WHERE id = ?),
                ?, 'pendiente', strftime('%Y-%m', 'now')
         WHERE NOT EXISTS (
             SELECT 1 FROM pagos
             WHERE alumno_id = ? AND taller_id = ? AND tipo = 'mensualidad'
               AND periodo = strftime('%Y-%m', 'now')
         )",
        [
            alumno_id.into(),
            taller_id.into(),
            tallerista_id.into(),
            taller_id.into(),
            taller_id.into(),
            num_ref.into(),
            alumno_id.into(),
            taller_id.into(),
        ],
    ))
    .await?;
    txn.commit().await?;
    Ok(())
}

pub async fn marcar_pago_cuenta(
    db: &DatabaseConnection,
    tallerista_id: i32,
    alumno: &str,
    taller: Option<&str>,
    estado: &str,
) -> Result<(), DbErr> {
    if estado != "pagado" && estado != "pendiente" && estado != "fallido" {
        return Err(aviso("el estado debe ser pagado, pendiente o fallido"));
    }
    let nombre = alumno.trim();
    if nombre.is_empty() {
        return Err(aviso("escribe el nombre del alumno"));
    }
    let taller_id = buscar_taller(db, tallerista_id, taller).await?;
    let alumno_id: i32 = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT a.id
             FROM alumnos a
             JOIN inscripciones_taller i ON i.alumno_id = a.id
             WHERE a.nombre = ? AND i.taller_id = ? AND i.activa = 1",
            [nombre.into(), taller_id.into()],
        ))
        .await?
        .ok_or_else(|| aviso("ese alumno no está en tu taller"))?
        .try_get("", "id")?;
    let pago = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id FROM pagos
             WHERE alumno_id = ? AND taller_id = ? AND tipo = 'mensualidad'
               AND periodo = strftime('%Y-%m', 'now')
             ORDER BY id DESC
             LIMIT 1",
            [alumno_id.into(), taller_id.into()],
        ))
        .await?;
    let ahora = ahora(db).await?;
    if let Some(fila) = pago {
        let id: i32 = fila.try_get("", "id")?;
        if estado == "pagado" {
            db.execute_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "UPDATE pagos
                 SET estado = 'pagado',
                     transaction_id = COALESCE(transaction_id, ?),
                     tipo_evento = COALESCE(tipo_evento, 'codi.qr.payment_received'),
                     pagado_en = COALESCE(pagado_en, ?)
                 WHERE id = ?",
                [format!("txn-cuenta-{id}").into(), ahora.into(), id.into()],
            ))
            .await?;
        } else {
            db.execute_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "UPDATE pagos SET estado = ? WHERE id = ?",
                [estado.into(), id.into()],
            ))
            .await?;
        }
    } else if estado == "pagado" {
        let num_ref = format!("pago-{tallerista_id}-{alumno_id}-{}", marca());
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO pagos (
                 alumno_id, taller_id, tallerista_id, tipo, canal, concepto, monto_mxn,
                 num_ref, transaction_id, tipo_evento, estado, periodo, pagado_en
             )
             SELECT ?, ?, ?, 'mensualidad', 'panel',
                    'Mensualidad ' || (SELECT nombre FROM talleres WHERE id = ?) || ' ' || strftime('%Y-%m', 'now'),
                    (SELECT cuota_mensual_mxn FROM talleres WHERE id = ?),
                    ?, ?, 'codi.qr.payment_received', 'pagado', strftime('%Y-%m', 'now'), ?",
            [
                alumno_id.into(),
                taller_id.into(),
                tallerista_id.into(),
                taller_id.into(),
                taller_id.into(),
                num_ref.clone().into(),
                format!("txn-{num_ref}").into(),
                ahora.into(),
            ],
        ))
        .await?;
    } else {
        return Err(aviso("todavía no hay una mensualidad de este mes para ese alumno"));
    }
    Ok(())
}

async fn buscar_taller(
    db: &DatabaseConnection,
    tallerista_id: i32,
    nombre: Option<&str>,
) -> Result<i32, DbErr> {
    if let Some(nombre) = nombre.map(str::trim).filter(|nombre| !nombre.is_empty()) {
        return db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT id FROM talleres WHERE tallerista_id = ? AND nombre = ? AND activo = 1",
                [tallerista_id.into(), nombre.into()],
            ))
            .await?
            .ok_or_else(|| aviso("no encontré ese taller en tu cuenta"))?
            .try_get("", "id");
    }
    let filas = db
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "SELECT id FROM talleres WHERE tallerista_id = ? AND activo = 1 ORDER BY id",
            [tallerista_id.into()],
        ))
        .await?;
    if filas.len() == 1 {
        filas[0].try_get("", "id")
    } else if filas.is_empty() {
        Err(aviso("primero crea el taller en /cuenta/taller"))
    } else {
        Err(aviso("indica el taller porque tienes más de uno"))
    }
}

fn dia_interno(dia: &str) -> Result<&'static str, DbErr> {
    match crate::auth::normalizar_usuario(dia).as_str() {
        "lunes" => Ok("lunes"),
        "martes" => Ok("martes"),
        "miercoles" => Ok("miercoles"),
        "jueves" => Ok("jueves"),
        "viernes" => Ok("viernes"),
        "sabado" => Ok("sabado"),
        "domingo" => Ok("domingo"),
        _ => Err(aviso(
            "el día debe ser lunes, martes, miércoles, jueves, viernes, sábado o domingo",
        )),
    }
}

fn hora(valor: &str) -> Result<String, DbErr> {
    let (horas, minutos) = valor
        .trim()
        .split_once(':')
        .ok_or_else(|| aviso("la hora debe verse como 16:00"))?;
    let horas: u32 = horas
        .parse()
        .map_err(|_| aviso("la hora debe verse como 16:00"))?;
    let minutos: u32 = minutos
        .parse()
        .map_err(|_| aviso("la hora debe verse como 16:00"))?;
    if horas > 23 || minutos > 59 {
        return Err(aviso("la hora debe verse como 16:00"));
    }
    Ok(format!("{horas:02}:{minutos:02}"))
}

fn marca() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duracion| duracion.as_millis())
        .unwrap_or(0)
}
