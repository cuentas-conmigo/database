//! Reglas de negocio y acceso a la base, separados por tema.
//!
//! Los de Amanda se usan como `db::...` (`use crate::services as db;`). Los de la app viven en sus
//! módulos: `acceso` (login de la app), `cobros` (pagos por CoDi), `pey` (cliente de Pey) y `eventos`
//! (lo que su esquema no guarda de cada cobro, en DynamoDB).
//!
//! Las rutas los usan como `db::...` (`use crate::services as db;`).

pub mod acceso;
mod alumnos;
pub mod cobros;
mod comun;
pub mod contactos;
mod cuentas;
mod esquema;
pub mod eventos;
mod pagos;
pub mod pey;
mod reportes;
mod sesiones;
mod talleres;

pub use alumnos::*;
pub use comun::*;
pub use cuentas::*;
pub use esquema::*;
pub use pagos::*;
pub use reportes::*;
pub use sesiones::*;
pub use talleres::*;
