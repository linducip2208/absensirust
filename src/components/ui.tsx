import React, { createContext, useContext, useState } from 'react';

export interface Toast {
  id: number;
  kind: 'success' | 'error' | 'info';
  message: string;
}

const ToastCtx = createContext<{ push: (kind: Toast['kind'], message: string) => void }>({
  push: () => {},
});

export const useToast = () => useContext(ToastCtx);

let seq = 1;

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const push = (kind: Toast['kind'], message: string) => {
    const id = seq++;
    setToasts((t) => [...t, { id, kind, message }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 5000);
  };
  return (
    <ToastCtx.Provider value={{ push }}>
      {children}
      <div style={{ position: 'fixed', right: 16, bottom: 16, zIndex: 9999, display: 'flex', flexDirection: 'column', gap: 8, maxWidth: 380 }}>
        {toasts.map((t) => (
          <div key={t.id} className={`alert alert-${t.kind === 'error' ? 'danger' : t.kind} alert-dismissible`} role="alert">
            <div>{t.message}</div>
            <button className="btn-close" onClick={() => setToasts((x) => x.filter((y) => y.id !== t.id))} aria-label="Tutup" />
          </div>
        ))}
      </div>
    </ToastCtx.Provider>
  );
}

export function ConfirmDialog({
  title, body, confirmLabel, onConfirm, onCancel,
}: {
  title: string; body: string; confirmLabel: string;
  onConfirm: () => void; onCancel: () => void;
}) {
  return (
    <div className="modal modal-blur fade show" style={{ display: 'block' }} role="dialog">
      <div className="modal-dialog modal-sm modal-dialog-centered" role="document">
        <div className="modal-content">
          <div className="modal-header">
            <h5 className="modal-title">{title}</h5>
            <button type="button" className="btn-close" onClick={onCancel} aria-label="Tutup" />
          </div>
          <div className="modal-body">{body}</div>
          <div className="modal-footer">
            <button className="btn btn-link link-secondary" onClick={onCancel}>Batal</button>
            <button className="btn btn-danger" onClick={onConfirm}>{confirmLabel}</button>
          </div>
        </div>
      </div>
    </div>
  );
}

export function Field({ label, children, error }: { label: string; children: React.ReactNode; error?: string }) {
  return (
    <div className="mb-3">
      <label className="form-label">{label}</label>
      {children}
      {error && <div className="invalid-feedback d-block">{error}</div>}
    </div>
  );
}

export function EmptyState({ title, hint }: { title: string; hint?: string }) {
  return (
    <div className="empty-state">
      <div className="empty-title">{title}</div>
      {hint && <p className="empty-subtitle text-secondary">{hint}</p>}
    </div>
  );
}

export function LoadingState({ text }: { text?: string }) {
  return (
    <div className="d-flex align-items-center gap-2 p-4">
      <div className="spinner-border spinner-border-sm text-primary" role="status" />
      <span className="text-secondary">{text ?? 'Memuat…'}</span>
    </div>
  );
}

export const STATUS_COLOR: Record<string, string> = {
  Present: 'success', Late: 'warning', Absent: 'danger', Leave: 'info',
  Permission: 'azure', Holiday: 'purple', DayOff: 'secondary',
  Incomplete: 'orange', NeedsReview: 'pink', aktif: 'success', nonaktif: 'secondary',
  online: 'success', offline: 'secondary', error: 'danger',
  approved: 'success', pending: 'warning', rejected: 'danger',
};

export function StatusBadge({ value }: { value: string | number | null }) {
  const v = String(value ?? '-');
  const color = STATUS_COLOR[v] ?? 'secondary';
  return <span className={`badge bg-${color}-lt`}>{v}</span>;
}
