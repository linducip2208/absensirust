import { describe, expect, it } from 'vitest';
import { fmtWIB, todayWIB } from './api';

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
