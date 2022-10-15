ALTER table adc_queue ADD COLUMN priority tinyint default 0;
ALTER table adc_dead_queue ADD COLUMN priority tinyint default 0;