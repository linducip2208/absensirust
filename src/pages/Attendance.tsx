import { useEffect, useState } from 'react';
import { api, Row, todayWIB } from '../api';
import Layout from '../components/Layout';
import { EmptyState, Field, LoadingState, StatusBadge, useToast } from '../components/ui';

export default function Attendance() {
  const toast = useToast();
  const [date, setDate] = useState(todayWIB());
  const [busy, setBusy] = useState(false);
  const [saving, setSaving] = useState(false);
  const [review, setReview] = useState<Row[]>([]);
  const [unmapped, setUnmapped] = useState<Row[]>([]);
  const [results, setResults] = useState<Row[]>([]);
  const [names, setNames] = useState<Record<string, string>>({});
  const [correcting, setCorrecting] = useState<Row | null>(null);
  const [newValue, setNewValue] = useState('');
  const [reason, setReason] = useState('');
  const [correctField, setCorrectField] = useState('clock_in');

  const load = async () => {
    try {
      const r = await api.attendanceReview();
      setReview(r.review);
      setUnmapped(r.unmapped);
      const res = await api.tableList('attendance_results', 1, 50, date, 'work_date', 'desc');
      setResults(res.rows);
      const emp = await api.tableList('employees', 1, 1000, '', 'name', 'asc');
      const map: Record<string, string> = {};
      for (const e of emp.rows) map[String(e.id)] = String(e.name ?? e.nik ?? e.id);
      setNames(map);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  useEffect(() => {
    load().catch(() => {});
  }, [date]);

  const compute = async () => {
    setBusy(true);
    try {
      const res = await api.attendanceCompute(date);
      toast.push('success', `Menghitung ${res.computed} karyawan untuk ${res.date}`);
      await load();
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const submitCorrection = async () => {
    if (!correcting || saving) return;
    if (!reason.trim()) {
      toast.push('error', 'Alasan wajib diisi');
      return;
    }
    setSaving(true);
    try {
      await api.attendanceCorrect(String(correcting.id), correctField, newValue, reason);
      toast.push('success', 'Koreksi tersimpan dengan jejak audit');
      setCorrecting(null);
      setNewValue('');
      setReason('');
      await load();
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Layout title="Absensi" breadcrumb="Kehadiran">
      <div className="card mb-3">
        <div className="card-body d-flex gap-2 align-items-end flex-wrap">
          <div>
            <label className="form-label">Tanggal kerja (WIB)</label>
            <input type="date" className="form-control" value={date} onChange={(e) => setDate(e.target.value)} />
          </div>
          <button className="btn btn-primary" disabled={busy} onClick={compute}>
            {busy ? 'Menghitung…' : 'Hitung ulang absensi tanggal ini'}
          </button>
          <button
            className="btn btn-outline-secondary"
            disabled={busy}
            title="Kembalikan event yang sudah diproses ke antrean (untuk dihitung ulang)"
            onClick={async () => {
              if (!confirm(`Kembalikan event ${date} ke antrean untuk dihitung ulang?`)) return;
              try {
                const r = await api.eventsRequeue(date, date);
                toast.push('success', `${r.requeued} event dikembalikan ke antrean`);
                await load();
              } catch (e) {
                toast.push('error', e instanceof Error ? e.message : String(e));
              }
            }}
          >
            Kembalikan ke antrean
          </button>
          <span className="text-secondary">
            Mesin hitung memproses raw event perangkat (tidak mengubah event asli). Shift malam 23:00–07:00
            dikaitkan ke tanggal kerja asal.
          </span>
        </div>
      </div>

      <div className="card mb-3">
        <div className="card-header"><h3 className="card-title">Hasil absensi ({results.length} terbaru)</h3></div>
        <div className="table-responsive">
          <table className="table card-table table-vcenter">
            <thead>
              <tr><th>Tanggal</th><th>Karyawan</th><th>Masuk</th><th>Pulang</th><th>Kerja</th><th>Telat</th><th>Status</th><th>Anomali</th><th>Aksi</th></tr>
            </thead>
            <tbody>
              {results.length === 0 && (
                <tr><td colSpan={9}><EmptyState title="Belum ada hasil" hint="Klik hitung ulang atau ubah tanggal kerja." /></td></tr>
              )}
              {results.map((r) => (
                <tr key={String(r.id)}>
                  <td>{String(r.work_date)}</td><td>{names[String(r.employee_id)] ?? String(r.employee_id)}</td>
                  <td>{String(r.clock_in || '-')}</td><td>{String(r.clock_out || '-')}</td>
                  <td>{String(r.work_minutes)} m</td><td>{String(r.late_minutes)} m</td>
                  <td><StatusBadge value={r.status} /></td>
                  <td>{String(r.anomaly || '-')}</td>
                  <td><button className="btn btn-sm btn-outline-warning" onClick={() => { setCorrecting(r); setNewValue(''); setReason(''); }}>Koreksi</button></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      <div className="row">
        <div className="col-lg-6">
          <div className="card">
            <div className="card-header"><h3 className="card-title">Perlu review ({review.length})</h3></div>
            <div className="card-body">
              {review.length === 0 ? <div className="text-secondary">Tidak ada kasus Incomplete / NeedsReview / Absent.</div> : (
                <ul className="list-group">
                  {review.map((r) => (
                    <li className="list-group-item" key={String(r.id)}>
                      <strong>{String(r.name)}</strong> · {String(r.date)} · <StatusBadge value={r.status} />
                      <div className="text-secondary">{String(r.anomaly || '')}</div>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </div>
        </div>
        <div className="col-lg-6">
          <div className="card">
            <div className="card-header"><h3 className="card-title">Log tanpa mapping karyawan ({unmapped.length})</h3></div>
            <div className="card-body">
              {unmapped.length === 0 ? <div className="text-secondary">Semua log sudah terpetakan.</div> : (
                <div className="text-secondary mb-2">Petakan via menu Karyawan → Mapping device agar tidak kehilangan data.</div>
              )}
              <ul className="list-group">
                {unmapped.slice(0, 20).map((r) => (
                  <li className="list-group-item" key={String(r.id)}>
                    {String(r.time)} · device {String(r.device)} · user {String(r.device_user)}
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>
      </div>

      {correcting && (
        <div className="modal modal-blur fade show" style={{ display: 'block' }} role="dialog">
          <div className="modal-dialog modal-dialog-centered" role="document">
            <div className="modal-content">
              <div className="modal-header">
                <h5 className="modal-title">Koreksi absensi (audit tersimpan)</h5>
                <button type="button" className="btn-close" onClick={() => setCorrecting(null)} aria-label="Tutup" />
              </div>
              <div className="modal-body">
                <Field label="Kolom">
                  <select className="form-select" value={correctField} onChange={(e) => setCorrectField(e.target.value)}>
                    <option value="clock_in">Jam masuk</option>
                    <option value="clock_out">Jam pulang</option>
                    <option value="status">Status</option>
                  </select>
                </Field>
                <Field label="Nilai baru">
                  <input className="form-control" value={newValue} onChange={(e) => setNewValue(e.target.value)} placeholder="YYYY-MM-DD HH:MM:SS atau status" />
                </Field>
                <Field label="Alasan (wajib)">
                  <textarea className="form-control" value={reason} onChange={(e) => setReason(e.target.value)} />
                </Field>
                {busy && <LoadingState />}
              </div>
              <div className="modal-footer">
                <button className="btn btn-link link-secondary" onClick={() => setCorrecting(null)}>Batal</button>
                <button className="btn btn-warning" disabled={saving} onClick={submitCorrection}>
                  {saving ? 'Menyimpan…' : 'Simpan koreksi'}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </Layout>
  );
}
