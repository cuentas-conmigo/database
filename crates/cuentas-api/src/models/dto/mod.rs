//! Formas JSON de la app (camelCase, como `docs/api.md` de la app), una por recurso.

pub mod alumnos;
pub mod pagos;
pub mod salud;
pub mod sesion;
pub mod talleres;
pub mod webhooks;

pub use alumnos::{HelperDto, PhotoDto, StudentDto};
pub use pagos::{NewPaymentBody, PaymentDto};
pub use salud::HealthDto;
pub use sesion::{NewAccountBody, SessionBody, SessionDto};
pub use talleres::{ChargeOptionDto, CodeDto, NewStudentBody, ReviewEntryDto, RosterEntryDto, WorkshopDto};
pub use webhooks::WebhookAckDto;
