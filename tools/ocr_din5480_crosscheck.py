#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
ocr_din5480_crosscheck.py — compare two OCR extractions of the same DIN 5480-2
nominal tables (e.g. the TSV per-row extraction vs. the parallel column-strip
extraction). Rows are matched by (page, row order); cells are compared as numbers.
Mismatches are printed and can be written to a file.

Usage:
  python3 ocr_din5480_crosscheck.py A.csv B.csv [--out diff.txt] [--max-show 40]
"""
import argparse
import csv
import os
import re
import sys

VALUE_COLS = ['d_B', 'z', 'd', 'd_b', 'x1_m', 'e2_eq_s1', 'd_f2', 'A_df2',
              'd_Ff2_min', 'd_a2', 'd_a1', 'd_Ff1_max', 'd_f1', 'A_df1']
ALIASES = {'x_m': 'x1_m', 'table_no': 'table', 'm': 'm', 'page': 'page'}


def read_rows(path):
    rows = []
    with open(path, newline='', encoding='utf-8') as f:
        header = None
        for line in f:
            line = line.rstrip('\n')
            if not line or line.startswith('#'):
                continue
            parts = next(csv.reader([line]))
            if header is None:
                header = [ALIASES.get(p.strip(), p.strip()) for p in parts]
                continue
            rows.append(dict(zip(header, parts)))
    return header, rows


def num(v):
    if v is None:
        return None
    v = v.strip()
    if not v or not re.fullmatch(r'-?\d+(?:[.,]\d+)?', v):
        return None
    return float(v.replace(',', '.'))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('a')
    ap.add_argument('b')
    ap.add_argument('--out', default=None)
    ap.add_argument('--max-show', type=int, default=40)
    args = ap.parse_args()

    ha, ra = read_rows(args.a)
    hb, rb = read_rows(args.b)
    print(f'A: {args.a}\n   {len(ra)} rows, header {ha}')
    print(f'B: {args.b}\n   {len(rb)} rows, header {hb}')

    by_page_a, by_page_b = {}, {}
    for r in ra:
        by_page_a.setdefault(r.get('page'), []).append(r)
    for r in rb:
        by_page_b.setdefault(r.get('page'), []).append(r)

    lines = []
    total_cells = total_num = diffs = 0
    for page in sorted(set(by_page_a) | set(by_page_b), key=lambda x: int(x) if str(x).isdigit() else 0):
        A = by_page_a.get(page, [])
        B = by_page_b.get(page, [])
        if len(A) != len(B):
            lines.append(f'p{page}: ROW COUNT differs A={len(A)} B={len(B)}')
        nz = nz_eq = 0
        for i, (a, b) in enumerate(zip(A, B)):
            for col in VALUE_COLS:
                va, vb = a.get(col), b.get(col)
                total_cells += 1
                na, nb = num(va), num(vb)
                if na is None and nb is None:
                    continue
                if na is None or nb is None:
                    nz += 1
                    lines.append(f'p{page} row{i+1} {col}: A={va!r} B={vb!r} (numeric/empty mismatch)')
                    continue
                total_num += 1
                if abs(na - nb) > 1e-9:
                    nz += 1
                    lines.append(f'p{page} row{i+1} {col}: A={va} B={vb} (diff={na - nb:+.6g})')
                else:
                    nz_eq += 1
        diffs += nz
        print(f'p{page}: rows A={len(A)} B={len(B)}, cell pairs={len(A) * len(VALUE_COLS)}, '
              f'numeric agree={nz_eq}, mismatches={nz}')

    print(f'\nTOTAL: {len(ra)} x {len(rb)} rows, {total_cells} cell pairs, '
          f'{total_num} numeric pairs, {diffs} mismatches')
    if lines:
        print('\n'.join(lines[:args.max_show]))
        if len(lines) > args.max_show:
            print(f'... and {len(lines) - args.max_show} more')
    if args.out:
        with open(args.out, 'w', encoding='utf-8') as f:
            f.write(f'# cross-check {os.path.basename(args.a)} vs {os.path.basename(args.b)}\n')
            f.write(f'# total cell pairs={total_cells}, numeric pairs={total_num}, mismatches={diffs}\n')
            f.write('\n'.join(lines) + '\n')
        print(f'diff written: {args.out}')
    return 0 if diffs == 0 else 1


if __name__ == '__main__':
    sys.exit(main())
