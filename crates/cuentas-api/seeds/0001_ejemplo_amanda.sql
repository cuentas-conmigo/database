-- Datos de ejemplo de Amanda (antes `SEED` y `HORARIOS` en `services/esquema.rs`).
-- Se puede correr muchas veces: lo que ya existe no se repite.
-- Las contraseñas de estas cuentas se asignan en código (`services::sembrar`), porque necesitan argon2.

INSERT INTO talleristas (id, nombre, oficio, pey_api_key_id) VALUES
	(1, 'Marichuy', 'tejido', 'ejemplo_pey_marichuy'),
	(2, 'Pepe', 'artes visuales', 'ejemplo_pey_pepe'),
	(3, 'María', 'matemáticas', 'ejemplo_pey_maria'),
	(4, 'Juan', 'deportes', 'ejemplo_pey_juan')
ON CONFLICT (id) DO NOTHING;

INSERT INTO talleres (id, tallerista_id, nombre, cuota_mensual_mxn) VALUES
	(1, 1, 'Tejido', 25),
	(2, 2, 'Artes visuales', 25),
	(3, 3, 'Matemáticas', 25),
	(4, 4, 'Deportes', 25)
ON CONFLICT (id) DO NOTHING;

INSERT INTO cambios_cuota (taller_id, cuota_anterior_mxn, cuota_nueva_mxn, vigente_desde)
SELECT v.taller_id, v.anterior, v.nueva, v.desde
FROM (VALUES
	(1, NULL::INTEGER, 25, '2026-01-01'),
	(2, NULL, 25, '2026-01-01'),
	(3, NULL, 25, '2026-01-01'),
	(4, NULL, 25, '2026-01-01')
) AS v (taller_id, anterior, nueva, desde)
WHERE NOT EXISTS (SELECT 1 FROM cambios_cuota c WHERE c.taller_id = v.taller_id AND c.vigente_desde = v.desde)
ORDER BY v.taller_id;

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
	(10, 'Emiliano Vargas', 'cartera_emiliano_vargas')
ON CONFLICT (id) DO NOTHING;

INSERT INTO dispositivos_nfc (tallerista_id, cara, chip_uid) VALUES
	(1, 'cobro', 'nfc-marichuy-cobro'),
	(1, 'inscripcion', 'nfc-marichuy-inscripcion'),
	(4, 'cobro', 'nfc-juan-cobro'),
	(4, 'inscripcion', 'nfc-juan-inscripcion')
ON CONFLICT (chip_uid) DO NOTHING;

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
	(10, 2026, 125)
ON CONFLICT (alumno_id, anio) DO NOTHING;

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
	(3, 4, 2026)
ON CONFLICT (alumno_id, taller_id, anio) DO NOTHING;

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
		'MENS-2026-11-13', 'qr-mens-13', 'txn-mens-13', 'codi.qr.payment_received', 'pagado', '2026-11', '2026-11-07 09:27:00')
ON CONFLICT (num_ref) DO NOTHING;

-- Los ids anteriores se escribieron a mano; las secuencias siguen después de ellos.
SELECT setval(pg_get_serial_sequence('talleristas', 'id'), (SELECT MAX(id) FROM talleristas));
SELECT setval(pg_get_serial_sequence('talleres', 'id'), (SELECT MAX(id) FROM talleres));
SELECT setval(pg_get_serial_sequence('alumnos', 'id'), (SELECT MAX(id) FROM alumnos));

INSERT INTO horarios (taller_id, dia, hora_inicio, hora_fin) VALUES
	(1, 'martes', '16:00', '18:00'),
	(1, 'jueves', '16:00', '18:00'),
	(2, 'lunes', '17:00', '19:00'),
	(2, 'miercoles', '17:00', '19:00'),
	(3, 'lunes', '10:00', '11:30'),
	(3, 'miercoles', '10:00', '11:30'),
	(3, 'viernes', '10:00', '11:30'),
	(4, 'sabado', '09:00', '11:00')
ON CONFLICT (taller_id, dia, hora_inicio) DO NOTHING;
