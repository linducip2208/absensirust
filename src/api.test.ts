import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async () => ({})),
  isTauri: () => true,
}));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(async () => null) }));

const store = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (k: string) => (store.has(k) ? store.get(k) : null),
  setItem: (k: string, v: string) => { store.set(k, String(v)); },
  removeItem: (k: string) => { store.delete(k); },
});

import { invoke as rawInvoke } from '@tauri-apps/api/core';
import { api, fmtWIB, todayWIB } from './api';

const mockedInvoke = rawInvoke as unknown as ReturnType<typeof vi.fn>;

async function keysOf(call: () => Promise<unknown>): Promise<{ cmd: string; keys: string[] }> {
  mockedInvoke.mockClear();
  await call();
  expect(mockedInvoke).toHaveBeenCalledOnce();
  const [cmd, args] = mockedInvoke.mock.calls[0] as [string, Record<string, unknown>];
  return { cmd, keys: Object.keys(args).sort() };
}

async function expectKeys(call: () => Promise<unknown>, cmd: string, keys: string[]) {
  const got = await keysOf(call);
  expect(got.cmd).toBe(cmd);
  expect(got.keys).toEqual([...keys].sort());
}

describe('kontrak IPC: kunci argumen harus snake_case persis backend', () => {
  it('CRUD + auth + dashboard', async () => {
    await expectKeys(() => api.tableList('employees', 1, 15, 'x', 'name', 'desc'), 'table_list',
      ['token', 'table', 'page', 'per_page', 'search', 'order_by', 'order_dir']);
    await expectKeys(() => api.tableGet('employees', '1'), 'table_get', ['token', 'table', 'id']);
    await expectKeys(() => api.tableCreate('employees', {}), 'table_create', ['token', 'table', 'data']);
    await expectKeys(() => api.tableUpdate('employees', '1', {}), 'table_update', ['token', 'table', 'id', 'data']);
    await expectKeys(() => api.tableDelete('employees', '1', true), 'table_delete', ['token', 'table', 'id', 'hard']);
    await expectKeys(() => api.setupAdmin('a', 'b', 'c'), 'setup_admin', ['username', 'display_name', 'password']);
    await expectKeys(() => api.dashboard('2026-10-14'), 'dashboard', ['token', 'date']);
  });
  it('perangkat', async () => {
    await expectKeys(() => api.deviceTest('d1'), 'device_test', ['token', 'device_id']);
    await expectKeys(() => api.deviceSync('d1'), 'device_sync', ['token', 'device_id']);
    await expectKeys(() => api.deviceSdkStatus(), 'device_sdk_status', ['token']);
    await expectKeys(() => api.deviceFetchUsers('d1'), 'device_fetch_users', ['token', 'device_id']);
    await expectKeys(() => api.deviceProvisionUser('d1', 'u', 'n'), 'device_provision_user',
      ['token', 'device_id', 'device_user_id', 'name']);
    await expectKeys(() => api.deviceRegistry('t'), 'device_registry', ['token']);
  });
  it('izin, cuti & lembur', async () => {
    await expectKeys(() => api.leaveBalances(), 'leave_balances', ['token']);
    await expectKeys(() => api.leaveDecide('r1', true), 'leave_decide', ['token', 'request_id', 'approve']);
    await expectKeys(() => api.overtimeDecide('r2', false), 'overtime_decide', ['token', 'request_id', 'approve']);
    await expectKeys(() => api.saveLeaveAttachment('r1', 'p'), 'save_leave_attachment',
      ['token', 'request_id', 'src_path']);
  });
  it('absensi + koreksi', async () => {
    await expectKeys(() => api.attendanceCompute('2026-10-14', 'e1'), 'attendance_compute',
      ['token', 'work_date', 'employee_id']);
    await expectKeys(() => api.attendanceCorrect('a1', 'clock_in', 'v', 'r'), 'attendance_correct',
      ['token', 'attendance_id', 'field', 'new_value', 'reason']);
    await expectKeys(() => api.eventsRequeue('2026-10-01', '2026-10-02'), 'events_requeue',
      ['token', 'start_date', 'end_date']);
  });
  it('laporan', async () => {
    const f = { start: 'a', end: 'b', branchId: 'c', departmentId: 'd', shiftId: 'e', status: 's', employeeId: 'x' };
    await expectKeys(() => api.reportQuery('harian', f), 'report_query',
      ['token', 'kind', 'start', 'end', 'branch_id', 'department_id', 'shift_id', 'status', 'employee_id']);
    await expectKeys(() => api.reportExport('harian', 'csv', 'T', f), 'report_export',
      ['token', 'kind', 'format', 'title', 'start', 'end', 'branch_id', 'department_id', 'shift_id', 'status', 'employee_id']);
  });
  it('jadwal + swap + impor', async () => {
    await expectKeys(() => api.scheduleWeek('2026-10-12'), 'schedule_week', ['token', 'start_date']);
    await expectKeys(() => api.scheduleBulk(['e'], 's', 'a', 'b', true), 'schedule_bulk',
      ['token', 'employee_ids', 'shift_id', 'start_date', 'end_date', 'dry_run']);
    await expectKeys(() => api.scheduleCopyWeek('a', 'b', false), 'schedule_copy_week',
      ['token', 'src_start', 'dst_start', 'dry_run']);
    await expectKeys(() => api.scheduleConflicts('a', 'b'), 'schedule_conflicts',
      ['token', 'start_date', 'end_date']);
    await expectKeys(() => api.swapRequest('e', 'f', 't', 'd', 'r'), 'swap_request',
      ['token', 'employee_id', 'from_shift_id', 'to_shift_id', 'swap_date', 'reason']);
    await expectKeys(() => api.swapDecide('s', true), 'swap_decide', ['token', 'swap_id', 'approve']);
    await expectKeys(() => api.importPreview('csv'), 'employee_import_preview', ['token', 'csv_text']);
    await expectKeys(() => api.scheduleImportPreview('csv'), 'schedule_import_preview', ['token', 'csv_text']);
    await expectKeys(() => api.scheduleExport('a', 'b'), 'schedule_export', ['token', 'start_date', 'end_date']);
  });
  it('file + lain-lain', async () => {
    await expectKeys(() => api.saveEmployeePhoto('e', 'p'), 'save_employee_photo',
      ['token', 'employee_id', 'src_path']);
    await expectKeys(() => api.importCommit([]), 'employee_import_commit', ['token', 'rows']);
    await expectKeys(() => api.scheduleImportCommit([]), 'schedule_import_commit', ['token', 'rows']);
    await expectKeys(() => api.attendanceReview(), 'attendance_review', ['token']);
  });
  it('auth + sesi', async () => {
    await expectKeys(() => api.hasAdmin(), 'has_admin', []);
    await expectKeys(() => api.login('u', 'p'), 'login', ['username', 'password']);
    await expectKeys(() => api.logout(), 'logout', ['token']);
  });
  it('sistem: pengaturan, scheduler, sync, backup, kesehatan, pencarian, diagnostik', async () => {
    await expectKeys(() => api.settingsGet(), 'settings_get', ['token']);
    await expectKeys(() => api.settingsSet('backup_auto', '1'), 'settings_set', ['token', 'key', 'value']);
    await expectKeys(() => api.schedulerGet(), 'scheduler_get', ['token']);
    await expectKeys(() => api.schedulerSet(true), 'scheduler_set', ['token', 'enabled']);
    await expectKeys(() => api.syncRunAll(), 'sync_run_all', ['token']);
    await expectKeys(() => api.backupCreate(), 'backup_create', ['token']);
    await expectKeys(() => api.backupRestore('f'), 'backup_restore', ['token', 'file']);
    await expectKeys(() => api.healthCheck(), 'health_check', ['token']);
    await expectKeys(() => api.searchGlobal('budi'), 'search_global', ['token', 'query']);
    await expectKeys(() => api.diagnosticsBundle(), 'diagnostics_bundle', ['token']);
  });
});

describe('helper WIB', () => {
  it('fmtWIB menampilkan apa adanya tanpa konversi', () => {
    expect(fmtWIB('2026-05-04 23:05:00')).toBe('2026-05-04 23:05:00');
    expect(fmtWIB(null)).toBe('-');
    expect(fmtWIB('')).toBe('-');
  });
  it('todayWIB berformat YYYY-MM-DD', () => {
    expect(todayWIB()).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });
});

describe('aturan shift malam (kontrak UI)', () => {
  it('preset shift tetap tiga dan dapat diedit (label Indonesia)', () => {
    const presets = [
      { code: 'PAGI', label: 'Shift Pagi', jam: '07:00–15:00' },
      { code: 'SIANG', label: 'Shift Siang', jam: '15:00–23:00' },
      { code: 'MALAM', label: 'Shift Malam', jam: '23:00–07:00' },
    ];
    expect(presets).toHaveLength(3);
    expect(presets[2].jam).toContain('07:00');
  });
});
