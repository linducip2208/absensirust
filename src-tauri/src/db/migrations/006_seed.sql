INSERT OR IGNORE INTO branches (id, code, name, address, created_at)
VALUES ('br-hq', 'HQ', 'Kantor Pusat', 'Jakarta', '2026-01-01 00:00:00');
INSERT OR IGNORE INTO departments (id, code, name, branch_id, created_at)
VALUES ('dept-umum', 'UMUM', 'Umum', 'br-hq', '2026-01-01 00:00:00');
INSERT OR IGNORE INTO positions (id, code, name, created_at)
VALUES ('pos-staff', 'STAFF', 'Staff', '2026-01-01 00:00:00');
INSERT OR IGNORE INTO shifts (id, code, name, start_time, end_time, crosses_midnight, break_minutes, late_tolerance_minutes, clockin_early_limit_minutes, clockin_late_limit_minutes, clockout_limit_minutes, overtime_before_minutes, overtime_after_minutes, applicable_days, is_active, version, created_at, updated_at)
VALUES
('sh-pagi', 'PAGI', 'Shift Pagi', '07:00', '15:00', 0, 60, 10, 120, 180, 240, 0, 60, '1,2,3,4,5', 1, 1, '2026-01-01 00:00:00', '2026-01-01 00:00:00'),
('sh-siang', 'SIANG', 'Shift Siang', '15:00', '23:00', 0, 60, 10, 120, 180, 240, 0, 60, '1,2,3,4,5', 1, 1, '2026-01-01 00:00:00', '2026-01-01 00:00:00'),
('sh-malam', 'MALAM', 'Shift Malam', '23:00', '07:00', 1, 60, 10, 120, 180, 240, 0, 60, '1,2,3,4,5', 1, 1, '2026-01-01 00:00:00', '2026-01-01 00:00:00');
INSERT OR IGNORE INTO attendance_policy (id, rounding_minutes, max_early_minutes, require_overtime_approval, allow_double_shift, updated_at)
VALUES ('default', 1, 120, 1, 0, '2026-01-01 00:00:00');
INSERT OR IGNORE INTO leave_types (id, code, name, quota_days, created_at)
VALUES
('lt-tahunan', 'TAHUNAN', 'Cuti Tahunan', 12, '2026-01-01 00:00:00'),
('lt-sakit', 'SAKIT', 'Sakit', 0, '2026-01-01 00:00:00'),
('lt-khusus', 'KHUSUS', 'Cuti Khusus', 3, '2026-01-01 00:00:00'),
('lt-izin', 'IZIN', 'Izin', 0, '2026-01-01 00:00:00');
