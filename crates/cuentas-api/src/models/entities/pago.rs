use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "pagos")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub id: i32,
	pub alumno_id: i32,
	pub taller_id: Option<i32>,
	pub tallerista_id: Option<i32>,
	pub tipo: String,
	pub canal: String,
	pub concepto: String,
	pub monto_mxn: i32,
	pub num_ref: String,
	pub codi_qr_id: Option<String>,
	pub transaction_id: Option<String>,
	pub tipo_evento: Option<String>,
	pub estado: String,
	pub periodo: String,
	pub pagado_en: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
