import { useState } from 'react';
import Crud, { Column } from '../components/Crud';
import Layout from '../components/Layout';

const SHIFT: Column[] = [
  { key: 'code', label: 'Kode', required: true },
  { key: 'name', label: 'Nama shift', required: true },
  { key: 'start_time', label: 'Mulai (HH:MM)', required: true },
  { key: 'end_time', label: 'Selesai (HH:MM)', required: true },
  { key: 'crosses_midnight', label: 'Lewat tengah malam (0/1)', type: 'select', options: [{ value: '0', label: 'Tidak' }, { value: '1', label: 'Ya' }] },
  { key: 'break_minutes', label: 'Istirahat (mnt)' },
  { key: 'late_tolerance_minutes', label: 'Toleransi telat (mnt)' },
  { key: 'clockin_early_limit_minutes', label: 'Batas awal clock-in (mnt)' },
  { key: 'clockin_late_limit_minutes', label: 'Batas akhir clock-in (mnt)' },
  { key: 'clockout_limit_minutes', label: 'Batas clock-out (mnt)' },
  { key: 'overtime_before_minutes', label: 'Lembur sebelum (mnt)' },
  { key: 'overtime_after_minutes', label: 'Lembur sesudah (mnt)' },
  { key: 'applicable_days', label: 'Hari berlaku (0=Sen..6=Min)' },
  { key: 'is_active', label: 'Aktif', type: 'select', badge: true, options: [{ value: '1', label: 'Aktif' }, { value: '0', label: 'Nonaktif' }] },
  { key: 'version', label: 'Versi', readOnly: true, hideInTable: true },
];

const ASSIGN: Column[] = [
  { key: 'employee_id', label: 'Employee ID', required: true },
  { key: 'shift_id', label: 'Shift ID', required: true },
  { key: 'effective_date', label: 'Tgl efektif', type: 'date', required: true },
  { key: 'end_date', label: 'Tgl akhir' },
  { key: 'created_by', label: 'Dibuat oleh' },
];

export default function Shifts() {
  const [tab, setTab] = useState<'shift' | 'jadwal'>('shift');
  return (
    <Layout title="Shift & Jadwal" breadcrumb="Waktu kerja">
      <div className="alert alert-info">
        Preset awal: Pagi 07:00–15:00, Siang 15:00–23:00, Malam 23:00–07:00 (melewati tengah malam).
        Semua jam, toleransi, dan batas dapat diubah. Perubahan aturan shift tercatat berversi agar
        perhitungan periode lalu tidak rusak — gunakan penugasan dengan tanggal efektif untuk rotasi.
      </div>
      <div className="btn-list mb-3">
        <button className={`btn ${tab === 'shift' ? 'btn-primary' : ''}`} onClick={() => setTab('shift')}>Daftar shift</button>
        <button className={`btn ${tab === 'jadwal' ? 'btn-primary' : ''}`} onClick={() => setTab('jadwal')}>Penugasan / jadwal</button>
      </div>
      {tab === 'shift' && <Crud table="shifts" title="Shift" columns={SHIFT} orderBy="code" />}
      {tab === 'jadwal' && (
        <Crud table="shift_assignments" title="Penugasan Shift (efektif per tanggal)" columns={ASSIGN} orderBy="effective_date" />
      )}
    </Layout>
  );
}
