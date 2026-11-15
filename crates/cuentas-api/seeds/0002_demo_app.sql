-- Datos de la demo de la app: María González, su Taller de tejido y cuatro alumnas.
-- La cuenta de María (usuario y contraseña) se asigna en código desde DEMO_USUARIO y DEMO_CLAVE.
-- Se puede correr muchas veces.

INSERT INTO talleristas (nombre, oficio, pey_api_key_id)
VALUES ('María González', 'tejido', 'demo_pey_maria_gonzalez')
ON CONFLICT (pey_api_key_id) DO NOTHING;

INSERT INTO talleres (tallerista_id, nombre, cuota_mensual_mxn)
SELECT id, 'Taller de tejido', 130 FROM talleristas WHERE pey_api_key_id = 'demo_pey_maria_gonzalez'
ON CONFLICT (tallerista_id, nombre) DO NOTHING;

INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin)
SELECT t.id, d.dia, '10:00', '12:00'
FROM talleres t
JOIN talleristas ta ON ta.id = t.tallerista_id
CROSS JOIN (VALUES ('martes'), ('jueves')) AS d (dia)
WHERE ta.pey_api_key_id = 'demo_pey_maria_gonzalez' AND t.nombre = 'Taller de tejido'
ON CONFLICT (taller_id, dia, hora_inicio) DO NOTHING;

INSERT INTO alumnos (nombre, cartera_ref) VALUES
	('Rosa Martínez', 'demo_rosa_martinez'),
	('Ana López', 'demo_ana_lopez'),
	('Élida Ramírez', 'demo_elida_ramirez'),
	('Carmen Ortiz', 'demo_carmen_ortiz')
ON CONFLICT (cartera_ref) DO NOTHING;

INSERT INTO inscripciones_taller (alumno_id, taller_id, anio)
SELECT a.id, t.id, 2026
FROM alumnos a
CROSS JOIN talleres t
JOIN talleristas ta ON ta.id = t.tallerista_id
WHERE a.cartera_ref IN ('demo_rosa_martinez', 'demo_ana_lopez', 'demo_elida_ramirez', 'demo_carmen_ortiz')
	AND ta.pey_api_key_id = 'demo_pey_maria_gonzalez'
	AND t.nombre = 'Taller de tejido'
ON CONFLICT (alumno_id, taller_id, anio) DO NOTHING;

-- Ana ya pagó diciembre.
INSERT INTO pagos (
	alumno_id, taller_id, tallerista_id, tipo, canal, concepto, monto_mxn,
	num_ref, transaction_id, tipo_evento, estado, periodo, pagado_en
)
SELECT a.id, t.id, ta.id, 'mensualidad', 'boton', 'Diciembre', 130,
	'DEMO-ANA-2026-12', 'demo-txn-ana-2026-12', 'codi.qr.payment_received', 'pagado', '2026-12', ahora_texto()
FROM alumnos a
CROSS JOIN talleres t
JOIN talleristas ta ON ta.id = t.tallerista_id
WHERE a.cartera_ref = 'demo_ana_lopez'
	AND ta.pey_api_key_id = 'demo_pey_maria_gonzalez'
	AND t.nombre = 'Taller de tejido'
ON CONFLICT (num_ref) DO NOTHING;
