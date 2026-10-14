import { invoke as tauriInvoke, isTauri as isTauriRuntime } from '@tauri-apps/api/core';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';

const TOKEN_KEY = 'attendx_token';
const USER_KEY = 'attendx_user';

export function getToken(): string {
  return localStorage.getItem(TOKEN_KEY) ?? '';
}

export function setSession(token: string, username: string, role: string) {
  localStorage.setItem(TOKEN_KEY, token);
  localStorage.setItem(USER_KEY, JSON.stringify({ username, role }));
}

export function clearSession() {
  localStorage.removeItem(TOKEN_KEY);
  localStorage.removeItem(USER_KEY);
}

export function getUser(): { username: string; role: string } | null {
  try {
    const raw = localStorage.getItem(USER_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function isTauri(): boolean {
  try {
    return isTauriRuntime();
  } catch {
    return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  }
}

/** Dialog pilih file bawaan OS (butuh runtime Tauri). */
export async function pickFile(filters: { name: string; extensions: string[] }[]): Promise<string | null> {
  if (!isTauri()) throw new Error('Pemilih file membutuhkan aplikasi desktop AttendX.');
  const sel = await dialogOpen({ multiple: false, filters });
  return typeof sel === 'string' ? sel : null;
}

/** Semua pemanggilan backend lewat Tauri IPC. Di luar Tauri, lempar error yang jelas. */
export async function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (!isTauri()) {
    throw new Error(
      `Perintah "${cmd}" membutuhkan aplikasi desktop AttendX (runtime Tauri tidak ditemukan). Jalankan aplikasi via Tauri, bukan browser biasa.`,
    );
  }
  return tauriInvoke<T>(cmd, args);
}

export const api = {
  hasAdmin: () => invoke<boolean>('has_admin'),
  setupAdmin: (username: string, display_name: string, password: string) =>
    invoke<string>('setup_admin', { username, display_name, password }),
  login: (username: string, password: string) =>
    invoke<{ token: string; username: string; role: string }>('login', { username, password }),
  logout: () => invoke<string>('logout', { token: getToken() }),
  dashboard: (date: string) => invoke<DashboardData>('dashboard', { token: getToken(), date }),
  tableList: (table: string, page: number, perPage: number, search = '', orderBy = '', orderDir = 'asc') =>
    invoke<{ rows: Row[]; total: number }>('table_list', {
      token: getToken(), table, page, perPage, search, orderBy, orderDir,
    }),
  tableGet: (table: string, id: string) => invoke<Row>('table_get', { token: getToken(), table, id }),
  tableCreate: (table: string, data: Record<string, unknown>) =>
    invoke<{ id: string }>('table_create', { token: getToken(), table, data }),
  tableUpdate: (table: string, id: string, data: Record<string, unknown>) =>
    invoke<string>('table_update', { token: getToken(), table, id, data }),
  tableDelete: (table: string, id: string, hard = false) =>
    invoke<string>('table_delete', { token: getToken(), table, id, hard }),
  deviceTest: (deviceId: string) => invoke<unknown>('device_test', { token: getToken(), deviceId }),
  deviceSync: (deviceId: string) => invoke<SyncStats>('device_sync', { token: getToken(), deviceId }),
  deviceRegistry: () => invoke<RegistryEntry[]>('device_registry'),
  deviceSdkStatus: () => invoke<SdkStatus[]>('device_sdk_status'),
  attendanceCompute: (workDate: string, employeeId?: string) =>
    invoke<{ computed: number; date: string }>('attendance_compute', {
      token: getToken(), workDate, employeeId: employeeId ?? null,
    }),
  attendanceReview: () =>
    invoke<{ review: Row[]; unmapped: Row[] }>('attendance_review', { token: getToken() }),
  attendanceCorrect: (attendanceId: string, field: string, newValue: string, reason: string) =>
    invoke<string>('attendance_correct', { token: getToken(), attendanceId, field, newValue, reason }),
  importPreview: (csvText: string) =>
    invoke<{ headers: string[]; rows: ImportRow[]; errors: string[] }>('employee_import_preview', {
      token: getToken(), csvText,
    }),
  importCommit: (rows: Record<string, string>[]) =>
    invoke<{ ok: number; fail: number; messages: string[] }>('employee_import_commit', {
      token: getToken(), rows,
    }),
  reportQuery: (kind: string, f: ReportFilter) =>
    invoke<{ headers: string[]; rows: string[][] }>('report_query', { token: getToken(), kind, ...f }),
  reportExport: (kind: string, format: string, title: string, f: ReportFilter) =>
    invoke<{ format: string; text?: string; base64?: string }>('report_export', {
      token: getToken(), kind, format, title, ...f,
    }),
  backupCreate: () => invoke<{ file: string }>('backup_create', { token: getToken() }),
  backupRestore: (file: string) => invoke<string>('backup_restore', { token: getToken(), file }),
  healthCheck: () => invoke<Health>('health_check', { token: getToken() }),
  settingsGet: () => invoke<Record<string, string | number>>('settings_get', { token: getToken() }),
  settingsSet: (key: string, value: string) =>
    invoke<string>('settings_set', { token: getToken(), key, value }),
  scheduleWeek: (startDate: string) =>
    invoke<ScheduleWeek>('schedule_week', { token: getToken(), startDate }),
  scheduleBulk: (employeeIds: string[], shiftId: string, startDate: string, endDate: string, dryRun: boolean) =>
    invoke<BulkReport>('schedule_bulk', { token: getToken(), employeeIds, shiftId, startDate, endDate, dryRun }),
  scheduleCopyWeek: (srcStart: string, dstStart: string, dryRun: boolean) =>
    invoke<BulkReport>('schedule_copy_week', { token: getToken(), srcStart, dstStart, dryRun }),
  scheduleConflicts: (startDate: string, endDate: string) =>
    invoke<ConflictReport>('schedule_conflicts', { token: getToken(), startDate, endDate }),
  swapRequest: (employeeId: string, fromShiftId: string, toShiftId: string, swapDate: string, reason: string) =>
    invoke<{ id: string }>('swap_request', { token: getToken(), employeeId, fromShiftId, toShiftId, swapDate, reason }),
  swapDecide: (swapId: string, approve: boolean) =>
    invoke<string>('swap_decide', { token: getToken(), swapId, approve }),
  syncRunAll: () => invoke<SyncAllRow[]>('sync_run_all', { token: getToken() }),
  schedulerGet: () => invoke<SchedulerState>('scheduler_get', { token: getToken() }),
  schedulerSet: (enabled: boolean) => invoke<string>('scheduler_set', { token: getToken(), enabled }),
  saveEmployeePhoto: (employeeId: string, srcPath: string) =>
    invoke<{ path: string }>('save_employee_photo', { token: getToken(), employeeId, srcPath }),
  saveLeaveAttachment: (requestId: string, srcPath: string) =>
    invoke<{ path: string }>('save_leave_attachment', { token: getToken(), requestId, srcPath }),
  eventsRequeue: (startDate: string, endDate: string) =>
    invoke<{ requeued: number }>('events_requeue', { token: getToken(), startDate, endDate }),
  deviceFetchUsers: (deviceId: string) =>
    invoke<DeviceUser[]>('device_fetch_users', { token: getToken(), deviceId }),
  deviceProvisionUser: (deviceId: string, deviceUserId: string, name: string) =>
    invoke<string>('device_provision_user', { token: getToken(), deviceId, deviceUserId, name }),
  leaveBalances: () => invoke<LeaveBalance[]>('leave_balances', { token: getToken() }),
  searchGlobal: (query: string) =>
    invoke<{ employees: SearchRow[]; devices: SearchRow[] }>('search_global', { token: getToken(), query }),
  scheduleExport: (startDate: string, endDate: string) =>
    invoke<{ csv: string; count: number }>('schedule_export', { token: getToken(), startDate, endDate }),
  scheduleImportPreview: (csvText: string) =>
    invoke<{ rows: ScheduleImportRow[]; errors: string[] }>('schedule_import_preview', { token: getToken(), csvText }),
  scheduleImportCommit: (rows: Record<string, unknown>[]) =>
    invoke<{ ok: number; fail: number; messages: string[] }>('schedule_import_commit', { token: getToken(), rows }),
  diagnosticsBundle: () => invoke<{ text: string }>('diagnostics_bundle', { token: getToken() }),
};

export type Row = Record<string, string | number | null>;

export interface DashboardData {
  date: string;
  active_employees: number;
  hadir: number;
  terlambat: number;
  tidak_hadir: number;
  cuti_izin: number;
  devices: { total: number; online: number; offline: number; error: number };
  daily: { date: string; hadir: number; terlambat: number; total: number }[];
  recent: { date: string; name: string; in: string | null; out: string | null; status: string }[];
  shifts: { code: string; name: string; count: number }[];
  last_sync: string | null;
  alerts: { id: string; name: string; status: string }[];
}

export interface SyncStats {
  device_id: string;
  fetched: number;
  inserted: number;
  duplicates: number;
  unmapped: number;
  failed: number;
  checkpoint: string;
  attempts: number;
}

export interface RegistryEntry {
  adapter_name: string;
  vendor: string;
  models: string[];
  sdk_version: string;
  dll_arch: string;
  status: string;
  supported_ops: string[];
  missing: string;
}

export interface SdkStatus {
  adapter: string;
  dll: string;
  found: boolean;
  path: string;
  arch: string;
  arch_ok?: boolean;
  registered?: boolean;
  loadable?: boolean;
  hint: string;
}

export interface ImportRow {
  line: number;
  data: Record<string, string>;
  errors: string[];
}

export interface ScheduleImportRow {
  line: number;
  nik: string;
  shift: string;
  eff: string;
  end: string;
  employee_id: string | null;
  shift_id: string | null;
  errors: string[];
}

export interface ReportFilter {
  start: string;
  end: string;
  branchId: string;
  departmentId: string;
  shiftId: string;
  status: string;
  employeeId: string;
}

export interface Health {
  app: string;
  db_version: number;
  integrity: string;
  wib_now: string;
}

export interface ScheduleWeek {
  days: string[];
  rows: { employee_id: string; name: string; cells: Record<string, string | null> }[];
  empty_days: string[];
}

export interface BulkReport {
  created?: number;
  copied?: number;
  skipped: { employee_id?: string; reason?: string }[] | number;
  dry_run: boolean;
}

export interface ConflictReport {
  overlaps: { employee_id: string; name: string; shift_a: string; shift_b: string; since: string }[];
  empty: { date: string; employee_id: string; name: string }[];
}

export interface SyncAllRow {
  device: string;
  ok: boolean;
  fetched?: number;
  inserted?: number;
  duplicates?: number;
  unmapped?: number;
  checkpoint?: string;
  error?: string;
}

export interface SchedulerState {
  enabled: boolean;
  interval_secs: string;
  running: boolean;
}

export interface DeviceUser {
  device_user_id: string;
  name: string;
}

export interface LeaveBalance {
  employee_id: string;
  nik: string;
  name: string;
  leave_code: string;
  leave_name: string;
  quota: number;
  used: number;
  remaining: number;
}

export interface SearchRow {
  kind: string;
  id: string;
  label: string;
  sub: string;
}

export function downloadFile(filename: string, content: string | Uint8Array, mime: string) {
  const blob =
    typeof content === 'string' ? new Blob([content], { type: mime }) : new Blob([content as BlobPart], { type: mime });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

export function base64ToBytes(b64: string): Uint8Array {
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** Format WIB "YYYY-MM-DD HH:MM:SS" -> tampil ringkas. Murni formatting, tanpa konversi zona. */
export function fmtWIB(s: string | number | null | undefined): string {
  if (s === null || s === undefined || s === '') return '-';
  return String(s);
}

export function todayWIB(): string {
  // WIB = UTC+7. Hitung dari UTC agar konsisten di semua zona browser.
  const now = new Date(Date.now() + 7 * 3600 * 1000);
  return now.toISOString().slice(0, 10);
}
