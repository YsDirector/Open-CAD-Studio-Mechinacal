#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
ocr_din5480_tables.py — OCR the DIN 5480-2 nominal dimension tables (odd pages).

Scope of this round: p11 (Table 2, m=0,5), p13 (Table 4, m=0,6), p15 (Table 6, m=0,75).
The script is page-range parameterised and can be re-run / extended to later pages.

Method (no manual number corrections anywhere):
  1. Detect the printed table grid: full-width horizontal rules (row bands) and the
     vertical column separators inside the data region (expects 15 separators = 14 columns).
  2. Paint the grid lines white, crop every data-row band as a strip.
  3. PRIMARY pass: OCR every strip (5x upscale + binarise, psm 7, digit/comma whitelist,
     TSV word boxes) with the legacy engine (--oem 0), which reads the printed decimal
     comma best; bin the words into the 14 known columns by x-overlap with the separators.
  4. RECHECK pass (targeted, not manual): for cells that look doubtful (empty, unparsed,
     a 3+-digit integer without a comma, multi-word, spanning/between columns, or involved
     in a failed row formula) re-OCR the single cell at 6x with legacy and LSTM engines.
     Recheck readings are recorded in the flags; a recheck value replaces the primary only
     when the primary is empty/unparsed or only a separator is missing (same digits).
  5. Sanity checks per page (mark only): d_B strictly increasing, d == m*z, printed
     d_b == trunc(d*cos(30 deg)) within 0.011 mm, x1_m constant and <= 0.45*m.
     The table itself truncates d_b to ~4 significant digits, hence the 0.011 tolerance.
  6. Append rows with source columns page,m,table_no to the CSV; append one stats line
     per page to the stats sidecar file.

Usage examples:
  python3 ocr_din5480_tables.py --pages 11 --show 10          # first page, print 10 rows
  python3 ocr_din5480_tables.py --pages 13 15 --show 10
  python3 ocr_din5480_tables.py --pages 17 --force            # re-extract an existing page
  python3 ocr_din5480_tables.py --pages 11 --dry-run          # OCR only, write nothing

