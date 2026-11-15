use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "cambios_cuota")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub id: i32,
	pub taller_id: i32,
	pub cuota_anterior_mxn: Option<i32>,
	pub cuota_nueva_mxn: i32,
	pub vigente_desde: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
