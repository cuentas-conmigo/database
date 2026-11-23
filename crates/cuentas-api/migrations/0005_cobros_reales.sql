-- Cuentas que el equipo aprobó para cobrar dinero real por CoDi. Cualquiera puede crear una cuenta;
-- con Pey en modo real, una cuenta sin fila aquí solo puede anotar pagos en efectivo.
CREATE TABLE cobros_reales (
	tallerista_id INTEGER PRIMARY KEY REFERENCES talleristas (id) ON DELETE CASCADE,
	aprobado_en TEXT NOT NULL DEFAULT ahora_texto()
);
