import { useEffect, useRef, useState } from 'react';
import { api, ConflictReport, downloadFile, Row, ScheduleImportRow, ScheduleWeek, todayWIB } from '../api';
import Layout from '../components/Layout';
import { ConfirmDialog, EmptyState, Field, LoadingState, StatusBadge, useToast } from '../components/ui';

function mondayOf(dateStr: string): string {
  const [y, m, d] = dateStr.split('-').map(Number);
  const dt = new Date(Date.UTC(y, m - 1, d));
  const dow = (dt.getUTCDay() + 6) % 7;
  dt.setUTCDate(dt.getUTCDate() - dow);
  return dt.toISOString().slice(0, 10);
}

function addDays(dateStr: string, n: number): string {
  const [y, m, d] = dateStr.split('-').map(Number);
  const dt = new Date(Date.UTC(y, m - 1, d + n));
  return dt.toISOString().slice(0, 10);
}

export default function Schedule() {
  const toast = useToast();  const [weekStart, setWeekStart] = useState(mondayOf(todayWIB()));
  const [data, setData] = useState<ScheduleWeek | null>(null);
  const [loading, setLoading] = useState(true);
  const [shifts, setShifts] = useState<Row[]>([]);
  const [employees, setEmployees] = useState<Row[]>([]);
  const [bulkShift, setBulkShift] = useState('');
  const [bulkEmps, setBulkEmps] = useState<string[]>([]);
  const [bulkStart, setBulkStart] = useState(weekStart);
  const [bulkEnd, setBulkEnd] = useState(addDays(weekStart, 6));
  const [bulkReport, setBulkReport] = useState<string>('');
  const [copyDst, setCopyDst] = useState(addDays(weekStart, 7));
  const [conflicts, setConflicts] = useState<ConflictReport | null>(null);
  const [swaps, setSwaps] = useState<Row[]>([]);
  const [swapForm, setSwapForm] = useState({ employee_id: '', from_shift_id: '', to_shift_id: '', swap_date: todayWIB(), reason: '' });
  const [busy, setBusy] = useState(false);
  const [impCsv, setImpCsv] = useState('');
  const [impPreview, setImpPreview] = useState<{ rows: ScheduleImportRow[]; errors: string[] } | null>(null);
  const [deciding, setDeciding] = useState<{ id: string; approve: boolean } | null>(null);
  const [swapBusy, setSwapBusy] = useState(false);

  const empName = (id: string) => employees.find((x) => String(x.id) === id)?.name ?? id;
  const shiftCode = (id: string) => shifts.find((x) => String(x.id) === id)?.code ?? id;

  const myReq = useRef(0);
  const load = async (ws: string) => {
    const id = ++myReq.current;
    setLoading(true);
    // Kartu kanan (bentrok/tukar) direset dulu: dulu bila scheduleConflicts
    // gagal, kartu menampilkan loading berputar/hasil minggu lama selamanya.
    setConflicts(null);
    try {
      const [w, s, e] = await Promise.all([
        api.scheduleWeek(ws),
        api.tableList('shifts', 1, 100, '', 'code', 'asc'),
        api.tableList('employees', 1, 200, '', 'name', 'asc'),
      ]);
      if (id !== myReq.current) return;
      setData(w);
      setShifts(s.rows.filter((r) => String(r.is_active) === '1'));
      setEmployees(e.rows);
    } catch (err) {
      toast.push('error', err instanceof Error ? err.message : String(err));
      if (id === myReq.current) { setLoading(false); }
      return;
    }
    const c = await api.scheduleConflicts(ws, addDays(ws, 6)).catch(() => null);
    if (id !== myReq.current) return;
    setConflicts(c);
    const sw = await api
      .tableList('shift_swaps', 1, 50, '', 'created_at', 'desc')
      .catch(() => null);
    if (id !== myReq.current) return;
    setSwaps(sw?.rows ?? []);
    if (id === myReq.current) setLoading(false);
  };

  useEffect(() => {
    load(weekStart).catch(() => {});
  }, [weekStart]);

  const doBulk = async (dryRun: boolean) => {
    if (!bulkShift || bulkEmps.length === 0) {
      toast.push('error', 'Pilih shift dan minimal satu karyawan');
      return;
    }
    setBusy(true);
    try {
      const r = await api.scheduleBulk(bulkEmps, bulkShift, bulkStart, bulkEnd, dryRun);
      const skip = Array.isArray(r.skipped) ? r.skipped : [];
      setBulkReport(
        `${dryRun ? '[PRATINJAU] ' : ''}${r.created} akan dibuat/dibuat, ${skip.length} dilewati. ` +
        skip.map((s) => `${s.employee_id}: ${s.reason}`).join(' | '),
      );
      if (!dryRun) await load(weekStart);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const doCopy = async (dryRun: boolean) => {
    setBusy(true);
    try {
      const r = await api.scheduleCopyWeek(weekStart, copyDst, dryRun);
      toast.push('success', `${dryRun ? '[Pratinjau] ' : ''}Disalin: ${r.copied}, dilewati: ${typeof r.skipped === 'number' ? r.skipped : 0}`);
      if (!dryRun) await load(weekStart);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const doSwapRequest = async () => {
    if (!swapForm.employee_id || !swapForm.from_shift_id || !swapForm.to_shift_id) {
      toast.push('error', 'Lengkapi pengajuan tukar shift');
      return;
    }
    if (swapBusy) return;
    setSwapBusy(true);
    try {
      await api.swapRequest(swapForm.employee_id, swapForm.from_shift_id, swapForm.to_shift_id, swapForm.swap_date, swapForm.reason);
      toast.push('success', 'Pengajuan tukar shift dibuat (menunggu persetujuan)');
      setSwapForm({ employee_id: '', from_shift_id: '', to_shift_id: '', swap_date: todayWIB(), reason: '' });
      await load(weekStart);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setSwapBusy(false);
    }
  };

  const doSwapDecide = async (id: string, approve: boolean) => {
    setDeciding(null);
    try {
      await api.swapDecide(id, approve);
      toast.push('success', approve ? 'Disetujui' : 'Ditolak');
      await load(weekStart);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  const doExport = async () => {
    try {
      const r = await api.scheduleExport(weekStart, addDays(weekStart, 6));
      downloadFile(`jadwal-${weekStart}.csv`, '\uFEFF' + r.csv, 'text/csv;charset=utf-8');
      toast.push('success', `${r.count} penugasan diekspor`);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  const doImpPreview = async () => {
    if (!impCsv.trim()) {
      toast.push('error', 'Tempel CSV dulu (nik,shift,efektif,akhir)');
      return;
    }
    setBusy(true);
    try {
      setImpPreview(await api.scheduleImportPreview(impCsv));
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const doImpCommit = async () => {
    if (!impPreview) return;
    const valid = impPreview.rows
      .filter((r) => r.errors.length === 0)
      .map((r) => ({ employee_id: r.employee_id, shift_id: r.shift_id, eff: r.eff, end: r.end }));
    if (valid.length === 0) {
      toast.push('error', 'Tidak ada baris valid');
      return;
    }
    setBusy(true);
    try {
      const r = await api.scheduleImportCommit(valid);
      toast.push(r.fail > 0 ? 'info' : 'success', `Impor jadwal: ${r.ok} ok, ${r.fail} gagal`);
      if (r.messages.length > 0) toast.push('error', r.messages.slice(0, 5).join(' | '));
      await load(weekStart);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Layout title="Jadwal & Kalender Shift" breadcrumb="Waktu kerja">
      <div className="card mb-3">
        <div className="card-body d-flex gap-2 align-items-end flex-wrap">
          <div>
            <label className="form-label">Minggu mulai (Senin)</label>
            <input type="date" className="form-control" value={weekStart} onChange={(e) => setWeekStart(mondayOf(e.target.value))} />
          </div>
          <button className="btn btn-outline-primary" onClick={() => setWeekStart(mondayOf(todayWIB()))}>Minggu ini</button>
          <button className="btn btn-outline-primary" onClick={() => setWeekStart(addDays(weekStart, -7))}>← Mundur</button>
          <button className="btn btn-outline-primary" onClick={() => setWeekStart(addDays(weekStart, 7))}>Maju →</button>
        </div>
      </div>

      {loading || !data ? (
        <LoadingState text="Memuat kalender…" />
      ) : (
        <div className="card mb-3">
          <div className="card-header"><h3 className="card-title">Kalender mingguan ({data.days[0]} s/d {data.days[6]})</h3></div>
          <div className="table-responsive">
            <table className="table card-table table-vcenter">
              <thead><tr><th>Karyawan</th>{data.days.map((d) => <th key={d}>{d.slice(5)}</th>)}</tr></thead>
            <tbody>
              {data.rows.length === 0 && (
                <tr><td colSpan={8}><EmptyState title="Belum ada penugasan minggu ini" hint="Gunakan penugasan massal, salin minggu, atau impor CSV." /></td></tr>
              )}
              {data.rows.map((r) => (
                  <tr key={r.employee_id}>
                    <td>{r.name}</td>
                    {data.days.map((d) => (
                      <td key={d}>{r.cells[d] ? <StatusBadge value={r.cells[d] as string} /> : <span className="text-secondary">—</span>}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {data.empty_days.length > 0 && (
            <div className="card-footer text-secondary">Hari tanpa penugasan sama sekali: {data.empty_days.join(', ')}</div>
          )}
        </div>
      )}

      <div className="row">
        <div className="col-lg-6">
          <div className="card mb-3">
            <div className="card-header"><h3 className="card-title">Penugasan massal (dengan pratinjau dampak)</h3></div>
            <div className="card-body">
              <Field label="Shift">
                <select className="form-select" value={bulkShift} onChange={(e) => setBulkShift(e.target.value)}>
                  <option value="">— Pilih —</option>
                  {shifts.map((s) => <option key={String(s.id)} value={String(s.id)}>{String(s.code)} — {String(s.name)}</option>)}
                </select>
              </Field>
              <div className="row">
                <div className="col-6"><Field label="Dari"><input type="date" className="form-control" value={bulkStart} onChange={(e) => setBulkStart(e.target.value)} /></Field></div>
                <div className="col-6"><Field label="Sampai"><input type="date" className="form-control" value={bulkEnd} onChange={(e) => setBulkEnd(e.target.value)} /></Field></div>
              </div>
              <Field label="Karyawan (tahan Ctrl untuk banyak)">
                <select multiple className="form-select" style={{ height: 140 }} value={bulkEmps} onChange={(e) => setBulkEmps(Array.from(e.target.selectedOptions, (o) => o.value))}>
                  {employees.map((x) => <option key={String(x.id)} value={String(x.id)}>{String(x.name)}</option>)}
                </select>
              </Field>
              <div className="btn-list">
                <button className="btn btn-outline-primary" disabled={busy} onClick={() => doBulk(true)}>Pratinjau dampak</button>
                <button className="btn btn-primary" disabled={busy} onClick={() => doBulk(false)}>Simpan penugasan</button>
              </div>
              {bulkReport && <div className="alert alert-info mt-3 mb-0">{bulkReport}</div>}
              <hr />
              <Field label="Salin minggu ini ke minggu mulai">
                <input type="date" className="form-control" value={copyDst} onChange={(e) => setCopyDst(e.target.value)} />
              </Field>
              <div className="btn-list">
                <button className="btn btn-outline-primary" disabled={busy} onClick={() => doCopy(true)}>Pratinjau salinan</button>
                <button className="btn btn-outline-primary" disabled={busy} onClick={() => doCopy(false)}>Salin minggu</button>
                <button className="btn btn-outline-primary" disabled={busy} onClick={doExport}>Ekspor CSV minggu ini</button>
              </div>
              <hr />
              <Field label="Impor jadwal CSV (header: nik,shift(nama kolom bebas — urutan: nik,shift,efektif,akhir))">
                <textarea className="form-control" rows={4} value={impCsv} onChange={(e) => setImpCsv(e.target.value)}
                  placeholder={'001,PAGI,2026-11-02,2026-11-08'} />
              </Field>
              <div className="btn-list">
                <button className="btn btn-outline-primary" disabled={busy} onClick={doImpPreview}>Preview impor</button>
                {impPreview && <button className="btn btn-primary" disabled={busy} onClick={doImpCommit}>Impor baris valid</button>}
              </div>
              {impPreview && impPreview.errors.length > 0 && (
                <div className="alert alert-danger mt-2 mb-0">
                  <ul className="mb-0">{impPreview.errors.slice(0, 8).map((e, i) => <li key={i}>{e}</li>)}</ul>
                </div>
              )}
            </div>
          </div>
        </div>
        <div className="col-lg-6">
          <div className="card mb-3">
            <div className="card-header"><h3 className="card-title">Deteksi bentrok & jadwal kosong</h3></div>
            <div className="card-body">
              {!conflicts ? <LoadingState /> : (
                <>
                  <h4>Bentrok ({conflicts.overlaps.length})</h4>
                  {conflicts.overlaps.length === 0 ? <div className="text-secondary mb-2">Tidak ada shift tumpang tindih.</div> : (
                    <ul className="list-group mb-3">
                      {conflicts.overlaps.map((o) => (
                        <li className="list-group-item" key={o.employee_id}>
                          <strong>{o.name}</strong>: {o.shift_a} vs {o.shift_b} (sejak {o.since})
                        </li>
                      ))}
                    </ul>
                  )}
                  <h4>Jadwal kosong ({conflicts.empty.length})</h4>
                  {conflicts.empty.length === 0 ? <div className="text-secondary">Semua karyawan terjadwal.</div> : (
                    <ul className="list-group">
                      {conflicts.empty.slice(0, 30).map((x, i) => (
                        <li className="list-group-item" key={i}>{x.date} — {x.name}</li>
                      ))}
                    </ul>
                  )}
                </>
              )}
            </div>
          </div>
        </div>
      </div>

      <div className="card">
        <div className="card-header"><h3 className="card-title">Tukar shift (dengan persetujuan)</h3></div>
        <div className="card-body">
          <div className="row g-2">
            <div className="col-md-3">
              <Field label="Karyawan">
                <select className="form-select" value={swapForm.employee_id} onChange={(e) => setSwapForm({ ...swapForm, employee_id: e.target.value })}>
                  <option value="">— Pilih —</option>
                  {employees.map((x) => <option key={String(x.id)} value={String(x.id)}>{String(x.name)}</option>)}
                </select>
              </Field>
            </div>
            <div className="col-md-2">
              <Field label="Dari shift">
                <select className="form-select" value={swapForm.from_shift_id} onChange={(e) => setSwapForm({ ...swapForm, from_shift_id: e.target.value })}>
                  <option value="">—</option>
                  {shifts.map((s) => <option key={String(s.id)} value={String(s.id)}>{String(s.code)}</option>)}
                </select>
              </Field>
            </div>
            <div className="col-md-2">
              <Field label="Ke shift">
                <select className="form-select" value={swapForm.to_shift_id} onChange={(e) => setSwapForm({ ...swapForm, to_shift_id: e.target.value })}>
                  <option value="">—</option>
                  {shifts.map((s) => <option key={String(s.id)} value={String(s.id)}>{String(s.code)}</option>)}
                </select>
              </Field>
            </div>
            <div className="col-md-2"><Field label="Tanggal"><input type="date" className="form-control" value={swapForm.swap_date} onChange={(e) => setSwapForm({ ...swapForm, swap_date: e.target.value })} /></Field></div>
            <div className="col-md-3"><Field label="Alasan"><input className="form-control" value={swapForm.reason} onChange={(e) => setSwapForm({ ...swapForm, reason: e.target.value })} /></Field></div>
          </div>
          <button className="btn btn-primary" onClick={doSwapRequest} disabled={swapBusy}>
            {swapBusy ? 'Mengajukan…' : 'Ajukan tukar shift'}
          </button>
          <hr />
          <div className="table-responsive">
            <table className="table table-vcenter">
              <thead><tr><th>Tanggal</th><th>Karyawan</th><th>Dari → Ke</th><th>Status</th><th>Aksi</th></tr></thead>
              <tbody>
                {swaps.length === 0 && (
                  <tr><td colSpan={5}><EmptyState title="Belum ada pengajuan tukar shift" hint="" /></td></tr>
                )}
                {swaps.map((w) => (
                  <tr key={String(w.id)}>
                    <td>{String(w.swap_date)}</td><td>{empName(String(w.employee_id))}</td>
                    <td>{shiftCode(String(w.from_shift_id))} → {shiftCode(String(w.to_shift_id))}</td>
                    <td><StatusBadge value={w.status} /></td>
                    <td>
                      {String(w.status) === 'pending' && (
                        <div className="btn-list flex-nowrap">
                          <button className="btn btn-sm btn-success" onClick={() => setDeciding({ id: String(w.id), approve: true })}>Setujui</button>
                          <button className="btn btn-sm btn-outline-danger" onClick={() => setDeciding({ id: String(w.id), approve: false })}>Tolak</button>
                        </div>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {deciding && (
            <ConfirmDialog
              title={deciding.approve ? 'Setujui tukar shift?' : 'Tolak tukar shift?'}
              body={deciding.approve ? 'Penugasan satu hari akan dibuat.' : 'Pengajuan akan ditandai ditolak.'}
              confirmLabel={deciding.approve ? 'Setujui' : 'Tolak'}
              onCancel={() => setDeciding(null)}
              onConfirm={() => doSwapDecide(deciding.id, deciding.approve)}
            />
          )}
        </div>
      </div>
    </Layout>
  );
}
