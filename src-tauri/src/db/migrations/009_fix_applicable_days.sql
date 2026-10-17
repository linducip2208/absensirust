-- Perbaikan data: konvensi hari berlaku adalah 0=Senin..6=Minggu (sesuai UI),
-- tetapi seed lama menulis '1,2,3,4,5' yang bermakna Selasa-Sabtu sehingga
-- Senin terhitung hari libur dan Sabtu dihitung hari kerja untuk shift bawaan.
UPDATE shifts
   SET applicable_days = '0,1,2,3,4'
 WHERE applicable_days = '1,2,3,4,5'
   AND code IN ('PAGI', 'SIANG', 'MALAM');
