use std::str::FromStr;

use sea_orm::sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, SqlxSqliteConnector, Statement,
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
    pey_api_key_id TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS talleres (
    id INTEGER PRIMARY KEY,
    tallerista_id INTEGER NOT NULL REFERENCES talleristas(id),
    nombre TEXT NOT NULL,
    cuota_mensual_mxn INTEGER NOT NULL CHECK (cuota_mensual_mxn > 0),
    activo INTEGER NOT NULL DEFAULT 1 CHECK (activo IN (0, 1)),
    UNIQUE (tallerista_id, nombre)
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

async fn preparar(db: &DatabaseConnection) -> Result<(), DbErr> {
    ejecutar_lote(db, SCHEMA).await?;
    use sea_orm::{EntityTrait, PaginatorTrait};
    let existentes = entity::tallerista::Entity::find().count(db).await?;
    if existentes == 0 {
        ejecutar_lote(db, SEED).await?;
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

pub async fn ahora(db: &DatabaseConnection) -> Result<String, DbErr> {
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
