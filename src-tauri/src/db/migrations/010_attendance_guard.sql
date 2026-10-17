-- Build 10: perlindungan koreksi manual + pembenahan data konvensi hari.
-- 1) Kolom penanda: baris yang telah dikoreksi manual TIDAK tertimpa
--    "hitung ulang" mesin (koreksi admin tidak lenyap diam-diam).
ALTER TABLE attendance_results
    ADD COLUMN manually_corrected INTEGER NOT NULL DEFAULT 0;

-- 2) Sapu tersisa konvensi lama ('1,2,3,4,5' = Selasa-Sabtu): shift yang
--    dibuat antara migrasi v9 dan pengikatan nilai default dapat terbawa.
--    Konvensi tetap 0=Senin..6=Minggu (sesuai label UI).
UPDATE shifts
   SET applicable_days = '0,1,2,3,4'
 WHERE applicable_days = '1,2,3,4,5';
