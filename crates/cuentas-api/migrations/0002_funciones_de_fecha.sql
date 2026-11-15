-- Reemplazos de las funciones de fecha de SQLite que usa el código de Amanda, con el mismo
-- formato de texto y en UTC, para que sus consultas sigan leyéndose igual:
--   datetime('now')                     ->  ahora_texto()
--   datetime('now', '+12 hours')        ->  ahora_texto(INTERVAL '12 hours')
--   strftime('%Y-%m', 'now')            ->  periodo_actual()
--   CAST(strftime('%Y', 'now') AS INT)  ->  anio_actual()

CREATE FUNCTION ahora_texto(desfase INTERVAL DEFAULT INTERVAL '0') RETURNS TEXT
	LANGUAGE sql STABLE
	AS $$ SELECT to_char((now() AT TIME ZONE 'UTC') + desfase, 'YYYY-MM-DD HH24:MI:SS') $$;

CREATE FUNCTION periodo_actual() RETURNS TEXT
	LANGUAGE sql STABLE
	AS $$ SELECT to_char(now() AT TIME ZONE 'UTC', 'YYYY-MM') $$;

CREATE FUNCTION anio_actual() RETURNS INTEGER
	LANGUAGE sql STABLE
	AS $$ SELECT EXTRACT(YEAR FROM now() AT TIME ZONE 'UTC')::INTEGER $$;
