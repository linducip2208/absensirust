import { useEffect, useState } from 'react';
import { api, LeaveBalance, pickFile, Row } from '../api';
import Crud, { Column } from '../components/Crud';
import Layout from '../components/Layout';
import { useToast } from '../components/ui';

const LEAVE_TYPE: Column[] = [
  { key: 'code', label: 'Kode', required: true },
  { key: 'name', label: 'Nama', required: true },
  { key: 'quota_days', label: 'Kuota (hari)' },
];
const LEAVE: Column[] = [
  { key: 'employee_id', label: 'Employee ID', required: true },
  { key: 'leave_type_id', label: 'Jenis cuti ID', required: true },
  { key: 'start_date', label: 'Mulai', type: 'date', required: true },
  { key: 'end_date', label: 'Selesai', type: 'date', required: true },
  { key: 'reason', label: 'Alasan', type: 'textarea' },
  { key: 'attachment_path', label: 'Lampiran', hideInTable: true },
  { key: 'status', label: 'Status', type: 'select', badge: true, options: [{ value: 'pending', label: 'Menunggu' }, { value: 'approved', label: 'Disetujui' }, { value: 'rejected', label: 'Ditolak' }] },
  { key: 'approved_by', label: 'Disetujui oleh', readOnly: true },
];
const OT: Column[] = [
  { key: 'employee_id', label: 'Employee ID', required: true },
  { key: 'work_date', label: 'Tanggal', type: 'date', required: true },
  { key: 'minutes_requested', label: 'Menit diajukan', type: 'number', required: true },
  { key: 'reason', label: 'Alasan', type: 'textarea' },
  { key: 'status', label: 'Status', type: 'select', badge: true, options: [{ value: 'pending', label: 'Menunggu' }, { value: 'approved', label: 'Disetujui' }, { value: 'rejected', label: 'Ditolak' }] },
  { key: 'approved_by', label: 'Disetujui oleh', readOnly: true },
];

export default function Leaves() {
  const [tab, setTab] = useState<'izin' | 'lembur' | 'jenis'>('izin');
  return (
    <Layout title="Izin, Cuti & Lembur" breadcrumb="Pengajuan">
      <div className="alert alert-info">
        Laporan hanya menghitung pengajuan <strong>approved</strong>. Lembur tidak otomatis disetujui
        hanya karena ada kelebihan jam kerja.
      </div>
      <div className="btn-list mb-3">
        <button className={`btn ${tab === 'izin' ? 'btn-primary' : ''}`} onClick={() => setTab('izin')}>Izin / cuti</button>
        <button className={`btn ${tab === 'lembur' ? 'btn-primary' : ''}`} onClick={() => setTab('lembur')}>Lembur</button>
        <button className={`btn ${tab === 'jenis' ? 'btn-primary' : ''}`} onClick={() => setTab('jenis')}>Jenis cuti</button>
      </div>
      {tab === 'izin' && <Crud table="leave_requests" title="Pengajuan izin/cuti" columns={LEAVE} orderBy="start_date" />}
      {tab === 'izin' && <AttachmentCard />}
      {tab === 'izin' && <BalanceCard />}
      {tab === 'lembur' && <Crud table="overtime_requests" title="Pengajuan lembur" columns={OT} orderBy="work_date" />}
      {tab === 'jenis' && <Crud table="leave_types" title="Jenis cuti" columns={LEAVE_TYPE} orderBy="code" />}
    </Layout>
  );
}

function AttachmentCard() {
  const toast = useToast();
  const [reqs, setReqs] = useState<Row[]>([]);
  const [sel, setSel] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    api.tableList('leave_requests', 1, 200, '', 'start_date', 'desc')
      .then((r) => setReqs(r.rows))
      .catch((e: Error) => toast.push('error', e.message));
  }, [toast]);
  const upload = async () => {
    if (!sel) {
      toast.push('error', 'Pilih pengajuan dulu');
      return;
    }
    try {
      const path = await pickFile([{ name: 'Lampiran', extensions: ['pdf', 'jpg', 'jpeg', 'png'] }]);
      if (!path) return;
      setBusy(true);
      const res = await api.saveLeaveAttachment(sel, path);
      toast.push('success', `Lampiran tersimpan: ${res.path}`);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="card mt-3">
      <div className="card-header"><h3 className="card-title">Lampiran pengajuan (pdf/jpg/png, maks 5 MB)</h3></div>
      <div className="card-body d-flex gap-2 align-items-end flex-wrap">
        <div>
          <label className="form-label">Pengajuan</label>
          <select className="form-select" value={sel} onChange={(e) => setSel(e.target.value)}>
            <option value="">— Pilih —</option>
            {reqs.map((x) => (
              <option key={String(x.id)} value={String(x.id)}>
                {String(x.start_date)} — {String(x.employee_id)} ({String(x.status)})
              </option>
            ))}
          </select>
        </div>
        <button className="btn btn-outline-primary" disabled={busy} onClick={upload}>
          {busy ? 'Mengunggah…' : 'Pilih & unggah lampiran'}
        </button>
      </div>
    </div>
  );
}

function BalanceCard() {
  const toast = useToast();
  const [rows, setRows] = useState<LeaveBalance[]>([]);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!open) return;
    api.leaveBalances().then(setRows).catch((e: Error) => toast.push('error', e.message));
  }, [open, toast]);
  return (
    <div className="card mt-3">
      <div className="card-header d-flex justify-content-between align-items-center">
        <h3 className="card-title">Saldo cuti (kuota − terpakai approved)</h3>
        <button className="btn btn-outline-primary btn-sm" onClick={() => setOpen((o) => !o)}>
          {open ? 'Sembunyikan' : 'Tampilkan'}
        </button>
      </div>
      {open && (
        <div className="table-responsive" style={{ maxHeight: 400 }}>
          <table className="table card-table table-vcenter">
            <thead><tr><th>NIK</th><th>Nama</th><th>Jenis</th><th>Kuota</th><th>Terpakai</th><th>Sisa</th></tr></thead>
            <tbody>
              {rows.map((r, i) => (
                <tr key={i} className={r.remaining < 0 ? 'table-danger' : ''}>
                  <td>{r.nik}</td><td>{r.name}</td><td>{r.leave_code}</td>
                  <td>{r.quota}</td><td>{r.used}</td><td><strong>{r.remaining}</strong></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
