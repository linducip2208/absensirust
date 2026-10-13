import { useState } from 'react';
import { api, base64ToBytes, downloadFile, ReportFilter, todayWIB } from '../api';
import Layout from '../components/Layout';
import { Field, LoadingState, useToast } from '../components/ui';

const KINDS: { value: string; label: string }[] = [
  { value: 'harian', label: 'Absensi harian' },
  { value: 'rekap_bulanan', label: 'Rekap bulanan' },
  { value: 'karyawan', label: 'Per karyawan' },
  { value: 'departemen', label: 'Per departemen' },
  { value: 'cabang', label: 'Per cabang' },
  { value: 'shift', label: 'Per shift' },
  { value: 'terlambat', label: 'Keterlambatan' },
  { value: 'pulang_cepat', label: 'Pulang cepat' },
  { value: 'lembur', label: 'Lembur disetujui' },
  { value: 'tidak_hadir', label: 'Tidak hadir' },
  { value: 'izin_cuti', label: 'Izin & cuti' },
  { value: 'rekon', label: 'Rekonsiliasi raw vs hasil' },
  { value: 'log_perangkat', label: 'Log perangkat' },
  { value: 'error_sync', label: 'Error sinkronisasi' },
  { value: 'riwayat_koreksi', label: 'Riwayat koreksi' },
];

export default function Reports() {
  const toast = useToast();
  const [kind, setKind] = useState('harian');
  const [f, setF] = useState<ReportFilter>({
    start: todayWIB(), end: todayWIB(), branchId: '', departmentId: '',
    shiftId: '', status: '', employeeId: '',
  });
  const [headers, setHeaders] = useState<string[]>([]);
  const [rows, setRows] = useState<string[][]>([]);
  const [busy, setBusy] = useState(false);

  const set = (k: keyof ReportFilter) => (e: React.ChangeEvent<HTMLInputElement>) =>
    setF((prev) => ({ ...prev, [k]: e.target.value }));

  const query = async () => {
    setBusy(true);
    try {
      const res = await api.reportQuery(kind, f);
      setHeaders(res.headers);
      setRows(res.rows);
      if (res.rows.length === 0) toast.push('info', 'Tidak ada data pada filter tersebut');
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const exp = async (format: 'csv' | 'xlsx' | 'pdf') => {
    try {
      const res = await api.reportExport(kind, format, `Laporan ${kind}`, f);
      if (format === 'csv' && res.text) {
        downloadFile(`laporan-${kind}.csv`, '\uFEFF' + res.text, 'text/csv;charset=utf-8');
      } else if (res.base64) {
        const mime = format === 'xlsx'
          ? 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
          : 'application/pdf';
        downloadFile(`laporan-${kind}.${format}`, base64ToBytes(res.base64), mime);
      }
      toast.push('success', `Laporan ${format.toUpperCase()} diunduh`);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <Layout title="Laporan" breadcrumb="Analitik">
      <div className="card mb-3">
        <div className="card-body">
          <div className="row g-2">
            <div className="col-md-3">
              <Field label="Jenis laporan">
                <select className="form-select" value={kind} onChange={(e) => setKind(e.target.value)}>
                  {KINDS.map((k) => <option key={k.value} value={k.value}>{k.label}</option>)}
                </select>
              </Field>
            </div>
            <div className="col-md-2"><Field label="Dari"><input type="date" className="form-control" value={f.start} onChange={set('start')} /></Field></div>
            <div className="col-md-2"><Field label="Sampai"><input type="date" className="form-control" value={f.end} onChange={set('end')} /></Field></div>
            <div className="col-md-2"><Field label="Status"><input className="form-control" value={f.status} onChange={set('status')} placeholder="Present/Late/…" /></Field></div>
            <div className="col-md-3"><Field label="ID karyawan"><input className="form-control" value={f.employeeId} onChange={set('employeeId')} /></Field></div>
          </div>
          <div className="btn-list">
            <button className="btn btn-primary" disabled={busy} onClick={query}>Tampilkan</button>
            <button className="btn btn-outline-primary" onClick={() => exp('csv')}>Ekspor CSV</button>
            <button className="btn btn-outline-primary" onClick={() => exp('xlsx')}>Ekspor XLSX</button>
            <button className="btn btn-outline-primary" onClick={() => exp('pdf')}>Ekspor PDF</button>
          </div>
          {busy && <LoadingState />}
        </div>
      </div>
      {headers.length > 0 && (
        <div className="card">
          <div className="card-header"><h3 className="card-title">Hasil ({rows.length} baris, tanggal WIB)</h3></div>
          <div className="table-responsive" style={{ maxHeight: 520 }}>
            <table className="table card-table table-vcenter">
              <thead><tr>{headers.map((h) => <th key={h}>{h}</th>)}</tr></thead>
              <tbody>
                {rows.slice(0, 500).map((r, i) => (
                  <tr key={i}>{r.map((c, j) => <td key={j}>{c}</td>)}</tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </Layout>
  );
}
