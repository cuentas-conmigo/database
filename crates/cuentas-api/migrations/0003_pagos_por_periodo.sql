-- Para la app: el estado del mes de cada alumno se busca por taller y periodo.
CREATE INDEX idx_pagos_taller_periodo ON pagos (taller_id, periodo, estado);
