use sea_orm::entity::prelude::*;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "inscripciones_taller")]
pub struct Model {
	#[sea_orm(primary_key)]
	pub id: i32,
	pub alumno_id: i32,
	pub taller_id: i32,
	pub anio: i32,
	pub activa: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
