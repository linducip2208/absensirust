import { useCallback, useEffect, useState } from 'react';
import { api, Row } from '../api';
import { ConfirmDialog, EmptyState, Field, LoadingState, StatusBadge, useToast } from './ui';

export interface Column {
  key: string;
  label: string;
  type?: 'text' | 'number' | 'date' | 'datetime' | 'select' | 'textarea' | 'hidden' | 'password';
  options?: { value: string; label: string }[];
  required?: boolean;
  readOnly?: boolean;
  badge?: boolean;
  hideInTable?: boolean;
}

export default function Crud({
  table, title, columns, orderBy, canWrite = true, allowHardDelete = false,
}: {
  table: string; title: string; columns: Column[]; orderBy?: string;
  canWrite?: boolean; allowHardDelete?: boolean;
}) {
  const toast = useToast();
  const [rows, setRows] = useState<Row[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState('');
  const [searchInput, setSearchInput] = useState('');
  const [order, setOrder] = useState(orderBy ?? '');
  const [dir, setDir] = useState<'asc' | 'desc'>('asc');
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState<Row | null>(null);
  const [isNew, setIsNew] = useState(false);
  const [form, setForm] = useState<Record<string, string>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [deleting, setDeleting] = useState<Row | null>(null);
  const perPage = 15;

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await api.tableList(table, page, perPage, search, order, dir);
      setRows(res.rows);
      setTotal(res.total);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [table, page, search, order, dir, toast]);

  useEffect(() => {
    load().catch(() => {});
  }, [load]);

  const openNew = () => {
    const init: Record<string, string> = {};
    for (const c of columns) init[c.key] = '';
    setForm(init);
    setErrors({});
    setEditing({});
    setIsNew(true);
  };

  const openEdit = async (row: Row) => {
    try {
      const full = await api.tableGet(table, String(row.id));
      const init: Record<string, string> = {};
      for (const c of columns) init[c.key] = full[c.key] == null ? '' : String(full[c.key]);
      setForm(init);
      setErrors({});
      setEditing(row);
      setIsNew(false);
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  const validate = (): boolean => {
    const errs: Record<string, string> = {};
    for (const c of columns) {
      if (c.required && !form[c.key]?.trim()) errs[c.key] = 'Wajib diisi';
      if (c.key === 'email' && form.email && !form.email.includes('@')) errs.email = 'Email tidak valid';
    }
    setErrors(errs);
    return Object.keys(errs).length === 0;
  };

  const save = async () => {
    if (!validate()) return;
    try {
      const payload: Record<string, unknown> = {};
      for (const c of columns) {
        if (c.readOnly && !isNew) continue;
        if (c.type === 'hidden') continue;
        payload[c.key] = form[c.key] ?? '';
      }
      if (isNew) {
        await api.tableCreate(table, payload);
        toast.push('success', 'Data berhasil ditambahkan');
      } else {
        await api.tableUpdate(table, String((editing as Row).id), payload);
        toast.push('success', 'Data berhasil diperbarui');
      }
      setEditing(null);
      await load();
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  const doDelete = async (hard: boolean) => {
    if (!deleting) return;
    try {
      await api.tableDelete(table, String(deleting.id), hard);
      toast.push('success', hard ? 'Data dihapus permanen' : 'Data dinonaktifkan/dihapus');
      setDeleting(null);
      await load();
    } catch (e) {
      toast.push('error', e instanceof Error ? e.message : String(e));
    }
  };

  const visible = columns.filter((c) => !c.hideInTable && c.type !== 'hidden');
  const pages = Math.max(1, Math.ceil(total / perPage));

  return (
    <div className="card">
      <div className="card-header d-flex justify-content-between align-items-center">
        <h3 className="card-title">{title}</h3>
        <div className="d-flex gap-2">
          <form
            className="d-flex gap-2"
            onSubmit={(e) => {
              e.preventDefault();
              setPage(1);
              setSearch(searchInput);
            }}
          >
            <input
              className="form-control"
              placeholder="Cari…"
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
            />
            <button className="btn btn-outline-primary" type="submit">Cari</button>
          </form>
          {canWrite && (
            <button className="btn btn-primary" onClick={openNew}>Tambah</button>
          )}
        </div>
      </div>
      {loading ? (
        <LoadingState />
      ) : rows.length === 0 ? (
        <EmptyState title="Belum ada data" hint="Klik Tambah untuk membuat data baru." />
      ) : (
        <div className="table-responsive">
          <table className="table table-vcenter card-table">
            <thead>
              <tr>
                {visible.map((c) => (
                  <th
                    key={c.key}
                    style={{ cursor: 'pointer' }}
                    title="Klik untuk urutkan"
                    onClick={() => {
                      if (order === c.key) {
                        setDir((d) => (d === 'asc' ? 'desc' : 'asc'));
                      } else {
                        setOrder(c.key);
                        setDir('asc');
                      }
                      setPage(1);
                    }}
                  >
                    {c.label}{order === c.key ? (dir === 'asc' ? ' ▲' : ' ▼') : ''}
                  </th>
                ))}
                {canWrite && <th className="w-1">Aksi</th>}
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={String(r.id)}>
                  {visible.map((c) => (
                    <td key={c.key}>
                      {c.badge ? <StatusBadge value={r[c.key]} /> : String(r[c.key] ?? '-')}
                    </td>
                  ))}
                  {canWrite && (
                    <td>
                      <div className="btn-list flex-nowrap">
                        <button className="btn btn-sm btn-outline-primary" onClick={() => openEdit(r)}>Ubah</button>
                        <button className="btn btn-sm btn-outline-danger" onClick={() => setDeleting(r)}>Hapus</button>
                      </div>
                    </td>
                  )}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <div className="card-footer d-flex justify-content-between align-items-center">
        <span className="text-secondary">Total {total} data · halaman {page}/{pages}</span>
        <div className="btn-list">
          <button className="btn btn-sm" disabled={page <= 1} onClick={() => setPage((p) => p - 1)}>← Sebelumnya</button>
          <button className="btn btn-sm" disabled={page >= pages} onClick={() => setPage((p) => p + 1)}>Berikutnya →</button>
        </div>
      </div>

      {editing && (
        <div className="modal modal-blur fade show" style={{ display: 'block' }} role="dialog">
          <div className="modal-dialog modal-lg modal-dialog-centered" role="document">
            <div className="modal-content">
              <div className="modal-header">
                <h5 className="modal-title">{isNew ? `Tambah ${title}` : `Ubah ${title}`}</h5>
                <button type="button" className="btn-close" onClick={() => setEditing(null)} aria-label="Tutup" />
              </div>
              <div className="modal-body">
                <div className="row">
                  {columns.filter((c) => c.type !== 'hidden' && !(c.readOnly && isNew && c.key === 'id')).map((c) => (
                    <div className="col-md-6" key={c.key}>
                      <Field label={`${c.label}${c.required ? ' *' : ''}`} error={errors[c.key]}>
                        {c.type === 'select' ? (
                          <select
                            className={`form-select${errors[c.key] ? ' is-invalid' : ''}`}
                            value={form[c.key] ?? ''}
                            disabled={c.readOnly && !isNew}
                            onChange={(e) => setForm((f) => ({ ...f, [c.key]: e.target.value }))}
                          >
                            <option value="">— Pilih —</option>
                            {c.options?.map((o) => (
                              <option key={o.value} value={o.value}>{o.label}</option>
                            ))}
                          </select>
                        ) : c.type === 'textarea' ? (
                          <textarea
                            className={`form-control${errors[c.key] ? ' is-invalid' : ''}`}
                            value={form[c.key] ?? ''}
                            disabled={c.readOnly && !isNew}
                            onChange={(e) => setForm((f) => ({ ...f, [c.key]: e.target.value }))}
                          />
                        ) : (
                          <input
                            className={`form-control${errors[c.key] ? ' is-invalid' : ''}`}
                            type={c.type === 'number' ? 'number' : c.type === 'date' ? 'date' : c.type === 'datetime' ? 'datetime-local' : c.type === 'password' ? 'password' : 'text'}
                            autoComplete={c.type === 'password' ? 'new-password' : undefined}
                            value={form[c.key] ?? ''}
                            disabled={c.readOnly && !isNew}
                            onChange={(e) => setForm((f) => ({ ...f, [c.key]: e.target.value }))}
                          />
                        )}
                      </Field>
                    </div>
                  ))}
                </div>
              </div>
              <div className="modal-footer">
                <button className="btn btn-link link-secondary" onClick={() => setEditing(null)}>Batal</button>
                <button className="btn btn-primary" onClick={save}>Simpan</button>
              </div>
            </div>
          </div>
        </div>
      )}

      {deleting && (
        <ConfirmDialog
          title="Hapus data?"
          body={`Hapus "${String((deleting as Row).name ?? (deleting as Row).code ?? deleting.id)}"? Untuk karyawan: hapus biasa = nonaktif (soft delete, riwayat audit tetap ada).`}
          confirmLabel={allowHardDelete ? 'Hapus (nonaktif)' : 'Hapus'}
          onCancel={() => setDeleting(null)}
          onConfirm={() => doDelete(false)}
        />
      )}
    </div>
  );
}