Dependencies: tesseract 5.x with eng + legacy engine (--oem 0), numpy, Pillow.
"""

import argparse
import csv
import math
import os
import re
import subprocess
import sys
import tempfile

import numpy as np
from PIL import Image

SRC_DIR = os.path.expanduser('~/桌面/OCSM/review/花键标准资料')
DEFAULT_IMG = 'DIN5480_p2_20459537_p{page:02d}.png'
DEFAULT_OUT = os.path.join(SRC_DIR, 'DIN5480-2_名义表.csv')
DEFAULT_STATS = os.path.join(SRC_DIR, 'DIN5480-2_名义表_stats.txt')

COLUMNS = ['d_B', 'z', 'd', 'd_b', 'x1_m', 'e2_eq_s1', 'd_f2', 'A_df2',
           'd_Ff2_min', 'd_a2', 'd_a1', 'd_Ff1_max', 'd_f1', 'A_df1']
DECIMAL_COLS = {2, 3, 4, 5, 7, 8, 9, 10, 11, 12, 13}
WHITELIST = '0123456789,.-'
NUM_RE = re.compile(r'^-?\d+(?:[.,]\d+)?$')

CSV_HEADER_COMMENTS = [
    '# DIN 5480-2:2015-03 nominal dimension tables, OCR-extracted from doc88 page images',
    f'# source dir: {SRC_DIR}  files DIN5480_p2_20459537_pNN.png (odd pages only this round: p11/p13/p15)',
    '# OCR: tesseract 5.5.3; printed table grid detected, row strips 5x upscale + binarise, psm 7,',
    '#      char whitelist 0123456789,.-; legacy engine (--oem 0) primary; doubtful cells re-OCR',
    '#      per cell at 6x with legacy and LSTM (--oem 1) engines; readings kept in flags.',
    '# printed decimal separator is a comma; an OCR "." was normalised to ",". No digits were changed by hand.',
    '# flags column: cell:COL:reason / row:reason / page:reason; recheck readings are shown as L=...,S=...',
    '# columns: d_B reference dia; z teeth; d pitch dia; d_b base dia; x1_m shaft addendum modification x1*m;',
    '#          e2_eq_s1 space width / tooth thickness e2=s1; d_f2 hub root dia; A_df2 deviation;',
    '#          d_Ff2_min hub root form dia min; d_a2 hub tip dia; d_a1 shaft tip dia;',
    '#          d_Ff1_max shaft root form dia max; d_f1 shaft root dia; A_df1 deviation',
]
CSV_FIELDS = ['page', 'm', 'table_no'] + COLUMNS + ['flags']

TMPDIR = tempfile.mkdtemp(prefix='ocr_din5480_')
TMP_PNG = os.path.join(TMPDIR, 'ocr.png')


# ----------------------------------------------------------------------------- utils
def eprint(*a):
    print(*a, file=sys.stderr)


def group_consecutive(idx, gap=3):
    if len(idx) == 0:
        return []
    out, s, p = [], idx[0], idx[0]
    for i in idx[1:]:
        if i - p > gap:
            out.append((int(s), int(p)))
            s = i
        p = i
    out.append((int(s), int(p)))
    return out


def detect_grid(dark):
    """Horizontal rules as (y0,y1) runs + vertical separator midpoints in data region."""
    H, W = dark.shape
    h_groups = group_consecutive(np.where(dark.sum(axis=1) > 0.50 * W)[0])
    if len(h_groups) < 4:
        raise RuntimeError(f'only {len(h_groups)} full-width horizontal rules found')
    mid = [(s + e) // 2 for s, e in h_groups]
    y0, y1 = mid[2], mid[-1]
    cov = dark[y0:y1 + 1, :].sum(axis=0)
    v_mids = [(s + e) // 2 for s, e in
              group_consecutive(np.where(cov > 0.85 * (y1 - y0 + 1))[0])]
    if len(v_mids) != 15:
        raise RuntimeError(f'expected 15 vertical separators (14 columns), got '
                           f'{len(v_mids)}: {v_mids}')
    return h_groups, v_mids


def run_tesseract_tsv(png_path, oem, psm='7'):
    cmd = ['tesseract', png_path, 'stdout', '-l', 'eng', '--oem', oem, '--psm', psm,
           '-c', 'tessedit_char_whitelist=' + WHITELIST, 'tsv']
    r = subprocess.run(cmd, capture_output=True, text=True)
    words = []
    for line in r.stdout.splitlines()[1:]:
        p = line.split('\t')
        if len(p) < 12:
            continue
        try:
            conf = float(p[10])
            x, y, w, h = int(p[6]), int(p[7]), int(p[8]), int(p[9])
        except ValueError:
            continue
        txt = p[11].strip()
        if conf < 0 or not txt:
            continue
        words.append({'x': x, 'y': y, 'w': w, 'h': h, 'text': txt, 'conf': conf})
    return words


def tesseract_text(png_path, oem, psm='7'):
    cmd = ['tesseract', png_path, 'stdout', '-l', 'eng', '--oem', oem, '--psm', psm,
           '-c', 'tessedit_char_whitelist=' + WHITELIST]
    r = subprocess.run(cmd, capture_output=True, text=True)
    return ''.join(r.stdout.split())


def preprocess(im, scale, thr=180):
    im = im.resize((im.width * scale, im.height * scale), Image.LANCZOS)
    arr = np.asarray(im)
    return Image.fromarray(np.where(arr > thr, 255, 0).astype(np.uint8))


def ocr_strip(strip, scale, oem):
    """OCR a row strip; return TSV words with x coords in original image pixels."""
    im = preprocess(Image.fromarray(strip), scale)
    im.save(TMP_PNG)
    words = run_tesseract_tsv(TMP_PNG, oem, psm='7')
    max_right = max((w['x'] + w['w'] for w in words), default=0)
    if max_right > im.width * 1.05:
        eprint(f'  ! WARNING: TSV x coords exceed strip width ({max_right}>{im.width})')
    for w in words:
        w['x0'] = w['x'] / scale
        w['x1'] = (w['x'] + w['w']) / scale
        w['xc'] = (w['x0'] + w['x1']) / 2.0
    return words


def assign_columns(words, crop_x0, col_edges, span_ratio=0.30):
    cells = {i: [] for i in range(len(col_edges) - 1)}
    issues = []
    for w in words:
        x0 = crop_x0 + w['x0']
        x1 = crop_x0 + w['x1']
        over = []
        for i in range(len(col_edges) - 1):
            lo, hi = col_edges[i], col_edges[i + 1]
            over.append(max(0.0, min(x1, hi) - max(x0, lo)))
        best = int(np.argmax(over))
        width = max(x1 - x0, 1e-6)
        if over[best] < 0.50 * width:
            issues.append(f"cell:{COLUMNS[best]}:word_between_columns({w['text']})")
        else:
            second = sorted(over, reverse=True)[1] if len(over) > 1 else 0
            if second > span_ratio * width:
                issues.append(f"cell:{COLUMNS[best]}:word_spans_columns({w['text']})")
        cells[best].append(w)
    return cells, issues


def join_cell(words):
    if not words:
        return '', None
    ordered = sorted(words, key=lambda w: w['xc'])
    text = ''.join(w['text'] for w in ordered)
    if len(words) > 1:
        return text, 'multiword(' + '|'.join(w['text'] for w in ordered) + ')'
    return text, None


def normalise_value(raw):
    if re.fullmatch(r'-?\d+\.\d+', raw):
        return raw.replace('.', ',')
    return raw


def digit_key(s):
    return re.sub(r'\D', '', s)


def parse_num(s):
    if not s or not NUM_RE.match(s):
        return None
    try:
        return float(s.replace(',', '.'))
    except ValueError:
        return None


def ocr_cell(work, band, x0, x1, scale, oem, psm='7'):
    """Re-OCR one cell (already grid-cleaned image). Returns joined text."""
    y0, y1 = band
    cell = work[y0 + 2:y1 - 1, x0 + 3:x1 - 2]
    if cell.size == 0:
        return ''
    im = preprocess(Image.fromarray(cell), scale)
    im.save(TMP_PNG)
    return tesseract_text(TMP_PNG, oem, psm=psm)


def header_module_table(gray, page_no, top=0.28):
    im = Image.fromarray(gray[:int(gray.shape[0] * top), :])
    path = os.path.join(TMPDIR, 'head.png')
    im.save(path)
    r = subprocess.run(['tesseract', path, 'stdout', '-l', 'eng'], capture_output=True, text=True)
    txt = r.stdout
    m = re.search(r'Module\s+m\s*=\s*([0-9]+[.,][0-9]+)', txt)
    t = re.search(r'Table\s+(\d+)', txt)
    m_str = m.group(1).replace('.', ',') if m else ''
    t_str = t.group(1) if t else ''
    if not m or not t:
        eprint(f'  ! WARNING page p{page_no}: could not read module/table title '
               f'(m={m_str!r}, table={t_str!r})')
    return m_str, t_str


# ----------------------------------------------------------------------------- page
def ocr_page(page, args):
    img_path = os.path.join(args.img_dir, args.img_template.format(page=page))
    if not os.path.exists(img_path):
        raise FileNotFoundError(img_path)
    gray = np.asarray(Image.open(img_path).convert('L'))
    dark = gray < 160

    h_groups, v_mids = detect_grid(dark)
    h_mid = [(s + e) // 2 for s, e in h_groups]
    data_start, data_end = h_mid[2], h_mid[-1]
    col_edges = v_mids

    work = gray.copy()
    for s, e in h_groups:
        work[max(0, s - 1):e + 2, max(0, col_edges[0] - 4):col_edges[-1] + 5] = 255
    for x in v_mids:
        work[max(0, data_start - 2):data_end + 3, max(0, x - 3):x + 4] = 255

    bands = [(h_mid[i], h_mid[i + 1]) for i in range(2, len(h_mid) - 1)]
    heights = [b[1] - b[0] for b in bands]
    med_h = float(np.median(heights)) if heights else 0.0
    kept, dropped_tall = [], []
    for b in bands:
        (kept if (b[1] - b[0]) <= 1.6 * med_h else dropped_tall).append(b)

    m_str, table_no = header_module_table(gray, page)
    m_val = parse_num(m_str)
    crop_x0 = col_edges[0] + 3
    crop_x1 = col_edges[-1] - 2

    # ---- primary pass: legacy engine on every row strip
    rows, page_issues = [], []
    for band in kept:
        strip = work[band[0] + 2:band[1] - 1, crop_x0:crop_x1]
        if strip.size == 0:
            continue
        words = ocr_strip(strip, args.scale, '0')
        cells, assign_issues = assign_columns(words, crop_x0, col_edges)
        values, word_issues = [], []
        for ci in range(len(COLUMNS)):
            raw, iss = join_cell(cells[ci])
            raw = raw.replace(' ', '')
            values.append(normalise_value(raw))
            word_issues.append(iss)
        n_num = sum(1 for v in values if parse_num(v) is not None)
        rows.append({'band': band, 'values': values, 'word_issues': word_issues,
                     'assign_issues': list(assign_issues), 'n_num': n_num})

    while rows and rows[-1]['n_num'] < 6:
        b = rows.pop()['band']
        page_issues.append(f'non-data band at y={b[0]}..{b[1]} dropped (numeric cells<6)')
    for b in dropped_tall:
        page_issues.append(f'band at y={b[0]}..{b[1]} dropped (height>{1.6 * med_h:.0f}px)')

    def parsed(col_idx):
        return [parse_num(r['values'][col_idx]) for r in rows]

    # ---- choose doubtful cells (targeting only; final flags are computed afterwards)
    pre_bad = set()
    z_pre, d_pre, db_pre = parsed(1), parsed(2), parsed(3)
    for i, r in enumerate(rows):
        if m_val is not None and d_pre[i] is not None and z_pre[i] is not None \
                and abs(d_pre[i] - m_val * z_pre[i]) > 5e-4:
            pre_bad.add(i)
        if d_pre[i] is not None and db_pre[i] is not None \
                and abs(db_pre[i] - d_pre[i] * math.cos(math.radians(30))) > 0.011:
            pre_bad.add(i)

    # ---- targeted per-cell recheck (legacy + LSTM on the isolated cell)
    recheck_log = {}
    recheck_count = 0
    for i, r in enumerate(rows):
        for ci in range(len(COLUMNS)):
            v = r['values'][ci]
            need = (not v) or (parse_num(v) is None) \
                or (ci in DECIMAL_COLS and re.fullmatch(r'-?\d{3,}', v)) \
                or bool(r['word_issues'][ci]) \
                or (i in pre_bad and ci in (2, 3))
            if not need:
                continue
            x0, x1 = col_edges[ci], col_edges[ci + 1]
            L = ocr_cell(work, r['band'], x0, x1, 6, '0', '7')
            S = ocr_cell(work, r['band'], x0, x1, 6, '1', '7')
            if not L and not S:
                L = ocr_cell(work, r['band'], x0, x1, 6, '0', '6')
            Ln, Sn = normalise_value(L), normalise_value(S)
            recheck_count += 1
            recheck_log[(i, ci)] = f'recheck(L={Ln or "-"}|S={Sn or "-"})'
            # accept a recheck reading only for empty/unparsed primary, or the same
            # digits with a comma where the primary lost the separator
            for cand in (Ln, Sn):
                if not cand:
                    continue
                if not v or parse_num(v) is None:
                    if parse_num(cand) is not None:
                        r['values'][ci] = cand
                        recheck_log[(i, ci)] += f' -> value_from_recheck({v or "empty"} -> {cand})'
                        break
                elif re.fullmatch(r'-?\d{3,}', v) and digit_key(cand) == digit_key(v) \
                        and ',' in cand:
                    r['values'][ci] = cand
                    recheck_log[(i, ci)] += f' -> value_from_recheck({v} -> {cand})'
                    break

    # ---- final parsed values and checks (mark only)
    z_vals, d_vals, db_vals = parsed(1), parsed(2), parsed(3)
    xm_vals, dB_vals = parsed(4), parsed(0)
    for i, r in enumerate(rows):
        flags = list(r['assign_issues'])
        for ci in range(len(COLUMNS)):
            v = r['values'][ci]
            name = COLUMNS[ci]
            if r['word_issues'][ci]:
                flags.append(f"cell:{name}:{r['word_issues'][ci]}")
            if not v:
                flags.append(f'cell:{name}:empty')
            elif parse_num(v) is None:
                flags.append(f'cell:{name}:unparsed({v})')
            elif ci in DECIMAL_COLS and re.fullmatch(r'-?\d{3,}', v):
                flags.append(f'cell:{name}:maybe_missing_separator({v})')
            if (i, ci) in recheck_log:
                flags.append(f"cell:{name}:{recheck_log[(i, ci)]}")
        if m_val is not None and d_vals[i] is not None and z_vals[i] is not None \
                and abs(d_vals[i] - m_val * z_vals[i]) > 5e-4:
            flags.append('row:d!=m*z')
        if d_vals[i] is not None and db_vals[i] is not None \
                and abs(db_vals[i] - d_vals[i] * math.cos(math.radians(30))) > 0.011:
            flags.append('row:d_b!=d*cos30')
        if i > 0 and dB_vals[i] is not None and dB_vals[i - 1] is not None \
                and dB_vals[i] <= dB_vals[i - 1]:
            flags.append('row:d_B_not_increasing')
        r['flags'] = flags

    xm_known = [v for v in xm_vals if v is not None]
    xm_const = len(xm_known) > 0 and (max(xm_known) - min(xm_known)) < 1e-9
    xm_limit_ok = (max(xm_known) <= 0.45 * m_val + 5e-3) if (m_val is not None and xm_known) else None
    # x1_m constancy is a page-level property (DIN tables for m>0,5 vary it row by row),
    # so it is reported in the stats/notes, not as a per-row CSV flag.
    if xm_known and not xm_const:
        ratios = [v / m_val for v in xm_known] if m_val else []
        page_issues.append('x1_m is NOT constant on this page (genuine table content): values '
                           + ','.join(f'{v:g}' for v in xm_known)
                           + ('; x1=x1_m/m in [' + ','.join(f'{r:g}' for r in ratios) + ']'
                              if ratios else ''))

    stats = {
        'page': page, 'table': table_no, 'm': m_str, 'rows': len(rows),
        'dropped_bands': len(dropped_tall) + (len(kept) - len(rows)),
        'empty_cells': sum(1 for r in rows for v in r['values'] if not v),
        'flagged_rows': sum(1 for r in rows if r['flags']),
        'unparsed': sum(1 for r in rows for f in r['flags'] if 'unparsed' in f),
        'missing_sep': sum(1 for r in rows for f in r['flags'] if 'maybe_missing_separator' in f),
        'multiword': sum(1 for r in rows for f in r['flags'] if 'multiword' in f),
        'rechecked': recheck_count,
        'recheck_diffs': sum(1 for r in rows for f in r['flags']
                             if 'recheck(' in f and 'value_from_recheck' not in f),
        'd_ne_mz': sum(1 for r in rows if 'row:d!=m*z' in r['flags']),
        'db_bad': sum(1 for r in rows if 'row:d_b!=d*cos30' in r['flags']),
        'dB_inc': all(v is None or (i == 0 or dB_vals[i - 1] is None or v > dB_vals[i - 1])
                      for i, v in enumerate(dB_vals)),
        'xm_values': sorted(set(xm_known)), 'xm_const': xm_const, 'xm_leq_045m': xm_limit_ok,
        'notes': page_issues,
    }
    return rows, stats


# ----------------------------------------------------------------------------- io
def read_existing_pages(out_path):
    pages = set()
    if not os.path.exists(out_path):
        return pages
    with open(out_path, newline='', encoding='utf-8') as f:
        for line in f:
            if line.startswith('#') or line.startswith('page,'):
                continue
            first = line.split(',', 1)[0]
            if first.strip().isdigit():
                pages.add(int(first.strip()))
    return pages


def write_csv(out_path, rows_by_page):
    new_file = not os.path.exists(out_path)
    with open(out_path, 'a', newline='', encoding='utf-8') as f:
        w = csv.writer(f, lineterminator='\n')
        if new_file:
            for c in CSV_HEADER_COMMENTS:
                f.write(c + '\n')
            w.writerow(CSV_FIELDS)
        for page, rows, m_str, table_no in rows_by_page:
            for r in rows:
                w.writerow([page, m_str, table_no] + r['values'] + ['; '.join(r['flags'])])


def stats_line(st):
    xm = ('const ' + ','.join(f'{v:g}' for v in st['xm_values'])) if st['xm_const'] \
        else ('NOT-const ' + ','.join(f'{v:g}' for v in st['xm_values']))
    return (f"page=p{st['page']} table={st['table'] or '?'} m={st['m'] or '?'} rows={st['rows']} "
            f"dropped_bands={st['dropped_bands']} empty_cells={st['empty_cells']} "
            f"flagged_rows={st['flagged_rows']} unparsed={st['unparsed']} missing_sep={st['missing_sep']} "
            f"multiword={st['multiword']} rechecked={st['rechecked']} recheck_diffs={st['recheck_diffs']} "
            f"d!=m*z={st['d_ne_mz']} d_b_bad={st['db_bad']} d_B_increasing={st['dB_inc']} "
            f"x1_m=({xm}) x1_m<=0.45m={st['xm_leq_045m']}")


# ----------------------------------------------------------------------------- main
def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--pages', type=int, nargs='+', required=True)
    ap.add_argument('--img-dir', default=SRC_DIR)
    ap.add_argument('--img-template', default=DEFAULT_IMG)
    ap.add_argument('--out', default=DEFAULT_OUT)
    ap.add_argument('--stats', default=DEFAULT_STATS)
    ap.add_argument('--scale', type=int, default=5)
    ap.add_argument('--show', type=int, default=0, help='print first N rows per page')
    ap.add_argument('--force', action='store_true', help='re-extract pages already in the CSV')
    ap.add_argument('--dry-run', action='store_true')
    args = ap.parse_args()

    existing = read_existing_pages(args.out) if not args.force else set()
    if existing:
        print(f'pages already present in CSV (skipped unless --force): {sorted(existing)}')

    results = []
    for page in args.pages:
        if page in existing:
            print(f'p{page}: already covered -> skipped')
            continue
        print(f'p{page}: OCR ...')
        rows, stats = ocr_page(page, args)
        print('  ' + stats_line(stats))
        for n in stats['notes']:
            print('  note: ' + n)
        if args.show:
            print(f'  first {min(args.show, len(rows))} rows:')
            print('  ' + ' | '.join(['page'] + COLUMNS + ['flags']))
            for r in rows[:args.show]:
                print('  ' + str(page) + ' | ' + ' | '.join(r['values']) + ' | ' +
                      '; '.join(r['flags']))
            print('  last 3 rows:')
            for r in rows[-3:]:
                print('  ' + str(page) + ' | ' + ' | '.join(r['values']) + ' | ' +
                      '; '.join(r['flags']))
        results.append((page, rows, stats))

    if args.dry_run or not results:
        print('dry-run: nothing written')
        return 0

    write_csv(args.out, [(p, r, st['m'], st['table']) for p, r, st in results])
    print(f'CSV appended: {args.out}')
    with open(args.stats, 'a', encoding='utf-8') as f:
        for _, _, st in results:
            f.write(stats_line(st) + '\n')
    print(f'stats appended: {args.stats}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
